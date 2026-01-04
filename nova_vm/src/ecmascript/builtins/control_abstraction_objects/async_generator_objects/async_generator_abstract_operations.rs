// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::create_iter_result_object,
        builtins::{
            ECMAScriptFunction,
            promise::Promise,
            promise_objects::{
                promise_abstract_operations::{
                    promise_capability_records::PromiseCapability,
                    promise_reaction_records::PromiseReactionHandler,
                },
                promise_prototype::inner_promise_then,
            },
        },
        execution::{
            Agent, JsResult, Realm,
            agent::{ExceptionType, JsError, unwrap_try},
        },
        types::Value,
    },
    engine::{
        ExecutionResult, Scoped, SuspendedVm,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
};

use super::{
    AsyncGenerator, AsyncGeneratorAwaitKind, AsyncGeneratorRequest,
    AsyncGeneratorRequestCompletion, AsyncGeneratorState,
};

/// ### [27.6.3.3 AsyncGeneratorValidate ( generator, generatorBrand )](https://tc39.es/ecma262/#sec-asyncgeneratorvalidate)
///
/// The abstract operation AsyncGeneratorValidate takes arguments generator (an
/// ECMAScript language value) and generatorBrand (a String or empty) and
/// returns either a normal completion containing unused or a throw completion.
pub(super) fn async_generator_validate<'a>(
    agent: &mut Agent,
    generator: Value,
    _generator_brand: (),
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, AsyncGenerator<'a>> {
    // 1. Perform ? RequireInternalSlot(generator, [[AsyncGeneratorContext]]).
    // 2. Perform ? RequireInternalSlot(generator, [[AsyncGeneratorState]]).
    // 3. Perform ? RequireInternalSlot(generator, [[AsyncGeneratorQueue]]).
    // 4. If generator.[[GeneratorBrand]] is not generatorBrand, throw a TypeError exception.
    // 5. Return unused.
    if let Value::AsyncGenerator(generator) = generator {
        Ok(generator.unbind())
    } else {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Not an async generator object",
            gc,
        ))
    }
}

/// ### [27.6.3.4 AsyncGeneratorEnqueue ( generator, completion, promiseCapability )](https://tc39.es/ecma262/#sec-asyncgeneratorenqueue)
///
/// The abstract operation AsyncGeneratorEnqueue takes arguments generator (an
/// AsyncGenerator), completion (a Completion Record), and promiseCapability
/// (a PromiseCapability Record) and returns unused.
pub(super) fn async_generator_enqueue(
    agent: &mut Agent,
    generator: AsyncGenerator,
    completion: AsyncGeneratorRequestCompletion,
    promise_capability: PromiseCapability,
) {
    // 1. Let request be AsyncGeneratorRequest { [[Completion]]: completion, [[Capability]]: promiseCapability }.
    let request = AsyncGeneratorRequest {
        completion: completion.unbind(),
        capability: promise_capability,
    };
    // 2. Append request to generator.[[AsyncGeneratorQueue]].
    generator.append_to_queue(agent, request);
    // 3. Return unused.
}

