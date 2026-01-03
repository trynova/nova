// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::VecDeque;

use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::try_define_property_or_throw,
        builtins::{
            ArgumentsList, ECMAScriptFunction, FunctionAstRef, OrdinaryFunctionCreateParams,
            ThisMode,
            async_generator_objects::{AsyncGeneratorHeapData, AsyncGeneratorState},
            control_abstraction_objects::{
                async_function_objects::await_reaction::AwaitReactionRecord,
                generator_objects::GeneratorState,
                promise_objects::{
                    promise_abstract_operations::{
                        promise_capability_records::PromiseCapability,
                        promise_reaction_records::PromiseReactionHandler,
                    },
                    promise_prototype::inner_promise_then,
                },
            },
            generator_objects::{GeneratorHeapData, SuspendedGeneratorState},
            make_constructor,
            ordinary::{
                ordinary_object_create_with_intrinsics, ordinary_populate_from_constructor,
            },
            ordinary_function_create,
            promise::Promise,
            set_function_name,
        },
        execution::{
            Agent, Environment, JsResult, PrivateEnvironment, ProtoIntrinsics, agent::unwrap_try,
        },
        scripts_and_modules::source_code::SourceCode,
        types::{BUILTIN_STRING_MEMORY, PropertyDescriptor, PropertyKey, Value},
    },
    engine::{
        Executable, ExecutionResult, Vm,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::CreateHeapData,
};
use oxc_ast::ast::{self};

/// ### [15.1.2 Static Semantics: ContainsExpression](https://tc39.es/ecma262/#sec-static-semantics-containsexpression)
/// The syntax-directed operation ContainsExpression takes no arguments and returns a Boolean.
pub(crate) trait ContainsExpression {
    fn contains_expression(&self) -> bool;
}

impl ContainsExpression for ast::BindingPattern<'_> {
    fn contains_expression(&self) -> bool {
        match &self.kind {
            ast::BindingPatternKind::BindingIdentifier(_) => false,
            ast::BindingPatternKind::ObjectPattern(pattern) => pattern.contains_expression(),
            ast::BindingPatternKind::ArrayPattern(pattern) => pattern.contains_expression(),
            ast::BindingPatternKind::AssignmentPattern(_) => true,
        }
    }
}

impl ContainsExpression for ast::ObjectPattern<'_> {
    fn contains_expression(&self) -> bool {
        for property in &self.properties {
            if property.computed || property.value.contains_expression() {
                return true;
            }
        }

        if let Some(rest) = &self.rest {
            debug_assert!(!rest.argument.contains_expression());
        }

        false
    }
}

impl ContainsExpression for ast::ArrayPattern<'_> {
    fn contains_expression(&self) -> bool {
        for pattern in self.elements.iter().flatten() {
            if pattern.contains_expression() {
                return true;
            }
        }
        if let Some(rest) = &self.rest {
            rest.argument.contains_expression()
        } else {
            false
        }
    }
}

