// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::{Executable, Vm},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
use ahash::AHashSet;
use oxc_ast::{
    ast::{BindingIdentifier, Program, VariableDeclarationKind},
    syntax_directed_operations::BoundNames,
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::SourceType;
use std::{
    any::Any,
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Index, IndexMut},
};

use super::source_code::SourceCode;

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

impl Index<ScriptIdentifier> for Agent {
    type Output = Script;

    fn index(&self, index: ScriptIdentifier) -> &Self::Output {
        &self.heap.scripts[index]
    }
}

impl IndexMut<ScriptIdentifier> for Agent {
    fn index_mut(&mut self, index: ScriptIdentifier) -> &mut Self::Output {
        &mut self.heap.scripts[index]
    }
}

impl Index<ScriptIdentifier> for Vec<Option<Script>> {
    type Output = Script;

    fn index(&self, index: ScriptIdentifier) -> &Self::Output {
        self.get(index.into_index())
            .expect("ScriptIdentifier out of bounds")
            .as_ref()
            .expect("ScriptIdentifier slot empty")
    }
}

impl IndexMut<ScriptIdentifier> for Vec<Option<Script>> {
    fn index_mut(&mut self, index: ScriptIdentifier) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("ScriptIdentifier out of bounds")
            .as_mut()
            .expect("ScriptIdentifier slot empty")
    }
}

impl HeapMarkAndSweep for ScriptIdentifier {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.scripts.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.scripts.get_shift_for_index(self_index));
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
    ///
    /// Note: The Program's drop code is never run. The referred structures
    /// live in the SourceCode heap data in its contained Allocator. The bump
    /// allocator drops all of the data in a single go. All that needs to be
    /// dropped here is the local Program itself, not any of its referred
    /// parts.
    pub(crate) ecmascript_code: ManuallyDrop<Program<'static>>,

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
    /// The source text is kept in the heap strings vector, through the
    /// SourceCode struct.
    pub(crate) source_code: SourceCode,
}

unsafe impl Send for Script {}

pub type ScriptOrErrors = Result<Script, Vec<OxcDiagnostic>>;

impl HeapMarkAndSweep for Script {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            realm,
            ecmascript_code: _,
            loaded_modules: _,
            host_defined: _,
            source_code,
        } = self;
        realm.mark_values(queues);
        source_code.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            realm,
            ecmascript_code: _,
            loaded_modules: _,
            host_defined: _,
            source_code,
        } = self;
        realm.sweep_values(compactions);
        source_code.sweep_values(compactions);
    }
}