/// ### [27.6.3.5 AsyncGeneratorCompleteStep ( generator, completion, done \[ , realm \] )](https://tc39.es/ecma262/#sec-asyncgeneratorcompletestep)
///
/// The abstract operation AsyncGeneratorCompleteStep takes arguments generator
/// (an AsyncGenerator), completion (a Completion Record), and done (a Boolean)
/// and optional argument realm (a Realm Record) and returns unused.
fn async_generator_complete_step(
    agent: &mut Agent,
    generator: AsyncGenerator,
    completion: AsyncGeneratorRequestCompletion,
    done: bool,
    realm: Option<Realm>,
    gc: NoGcScope,
) {
    let completion = completion.bind(gc);
    // 1. Assert: generator.[[AsyncGeneratorQueue]] is not empty.
    assert!(!generator.queue_is_empty(agent));
    // 2. Let next be the first element of generator.[[AsyncGeneratorQueue]].
    // 3. Remove the first element from generator.[[AsyncGeneratorQueue]].
    let next = generator.pop_first(agent, gc);
    // 4. Let promiseCapability be next.[[Capability]].
    let promise_capability = next.capability;
    // 5. Let value be completion.[[Value]].
    let value = match completion {
        AsyncGeneratorRequestCompletion::Ok(value) => value,
        // 6. If completion is a throw completion, then
        // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « value »).
        AsyncGeneratorRequestCompletion::Err(err) => {
            promise_capability.reject(agent, err.value(), gc);
            // 8. Return unused.
            return;
        }
        // 7. Else,
        // a. Assert: completion is a normal completion.
        _ => unreachable!(),
    };
    // b. If realm is present, then
    let iterator_result = if let Some(realm) = realm {
        // i. Let oldRealm be the running execution context's Realm.
        let old_realm = agent.current_realm(gc);
        let set_realm = realm != old_realm;
        // ii. Set the running execution context's Realm to realm.
        if set_realm {
            agent.set_current_realm(realm);
        }
        // iii. Let iteratorResult be CreateIteratorResultObject(value, done).
        let iterator_result =
            create_iter_result_object(agent, value, done, gc).expect("Should perform GC here");
        // iv. Set the running execution context's Realm to oldRealm.
        if set_realm {
            agent.set_current_realm(old_realm);
        }
        iterator_result
    } else {
        // c. Else,
        // i. Let iteratorResult be CreateIteratorResultObject(value, done).
        create_iter_result_object(agent, value, done, gc).expect("Should perform GC here")
    };
    // d. Perform ! Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »).
    unwrap_try(promise_capability.try_resolve(agent, iterator_result.into(), gc));
    // 8. Return unused.
}

/// ### [27.6.3.6 AsyncGeneratorResume ( generator, completion )](https://tc39.es/ecma262/#sec-asyncgeneratorresume)
///
/// The abstract operation AsyncGeneratorResume takes arguments generator (an
/// AsyncGenerator) and completion (a Completion Record) and returns unused.
pub(super) fn async_generator_resume(
    agent: &mut Agent,
    generator: AsyncGenerator,
    completion: AsyncGeneratorRequestCompletion,
    mut gc: GcScope,
) {
    let nogc = gc.nogc();
    let generator = generator.bind(nogc);
    let completion = completion.bind(nogc);
    // 1. Assert: generator.[[AsyncGeneratorState]] is either suspended-start or suspended-yield.
    // 2. Let genContext be generator.[[AsyncGeneratorContext]].
    // 5. Set generator.[[AsyncGeneratorState]] to executing.
    assert!(generator.is_suspended_start(agent) || generator.is_suspended_yield(agent));
    let (vm, gen_context, executable) = generator.transition_to_executing(agent, gc.nogc());
    let executable = executable.scope(agent, gc.nogc());

    // 3. Let callerContext be the running execution context.
    // 4. Suspend callerContext.
    // 6. Push genContext onto the execution context stack; genContext is now
    //    the running execution context.
    agent.push_execution_context(gen_context);

    let scoped_generator = generator.scope(agent, nogc);

    // 7. Resume the suspended evaluation of genContext using completion as the
    //    result of the operation that suspended it. Let result be the
    //    Completion Record returned by the resumed computation.
    let execution_result = match completion {
        AsyncGeneratorRequestCompletion::Ok(value) => {
            vm.resume(agent, executable, value.unbind(), gc.reborrow())
        }
        AsyncGeneratorRequestCompletion::Err(err) => {
            vm.resume_throw(agent, executable, err.value().unbind(), gc.reborrow())
        }
        AsyncGeneratorRequestCompletion::Return(value) => {
            vm.resume_return(agent, executable, value.unbind(), gc.reborrow())
        }
    };
    // 8. Assert: result is never an abrupt completion.
    // 9. Assert: When we return here, genContext has already been removed from
    //    the execution context stack and callerContext is the currently
    //    running execution context.
    resume_handle_result(agent, execution_result.unbind(), scoped_generator, gc);
    // 10. Return unused.
}

