// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashSet;
use oxc_ast::{
    ast::{BindingIdentifier, Program, VariableDeclarationKind},
    syntax_directed_operations::BoundNames,
};
use oxc_span::SourceType;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_number,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        execution::{
            agent::ExceptionType, get_this_environment, new_declarative_environment, Agent,
            ECMAScriptCodeEvaluationState, EnvironmentIndex, ExecutionContext, JsResult,
            PrivateEnvironmentIndex, RealmIdentifier,
        },
        scripts_and_modules::source_code::SourceCode,
        syntax_directed_operations::{
            miscellaneous::instantiate_function_object,
            scope_analysis::{
                script_lexically_scoped_declarations, script_var_declared_names,
                script_var_scoped_declarations, LexicallyScopedDeclaration, VarScopedDeclaration,
            },
        },
        types::{Function, IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::{Executable, Vm},
    heap::IntrinsicFunctionIndexes,
};

use super::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic};

pub(crate) struct GlobalObject;

struct GlobalObjectEval;
impl Builtin for GlobalObjectEval {
    const NAME: String = BUILTIN_STRING_MEMORY.eval;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::eval);
}
impl BuiltinIntrinsic for GlobalObjectEval {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::Eval;
}
struct GlobalObjectIsFinite;
impl Builtin for GlobalObjectIsFinite {
    const NAME: String = BUILTIN_STRING_MEMORY.isFinite;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::is_finite);
}
impl BuiltinIntrinsic for GlobalObjectIsFinite {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::IsFinite;
}
struct GlobalObjectIsNaN;
impl Builtin for GlobalObjectIsNaN {
    const NAME: String = BUILTIN_STRING_MEMORY.isNaN;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::is_nan);
}
impl BuiltinIntrinsic for GlobalObjectIsNaN {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::IsNaN;
}
struct GlobalObjectParseFloat;
impl Builtin for GlobalObjectParseFloat {
    const NAME: String = BUILTIN_STRING_MEMORY.parseFloat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::parse_float);
}
impl BuiltinIntrinsic for GlobalObjectParseFloat {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ParseFloat;
}
struct GlobalObjectParseInt;
impl Builtin for GlobalObjectParseInt {
    const NAME: String = BUILTIN_STRING_MEMORY.parseInt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::parse_int);
}
impl BuiltinIntrinsic for GlobalObjectParseInt {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ParseInt;
}
struct GlobalObjectDecodeURI;
impl Builtin for GlobalObjectDecodeURI {
    const NAME: String = BUILTIN_STRING_MEMORY.decodeURI;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::decode_uri);
}
impl BuiltinIntrinsic for GlobalObjectDecodeURI {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::DecodeURI;
}
struct GlobalObjectDecodeURIComponent;
impl Builtin for GlobalObjectDecodeURIComponent {
    const NAME: String = BUILTIN_STRING_MEMORY.decodeURIComponent;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::decode_uri_component);
}
impl BuiltinIntrinsic for GlobalObjectDecodeURIComponent {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::DecodeURIComponent;
}
struct GlobalObjectEncodeURI;
impl Builtin for GlobalObjectEncodeURI {
    const NAME: String = BUILTIN_STRING_MEMORY.encodeURI;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::encode_uri);
}
impl BuiltinIntrinsic for GlobalObjectEncodeURI {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::EncodeURI;
}
struct GlobalObjectEncodeURIComponent;
impl Builtin for GlobalObjectEncodeURIComponent {
    const NAME: String = BUILTIN_STRING_MEMORY.encodeURIComponent;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::encode_uri_component);
}
impl BuiltinIntrinsic for GlobalObjectEncodeURIComponent {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::EncodeURIComponent;
}
struct GlobalObjectEscape;
impl Builtin for GlobalObjectEscape {
    const NAME: String = BUILTIN_STRING_MEMORY.escape;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::escape);
}
impl BuiltinIntrinsic for GlobalObjectEscape {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::Escape;
}
struct GlobalObjectUnescape;
impl Builtin for GlobalObjectUnescape {
    const NAME: String = BUILTIN_STRING_MEMORY.unescape;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::unescape);
}
impl BuiltinIntrinsic for GlobalObjectUnescape {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::Unescape;
}