/// ### [15.2.4 Runtime Semantics: InstantiateOrdinaryFunctionObject](https://tc39.es/ecma262/#sec-runtime-semantics-instantiateordinaryfunctionobject)
///
/// The syntax-directed operation InstantiateOrdinaryFunctionObject takes
/// arguments env (an Environment Record) and privateEnv (a PrivateEnvironment
/// Record or null) and returns an ECMAScript function object.
pub(crate) fn instantiate_ordinary_function_object<'a>(
    agent: &mut Agent,
    function: &ast::Function<'_>,
    env: Environment<'a>,
    private_env: Option<PrivateEnvironment<'a>>,
    gc: NoGcScope<'a, '_>,
) -> ECMAScriptFunction<'a> {
    // FunctionDeclaration : function BindingIdentifier ( FormalParameters ) { FunctionBody }
    let pk_name = if let Some(id) = &function.id {
        // 1. Let name be StringValue of BindingIdentifier.
        let name = &id.name;
        // 4. Perform SetFunctionName(F, name).
        PropertyKey::from_str(agent, name, gc)
    } else {
        // 3. Perform SetFunctionName(F, "default").
        PropertyKey::from(BUILTIN_STRING_MEMORY.default)
    };

    // 2. Let sourceText be the source text matched by FunctionDeclaration.
    let source_text = function.span;
    // 3. Let F be OrdinaryFunctionCreate(%Function.prototype%, sourceText, FormalParameters, FunctionBody, NON-LEXICAL-THIS, env, privateEnv).
    let params = OrdinaryFunctionCreateParams {
        function_prototype: None,
        source_code: None,
        source_text,
        ast: FunctionAstRef::from(function),
        lexical_this: false,
        env,
        private_env,
    };
    let f = ordinary_function_create(agent, params, gc);

    // 4. Perform SetFunctionName(F, name).
    set_function_name(agent, f, pk_name, None, gc);
    // 5. Perform MakeConstructor(F).
    if !function.r#async && !function.generator {
        make_constructor(agent, f, None, None, gc);
    }

    if function.generator {
        // InstantiateGeneratorFunctionObject
        // 5. Let prototype be OrdinaryObjectCreate(%GeneratorFunction.prototype.prototype%).

        // InstantiateAsyncGeneratorFunctionObject
        // 5. Let prototype be OrdinaryObjectCreate(%AsyncGeneratorPrototype%).

        // NOTE: Although `prototype` has the generator prototype, it doesn't have the generator
        // internals slots, so it's created as an ordinary object.
        let prototype = ordinary_object_create_with_intrinsics(
            agent,
            ProtoIntrinsics::Object,
            Some(if function.r#async {
                agent
                    .current_realm_record()
                    .intrinsics()
                    .async_generator_prototype()
                    .into()
            } else {
                agent
                    .current_realm_record()
                    .intrinsics()
                    .generator_prototype()
                    .into()
            }),
            gc,
        );
        // 6. Perform ! DefinePropertyOrThrow(F, "prototype", PropertyDescriptor {
        unwrap_try(try_define_property_or_throw(
            agent,
            f,
            BUILTIN_STRING_MEMORY.prototype.to_property_key(),
            PropertyDescriptor {
                // [[Value]]: prototype,
                value: Some(prototype.into().unbind()),
                // [[Writable]]: true,
                writable: Some(true),
                // [[Enumerable]]: false,
                enumerable: Some(false),
                // [[Configurable]]: false
                configurable: Some(false),
                ..Default::default()
            },
            None,
            gc,
        ));
        // }).
    }

    // 6. Return F.
    f
    // NOTE
    // An anonymous FunctionDeclaration can only occur as part of an export
    // default declaration, and its function code is therefore always strict
    // mode code.
}

pub(crate) struct CompileFunctionBodyData<'a> {
    pub(crate) ast: FunctionAstRef<'a>,
    pub(crate) source_code: SourceCode<'a>,
    pub(crate) is_strict: bool,
    pub(crate) is_lexical: bool,
}

impl<'a> CompileFunctionBodyData<'a> {
    fn new(agent: &mut Agent, function: ECMAScriptFunction<'a>, gc: NoGcScope<'a, '_>) -> Self {
        let ast = function.get_ast(agent, gc);
        let source_code = function.get_source_code(agent);
        let is_strict = function.is_strict(agent);
        let this_mode = function.get_this_mode(agent);
        CompileFunctionBodyData {
            ast,
            source_code,
            is_strict,
            is_lexical: this_mode == ThisMode::Lexical,
        }
    }
}

/// ### [15.2.3 Runtime Semantics: EvaluateFunctionBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluatefunctionbody)
/// The syntax-directed operation EvaluateFunctionBody takes arguments
/// functionObject (an ECMAScript function object) and argumentsList (a List of
/// ECMAScript language values) and returns either a normal completion
/// containing an ECMAScript language value or an abrupt completion.
pub(crate) fn evaluate_function_body<'gc>(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let arguments_list = arguments_list.bind(gc.nogc());
    let function_object = function_object.bind(gc.nogc());
    // 1. Perform ? FunctionDeclarationInstantiation(functionObject, argumentsList).
    //function_declaration_instantiation(agent, function_object, arguments_list).unbind()?.bind(gc.nogc());
    // 2. Return ? Evaluation of FunctionStatementList.
    let exe = if let Some(exe) = agent[function_object].compiled_bytecode {
        exe.bind(gc.nogc())
    } else {
        let data = CompileFunctionBodyData::new(agent, function_object, gc.nogc());
        let exe = Executable::compile_function_body(agent, data, gc.nogc());
        agent[function_object].compiled_bytecode = Some(exe.unbind());
        exe
    };
    let exe = exe.scope(agent, gc.nogc());
    Vm::execute(agent, exe, Some(arguments_list.unbind().as_mut_slice()), gc).into_js_result()
}