pub(super) fn resume_handle_result(
    agent: &mut Agent,
    execution_result: ExecutionResult,
    scoped_generator: Scoped<AsyncGenerator>,
    mut gc: GcScope,
) {
    match execution_result {
        ExecutionResult::Return(result) => {
            // Function is done.
            let _ = agent.pop_execution_context().unwrap();
            let generator = scoped_generator.get(agent).bind(gc.nogc());
            // AsyncGeneratorStart step 4:
            // g. Set acGenerator.[[AsyncGeneratorState]] to draining-queue.
            generator.transition_to_draining_queue(agent);

            // i. If result is a return completion, set result to NormalCompletion(result.[[Value]]).
            // j. Perform AsyncGeneratorCompleteStep(acGenerator, result, true).
            async_generator_complete_step(
                agent,
                generator.unbind(),
                AsyncGeneratorRequestCompletion::Ok(result),
                true,
                None,
                gc.nogc(),
            );
            // k. Perform AsyncGeneratorDrainQueue(acGenerator).
            async_generator_drain_queue(agent, scoped_generator, gc.reborrow());
            // l. Return undefined.
        }
        ExecutionResult::Throw(err) => {
            // Function is done.
            let _ = agent.pop_execution_context().unwrap();
            let generator = scoped_generator.get(agent).bind(gc.nogc());
            // AsyncGeneratorStart step 4:
            // g. Set acGenerator.[[AsyncGeneratorState]] to draining-queue.
            generator.transition_to_draining_queue(agent);
            // j. Perform AsyncGeneratorCompleteStep(acGenerator, result, true).
            async_generator_complete_step(
                agent,
                generator.unbind(),
                AsyncGeneratorRequestCompletion::Err(err),
                true,
                None,
                gc.nogc(),
            );
            // k. Perform AsyncGeneratorDrainQueue(acGenerator).
            async_generator_drain_queue(agent, scoped_generator, gc.reborrow());
            // l. Return undefined.
        }
        ExecutionResult::Yield { vm, yielded_value } => {
            // 27.5.3.7 Yield ( value )
            // If generatorKind is async, return ? AsyncGeneratorYield(? Await(value)).
            // NOTE: Await is performed in the bytecode.
            async_generator_yield(agent, yielded_value, scoped_generator, vm, gc);
        }
        ExecutionResult::Await { vm, awaited_value } => {
            async_generator_perform_await(
                agent,
                scoped_generator,
                vm,
                awaited_value,
                AsyncGeneratorAwaitKind::Await,
                gc,
            );
        }
    }
}

fn async_generator_perform_await(
    agent: &mut Agent,
    scoped_generator: Scoped<AsyncGenerator>,
    vm: SuspendedVm,
    awaited_value: Value,
    kind: AsyncGeneratorAwaitKind,
    mut gc: GcScope,
) {
    // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
    let execution_context = agent.pop_execution_context().unwrap();
    let generator = scoped_generator.get(agent).bind(gc.nogc());
    generator.transition_to_awaiting(agent, vm, kind, execution_context);
    // 8. Remove asyncContext from the execution context stack and
    //    restore the execution context that is at the top of the
    //    execution context stack as the running execution context.
    let handler = PromiseReactionHandler::AsyncGenerator(generator.unbind());
    // 2. Let promise be ? PromiseResolve(%Promise%, value).
    let promise = Promise::resolve(agent, awaited_value, gc.reborrow())
        .unbind()
        .bind(gc.nogc());

    // 7. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
    inner_promise_then(agent, promise, handler, handler, None, gc.nogc());
}