/// ### [19.2.1.1 PerformEval ( x, strictCaller, direct )](https://tc39.es/ecma262/#sec-performeval)
///
/// The abstract operation PerformEval takes arguments x (an ECMAScript
/// language value), strictCaller (a Boolean), and direct (a Boolean) and
/// returns either a normal completion containing an ECMAScript language value
/// or a throw completion.
pub fn perform_eval(
    agent: &mut Agent,
    x: Value,
    direct: bool,
    strict_caller: bool,
) -> JsResult<Value> {
    // 1. Assert: If direct is false, then strictCaller is also false.
    assert!(direct || !strict_caller);

    // 2. If x is not a String, return x.
    let Ok(x) = String::try_from(x) else {
        return Ok(x);
    };

    // 3. Let evalRealm be the current Realm Record.
    let eval_realm = agent.current_realm_id();

    // 4. NOTE: In the case of a direct eval, evalRealm is the realm of both the caller of eval and of the eval function itself.
    // 5. Perform ? HostEnsureCanCompileStrings(evalRealm, « », x, direct).
    agent
        .host_hooks
        .host_ensure_can_compile_strings(&mut agent[eval_realm])?;

    // 6. Let inFunction be false.
    let mut _in_function = false;
    // 7. Let inMethod be false.
    let mut _in_method = false;
    // 8. Let inDerivedConstructor be false.
    let mut _in_derived_constructor = false;
    // 9. Let inClassFieldInitializer be false.
    let _in_class_field_initializer = false;

    // 10. If direct is true, then
    if direct {
        // a. Let thisEnvRec be GetThisEnvironment().
        let this_env_rec = get_this_environment(agent);
        // b. If thisEnvRec is a Function Environment Record, then
        if let EnvironmentIndex::Function(this_env_rec) = this_env_rec {
            // i. Let F be thisEnvRec.[[FunctionObject]].
            let f = agent[this_env_rec].function_object;
            // ii. Set inFunction to true.
            _in_function = true;
            // iii. Set inMethod to thisEnvRec.HasSuperBinding().
            _in_method = this_env_rec.has_super_binding(agent);
            // iv. If F.[[ConstructorKind]] is derived, set inDerivedConstructor to true.
            _in_derived_constructor = match f {
                Function::ECMAScriptFunction(idx) => agent[idx]
                    .ecmascript_function
                    .constructor_status
                    .is_derived_class(),
                _ => todo!(),
            };

            // TODO:
            // v. Let classFieldInitializerName be F.[[ClassFieldInitializerName]].
            // vi. If classFieldInitializerName is not empty, set inClassFieldInitializer to true.
        }
    }

    // 11. Perform the following substeps in an implementation-defined order, possibly interleaving parsing and error detection:
    // a. Let script be ParseText(x, Script).
    let source_type = if strict_caller {
        SourceType::default().with_module(true)
    } else {
        SourceType::default().with_script(true)
    };
    // SAFETY: Script is only kept alive for the duration of this call, and any
    // references made to it by functions being created in the eval call will
    // take a copy of the SourceCode. The SourceCode is also kept in the
    // evaluation context and thus cannot be garbage collected while the eval
    // call happens.
    // The Program thus refers to a valid, live Allocator for the duration of
    // this call.
    let parse_result = unsafe { SourceCode::parse_source(agent, x, source_type) };

    // b. If script is a List of errors, throw a SyntaxError exception.
    let Ok((script, source_code)) = parse_result else {
        // TODO: Include error messages in the exception.
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::SyntaxError,
            "Invalid eval source text.",
        ));
    };

    // c. If script Contains ScriptBody is false, return undefined.
    if script.is_empty() {
        return Ok(Value::Undefined);
    }

    // TODO:
    // d. Let body be the ScriptBody of script.
    // e. If inFunction is false and body Contains NewTarget, throw a SyntaxError exception.
    // f. If inMethod is false and body Contains SuperProperty, throw a SyntaxError exception.
    // g. If inDerivedConstructor is false and body Contains SuperCall, throw a SyntaxError exception.
    // h. If inClassFieldInitializer is true and ContainsArguments of body is true, throw a SyntaxError exception.

    // 12. If strictCaller is true, let strictEval be true.
    // 13. Else, let strictEval be ScriptIsStrict of script.
    let strict_eval = script.is_strict();
    if strict_caller {
        debug_assert!(strict_eval);
    }

    // 14. Let runningContext be the running execution context.
    // 15. NOTE: If direct is true, runningContext will be the execution context that performed the direct eval. If direct is false, runningContext will be the execution context for the invocation of the eval function.

    // 16. If direct is true, then
    let mut ecmascript_code = if direct {
        let ECMAScriptCodeEvaluationState {
            lexical_environment: running_context_lex_env,
            variable_environment: running_context_var_env,
            private_environment: running_context_private_env,
            ..
        } = *agent
            .running_execution_context()
            .ecmascript_code
            .as_ref()
            .unwrap();

        ECMAScriptCodeEvaluationState {
            // a. Let lexEnv be NewDeclarativeEnvironment(runningContext's LexicalEnvironment).
            lexical_environment: EnvironmentIndex::Declarative(new_declarative_environment(
                agent,
                Some(running_context_lex_env),
            )),
            // b. Let varEnv be runningContext's VariableEnvironment.
            variable_environment: running_context_var_env,
            // c. Let privateEnv be runningContext's PrivateEnvironment.
            private_environment: running_context_private_env,
            is_strict_mode: strict_eval,
            // The code running inside eval is defined inside the eval source.
            source_code,
        }
    } else {
        // 17. Else,
        let global_env = EnvironmentIndex::Global(agent[eval_realm].global_env.unwrap());

        ECMAScriptCodeEvaluationState {
            // a. Let lexEnv be NewDeclarativeEnvironment(evalRealm.[[GlobalEnv]]).
            lexical_environment: EnvironmentIndex::Declarative(new_declarative_environment(
                agent,
                Some(global_env),
            )),
            // b. Let varEnv be evalRealm.[[GlobalEnv]].
            variable_environment: global_env,
            // c. Let privateEnv be null.
            private_environment: None,
            is_strict_mode: strict_eval,
            // The code running inside eval is defined inside the eval source.
            source_code,
        }
    };

    // 18. If strictEval is true, set varEnv to lexEnv.
    if strict_eval {
        ecmascript_code.variable_environment = ecmascript_code.lexical_environment;
    }

    // 19. If runningContext is not already suspended, suspend runningContext.
    agent.running_execution_context().suspend();

    // 20. Let evalContext be a new ECMAScript code execution context.
    let eval_context = ExecutionContext {
        // 21. Set evalContext's Function to null.
        function: None,
        // 22. Set evalContext's Realm to evalRealm.
        realm: eval_realm,
        // 23. Set evalContext's ScriptOrModule to runningContext's ScriptOrModule.
        script_or_module: agent.running_execution_context().script_or_module,
        // 24. Set evalContext's VariableEnvironment to varEnv.
        // 25. Set evalContext's LexicalEnvironment to lexEnv.
        // 26. Set evalContext's PrivateEnvironment to privateEnv.
        ecmascript_code: Some(ecmascript_code),
    };
    // 27. Push evalContext onto the execution context stack; evalContext is now the running execution context.
    agent.execution_context_stack.push(eval_context);

    // 28. Let result be Completion(EvalDeclarationInstantiation(body, varEnv, lexEnv, privateEnv, strictEval)).
    let result = eval_declaration_instantiation(
        agent,
        &script,
        ecmascript_code.variable_environment,
        ecmascript_code.lexical_environment,
        ecmascript_code.private_environment,
        strict_eval,
    );

    // 29. If result is a normal completion, then
    let result = if result.is_ok() {
        let exe = Executable::compile_eval_body(agent, &script.body);
        // a. Set result to Completion(Evaluation of body).
        // 30. If result is a normal completion and result.[[Value]] is empty, then
        // a. Set result to NormalCompletion(undefined).
        let result = Vm::execute(agent, exe, None).into_js_result();
        // SAFETY: No one can access the bytecode anymore.
        unsafe { exe.try_drop(agent) };
        result
    } else {
        Err(result.err().unwrap())
    };

    // 31. Suspend evalContext and remove it from the execution context stack.
    agent.execution_context_stack.pop().unwrap().suspend();

    // TODO:
    // 32. Resume the context that is now on the top of the execution context stack as the running execution context.

    // 33. Return ? result.
    result
}