/// ### [15.8.4 Runtime Semantics: EvaluateAsyncFunctionBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluateasyncfunctionbody)
pub(crate) fn evaluate_async_function_body<'a>(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
    mut gc: GcScope<'a, '_>,
) -> Promise<'a> {
    let arguments_list = arguments_list.bind(gc.nogc());
    let function_object = function_object.bind(gc.nogc());
    let scoped_function_object = function_object.scope(agent, gc.nogc());
    // 1. Let promiseCapability be ! NewPromiseCapability(%Promise%).
    let PromiseCapability {
        promise,
        must_be_unresolved,
    } = PromiseCapability::new(agent, gc.nogc());
    let promise = promise.scope(agent, gc.nogc());
    // 2. Let declResult be Completion(FunctionDeclarationInstantiation(functionObject, argumentsList)).
    // 3. If declResult is an abrupt completion, then
    // 4. Else,
    // a. Perform AsyncFunctionStart(promiseCapability, FunctionBody).
    // Note: FunctionDeclarationInstantiation is performed as the first part of
    // the compiled function body; we do not need to run it and
    // AsyncFunctionStart separately.
    let exe = if let Some(exe) = agent[function_object].compiled_bytecode {
        exe.bind(gc.nogc())
    } else {
        let data = CompileFunctionBodyData::new(agent, function_object, gc.nogc());
        let exe = Executable::compile_function_body(agent, data, gc.nogc());
        agent[function_object].compiled_bytecode = Some(exe.unbind());
        exe
    };
    let exe = exe.scope(agent, gc.nogc());

    // AsyncFunctionStart will run the function until it returns, throws or
    // gets suspended with an await.
    match Vm::execute(
        agent,
        exe,
        Some(arguments_list.unbind().as_mut_slice()),
        gc.reborrow(),
    ) {
        ExecutionResult::Return(result) => {
            let result = result.unbind().bind(gc.nogc());
            let promise = promise.get(agent).bind(gc.nogc());
            let promise_capability = PromiseCapability::from_promise(promise, must_be_unresolved);
            // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
            // 2. e. If result is a normal completion, then
            //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « undefined »).
            //    f. Else if result is a return completion, then
            //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « result.[[Value]] »).
            promise_capability
                .unbind()
                .resolve(agent, result.unbind(), gc.reborrow());
        }
        ExecutionResult::Throw(err) => {
            let err = err.unbind().bind(gc.nogc());
            let promise = promise.get(agent).bind(gc.nogc());
            let promise_capability = PromiseCapability::from_promise(promise, must_be_unresolved);
            // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
            // 2. g. i. Assert: result is a throw completion.
            //       ii. Perform ! Call(promiseCapability.[[Reject]], undefined, « result.[[Value]] »).
            promise_capability.reject(agent, err.value(), gc.nogc());
        }
        ExecutionResult::Await { vm, awaited_value } => {
            // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
            // `handler` corresponds to the `fulfilledClosure` and `rejectedClosure` functions,
            // which resume execution of the function.
            // 2. Let promise be ? PromiseResolve(%Promise%, value).
            let resolve_promise = Promise::resolve(agent, awaited_value.unbind(), gc.reborrow())
                .unbind()
                .bind(gc.nogc());

            let promise = promise.get(agent).bind(gc.nogc());
            let promise_capability = PromiseCapability::from_promise(promise, must_be_unresolved);

            // NOTE: the execution context has to be cloned because it will be popped when we
            // return to `ECMAScriptFunction::internal_call`. Popping it here rather than
            // cloning it would mess up the execution context stack.
            let handler = PromiseReactionHandler::Await(agent.heap.create(AwaitReactionRecord {
                vm: Some(vm),
                async_executable: Some(scoped_function_object.get(agent).into()),
                execution_context: Some(agent.running_execution_context().clone()),
                return_promise_capability: promise_capability,
            }));

            // 7. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
            inner_promise_then(
                agent,
                resolve_promise.unbind(),
                handler,
                handler,
                None,
                gc.nogc(),
            );
        }
        ExecutionResult::Yield { .. } => unreachable!(),
    }
    //}

    // 5. Return Completion Record { [[Type]]: return, [[Value]]: promiseCapability.[[Promise]], [[Target]]: empty }.
    promise.get(agent).bind(gc.into_nogc())
}