/// ### [27.6.3.7 AsyncGeneratorUnwrapYieldResumption ( resumptionValue )](https://tc39.es/ecma262/#sec-asyncgeneratorunwrapyieldresumption)
///
/// The abstract operation AsyncGeneratorUnwrapYieldResumption takes argument
/// resumptionValue (a Completion Record) and returns either a normal
/// completion containing an ECMAScript language value or an abrupt completion.
fn async_generator_unwrap_yield_resumption(
    agent: &mut Agent,
    vm: SuspendedVm,
    generator: Scoped<AsyncGenerator>,
    resumption_value: AsyncGeneratorRequestCompletion,
    mut gc: GcScope,
) {
    let resumption_value = resumption_value.bind(gc.nogc());
    // 1. If resumptionValue is not a return completion, return ? resumptionValue.
    let execution_result = match resumption_value {
        AsyncGeneratorRequestCompletion::Ok(v) => {
            let executable = generator
                .get(agent)
                .get_executable(agent, gc.nogc())
                .scope(agent, gc.nogc());
            vm.resume(agent, executable, v.unbind(), gc.reborrow())
        }
        AsyncGeneratorRequestCompletion::Err(e) => {
            let executable = generator
                .get(agent)
                .get_executable(agent, gc.nogc())
                .scope(agent, gc.nogc());
            vm.resume_throw(agent, executable, e.value().unbind(), gc.reborrow())
        }
        AsyncGeneratorRequestCompletion::Return(value) => {
            let executable = generator
                .get(agent)
                .get_executable(agent, gc.nogc())
                .scope(agent, gc.nogc());
            // 2. Let awaited be Completion(Await(resumptionValue.[[Value]])).
            // Note: the Await instruction is performed in the bytecode.
            vm.resume_return(agent, executable, value.unbind(), gc.reborrow())
        }
    };
    resume_handle_result(agent, execution_result.unbind(), generator, gc);
}

/// ### [27.6.3.8 AsyncGeneratorYield ( value )](https://tc39.es/ecma262/#sec-asyncgeneratoryield)
///
/// The abstract operation AsyncGeneratorYield takes argument value (an
/// ECMAScript language value) and returns either a normal completion
/// containing an ECMAScript language value or an abrupt completion.
pub(super) fn async_generator_yield(
    agent: &mut Agent,
    value: Value,
    generator: Scoped<AsyncGenerator>,
    vm: SuspendedVm,
    gc: GcScope,
) {
    // 1. Let genContext be the running execution context.
    let gen_context = agent.running_execution_context();
    // 2. Assert: genContext is the execution context of a generator.
    // 3. Let generator be the value of the Generator component of genContext.
    // 4. Assert: GetGeneratorKind() is async.
    let generator_function = ECMAScriptFunction::try_from(gen_context.function.unwrap()).unwrap();
    let f = generator_function.get_ast(agent, gc.nogc());
    assert!(f.is_async() && f.is_generator());
    // 5. Let completion be NormalCompletion(value).
    let completion = AsyncGeneratorRequestCompletion::Ok(value);
    // 6. Assert: The execution context stack has at least two elements.
    // 7. Let previousContext be the second to top element of the execution context stack.
    // 8. Let previousRealm be previousContext's Realm.
    let previous_realm = agent.get_previous_context_realm(gc.nogc());
    // 9. Perform AsyncGeneratorCompleteStep(generator, completion, false, previousRealm).
    async_generator_complete_step(
        agent,
        generator.get(agent),
        completion,
        false,
        Some(previous_realm),
        gc.nogc(),
    );
    // 10. Let queue be generator.[[AsyncGeneratorQueue]].
    // 11. If queue is not empty, then
    if !generator.get(agent).queue_is_empty(agent) {
        // a. NOTE: Execution continues without suspending the generator.
        // b. Let toYield be the first element of queue.
        let to_yield = generator.get(agent).peek_first(agent, gc.nogc());
        // c. Let resumptionValue be Completion(toYield.[[Completion]]).
        let resumption_value = to_yield.completion;
        // d. Return ? AsyncGeneratorUnwrapYieldResumption(resumptionValue).
        async_generator_unwrap_yield_resumption(
            agent,
            vm,
            generator,
            resumption_value.unbind(),
            gc,
        );
    } else {
        // 12. Else,
        // a. Set generator.[[AsyncGeneratorState]] to suspended-yield.
        let generator = generator.get(agent).bind(gc.nogc());
        let gen_context = agent.pop_execution_context().unwrap();
        // b. Remove genContext from the execution context stack and restore
        //    the execution context that is at the top of the execution context
        //    stack as the running execution context.
        generator.transition_to_suspended(agent, vm, gen_context);
        // c. Let callerContext be the running execution context.
        // d. Resume callerContext passing undefined. If genContext is ever
        //    resumed again, let resumptionValue be the Completion Record with
        //    which it is resumed.
        // e. Assert: If control reaches here, then genContext is the running execution context again.
        // f. Return ? AsyncGeneratorUnwrapYieldResumption(resumptionValue).
    }
}