/// ### [19.2.1.3 EvalDeclarationInstantiation ( body, varEnv, lexEnv, privateEnv, strict )](https://tc39.es/ecma262/#sec-evaldeclarationinstantiation)
///
/// The abstract operation EvalDeclarationInstantiation takes arguments body
/// (a ScriptBody Parse Node), varEnv (an Environment Record), lexEnv (a
/// Declarative Environment Record), privateEnv (a PrivateEnvironment Record or
/// null), and strict (a Boolean) and returns either a normal completion
/// containing UNUSED or a throw completion.
pub fn eval_declaration_instantiation(
    agent: &mut Agent,
    script: &Program,
    var_env: EnvironmentIndex,
    lex_env: EnvironmentIndex,
    private_env: Option<PrivateEnvironmentIndex>,
    strict_eval: bool,
) -> JsResult<()> {
    // 1. Let varNames be the VarDeclaredNames of body.
    let var_names = script_var_declared_names(script);

    // 2. Let varDeclarations be the VarScopedDeclarations of body.
    let var_declarations = script_var_scoped_declarations(script);

    // 3. If strict is false, then
    if !strict_eval {
        // a. If varEnv is a Global Environment Record, then
        if let EnvironmentIndex::Global(var_env) = var_env {
            // i. For each element name of varNames, do
            for name in &var_names {
                let name = String::from_str(agent, name.as_str());
                // 1. If varEnv.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
                // 2. NOTE: eval will not create a global var declaration that would be shadowed by a global lexical declaration.
                if var_env.has_lexical_declaration(agent, name) {
                    return Err(agent.throw_exception(
                        ExceptionType::SyntaxError,
                        format!(
                            "Redeclaration of lexical declaration '{}'",
                            name.as_str(agent)
                        ),
                    ));
                }
            }
        }

        // b. Let thisEnv be lexEnv.
        let mut this_env = lex_env;

        // c. Assert: The following loop will terminate.
        // d. Repeat, while thisEnv and varEnv are not the same Environment Record,
        while this_env != var_env {
            // i. If thisEnv is not an Object Environment Record, then
            if !matches!(this_env, EnvironmentIndex::Object(_)) {
                // 1. NOTE: The environment of with statements cannot contain any lexical declaration so it doesn't need to be checked for var/let hoisting conflicts.
                // 2. For each element name of varNames, do
                for name in &var_names {
                    let name = String::from_str(agent, name.as_str());
                    // a. If ! thisEnv.HasBinding(name) is true, then
                    // b. NOTE: A direct eval will not hoist var declaration over a like-named lexical declaration.
                    if this_env.has_binding(agent, name).unwrap() {
                        // i. Throw a SyntaxError exception.
                        // ii. NOTE: Annex B.3.4 defines alternate semantics for the above step.
                        return Err(agent.throw_exception(
                            ExceptionType::SyntaxError,
                            format!("Redeclaration of variable '{}'", name.as_str(agent)),
                        ));
                    }
                }
            }
            // ii. Set thisEnv to thisEnv.[[OuterEnv]].
            this_env = this_env.get_outer_env(agent).unwrap();
        }
    }

    // 4. Let privateIdentifiers be a new empty List.
    let mut private_identifiers = vec![];

    // 5. Let pointer be privateEnv.
    let mut pointer = private_env;

    // 6. Repeat, while pointer is not null,
    while let Some(index) = pointer {
        let env = &agent[index];

        // a. For each Private Name binding of pointer.[[Names]], do
        for name in env.names.values() {
            // i. If privateIdentifiers does not contain binding.[[Description]], append binding.[[Description]] to privateIdentifiers.
            if private_identifiers.contains(&name.description()) {
                private_identifiers.push(name.description());
            }
        }

        // b. Set pointer to pointer.[[OuterPrivateEnvironment]].
        pointer = env.outer_private_environment;
    }

    // TODO:
    // 7. If AllPrivateIdentifiersValid of body with argument privateIdentifiers is false, throw a SyntaxError exception.

    // 8. Let functionsToInitialize be a new empty List.
    let mut functions_to_initialize = vec![];
    // 9. Let declaredFunctionNames be a new empty List.
    let mut declared_function_names = AHashSet::default();

    // 10. For each element d of varDeclarations, in reverse List order, do
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
                // 1. If varEnv is a Global Environment Record, then
                if let EnvironmentIndex::Global(var_env) = var_env {
                    // a. Let fnDefinable be ? varEnv.CanDeclareGlobalFunction(fn).
                    let function_name = String::from_str(agent, function_name.as_str());
                    let fn_definable = var_env.can_declare_global_function(agent, function_name)?;

                    // b. If fnDefinable is false, throw a TypeError exception.
                    if !fn_definable {
                        return Err(agent.throw_exception(
                            ExceptionType::TypeError,
                            format!(
                                "Cannot declare global function '{}'.",
                                function_name.as_str(agent)
                            ),
                        ));
                    }
                }

                // 2. Append fn to declaredFunctionNames.
                // 3. Insert d as the first element of functionsToInitialize.
                functions_to_initialize.push(d);
            }
        }
    }

    // 11. Let declaredVarNames be a new empty List.
    let mut declared_var_names = AHashSet::default();

    // 12. For each element d of varDeclarations, do
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
                    let vn = String::from_str(agent, vn.as_str());
                    // a. If varEnv is a Global Environment Record, then
                    if let EnvironmentIndex::Global(var_env) = var_env {
                        // i. Let vnDefinable be ? varEnv.CanDeclareGlobalVar(vn).
                        let vn_definable = var_env.can_declare_global_var(agent, vn)?;
                        // ii. If vnDefinable is false, throw a TypeError exception.
                        if !vn_definable {
                            return Err(agent.throw_exception(
                                ExceptionType::TypeError,
                                format!("Cannot declare global variable '{}'.", vn.as_str(agent)),
                            ));
                        }
                    }
                    // b. If declaredVarNames does not contain vn, then
                    // i. Append vn to declaredVarNames.
                    declared_var_names.insert(vn);
                }
            }
        }
    }

    // 13. NOTE: Annex B.3.2.3 adds additional steps at this point.
    // 14. NOTE: No abnormal terminations occur after this algorithm step unless varEnv is a Global Environment Record and the global object is a Proxy exotic object.

    // 15. Let lexDeclarations be the LexicallyScopedDeclarations of body.
    let lex_declarations = script_lexically_scoped_declarations(script);

    // 16. For each element d of lexDeclarations, do
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
            // 1. Perform ? lexEnv.CreateImmutableBinding(dn, true).
            lex_env.create_immutable_binding(agent, dn, true)?;
        }
        for dn in bound_names {
            // ii. Else,
            // 1. Perform ? lexEnv.CreateMutableBinding(dn, false).
            lex_env.create_mutable_binding(agent, dn, false)?;
        }
    }

    // 17. For each Parse Node f of functionsToInitialize, do
    for f in functions_to_initialize {
        // a. Let fn be the sole element of the BoundNames of f.
        let mut function_name = None;
        f.bound_names(&mut |identifier| {
            assert!(function_name.is_none());
            function_name = Some(identifier.name.clone());
        });
        let function_name = String::from_str(agent, function_name.unwrap().as_str());

        // b. Let fo be InstantiateFunctionObject of f with arguments lexEnv and privateEnv.
        let fo = instantiate_function_object(agent, f, lex_env, private_env).into_value();

        // c. If varEnv is a Global Environment Record, then
        if let EnvironmentIndex::Global(var_env) = var_env {
            // i. Perform ? varEnv.CreateGlobalFunctionBinding(fn, fo, true).
            var_env.create_global_function_binding(agent, function_name, fo, true)?;
        } else {
            // d. Else,
            // i. Let bindingExists be ! varEnv.HasBinding(fn).
            let binding_exists = var_env.has_binding(agent, function_name).unwrap();

            // ii. If bindingExists is false, then
            if !binding_exists {
                // 1. NOTE: The following invocation cannot return an abrupt completion because of the validation preceding step 14.
                // 2. Perform ! varEnv.CreateMutableBinding(fn, true).
                var_env
                    .create_mutable_binding(agent, function_name, true)
                    .unwrap();
                // 3. Perform ! varEnv.InitializeBinding(fn, fo).
                var_env
                    .initialize_binding(agent, function_name, fo)
                    .unwrap();
            } else {
                // iii. Else,
                // 1. Perform ! varEnv.SetMutableBinding(fn, fo, false).
                var_env
                    .set_mutable_binding(agent, function_name, fo, false)
                    .unwrap();
            }
        }
    }
    // 18. For each String vn of declaredVarNames, do
    for vn in declared_var_names {
        // a. If varEnv is a Global Environment Record, then
        if let EnvironmentIndex::Global(var_env) = var_env {
            // i. Perform ? varEnv.CreateGlobalVarBinding(vn, true).
            var_env.create_global_var_binding(agent, vn, true)?;
        } else {
            // b. Else,
            // i. Let bindingExists be ! varEnv.HasBinding(vn).
            let binding_exists = var_env.has_binding(agent, vn).unwrap();

            // ii. If bindingExists is false, then
            if !binding_exists {
                // 1. NOTE: The following invocation cannot return an abrupt completion because of the validation preceding step 14.
                // 2. Perform ! varEnv.CreateMutableBinding(vn, true).
                var_env.create_mutable_binding(agent, vn, true).unwrap();
                // 3. Perform ! varEnv.InitializeBinding(vn, undefined).
                var_env
                    .initialize_binding(agent, vn, Value::Undefined)
                    .unwrap();
            }
        }
    }

    // 19. Return UNUSED.
    Ok(())
}

