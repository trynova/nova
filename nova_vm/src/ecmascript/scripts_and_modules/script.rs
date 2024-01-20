use crate::{
    ecmascript::{
        execution::{
            agent::ExceptionType, Agent, ECMAScriptCode, EnvironmentIndex, ExecutionContext,
            GlobalEnvironmentIndex, JsResult, RealmIdentifier,
        },
        scripts_and_modules::ScriptOrModule,
        types::Value, syntax_directed_operations::scope_analysis::{LexicallyDeclaredNames, script_lexically_declared_names, script_var_declared_names},
    },
    engine::{Executable, Vm},
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPatternKind, Declaration, Program, Statement};
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use std::{any::Any, marker::PhantomData};

pub type HostDefined = &'static mut dyn Any;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ScriptIdentifier(u32, PhantomData<Script>);

impl ScriptIdentifier {
    /// Creates a script identififer from a usize.
    ///
    /// ## Panics
    /// If the given index is greater than `u32::MAX`.
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self(value as u32, PhantomData)
    }

    /// Creates a module identififer from a u32.
    pub(crate) const fn from_u32(value: u32) -> Self {
        Self(value, PhantomData)
    }

    pub(crate) fn last(scripts: &Vec<Option<Script>>) -> Self {
        let index = scripts.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn into_u32(self) -> u32 {
        self.0
    }
}

/// ### [16.1.4 Script Records](https://tc39.es/ecma262/#sec-script-records)
///
/// A Script Record encapsulates information about a script being evaluated.
#[derive(Debug)]
pub struct Script {
    /// ### \[\[Realm]]
    ///
    /// The realm within which this script was created. undefined if not yet
    /// assigned.
    // TODO: This should be able to be undefined sometimes.
    pub(crate) realm: RealmIdentifier,

    /// ### \[\[ECMAScriptCode]]
    ///
    /// The result of parsing the source text of this script.
    pub(crate) ecmascript_code: Program<'static>,

    /// ### \[\[LoadedModules]]
    ///
    /// A map from the specifier strings imported by this script to the resolved
    /// Module Record. The list does not contain two different Records with the
    /// same \[\[Specifier]].
    pub(crate) loaded_modules: (),

    /// ### \[\[HostDefined]]
    ///
    /// Field reserved for use by host environments that need to associate
    /// additional information with a script.
    pub(crate) host_defined: Option<HostDefined>,

    /// Source text of the script
    ///
    /// Parsing a script takes ownership of the text.
    source_text: Box<str>,
}

unsafe impl Send for Script {}

pub type ScriptOrErrors = Result<Script, Vec<oxc_diagnostics::Error>>;

/// ### [16.1.5 ParseScript ( sourceText, realm, hostDefined )](https://tc39.es/ecma262/#sec-parse-script)
///
/// The abstract operation ParseScript takes arguments sourceText (ECMAScript
/// source text), realm (a Realm Record or undefined), and hostDefined
/// (anything) and returns a Script Record or a non-empty List of SyntaxError
/// objects. It creates a Script Record based upon the result of parsing
/// sourceText as a Script.
pub fn parse_script(
    allocator: &Allocator,
    source_text: Box<str>,
    realm: RealmIdentifier,
    host_defined: Option<HostDefined>,
) -> ScriptOrErrors {
    // 1. Let script be ParseText(sourceText, Script).
    let parser = Parser::new(allocator, &source_text, SourceType::default());
    let ParserReturn {
        errors, program, ..
    } = parser.parse();

    // 2. If script is a List of errors, return script.
    if !errors.is_empty() {
        return Err(errors);
    }

    // 3. Return Script Record {
    Ok(Script {
        // [[Realm]]: realm,
        realm,
        // [[ECMAScriptCode]]: script,
        // SAFETY: Program retains at least a logical connection to `source_text`, possibly even
        // direct references. This should be safe because we move the `source_text` into the Script
        // struct, making it self-referential. Hence we must use the 'static lifetime.
        ecmascript_code: unsafe { std::mem::transmute::<Program<'_>, Program<'static>>(program) },
        // [[LoadedModules]]: « »,
        loaded_modules: (),
        // [[HostDefined]]: hostDefined,
        host_defined,
        source_text,
    })
    // }
}