/// ### [27.6.3.9 AsyncGeneratorAwaitReturn ( generator )](https://tc39.es/ecma262/#sec-asyncgeneratorawaitreturn)
///
/// The abstract operation AsyncGeneratorAwaitReturn takes argument generator
/// (an AsyncGenerator) and returns unused.
pub(super) fn async_generator_await_return(
    agent: &mut Agent,
    scoped_generator: Scoped<AsyncGenerator>,
    mut gc: GcScope,
) {
    let generator = scoped_generator.get(agent).bind(gc.nogc());
    // 1. Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    assert!(generator.is_draining_queue(agent));
    // 2. Let queue be generator.[[AsyncGeneratorQueue]].
    // 3. Assert: queue is not empty.
    assert!(!generator.queue_is_empty(agent));
    // 4. Let next be the first element of queue.
    let next = generator.peek_first(agent, gc.nogc());
    // 5. Let completion be Completion(next.[[Completion]]).
    let completion = next.completion;
    // 6. Assert: completion is a return completion.
    let AsyncGeneratorRequestCompletion::Return(value) = completion else {
        unreachable!()
    };
    // 7. Let promiseCompletion be Completion(PromiseResolve(%Promise%, completion.[[Value]])).
    // 8. If promiseCompletion is an abrupt completion, then
    //         a. Perform AsyncGeneratorCompleteStep(generator, promiseCompletion, true).
    //         b. Perform AsyncGeneratorDrainQueue(generator).
    //         c. Return unused.
    // 9. Assert: promiseCompletion is a normal completion.
    // 10. Let promise be promiseCompletion.[[Value]].
    let promise = Promise::resolve(agent, value.unbind(), gc.reborrow())
        .unbind()
        .bind(gc.nogc());
    // 11. ... onFulfilled ...
    // 12. Let onFulfilled be CreateBuiltinFunction(fulfilledClosure, 1, "", « »).
    // 13. ... onRejected ...
    // 14. Let onRejected be CreateBuiltinFunction(rejectedClosure, 1, "", « »).
    // 15. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
    let handler =
        PromiseReactionHandler::AsyncGenerator(scoped_generator.get(agent).bind(gc.nogc()));
    inner_promise_then(agent, promise, handler, handler, None, gc.nogc());
    // 16. Return unused.
}

pub(crate) fn async_generator_await_return_on_fulfilled(
    agent: &mut Agent,
    generator: AsyncGenerator,
    value: Value,
    gc: GcScope,
) {
    let generator = generator.bind(gc.nogc());
    let value = value.bind(gc.nogc());
    // 11. Let fulfilledClosure be a new Abstract Closure with parameters
    //     (value) that captures generator and performs the following steps
    //     when called:
    // a. Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    assert!(generator.is_draining_queue(agent));
    // b. Let result be NormalCompletion(value).
    // c. Perform AsyncGeneratorCompleteStep(generator, result, true).
    let scoped_generator = generator.scope(agent, gc.nogc());
    async_generator_complete_step(
        agent,
        generator.unbind(),
        AsyncGeneratorRequestCompletion::Ok(value),
        true,
        None,
        gc.nogc(),
    );
    // d. Perform AsyncGeneratorDrainQueue(generator).
    async_generator_drain_queue(agent, scoped_generator, gc);
    // e. Return undefined.
}

