// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            control_abstraction_objects::{
                async_function_objects::await_reaction::AwaitReaction,
                promise_objects::{
                    promise_abstract_operations::{
                        promise_capability_records::PromiseCapability,
                        promise_reaction_records::PromiseReactionHandler,
                    },
                    promise_prototype::inner_promise_then,
                },
            },
            function_declaration_instantiation, make_constructor, ordinary_function_create,
            promise::Promise,
            set_function_name, ArgumentsList, ECMAScriptFunction, OrdinaryFunctionCreateParams,
        },
        execution::{
            Agent, ECMAScriptCodeEvaluationState, EnvironmentIndex, JsResult,
            PrivateEnvironmentIndex,
        },
        types::{PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::{Executable, ExecutionResult, FunctionExpression, Vm},
    heap::CreateHeapData,
};
use oxc_ast::ast::{self};

/// ### [15.2.4 Runtime Semantics: InstantiateOrdinaryFunctionObject](https://tc39.es/ecma262/#sec-runtime-semantics-instantiateordinaryfunctionobject)
///
/// The syntax-directed operation InstantiateOrdinaryFunctionObject takes
/// arguments env (an Environment Record) and privateEnv (a PrivateEnvironment
/// Record or null) and returns an ECMAScript function object.
pub(crate) fn instantiate_ordinary_function_object(
    agent: &mut Agent,
    function: &ast::Function<'_>,
    env: EnvironmentIndex,
    private_env: Option<PrivateEnvironmentIndex>,
) -> ECMAScriptFunction {
    // FunctionDeclaration : function BindingIdentifier ( FormalParameters ) { FunctionBody }
    if let Some(id) = &function.id {
        // 1. Let name be StringValue of BindingIdentifier.
        let name = &id.name;
        // 2. Let sourceText be the source text matched by FunctionDeclaration.
        let source_text = function.span;
        // 3. Let F be OrdinaryFunctionCreate(%Function.prototype%, sourceText, FormalParameters, FunctionBody, NON-LEXICAL-THIS, env, privateEnv).
        let params = OrdinaryFunctionCreateParams {
            function_prototype: None,
            source_text,
            parameters_list: &function.params,
            body: function.body.as_deref().unwrap(),
            is_concise_arrow_function: false,
            is_async: function.r#async,
            is_generator: function.generator,
            lexical_this: false,
            env,
            private_env,
        };
        let f = ordinary_function_create(agent, params);

        // 4. Perform SetFunctionName(F, name).
        let pk_name = PropertyKey::from_str(agent, name);
        set_function_name(agent, f, pk_name, None);
        // 5. Perform MakeConstructor(F).
        if !function.r#async && !function.generator {
            make_constructor(agent, f, None, None);
        }
        // 6. Return F.
        f
    } else {
        // FunctionDeclaration : function ( FormalParameters ) { FunctionBody }
        // 1. Let sourceText be the source text matched by FunctionDeclaration.
        let source_text = function.span;
        // 2. Let F be OrdinaryFunctionCreate(%Function.prototype%, sourceText, FormalParameters, FunctionBody, NON-LEXICAL-THIS, env, privateEnv).
        let params = OrdinaryFunctionCreateParams {
            function_prototype: None,
            source_text,
            parameters_list: &function.params,
            body: function.body.as_ref().unwrap(),
            is_concise_arrow_function: false,
            is_async: function.r#async,
            is_generator: function.generator,
            lexical_this: false,
            env,
            private_env,
        };
        let f = ordinary_function_create(agent, params);

        // 3. Perform SetFunctionName(F, "default").
        let pk_name = PropertyKey::from(BUILTIN_STRING_MEMORY.default);
        set_function_name(agent, f, pk_name, None);
        // 4. Perform MakeConstructor(F).
        if !function.r#async && !function.generator {
            make_constructor(agent, f, None, None);
        }
        // 5. Return F.
        f
    }
    // NOTE
    // An anonymous FunctionDeclaration can only occur as part of an export default declaration, and its function code is therefore always strict mode code.
}

// 15.2.5 Runtime Semantics: InstantiateOrdinaryFunctionExpression
// The syntax-directed operation InstantiateOrdinaryFunctionExpression takes optional argument name (a property key or a Private Name) and returns an ECMAScript function object. It is defined piecewise over the following productions:

pub(crate) fn instantiate_ordinary_function_expression(
    agent: &mut Agent,
    function: &FunctionExpression,
    name: Option<String>,
) -> ECMAScriptFunction {
    if let Some(_identifier) = function.identifier {
        todo!();
    } else {
        // 1. If name is not present, set name to "".
        let name = name.map_or_else(|| String::EMPTY_STRING, |name| name);
        // 2. Let env be the LexicalEnvironment of the running execution context.
        // 3. Let privateEnv be the running execution context's PrivateEnvironment.
        let ECMAScriptCodeEvaluationState {
            lexical_environment,
            private_environment,
            ..
        } = *agent
            .running_execution_context()
            .ecmascript_code
            .as_ref()
            .unwrap();
        // 4. Let sourceText be the source text matched by FunctionExpression.
        let source_text = function.expression.get().span;
        // 5. Let closure be OrdinaryFunctionCreate(%Function.prototype%, sourceText, FormalParameters, FunctionBody, NON-LEXICAL-THIS, env, privateEnv).
        let params = OrdinaryFunctionCreateParams {
            function_prototype: None,
            source_text,
            parameters_list: &function.expression.get().params,
            body: function.expression.get().body.as_ref().unwrap(),
            is_concise_arrow_function: false,
            is_async: function.expression.get().r#async,
            is_generator: function.expression.get().generator,
            lexical_this: false,
            env: lexical_environment,
            private_env: private_environment,
        };
        let closure = ordinary_function_create(agent, params);
        // 6. Perform SetFunctionName(closure, name).
        let name = PropertyKey::from(name);
        set_function_name(agent, closure, name, None);
        // 7. Perform MakeConstructor(closure).
        if !function.expression.get().r#async && !function.expression.get().generator {
            make_constructor(agent, closure, None, None);
        }
        // 8. Return closure.
        closure
    }
}

/// ### [15.2.3 Runtime Semantics: EvaluateFunctionBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluatefunctionbody)
/// The syntax-directed operation EvaluateFunctionBody takes arguments
/// functionObject (an ECMAScript function object) and argumentsList (a List of
/// ECMAScript language values) and returns either a normal completion
/// containing an ECMAScript language value or an abrupt completion.
pub(crate) fn evaluate_function_body(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
) -> JsResult<Value> {
    // 1. Perform ? FunctionDeclarationInstantiation(functionObject, argumentsList).
    function_declaration_instantiation(agent, function_object, arguments_list)?;
    // 2. Return ? Evaluation of FunctionStatementList.
    // SAFETY: We're alive so SourceCode must be too.
    let body = unsafe {
        agent[function_object]
            .ecmascript_function
            .ecmascript_code
            .as_ref()
    };
    let is_concise_arrow_function = agent[function_object]
        .ecmascript_function
        .is_concise_arrow_function;
    let exe = Executable::compile_function_body(agent, body, is_concise_arrow_function);
    Vm::execute(agent, &exe).into_js_result()
}

/// ### [15.8.4 Runtime Semantics: EvaluateAsyncFunctionBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluateasyncfunctionbody)
pub(crate) fn evaluate_async_function_body(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
) -> Promise {
    // 1. Let promiseCapability be ! NewPromiseCapability(%Promise%).
    let promise_capability = PromiseCapability::new(agent);
    // 2. Let declResult be Completion(FunctionDeclarationInstantiation(functionObject, argumentsList)).
    // 3. If declResult is an abrupt completion, then
    if let Err(err) = function_declaration_instantiation(agent, function_object, arguments_list) {
        // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « declResult.[[Value]] »).
        promise_capability.reject(agent, err.value());
    } else {
        // 4. Else,
        // a. Perform AsyncFunctionStart(promiseCapability, FunctionBody).
        let body = unsafe {
            agent[function_object]
                .ecmascript_function
                .ecmascript_code
                .as_ref()
        };
        let is_concise_arrow_function = agent[function_object]
            .ecmascript_function
            .is_concise_arrow_function;
        let exe = Executable::compile_function_body(agent, body, is_concise_arrow_function);

        // AsyncFunctionStart will run the function until it returns, throws or gets suspended with
        // an await.
        match Vm::execute(agent, &exe) {
            ExecutionResult::Return(result) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. e. If result is a normal completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « undefined »).
                //    f. Else if result is a return completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « result.[[Value]] »).
                promise_capability.resolve(agent, result);
            }
            ExecutionResult::Throw(err) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. g. i. Assert: result is a throw completion.
                //       ii. Perform ! Call(promiseCapability.[[Reject]], undefined, « result.[[Value]] »).
                promise_capability.reject(agent, err.value());
            }
            ExecutionResult::Await { vm, awaited_value } => {
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // `handler` corresponds to the `fulfilledClosure` and `rejectedClosure` functions,
                // which resume execution of the function.
                // NOTE: the execution context has to be cloned because it will be popped when we
                // return to `ECMAScriptFunction::internal_call`. Popping it here rather than
                // cloning it would mess up the execution context stack.
                let handler = PromiseReactionHandler::Await(agent.heap.create(AwaitReaction {
                    vm: Some(vm),
                    executable: Some(exe),
                    execution_context: Some(agent.running_execution_context().clone()),
                    return_promise_capability: promise_capability,
                }));
                // 2. Let promise be ? PromiseResolve(%Promise%, value).
                let promise = Promise::resolve(agent, awaited_value);
                // 7. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
                inner_promise_then(agent, promise, handler, handler, None);
            }
        }
    }

    // 5. Return Completion Record { [[Type]]: return, [[Value]]: promiseCapability.[[Promise]], [[Target]]: empty }.
    promise_capability.promise()
}
