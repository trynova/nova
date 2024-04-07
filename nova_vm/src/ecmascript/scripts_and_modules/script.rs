use crate::{
    ecmascript::{
        execution::{
            agent::ExceptionType, Agent, ECMAScriptCode, EnvironmentIndex, ExecutionContext,
            GlobalEnvironmentIndex, JsResult, RealmIdentifier,
        },
        scripts_and_modules::ScriptOrModule,
        syntax_directed_operations::{
            miscellaneous::instantiate_function_object,
            scope_analysis::{
                script_lexically_declared_names, script_lexically_scoped_declarations,
                script_var_declared_names, script_var_scoped_declarations,
                LexicallyScopedDeclaration, VarScopedDeclaration,
            },
        },
        types::{IntoValue, Value},
    },
    engine::{Executable, Vm},
};
use oxc_allocator::Allocator;
use oxc_ast::{
    ast::{BindingIdentifier, Program, VariableDeclarationKind},
    syntax_directed_operations::BoundNames,
};
use oxc_parser::{Parser, ParserReturn};
use oxc_span::{Atom, SourceType};
use std::{any::Any, collections::HashSet, marker::PhantomData};

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

    pub(crate) fn last(scripts: &[Option<Script>]) -> Self {
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
    /// A map from the specifier strings imported by this script to the
    /// resolved Module Record. The list does not contain two different Records
    /// with the same \[\[Specifier]].
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
    let result = global_declaration_instantiation(agent, script, global_env.unwrap());

    // 13. If result.[[Type]] is normal, then
    let result: JsResult<Value> = if result.is_ok() {
        let exe = Executable::compile_script(agent, script);
        // a. Set result to Completion(Evaluation of script).
        let result = Vm::execute(agent, &exe);
        // b. If result.[[Type]] is normal and result.[[Value]] is empty, then
        if let Ok(result) = result {
            if let Some(result) = result {
                Ok(result)
            } else {
                // i. Set result to NormalCompletion(undefined).
                Ok(Value::Undefined)
            }
        } else {
            Err(result.err().unwrap())
        }
    } else {
        Err(result.err().unwrap())
    };

    // 14. Suspend scriptContext and remove it from the execution context stack.
    _ = agent.execution_context_stack.pop();

    // TODO: 15. Assert: The execution context stack is not empty.
    // This is not currently true as we do not push an "empty" context stack to the root before running script evaluation.
    // debug_assert!(!agent.execution_context_stack.is_empty());

    // 16. Resume the context that is now on the top of the execution context stack as the
    //     running execution context.
    // NOTE: This is done automatically.

    // 17. Return ? result.
    result
}