/// ### [16.1.6 ScriptEvaluation ( scriptRecord )](https://tc39.es/ecma262/#sec-runtime-semantics-scriptevaluation)
///
/// The abstract operation ScriptEvaluation takes argument scriptRecord (a
/// Script Record) and returns either a normal completion containing an
/// ECMAScript language value or an abrupt completion.
pub fn script_evaluation(agent: &mut Agent, script: Script) -> JsResult<Value> {
    let realm_id = script.realm;
    let script = agent.heap.add_script(script);
    let realm = agent.get_realm(realm_id);

    // 1. Let globalEnv be scriptRecord.[[Realm]].[[GlobalEnv]].
    let global_env = realm.global_env;

    // 2. Let scriptContext be a new ECMAScript code execution context.
    let script_context = ExecutionContext {
        // 3. Set the Function of scriptContext to null.
        function: None,

        // 4. Set the Realm of scriptContext to scriptRecord.[[Realm]].
        realm: realm_id,

        // 5. Set the ScriptOrModule of scriptContext to scriptRecord.
        script_or_module: Some(ScriptOrModule::Script(script)),

        ecmascript_code: Some(ECMAScriptCode {
            // 6. Set the VariableEnvironment of scriptContext to globalEnv.
            variable_environment: EnvironmentIndex::Global(global_env.unwrap()),

            // 7. Set the LexicalEnvironment of scriptContext to globalEnv.
            lexical_environment: EnvironmentIndex::Global(global_env.unwrap()),

            // 8. Set the PrivateEnvironment of scriptContext to null.
            private_environment: None,
        }),
    };

    // TODO: 9. Suspend the running execution context.

    // 10. Push scriptContext onto the execution context stack; scriptContext is now the running execution context.
    agent.execution_context_stack.push(script_context);

    // 11. Let script be scriptRecord.[[ECMAScriptCode]].
    // NOTE: We cannot define the script here due to reference safety.

    // 12. Let result be Completion(GlobalDeclarationInstantiation(script, globalEnv)).
    global_declaration_instantiation(agent, script, global_env.unwrap())?;

    // TODO: Follow step 13.
    // 13. If result.[[Type]] is normal, then
    //     a. Set result to Completion(Evaluation of script).
    //     b. If result.[[Type]] is normal and result.[[Value]] is empty, then
    //        i. Set result to NormalCompletion(undefined).
    let exe = Executable::compile(agent, script);
    let result = Vm::execute(agent, &exe)?;

    // 14. Suspend scriptContext and remove it from the execution context stack.
    _ = agent.execution_context_stack.pop();

    // TODO: 15. Assert: The execution context stack is not empty.
    // debug_assert!(!agent.execution_context_stack.is_empty());

    // 16. Resume the context that is now on the top of the execution context stack as the
    //     running execution context.
    // NOTE: This is done automatically.

    // 17. Return ? result.
    Ok(result)
}