impl GlobalObject {
    /// ### [19.2.1 eval ( x )](https://tc39.es/ecma262/#sec-eval-x)
    ///
    /// This function is the %eval% intrinsic object.
    fn eval(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let x = arguments.get(0);

        // 1. Return ? PerformEval(x, false, false).
        perform_eval(agent, x, false, false)
    }

    /// ### [19.2.2 isFinite ( number )](https://tc39.es/ecma262/#sec-isfinite-number)
    ///
    /// This function is the %isFinite% intrinsic object.
    fn is_finite(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let number = arguments.get(0);
        // 1. Let num be ? ToNumber(number).
        let num = to_number(agent, number)?;
        // 2. If num is not finite, return false.
        // 3. Otherwise, return true.
        Ok(num.is_finite(agent).into())
    }

    /// ### [19.2.3 isNaN ( number )](https://tc39.es/ecma262/#sec-isnan-number)
    ///
    /// This function is the %isNaN% intrinsic object.
    ///
    /// > NOTE: A reliable way for ECMAScript code to test if a value X is NaN
    /// > is an expression of the form X !== X. The result will be true if and
    /// > only if X is NaN.
    fn is_nan(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let number = arguments.get(0);
        // 1. Let num be ? ToNumber(number).
        let num = to_number(agent, number)?;
        // 2. If num is NaN, return true.
        // 3. Otherwise, return false.
        Ok(num.is_nan(agent).into())
    }
    fn parse_float(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn parse_int(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn decode_uri(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn decode_uri_component(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn encode_uri(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn encode_uri_component(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn escape(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn unescape(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEval>(agent, realm).build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectIsFinite>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectIsNaN>(agent, realm).build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectParseFloat>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectParseInt>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectDecodeURI>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectDecodeURIComponent>(
            agent, realm,
        )
        .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEncodeURI>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEncodeURIComponent>(
            agent, realm,
        )
        .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEscape>(agent, realm).build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectUnescape>(agent, realm)
            .build();
    }
}