pub(crate) fn async_generator_await_return_on_rejected(
    agent: &mut Agent,
    generator: AsyncGenerator,
    value: Value,
    gc: GcScope,
) {
    let generator = generator.bind(gc.nogc());
    let value = value.bind(gc.nogc());
    // 13. Let rejectedClosure be a new Abstract Closure with parameters
    //     (reason) that captures generator and performs the following steps
    //     when called:
    // a. Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    assert!(generator.is_draining_queue(agent));
    // b. Let result be ThrowCompletion(reason).
    let scoped_generator = generator.scope(agent, gc.nogc());
    // c. Perform AsyncGeneratorCompleteStep(generator, result, true).
    async_generator_complete_step(
        agent,
        generator.unbind(),
        AsyncGeneratorRequestCompletion::Err(JsError::new(value.unbind())),
        true,
        None,
        gc.nogc(),
    );
    // d. Perform AsyncGeneratorDrainQueue(generator).
    async_generator_drain_queue(agent, scoped_generator, gc);
    // e. Return undefined.
}

/// ### [27.6.3.10 AsyncGeneratorDrainQueue ( generator )](https://tc39.es/ecma262/#sec-asyncgeneratordrainqueue)
///
/// The abstract operation AsyncGeneratorDrainQueue takes argument generator
/// (an AsyncGenerator) and returns unused. It drains the generator's
/// AsyncGeneratorQueue until it encounters an AsyncGeneratorRequest which
/// holds a return completion.
fn async_generator_drain_queue(
    agent: &mut Agent,
    scoped_generator: Scoped<AsyncGenerator>,
    mut gc: GcScope,
) {
    let generator = scoped_generator.get(agent).bind(gc.nogc());
    // Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    // 2. Let queue be generator.[[AsyncGeneratorQueue]].
    let Some(AsyncGeneratorState::DrainingQueue(queue)) =
        &mut generator.get(agent).async_generator_state
    else {
        unreachable!()
    };
    // 3. If queue is empty, then
    if queue.is_empty() {
        // a. Set generator.[[AsyncGeneratorState]] to completed.
        generator
            .get(agent)
            .async_generator_state
            .replace(AsyncGeneratorState::Completed(Default::default()));
        // b. Return unused.
        return;
    }

    // 4. Let done be false.
    // 5. Repeat, while done is false,
    loop {
        // a. Let next be the first element of queue.
        let next = generator.peek_first(agent, gc.nogc());
        // b. Let completion be Completion(next.[[Completion]]).
        let completion = next.completion;
        // c. If completion is a return completion, then
        if let AsyncGeneratorRequestCompletion::Return(_) = completion {
            // i. Perform AsyncGeneratorAwaitReturn(generator).
            async_generator_await_return(agent, scoped_generator, gc.reborrow());
            // ii. Set done to true.
            return;
        } else {
            // d. Else,
            // i. If completion is a normal completion, then
            let completion = if let AsyncGeneratorRequestCompletion::Ok(_) = completion {
                // 1. Set completion to NormalCompletion(undefined).
                AsyncGeneratorRequestCompletion::Ok(Value::Undefined)
            } else {
                completion
            };
            // ii. Perform AsyncGeneratorCompleteStep(generator, completion, true).
            async_generator_complete_step(agent, generator, completion, true, None, gc.nogc());
            // iii. If queue is empty, then
            let Some(AsyncGeneratorState::DrainingQueue(queue)) =
                &mut generator.get(agent).async_generator_state
            else {
                unreachable!()
            };
            if queue.is_empty() {
                // 1. Set generator.[[AsyncGeneratorState]] to completed.
                generator
                    .get(agent)
                    .async_generator_state
                    .replace(AsyncGeneratorState::Completed(Default::default()));
                // 2. Set done to true.
                return;
            }
        }
    }

    // 6. Return unused.
}