/// ### [16.1.5 ParseScript ( sourceText, realm, hostDefined )](https://tc39.es/ecma262/#sec-parse-script)
///
/// The abstract operation ParseScript takes arguments sourceText (ECMAScript
/// source text), realm (a Realm Record), and hostDefined (anything) and
/// returns a Script Record or a non-empty List of SyntaxError objects. It
/// creates a Script Record based upon the result of parsing sourceText as a
/// Script.
pub fn parse_script(
    agent: &mut Agent,
    source_text: String,
    realm: RealmIdentifier,
    strict_mode: bool,
    host_defined: Option<HostDefined>,
) -> ScriptOrErrors {
    // 1. Let script be ParseText(sourceText, Script).
    let mut source_type = if strict_mode {
        // Strict mode script is equal to module code.
        SourceType::default().with_module(true)
    } else {
        // Loose mode script is just script code.
        SourceType::default().with_script(true)
    };
    if cfg!(feature = "typescript") {
        source_type = source_type.with_typescript(true);
    }

    // SAFETY: Script keeps the SourceCode reference alive in the Heap, thus
    // making the Program's references point to a live Allocator.
    let parse_result = unsafe { SourceCode::parse_source(agent, source_text, source_type) };

    let (program, source_code) = match parse_result {
        // 2. If script is a List of errors, return script.
        Ok(result) => result,
        Err(errors) => {
            return Err(errors);
        }
    };

    // 3. Return Script Record {
    Ok(Script {
        // [[Realm]]: realm,
        realm,
        // [[ECMAScriptCode]]: script,
        ecmascript_code: ManuallyDrop::new(program),
        // [[LoadedModules]]: « »,
        loaded_modules: (),
        // [[HostDefined]]: hostDefined,
        host_defined,
        source_code,
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
    let is_strict_mode = script.ecmascript_code.is_strict();
    let source_code = script.source_code;
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

            is_strict_mode,

            source_code,
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
        let bytecode = Executable::compile_script(agent, script);
        // a. Set result to Completion(Evaluation of script).
        // b. If result.[[Type]] is normal and result.[[Value]] is empty, then
        // i. Set result to NormalCompletion(undefined).
        let result = Vm::execute(agent, bytecode, None).into_js_result();
        // SAFETY: The bytecode is not accessible by anyone and no one will try
        // to re-run it.
        unsafe { bytecode.try_drop(agent) };
        result
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
    // SAFETY: Analysing the script cannot cause the environment to move even though we change other parts of the Heap.
    let (lex_names, var_names, var_declarations, lex_declarations) = {
        let Script {
            ecmascript_code: script,
            ..
        } = &agent[script];
        // SAFETY: The borrow of Program is valid for the duration of this
        // block; the contents of Program are guaranteed to be valid for as
        // long as the Script is alive in the heap as they are not reallocated.
        // Thus in effect VarScopedDeclaration<'_> is valid for the duration
        // of the global_declaration_instantiation call.
        let script =
            unsafe { std::mem::transmute::<&Program<'_>, &'static Program<'static>>(script) };
        // 1. Let lexNames be the LexicallyDeclaredNames of script.
        let lex_names = script_lexically_declared_names(script);
        // 2. Let varNames be the VarDeclaredNames of script.
        let var_names = script_var_declared_names(script);
        // 5. Let varDeclarations be the VarScopedDeclarations of script.
        let var_declarations = script_var_scoped_declarations(script);
        // 13. Let lexDeclarations be the LexicallyScopedDeclarations of script.
        let lex_declarations = script_lexically_scoped_declarations(script);
        (lex_names, var_names, var_declarations, lex_declarations)
    };

    // 3. For each element name of lexNames, do
    for name in lex_names {
        let name = String::from_str(agent, name.as_str());
        if
        // a. If env.HasVarDeclaration(name) is true, throw a SyntaxError exception.
        env.has_var_declaration(agent, name)
            // b. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
            || env.has_lexical_declaration(agent, name)
            // c. Let hasRestrictedGlobal be ? env.HasRestrictedGlobalProperty(name).
            // d. If hasRestrictedGlobal is true, throw a SyntaxError exception.
            || env.has_restricted_global_property(agent, name)?
        {
            let error_message = format!(
                "Redeclaration of restricted global property '{}'.",
                name.as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::SyntaxError, error_message));
        }
    }

    // 4. For each element name of varNames, do
    for name in &var_names {
        // a. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
        let name = String::from_str(agent, name.as_str());
        if env.has_lexical_declaration(agent, name) {
            let error_message =
                format!("Redeclaration of lexical binding '{}'.", name.as_str(agent));
            return Err(agent.throw_exception(ExceptionType::SyntaxError, error_message));
        }
    }

    // 6. Let functionsToInitialize be a new empty List.
    let mut functions_to_initialize = vec![];
    // 7. Let declaredFunctionNames be a new empty List.
    let mut declared_function_names = AHashSet::default();
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
                let function_name = String::from_str(agent, function_name.as_str());
                let fn_definable = env.can_declare_global_function(agent, function_name)?;
                // 2. If fnDefinable is false, throw a TypeError exception.
                if !fn_definable {
                    let error_message = format!(
                        "Cannot declare of global function '{}'.",
                        function_name.as_str(agent)
                    );
                    return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
                }
                // 3. Append fn to declaredFunctionNames.
                // 4. Insert d as the first element of functionsToInitialize.
                functions_to_initialize.push(d);
            }
        }
    }

    // 9. Let declaredVarNames be a new empty List.
    let mut declared_var_names = AHashSet::default();
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
                    let vn = String::from_str(agent, vn.as_str());
                    let vn_definable = env.can_declare_global_var(agent, vn)?;
                    // b. If vnDefinable is false, throw a TypeError exception.
                    if !vn_definable {
                        let error_message =
                            format!("Cannot declare global variable '{}'.", vn.as_str(agent));
                        return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
                    }
                    // c. If declaredVarNames does not contain vn, then
                    // i. Append vn to declaredVarNames.
                    declared_var_names.insert(vn);
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
            bound_names.push(String::from_str(agent, identifier.name.as_str()));
        };
        match d {
            LexicallyScopedDeclaration::Variable(decl) => {
                if decl.kind == VariableDeclarationKind::Const {
                    decl.id.bound_names(&mut |identifier| {
                        const_bound_names.push(String::from_str(agent, identifier.name.as_str()))
                    });
                } else {
                    decl.id.bound_names(&mut closure)
                }
            }
            LexicallyScopedDeclaration::Function(decl) => decl.bound_names(&mut closure),
            LexicallyScopedDeclaration::Class(decl) => decl.bound_names(&mut closure),
            LexicallyScopedDeclaration::DefaultExport => {
                bound_names.push(BUILTIN_STRING_MEMORY._default_)
            }
        }
        // b. For each element dn of the BoundNames of d, do
        for dn in const_bound_names {
            // i. If IsConstantDeclaration of d is true, then
            // 1. Perform ? env.CreateImmutableBinding(dn, true).
            env.create_immutable_binding(agent, dn, true)?;
        }
        for dn in bound_names {
            // ii. Else,
            // 1. Perform ? env.CreateMutableBinding(dn, false).
            env.create_mutable_binding(agent, dn, false)?;
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
        let function_name = String::from_str(agent, function_name.unwrap().as_str());
        // b. Let fo be InstantiateFunctionObject of f with arguments env and privateEnv.
        let fo = instantiate_function_object(agent, f, EnvironmentIndex::Global(env), private_env);
        // c. Perform ? env.CreateGlobalFunctionBinding(fn, fo, false).
        env.create_global_function_binding(agent, function_name, fo.into_value(), false)?;
    }

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
    use crate::{
        ecmascript::{
            abstract_operations::operations_on_objects::create_data_property_or_throw,
            builders::builtin_function_builder::BuiltinFunctionBuilder,
            builtins::{ArgumentsList, Behaviour, Builtin},
            execution::{
                agent::Options, create_realm, initialize_default_realm, set_realm_global_object,
                Agent, DefaultHostHooks, ExecutionContext,
            },
            scripts_and_modules::script::{parse_script, script_evaluation},
            types::{InternalMethods, IntoValue, Number, Object, PropertyKey, String, Value},
        },
        SmallInteger,
    };

    #[test]
    fn empty_script() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn basic_constants() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "true");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, true.into());
    }

    #[test]
    fn unary_minus() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "-2");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (-2).into());
    }

    #[test]
    fn unary_void() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "void (2 + 2 + 6)");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn unary_plus() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "+(54)");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (54).into());
    }

    #[test]
    fn logical_not() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "!true");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (false).into());
    }

    #[test]
    fn bitwise_not() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "~0b1111");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (-16).into());
    }

    #[test]
    fn unary_typeof() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "typeof undefined");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "undefined"));

        let source_text = String::from_static_str(&mut agent, "typeof null");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "object"));

        let source_text = String::from_static_str(&mut agent, "typeof \"string\"");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "string"));

        let source_text = String::from_static_str(&mut agent, "typeof Symbol()");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "symbol"));

        let source_text = String::from_static_str(&mut agent, "typeof true");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "boolean"));

        let source_text = String::from_static_str(&mut agent, "typeof 3");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "number"));

        let source_text = String::from_static_str(&mut agent, "typeof 3n");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "bigint"));

        let source_text = String::from_static_str(&mut agent, "typeof {}");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "object"));

        let source_text = String::from_static_str(&mut agent, "typeof (function() {})");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "function"));
    }

    #[test]
    fn binary_add() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "2 + 2 + 6");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();

        let result = script_evaluation(&mut agent, script).unwrap();

        assert_eq!(result, (10).into());
    }

    #[test]
    fn var_assign() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "var foo = 3;");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn empty_object() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "var foo = {};");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
        let key = PropertyKey::from_static_str(&mut agent, "foo");
        let foo = agent
            .get_realm(realm)
            .global_object
            .internal_get_own_property(&mut agent, key)
            .unwrap()
            .unwrap()
            .value
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
        assert!(result
            .internal_own_property_keys(&mut agent)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn non_empty_object() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "var foo = { a: 3 };");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
        let key = PropertyKey::from_static_str(&mut agent, "foo");
        let foo = agent
            .get_realm(realm)
            .global_object
            .internal_get_own_property(&mut agent, key)
            .unwrap()
            .unwrap()
            .value
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
        let key = PropertyKey::from_static_str(&mut agent, "a");
        assert!(result.internal_has_property(&mut agent, key).unwrap());
        assert_eq!(
            result
                .internal_get_own_property(&mut agent, key)
                .unwrap()
                .unwrap()
                .value,
            Some(Value::from(3))
        );
    }

    #[test]
    fn empty_array() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        agent.execution_context_stack.push(ExecutionContext {
            ecmascript_code: None,
            function: None,
            realm,
            script_or_module: None,
        });

        let source_text = String::from_static_str(&mut agent, "var foo = [];");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
        let foo_key = String::from_static_str(&mut agent, "foo");
        let foo = agent
            .get_realm(realm)
            .global_env
            .unwrap()
            .get_binding_value(&mut agent, foo_key, true)
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
        assert!(result
            .internal_own_property_keys(&mut agent)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn non_empty_array() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "var foo = [ 'a', 3 ];");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
        let foo_key = String::from_static_str(&mut agent, "foo");
        let foo = agent
            .get_realm(realm)
            .global_env
            .unwrap()
            .get_binding_value(&mut agent, foo_key, true)
            .unwrap();
        assert!(foo.is_object());
        let result = Object::try_from(foo).unwrap();
        let key = PropertyKey::Integer(0.into());
        assert!(result.internal_has_property(&mut agent, key).unwrap());
        assert_eq!(
            result
                .internal_get_own_property(&mut agent, key)
                .unwrap()
                .unwrap()
                .value,
            Some(Value::from_static_str(&mut agent, "a"))
        );
        let key = PropertyKey::Integer(1.into());
        assert!(result.internal_has_property(&mut agent, key).unwrap());
        assert_eq!(
            result
                .internal_get_own_property(&mut agent, key)
                .unwrap()
                .unwrap()
                .value,
            Some(Value::from(3))
        );
    }

    #[test]
    fn empty_function() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "function foo() {}");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_function());
    }

    #[test]
    fn empty_iife_function_call() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "(function() {})()");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn empty_named_function_call() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "var f = function() {}; f();");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn empty_declared_function_call() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "function f() {}; f();");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn non_empty_iife_function_call() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "(function() { return 3 })()");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }

    #[test]
    fn builtin_function_call() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);
        let global = agent[realm].global_object;

        agent.execution_context_stack.push(ExecutionContext {
            ecmascript_code: None,
            function: None,
            realm,
            script_or_module: None,
        });

        let key = PropertyKey::from_static_str(&mut agent, "test");

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

        let source_text = String::from_static_str(&mut agent, "test(true)");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from(3));

        let source_text = String::from_static_str(&mut agent, "test()");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Null);

        let source_text = String::from_static_str(&mut agent, "test({})");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn if_statement() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "if (true) 3");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());

        let source_text = String::from_static_str(&mut agent, "if (false) 3");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn if_else_statement() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(
            &mut agent,
            "var foo = function() { if (true) { return 3; } else { return 5; } }; foo()",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());

        let source_text = String::from_static_str(
            &mut agent,
            "var bar = function() { if (false) { return 3; } else { return 5; } }; bar()",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(5).into_value());
    }

    #[test]
    fn static_property_access() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(&mut agent, "var foo = { a: 3 }; foo.a");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }

    #[test]
    fn deep_static_property_access() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text = String::from_static_str(
            &mut agent,
            "var fn = function() { return 3; }; var foo = { a: { b: fn } }; foo.a.b()",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }

    #[test]
    fn computed_property_access() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);

        let source_text =
            String::from_static_str(&mut agent, "var foo = { a: 3 }; var prop = 'a'; foo[prop]");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Number::from(3).into_value());
    }
    #[test]
    fn for_loop() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = create_realm(&mut agent);
        set_realm_global_object(&mut agent, realm, None, None);
        let source_text = String::from_static_str(&mut agent, "var i = 0; for (; i < 3; i++) {}");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);
        let key = PropertyKey::from_static_str(&mut agent, "i");
        let i: Value = agent
            .get_realm(realm)
            .global_object
            .internal_get_own_property(&mut agent, key)
            .unwrap()
            .unwrap()
            .value
            .unwrap();
        assert_eq!(i, Value::from(3));
    }

    #[test]
    fn lexical_declarations() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "let i = 0; const a = 'foo'; i = 3;");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        script_evaluation(&mut agent, script).unwrap();

        let global_env = agent.get_realm(realm).global_env.unwrap();
        let a_key = String::from_static_str(&mut agent, "a");
        let i_key = String::from_static_str(&mut agent, "i");
        assert!(global_env.has_binding(&mut agent, a_key).unwrap());
        assert!(global_env.has_binding(&mut agent, i_key).unwrap());
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, a_key, true)
                .unwrap(),
            String::from_small_string("foo").into_value()
        );
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, i_key, true)
                .unwrap(),
            Value::from(3)
        );
    }

    #[test]
    fn lexical_declarations_in_block() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text =
            String::from_static_str(&mut agent, "{ let i = 0; const a = 'foo'; i = 3; }");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);

        let a_key = String::from_static_str(&mut agent, "a");
        let i_key = String::from_static_str(&mut agent, "i");
        let global_env = agent.get_realm(realm).global_env.unwrap();
        assert!(!global_env.has_lexical_declaration(&agent, a_key));
        assert!(!global_env.has_lexical_declaration(&agent, i_key));
    }

    #[test]
    fn object_property_assignment() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "var foo = {}; foo.a = 42; foo");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        let object = Object::try_from(result).unwrap();

        let pk = PropertyKey::from_static_str(&mut agent, "a");
        assert_eq!(
            object
                .internal_get(&mut agent, pk, object.into_value())
                .unwrap(),
            Value::Integer(SmallInteger::from(42))
        );
    }

    #[test]
    fn try_catch_not_thrown() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(
            &mut agent,
            "let a = 0; try { a++; } catch { a = 500; }; a++; a",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(2)));
    }

    #[test]
    fn try_catch_thrown() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(
            &mut agent,
            "let a = 0; try { throw null; a = 500 } catch { a++; }; a++; a",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(2)));
    }

    #[test]
    fn catch_binding() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(
            &mut agent,
            "let err; try { throw 'thrown'; } catch(e) { err = e; }; err",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "thrown"));
    }

    #[test]
    fn throwing_in_try_restores_lexical_environment() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(
            &mut agent,
            "let a = 42; try { let a = 62; throw 'thrown'; } catch { }; a",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(42)));
    }

    #[test]
    fn function_argument_bindings() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(
            &mut agent,
            "const foo = function (a) { return a + 10; }; foo(32)",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(42)));
    }

    #[test]
    fn logical_and() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "true && true");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let source_text = String::from_static_str(&mut agent, "true && false && true");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn logical_or() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "false || false");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Boolean(false));

        let source_text = String::from_static_str(&mut agent, "true || false || true");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn nullish_coalescing() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "null ?? 42");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(42)));

        let source_text = String::from_static_str(&mut agent, "'foo' ?? 12");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "foo"));

        let source_text = String::from_static_str(&mut agent, "undefined ?? null");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn string_concat() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "'foo' + '' + 'bar'");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "foobar"));

        let source_text = String::from_static_str(&mut agent, "'foo' + ' a heap string'");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(
            result,
            Value::from_static_str(&mut agent, "foo a heap string")
        );

        let source_text =
            String::from_static_str(&mut agent, "'Concatenating ' + 'two heap strings'");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(
            result,
            Value::from_static_str(&mut agent, "Concatenating two heap strings")
        );
    }

    #[test]
    fn property_access_on_functions() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "function foo() {}; foo.bar");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);

        let source_text = String::from_static_str(&mut agent, "foo.bar = 42; foo.bar");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(42)));

        let source_text = String::from_static_str(&mut agent, "foo.name");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "foo"));

        let source_text = String::from_static_str(&mut agent, "foo.length");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::zero()));

        let source_text = String::from_static_str(&mut agent, "foo.prototype");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert!(result.is_object())
    }

    #[test]
    fn name_and_length_on_builtin_functions() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "TypeError.name");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::from_static_str(&mut agent, "TypeError"));

        let source_text = String::from_static_str(&mut agent, "TypeError.length");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(1)));
    }

    #[test]
    fn constructor() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "function foo() {}; foo.prototype");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        let foo_prototype = Object::try_from(result).unwrap();

        let source_text = String::from_static_str(&mut agent, "new foo()");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = match script_evaluation(&mut agent, script) {
            Ok(result) => result,
            Err(err) => panic!("{}", err.to_string(&mut agent).as_str(&agent)),
        };
        let instance = Object::try_from(result).unwrap();
        assert_eq!(
            instance.internal_get_prototype_of(&mut agent).unwrap(),
            Some(foo_prototype)
        );
    }

    #[test]
    fn this_expression() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(
            &mut agent,
            "function foo() { this.bar = 42; }; new foo().bar",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(42)));

        let source_text = String::from_static_str(
            &mut agent,
            "foo.prototype.baz = function() { return this.bar + 10; }; (new foo()).baz()",
        );
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Integer(SmallInteger::from(52)));
    }

    #[test]
    fn symbol_stringification() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "+Symbol()");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert!(script_evaluation(&mut agent, script).is_err());

        let source_text = String::from_static_str(&mut agent, "+Symbol('foo')");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert!(script_evaluation(&mut agent, script).is_err());

        let source_text = String::from_static_str(&mut agent, "String(Symbol())");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let value = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(value, Value::from_static_str(&mut agent, "Symbol()"));

        let source_text = String::from_static_str(&mut agent, "String(Symbol('foo'))");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let value = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(value, Value::from_static_str(&mut agent, "Symbol(foo)"));
    }

    #[test]
    fn instanceof() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "3 instanceof Number");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert_eq!(script_evaluation(&mut agent, script).unwrap(), false.into());

        let source_text = String::from_static_str(&mut agent, "'foo' instanceof String");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert_eq!(script_evaluation(&mut agent, script).unwrap(), false.into());

        let source_text = String::from_static_str(&mut agent, "({}) instanceof Object");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert_eq!(script_evaluation(&mut agent, script).unwrap(), true.into());

        let source_text = String::from_static_str(&mut agent, "({}) instanceof Array");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert_eq!(script_evaluation(&mut agent, script).unwrap(), false.into());

        let source_text = String::from_static_str(&mut agent, "([]) instanceof Object");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert_eq!(script_evaluation(&mut agent, script).unwrap(), true.into());

        let source_text = String::from_static_str(&mut agent, "([]) instanceof Array");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        assert_eq!(script_evaluation(&mut agent, script).unwrap(), true.into());
    }

    #[test]
    fn array_binding_pattern() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "const [a, b, , c] = [1, 2, 3, 4];");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        script_evaluation(&mut agent, script).unwrap();
        let a_key = String::from_static_str(&mut agent, "a");
        let b_key = String::from_static_str(&mut agent, "b");
        let c_key = String::from_static_str(&mut agent, "c");
        let global_env = agent.get_realm(realm).global_env.unwrap();
        assert!(global_env.has_lexical_declaration(&agent, a_key));
        assert!(global_env.has_lexical_declaration(&agent, b_key));
        assert!(global_env.has_lexical_declaration(&agent, c_key));
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, a_key, true)
                .unwrap(),
            1.into()
        );
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, b_key, true)
                .unwrap(),
            2.into()
        );
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, c_key, true)
                .unwrap(),
            4.into()
        );
    }

    #[test]
    fn do_while() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text =
            String::from_static_str(&mut agent, "let i = 0; do { i++ } while(i < 10)");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        script_evaluation(&mut agent, script).unwrap();

        let i_key = String::from_static_str(&mut agent, "i");
        let global_env = agent.get_realm(realm).global_env.unwrap();
        assert!(global_env.has_lexical_declaration(&agent, i_key));
        assert_eq!(
            global_env
                .get_binding_value(&mut agent, i_key, true)
                .unwrap(),
            10.into()
        );
    }

    #[test]
    fn no_implicit_return() {
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        let realm = agent.current_realm_id();

        let source_text = String::from_static_str(&mut agent, "function foo() { 42; }; foo()");
        let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
        let result = script_evaluation(&mut agent, script).unwrap();
        assert_eq!(result, Value::Undefined);
    }
}