/// ### [15.5.2 Runtime Semantics: EvaluateGeneratorBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluategeneratorbody)
/// The syntax-directed operation EvaluateGeneratorBody takes arguments
/// functionObject (an ECMAScript function object) and argumentsList (a List of
/// ECMAScript language values) and returns a throw completion or a return
/// completion.
pub(crate) fn evaluate_generator_body<'gc>(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let arguments_list = arguments_list.bind(gc.nogc());
    let function_object = function_object.bind(gc.nogc());

    let exe = if let Some(exe) = agent[function_object].compiled_bytecode {
        exe.scope(agent, gc.nogc())
    } else {
        let data = CompileFunctionBodyData::new(agent, function_object, gc.nogc());
        let exe = Executable::compile_function_body(agent, data, gc.nogc());
        agent[function_object].compiled_bytecode = Some(exe.unbind());
        exe.scope(agent, gc.nogc())
    };

    let function_object = function_object.scope(agent, gc.nogc());

    // 1. Perform ? FunctionDeclarationInstantiation(functionObject, argumentsList).
    // Note: FunctionDeclarationInstantiation is done at the beginning of the
    // bytecode, followed by a Yield.
    let vm = match Vm::execute(
        agent,
        exe.clone(),
        Some(arguments_list.unbind().as_mut_slice()),
        gc.reborrow(),
    ) {
        ExecutionResult::Throw(err) => {
            return Err(err.unbind().bind(gc.into_nogc()));
        }
        ExecutionResult::Yield { vm, yielded_value } => {
            debug_assert!(yielded_value.is_undefined());
            vm
        }
        _ => unreachable!(),
    };

    // 2. Let G be ? OrdinaryCreateFromConstructor(functionObject,
    // "%GeneratorFunction.prototype.prototype%", « [[GeneratorState]],
    // [[GeneratorContext]], [[GeneratorBrand]] »).
    // 3. Set G.[[GeneratorBrand]] to empty.
    // 4. Perform GeneratorStart(G, FunctionBody).
    // 5. Return Completion Record { [[Type]]: return, [[Value]]: G, [[Target]]: empty }.
    let g = agent
        .heap
        .create(GeneratorHeapData {
            object_index: None,
            generator_state: Some(GeneratorState::SuspendedStart(SuspendedGeneratorState {
                vm,
                // SAFETY: exe is not shared.
                executable: unsafe { exe.take(agent) },
                execution_context: agent.running_execution_context().clone(),
            })),
        })
        .bind(gc.nogc());
    ordinary_populate_from_constructor(
        agent,
        g.into().unbind(),
        // SAFETY: not shared.
        unsafe { function_object.take(agent) }.into(),
        ProtoIntrinsics::Generator,
        gc,
    )
    .map(Into::into)
}

/// ### [15.6.2 Runtime Semantics: EvaluateAsyncGeneratorBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluateasyncgeneratorbody)
///
/// The syntax-directed operation EvaluateAsyncGeneratorBody takes arguments
/// functionObject (an ECMAScript function object) and argumentsList (a List of
/// ECMAScript language values) and returns a throw completion or a return
/// completion.
pub(crate) fn evaluate_async_generator_body<'gc>(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let function_object = function_object.bind(gc.nogc());
    let arguments_list = arguments_list.bind(gc.nogc());

    let exe = if let Some(exe) = agent[function_object].compiled_bytecode {
        exe.scope(agent, gc.nogc())
    } else {
        let data = CompileFunctionBodyData::new(agent, function_object, gc.nogc());
        let exe = Executable::compile_function_body(agent, data, gc.nogc());
        agent[function_object].compiled_bytecode = Some(exe.unbind());
        exe.scope(agent, gc.nogc())
    };

    let function_object = function_object.scope(agent, gc.nogc());

    // 1. Perform ? FunctionDeclarationInstantiation(functionObject, argumentsList).
    // Note: FunctionDeclarationInstantiation is done at the beginning of the
    // bytecode, followed by a Yield.
    let vm = match Vm::execute(
        agent,
        exe.clone(),
        Some(arguments_list.unbind().as_mut_slice()),
        gc.reborrow(),
    ) {
        ExecutionResult::Throw(err) => {
            return Err(err.unbind().bind(gc.into_nogc()));
        }
        ExecutionResult::Yield { vm, yielded_value } => {
            debug_assert!(yielded_value.is_undefined());
            vm
        }
        _ => unreachable!(),
    };

    // 2. Let generator be ? OrdinaryCreateFromConstructor(functionObject,
    //    "%AsyncGeneratorPrototype%", « [[AsyncGeneratorState]],
    //    [[AsyncGeneratorContext]], [[AsyncGeneratorQueue]],
    //    [[GeneratorBrand]] »).
    // 3. Set generator.[[GeneratorBrand]] to empty.
    // 4. Set generator.[[AsyncGeneratorState]] to suspended-start.
    // 5. Perform AsyncGeneratorStart(generator, FunctionBody).
    // 6. Return ReturnCompletion(generator).
    let generator = agent
        .heap
        .create(AsyncGeneratorHeapData {
            object_index: None,
            // SAFETY: exe is not shared.
            executable: Some(unsafe { exe.take(agent) }),
            async_generator_state: Some(AsyncGeneratorState::SuspendedStart {
                vm,
                execution_context: agent.running_execution_context().clone(),
                queue: VecDeque::new(),
            }),
        })
        .bind(gc.nogc());
    ordinary_populate_from_constructor(
        agent,
        generator.into().unbind(),
        // SAFETY: not shared.
        unsafe { function_object.take(agent) }.into(),
        ProtoIntrinsics::AsyncGenerator,
        gc,
    )
    .map(Into::into)
}