/// ### [16.1.7 GlobalDeclarationInstantiation ( script, env )](https://tc39.es/ecma262/#sec-globaldeclarationinstantiation)
///
/// The abstract operation GlobalDeclarationInstantiation takes arguments
/// script (a Script Parse Node) and env (a Global Environment Record) and
/// returns either a normal completion containing UNUSED or a throw completion.
/// script is the Script for which the execution context is being established.
/// env is the global environment in which bindings are to be created.
pub(crate) fn global_declaration_instantiation(
    agent: &mut Agent,
    script: ScriptIdentifier,
    env: GlobalEnvironmentIndex,
) -> JsResult<()> {
    // 11. Let script be scriptRecord.[[ECMAScriptCode]].
    let Script {
        ecmascript_code: script,
        ..
    } = agent.heap.get_script(script);
    // SAFETY: Analysing the script cannot cause the environment to move even though we change other parts of the Heap.

    // 1. Let lexNames be the LexicallyDeclaredNames of script.
    let lex_names = script_lexically_declared_names(script);
    // 2. Let varNames be the VarDeclaredNames of script.
    let var_names = script_var_declared_names(script);

    // 5. Let varDeclarations be the VarScopedDeclarations of script.
    let var_declarations = {
        // SAFETY: The borrow of Program is valid for the duration of this
        // block; the contents of Program are guaranteed to be valid for as
        // long as the Script is alive in the heap as they are not reallocated.
        // Thus in effect VarScopedDeclaration<'_> is valid for the duration
        // of the global_declaration_instantiation call.
        let script =
            unsafe { std::mem::transmute::<&Program<'_>, &'static Program<'static>>(script) };
        script_var_scoped_declarations(script)
    };
    // 13. Let lexDeclarations be the LexicallyScopedDeclarations of script.
    let lex_declarations = {
        // SAFETY: As above, Program is valid in this block, declarations are
        // valid for the duration of global_declaration_instantiation call.
        let script =
            unsafe { std::mem::transmute::<&Program<'_>, &'static Program<'static>>(script) };
        script_lexically_scoped_declarations(script)
    };

    // 3. For each element name of lexNames, do
    for name in lex_names {
        if
        // a. If env.HasVarDeclaration(name) is true, throw a SyntaxError exception.
        env.has_var_declaration(agent, &name)
            // b. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
            || env.has_lexical_declaration(agent, &name)
            // c. Let hasRestrictedGlobal be ? env.HasRestrictedGlobalProperty(name).
            // d. If hasRestrictedGlobal is true, throw a SyntaxError exception.
            || env.has_restricted_global_property(agent, &name)?
        {
            return Err(
                agent.throw_exception(ExceptionType::SyntaxError, "Variable already defined.")
            );
        }
    }

    // 4. For each element name of varNames, do
    for name in &var_names {
        // a. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
        if env.has_lexical_declaration(agent, name) {
            return Err(
                agent.throw_exception(ExceptionType::SyntaxError, "Variable already defined.")
            );
        }
    }

    // 6. Let functionsToInitialize be a new empty List.
    let mut functions_to_initialize = vec![];
    // 7. Let declaredFunctionNames be a new empty List.
    let mut declared_function_names = HashSet::new();
    // 8. For each element d of varDeclarations, in reverse List order, do
    for d in var_declarations.iter().rev() {
        // a. If d is not either a VariableDeclaration, a ForBinding, or a BindingIdentifier, then
        if let VarScopedDeclaration::Function(d) = *d {
            // i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
            // ii. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
            // iii. Let fn be the sole element of the BoundNames of d.
            let mut function_name = None;
            d.bound_names(&mut |identifier| {
                assert!(function_name.is_none());
                function_name = Some(identifier.name.clone());
            });
            let function_name = function_name.unwrap();
            // iv. If declaredFunctionNames does not contain fn, then
            if declared_function_names.insert(function_name.clone()) {
                // 1. Let fnDefinable be ? env.CanDeclareGlobalFunction(fn).
                let fn_definable =
                    env.can_declare_global_function(agent, function_name.as_str())?;
                // 2. If fnDefinable is false, throw a TypeError exception.
                if !fn_definable {
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        "Cannot declare global function.",
                    ));
                }
                // 3. Append fn to declaredFunctionNames.
                // 4. Insert d as the first element of functionsToInitialize.
                functions_to_initialize.push(d);
            }
        }
    }

    // 9. Let declaredVarNames be a new empty List.
    let mut declared_var_names = HashSet::new();
    // 10. For each element d of varDeclarations, do
    for d in var_declarations {
        // a. If d is either a VariableDeclaration, a ForBinding, or a BindingIdentifier, then
        if let VarScopedDeclaration::Variable(d) = d {
            // i. For each String vn of the BoundNames of d, do
            let mut bound_names = vec![];
            d.id.bound_names(&mut |identifier| {
                bound_names.push(identifier.name.clone());
            });
            for vn in bound_names {
                // 1. If declaredFunctionNames does not contain vn, then
                if !declared_function_names.contains(&vn) {
                    // a. Let vnDefinable be ? env.CanDeclareGlobalVar(vn).
                    let vn_definable = env.can_declare_global_var(agent, &vn)?;
                    // b. If vnDefinable is false, throw a TypeError exception.
                    if !vn_definable {
                        return Err(agent.throw_exception(
                            ExceptionType::TypeError,
                            "Cannot declare global variable.",
                        ));
                    }
                    // c. If declaredVarNames does not contain vn, then
                    // i. Append vn to declaredVarNames.
                    declared_var_names.insert(vn.clone());
                }
            }
        }
    }

    // 11. NOTE: No abnormal terminations occur after this algorithm step if the
    //     global object is an ordinary object. However, if the global object is
    //     a Proxy exotic object it may exhibit behaviours that cause abnormal
    //     terminations in some of the following steps.
    // 12. NOTE: Annex B.3.2.2 adds additional steps at this point.

    // 14. Let privateEnv be null.
    let private_env = None;
    // 15. For each element d of lexDeclarations, do
    for d in lex_declarations {
        // a. NOTE: Lexically declared names are only instantiated here but not initialized.
        let mut bound_names = vec![];
        let mut const_bound_names = vec![];
        let mut closure = |identifier: &BindingIdentifier| {
            bound_names.push(identifier.name.clone());
        };
        match d {
            LexicallyScopedDeclaration::Variable(decl) => {
                if decl.kind == VariableDeclarationKind::Const {
                    decl.id.bound_names(&mut |identifier| {
                        const_bound_names.push(identifier.name.clone())
                    });
                } else {
                    decl.id.bound_names(&mut closure)
                }
            }
            LexicallyScopedDeclaration::Function(decl) => decl.bound_names(&mut closure),
            LexicallyScopedDeclaration::Class(decl) => decl.bound_names(&mut closure),
            LexicallyScopedDeclaration::DefaultExport => {
                bound_names.push(Atom::new_inline("*default*"))
            }
        }
        // b. For each element dn of the BoundNames of d, do
        for dn in const_bound_names {
            // i. If IsConstantDeclaration of d is true, then
            // 1. Perform ? env.CreateImmutableBinding(dn, true).
            env.create_immutable_binding(agent, &dn, true)?;
        }
        for dn in bound_names {
            // ii. Else,
            // 1. Perform ? env.CreateMutableBinding(dn, false).
            env.create_mutable_binding(agent, &dn, false)?;
        }
    }

    // 16. For each Parse Node f of functionsToInitialize, do
    for f in functions_to_initialize {
        // a. Let fn be the sole element of the BoundNames of f.
        let mut function_name = None;
        f.bound_names(&mut |identifier| {
            assert!(function_name.is_none());
            function_name = Some(identifier.name.clone());
        });
        let function_name = function_name.unwrap();
        // b. Let fo be InstantiateFunctionObject of f with arguments env and privateEnv.
        let fo = instantiate_function_object(agent, f, EnvironmentIndex::Global(env), private_env);
        // c. Perform ? env.CreateGlobalFunctionBinding(fn, fo, false).
        env.create_global_function_binding(agent, function_name, fo.into_value(), false)?;
    }
    {}
    // 17. For each String vn of declaredVarNames, do
    for vn in declared_var_names {
        // a. Perform ? env.CreateGlobalVarBinding(vn, false).
        env.create_global_var_binding(agent, vn, false)?;
    }
    // 18. Return UNUSED.
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::ecmascript::{
        abstract_operations::operations_on_objects::create_data_property_or_throw,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{
            agent::Options, create_realm, initialize_default_realm, set_realm_global_object, Agent,
            DefaultHostHooks, ExecutionContext,
        },
        scripts_and_modules::script::{parse_script, script_evaluation},
        types::{InternalMethods, IntoValue, Number, Object, PropertyKey, String, Value},
    };
    use oxc_allocator::Allocator;
    use oxc_span::Atom;

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
    fn unary_plus() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "+(54)".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (54).into());
    }

    #[test]
    fn logical_not() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "!true".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (false).into());
    }

    #[test]
    fn bitwise_not() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "~0b1111".into(), realm, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (-16).into());
    }

    #[test]
    fn unary_typeof() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

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
        assert!(result.is_undefined());
        let key = PropertyKey::from_str(&mut agent.heap, "foo");
        let foo = agent
            .get_realm(realm)
            .global_object
            .get_own_property(&mut agent, key)
            .unwrap()
            .unwrap()
            .value
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
        assert!(result.own_property_keys(&mut agent).unwrap().is_empty());
    }

    #[test]
    fn non_empty_object() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "var foo = { a: 3 };".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
        let key = PropertyKey::from_str(&mut agent.heap, "foo");
        let foo = agent
            .get_realm(realm)
            .global_object
            .get_own_property(&mut agent, key)
            .unwrap()
            .unwrap()
            .value
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
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
    fn empty_array() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        agent.execution_context_stack.push(ExecutionContext {
            ecmascript_code: None,
            function: None,
            realm,
            script_or_module: None,
        });

        let script = parse_script(&allocator, "var foo = [];".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
        let foo = agent
            .get_realm(realm)
            .global_env
            .unwrap()
            .get_binding_value(&mut agent, &Atom::new_inline("foo"), true)
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
        assert!(result.own_property_keys(&mut agent).unwrap().is_empty());
    }

    #[test]
    fn non_empty_array() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "var foo = [ 'a', 3 ];".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
        let foo = agent
            .get_realm(realm)
            .global_env
            .unwrap()
            .get_binding_value(&mut agent, &Atom::new_inline("foo"), true)
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
        let key = PropertyKey::Integer(0.into());
        assert!(result.has_property(&mut agent, key).unwrap());
        assert_eq!(
            result
                .get_own_property(&mut agent, key)
                .unwrap()
                .unwrap()
                .value,
            Some(Value::from_str(&mut agent.heap, "a"))
        );
        let key = PropertyKey::Integer(1.into());
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

        let script = parse_script(&allocator, "function foo() {}".into(), realm, None).unwrap();
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

    #[test]
    fn non_empty_iife_function_call() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(
            &allocator,
            "(function() { return 3 })()".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }

    #[test]
    fn builtin_function_call() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);
        let global = agent.heap.get_realm(realm).global_object;

        agent.execution_context_stack.push(ExecutionContext {
            ecmascript_code: None,
            function: None,
            realm,
            script_or_module: None,
        });

        let key = PropertyKey::from_str(&mut agent.heap, "test");

        struct TestBuiltinFunction;

        impl Builtin for TestBuiltinFunction {
            const NAME: String = String::from_small_string("test");

            const LENGTH: u8 = 1;

            const BEHAVIOUR: Behaviour =
                Behaviour::Regular(|_: &mut Agent, _: Value, arguments_list: ArgumentsList| {
                    let arg_0 = arguments_list.get(0);
                    if Value::Boolean(true) == arg_0 {
                        Ok(Value::from(3))
                    } else {
                        Ok(Value::Null)
                    }
                });
        }

        let func = BuiltinFunctionBuilder::new::<TestBuiltinFunction>(&mut agent, realm).build();

        create_data_property_or_throw(&mut agent, global, key, func.into_value()).unwrap();

        let script = parse_script(&allocator, "test(true)".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from(3));

        let script = parse_script(&allocator, "test()".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Null);

        let script = parse_script(&allocator, "test({})".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn if_statement() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(&allocator, "if (true) 3".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());

        let script = parse_script(&allocator, "if (false) 3".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn if_else_statement() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(
            &allocator,
            "var foo = function() { if (true) { return 3; } else { return 5; } }; foo()".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());

        let script = parse_script(
            &allocator,
            "var bar = function() { if (false) { return 3; } else { return 5; } }; bar()".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(5).into_value());
    }

    #[test]
    fn static_property_access() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script =
            parse_script(&allocator, "var foo = { a: 3 }; foo.a".into(), realm, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }

    #[test]
    fn deep_static_property_access() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(
            &allocator,
            "var fn = function() { return 3; }; var foo = { a: { b: fn } }; foo.a.b()".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }

    #[test]
    fn computed_property_access() {
        let allocator: Allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let script = parse_script(
            &allocator,
            "var foo = { a: 3 }; var prop = 'a'; foo[prop]".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }
    #[test]
    fn for_loop() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);
        let script = parse_script(
            &allocator,
            "var i = 0; for (; i < 3; i++) {}".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);
        let key = PropertyKey::from_str(&mut agent.heap, "i");
        let i: Value = agent
            .get_realm(realm)
            .global_object
            .get_own_property(&mut agent, key)
            .unwrap()
            .unwrap()
            .value
            .unwrap();
        assert_eq!(i, Value::from(3));
    }

    #[test]
    fn lexical_declarations() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let script = parse_script(
            &allocator,
            "let i = 0; const a = 'foo'; i = 3;".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);

        let global_env = agent.get_realm(realm).global_env.unwrap();
        assert!(global_env
            .has_binding(&mut agent, &Atom::new_inline("a"))
            .unwrap());
        assert!(global_env
            .has_binding(&mut agent, &Atom::new_inline("i"))
            .unwrap());
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, &Atom::new_inline("a"), true)
                .unwrap(),
            String::from_small_string("foo").into_value()
        );
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, &Atom::new_inline("i"), true)
                .unwrap(),
            Value::from(3)
        );
    }

    #[test]
    fn lexical_declarations_in_block() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let script = parse_script(
            &allocator,
            "{ let i = 0; const a = 'foo'; i = 3; }".into(),
            realm,
            None,
        )
        .unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);

        let global_env = agent.get_realm(realm).global_env.unwrap();
        assert!(!global_env.has_lexical_declaration(&agent, &Atom::new_inline("a")));
        assert!(!global_env.has_lexical_declaration(&agent, &Atom::new_inline("i")));
    }
}