/// ### [16.1.7 GlobalDeclarationInstantiation ( script, env )](https://tc39.es/ecma262/#sec-globaldeclarationinstantiation)
///
/// The abstract operation GlobalDeclarationInstantiation takes arguments script
/// (a Script Parse Node) and env (a Global Environment Record) and returns
/// either a normal completion containing UNUSED or a throw completion. script
/// is the Script for which the execution context is being established. env is
/// the global environment in which bindings are to be created.
pub(crate) fn global_declaration_instantiation(
    agent: &mut Agent,
    script: ScriptIdentifier,
    env_index: GlobalEnvironmentIndex,
) -> JsResult<()> {
    // 11. Let script be scriptRecord.[[ECMAScriptCode]].
    let script = agent.heap.get_script(script);
    let env = agent.heap.environments.get_global_environment(env_index);

    // 1. Let lexNames be the LexicallyDeclaredNames of script.
    let lex_names = script_lexically_declared_names(&script.ecmascript_code);
    // 2. Let varNames be the VarDeclaredNames of script.
    let var_names = script_var_declared_names(&script.ecmascript_code);

    // 3. For each element name of lexNames, do
    for name in lex_names {
        if
        // a. If env.HasVarDeclaration(name) is true, throw a SyntaxError exception.
        env.has_var_declaration(&name)
            // b. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
            || env.has_lexical_declaration(&name)
            // c. Let hasRestrictedGlobal be ? env.HasRestrictedGlobalProperty(name).
            // d. If hasRestrictedGlobal is true, throw a SyntaxError exception.
            || env.has_restricted_global_property(&name)
        {
            return Err(agent.throw_exception(
                ExceptionType::SyntaxError,
                "Variable already defined.",
            ));
        }
        
    }

    // 4. For each element name of varNames, do
    for name in var_names {
        // a. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
        if env.has_lexical_declaration(&name) {
            return Err(agent.throw_exception(
                ExceptionType::SyntaxError,
                "Variable already defined.",
            ));
        }
    }

    // 5. Let varDeclarations be the VarScopedDeclarations of script.
    let var_declarations = script_var_scoped_declarations

    // TODO: Remove this once steps 5-17 are implemented.
    let env = agent
        .heap
        .environments
        .get_global_environment_mut(env_index);

    for var_name in var_names {
        eprintln!("var name: {:?}", var_name);
        env.declarative_record
            .create_mutable_binding(var_name.clone(), false);
        env.declarative_record
            .initialize_binding(&var_name, Value::Undefined);
    }

    // TODO: Finish this.
    // 6. Let functionsToInitialize be a new empty List.
    // 7. Let declaredFunctionNames be a new empty List.
    // 8. For each element d of varDeclarations, in reverse List order, do
    {
        // a. If d is not either a VariableDeclaration, a ForBinding, or a BindingIdentifier, then
        {
            // i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
            // ii. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
            // iii. Let fn be the sole element of the BoundNames of d.
            // iv. If declaredFunctionNames does not contain fn, then
            {

                // 1. Let fnDefinable be ? env.CanDeclareGlobalFunction(fn).
                // 2. If fnDefinable is false, throw a TypeError exception.
                // 3. Append fn to declaredFunctionNames.
                // 4. Insert d as the first element of functionsToInitialize.
            }
        }
    }
    // 9. Let declaredVarNames be a new empty List.
    // 10. For each element d of varDeclarations, do
    {
        // a. If d is either a VariableDeclaration, a ForBinding, or a BindingIdentifier, then
        {
            // i. For each String vn of the BoundNames of d, do
        }
        // 1. If declaredFunctionNames does not contain vn, then
        {
            // a. Let vnDefinable be ? env.CanDeclareGlobalVar(vn).
            // b. If vnDefinable is false, throw a TypeError exception.
            // c. If declaredVarNames does not contain vn, then
            {
                // i. Append vn to declaredVarNames.
            }
        }
    }
    // 11. NOTE: No abnormal terminations occur after this algorithm step if the
    //     global object is an ordinary object. However, if the global object is
    //     a Proxy exotic object it may exhibit behaviours that cause abnormal
    //     terminations in some of the following steps.
    // 12. NOTE: Annex B.3.2.2 adds additional steps at this point.
    // 13. Let lexDeclarations be the LexicallyScopedDeclarations of script.
    // 14. Let privateEnv be null.
    // 15. For each element d of lexDeclarations, do
    {
        // a. NOTE: Lexically declared names are only instantiated here but not initialized.
        // b. For each element dn of the BoundNames of d, do
        {
            // i. If IsConstantDeclaration of d is true, then
            {
                // 1. Perform ? env.CreateImmutableBinding(dn, true).
            }
            // ii. Else,
            {
                // 1. Perform ? env.CreateMutableBinding(dn, false).
            }
        }
    }
    // 16. For each Parse Node f of functionsToInitialize, do
    {
        // a. Let fn be the sole element of the BoundNames of f.
        // b. Let fo be InstantiateFunctionObject of f with arguments env and privateEnv.
        // c. Perform ? env.CreateGlobalFunctionBinding(fn, fo, false).
    }
    // 17. For each String vn of declaredVarNames, do
    {
        // a. Perform ? env.CreateGlobalVarBinding(vn, false).
    }
    // 18. Return UNUSED.
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::ecmascript::{
        execution::{
            agent::Options, create_realm, set_realm_global_object, Agent, DefaultHostHooks,
        },
        scripts_and_modules::script::{parse_script, script_evaluation},
        types::{InternalMethods, Object, PropertyKey, Value},
    };
    use oxc_allocator::Allocator;

    #[test]
    fn empty_script() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn basic_constants() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "true".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, true.into());
    }

    #[test]
    fn unary_minus() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "-2".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (-2).into());
    }

    #[test]
    fn unary_void() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "void (2 + 2 + 6)".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn unary_typeof() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "typeof undefined".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_str(&mut agent.heap, "undefined"));

        let script = parse_script(&allocator, "typeof null".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_str(&mut agent.heap, "object"));

        let script = parse_script(&allocator, "typeof \"string\"".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_str(&mut agent.heap, "string"));

        // let script = parse_script(&allocator, "typeof Symbol()".into(), realm, None).unwrap();
        // let result = script_evaluation(&mut agent, script).unwrap();
        // assert_eq!(result, Value::from_str(&mut agent.heap, "symbol"));

        let script = parse_script(&allocator, "typeof true".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_str(&mut agent.heap, "boolean"));

        let script = parse_script(&allocator, "typeof 3".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_str(&mut agent.heap, "number"));

        let script = parse_script(&allocator, "typeof 3n".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_str(&mut agent.heap, "bigint"));

        // let script = parse_script(&allocator, "typeof {}".into(), realm, None).unwrap();
        // let result = script_evaluation(&mut agent, script).unwrap();
        // assert_eq!(result, Value::from_str(&mut agent.heap, "object"));

        // let script = parse_script(&allocator, "typeof () => {}".into(), realm, None).unwrap();
        // let result = script_evaluation(&mut agent, script).unwrap();
        // assert_eq!(result, Value::from_str(&mut agent.heap, "function"));
    }

    #[test]
    fn binary_add() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "2 + 2 + 6".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (10).into());
    }

    #[test]
    fn var_assign() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "var foo = 3;".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn empty_object() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "var foo = {};".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_object());
    }

    #[test]
    fn non_empty_object() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "var foo = { a: 3 };".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_object());
        let result = Object::try_from(result).unwrap();
        let key = PropertyKey::from_str(&mut agent.heap, "a");
        assert!(result.has_property(&mut agent, key).unwrap());
        assert_eq!(
            result
                .get_own_property(&mut agent, key)
                .unwrap()
                .unwrap()
                .value,
            Some(Value::from(3))
        );
    }

    #[test]
    fn empty_function() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "function() {}".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_function());
    }

    #[test]
    fn empty_iife_function_call() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "(function() {})()".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn empty_named_function_call() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(
            &allocator,
            "var f = function() {}; f();".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
    }
}
