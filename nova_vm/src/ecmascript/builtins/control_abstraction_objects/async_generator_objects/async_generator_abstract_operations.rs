// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::create_iter_result_object,
        builtins::{
            promise::Promise,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::Value,
    },
    engine::{
        context::{GcScope, NoGcScope},
        ExecutionResult, Vm,
    },
};

use super::{
    AsyncGenerator, AsyncGeneratorRequest, AsyncGeneratorRequestCompletion, AsyncGeneratorState,
    SuspendedGeneratorState, VmOrArguments,
};

/// ### [27.6.3.3 AsyncGeneratorValidate ( generator, generatorBrand )](https://tc39.es/ecma262/#sec-asyncgeneratorvalidate)
///
/// The abstract operation AsyncGeneratorValidate takes arguments generator (an ECMAScript language value) and generatorBrand (a String or empty) and returns either a normal completion containing unused or a throw completion. It performs the following steps when called:
pub(super) fn async_generator_validate<'a>(
    agent: &mut Agent,
    generator: Value,
    _generator_brand: (),
    gc: NoGcScope<'a, '_>,
) -> JsResult<AsyncGenerator<'a>> {
    // 1. Perform ? RequireInternalSlot(generator, [[AsyncGeneratorContext]]).
    // 2. Perform ? RequireInternalSlot(generator, [[AsyncGeneratorState]]).
    // 3. Perform ? RequireInternalSlot(generator, [[AsyncGeneratorQueue]]).
    // 4. If generator.[[GeneratorBrand]] is not generatorBrand, throw a TypeError exception.
    // 5. Return unused.
    if let Value::AsyncGenerator(generator) = generator {
        Ok(generator)
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
        completion,
        capability: promise_capability,
    };
    // 2. Append request to generator.[[AsyncGeneratorQueue]].
    match agent[generator].async_generator_state.as_mut().unwrap() {
        AsyncGeneratorState::Suspended { queue, .. }
        | AsyncGeneratorState::Executing(queue)
        | AsyncGeneratorState::DrainingQueue(queue) => queue.push_back(request),
        AsyncGeneratorState::Completed => todo!(),
    }
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
    realm: Option<RealmIdentifier>,
    gc: GcScope,
) {
    // 1. Assert: generator.[[AsyncGeneratorQueue]] is not empty.
    assert!(generator.queue_is_empty(agent));
    // 2. Let next be the first element of generator.[[AsyncGeneratorQueue]].
    // 3. Remove the first element from generator.[[AsyncGeneratorQueue]].
    let next = generator.pop_first(agent);
    // 4. Let promiseCapability be next.[[Capability]].
    let promise_capability = next.capability;
    // 5. Let value be completion.[[Value]].
    let value = match completion {
        AsyncGeneratorRequestCompletion::Ok(value) => value,
        // 6. If completion is a throw completion, then
        // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « value »).
        AsyncGeneratorRequestCompletion::Err(_) => unreachable!("We've manually inlined this path"),
        // 7. Else,
        // a. Assert: completion is a normal completion.
        AsyncGeneratorRequestCompletion::Return(_) => unreachable!(),
    };
    // b. If realm is present, then
    let iterator_result = if let Some(realm) = realm {
        // i. Let oldRealm be the running execution context's Realm.
        let old_realm = agent.running_execution_context().realm;
        let set_realm = realm != old_realm;
        // ii. Set the running execution context's Realm to realm.
        if set_realm {
            agent.running_execution_context_mut().realm = realm;
        }
        // iii. Let iteratorResult be CreateIteratorResultObject(value, done).
        let iterator_result = create_iter_result_object(agent, value, done, gc.nogc());
        // iv. Set the running execution context's Realm to oldRealm.
        if set_realm {
            agent.running_execution_context_mut().realm = old_realm;
        }
        iterator_result
    } else {
        // c. Else,
        // i. Let iteratorResult be CreateIteratorResultObject(value, done).
        create_iter_result_object(agent, value, done, gc.nogc())
    };
    // d. Perform ! Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »).
    promise_capability.resolve(agent, iterator_result.into_value(), gc);
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
    // 1. Assert: generator.[[AsyncGeneratorState]] is either suspended-start or suspended-yield.
    let async_generator_state = &mut agent[generator].async_generator_state;
    let AsyncGeneratorState::Suspended {
        state:
            SuspendedGeneratorState {
                vm_or_args,
                executable,
                execution_context,
            },
        queue,
    } = async_generator_state.take().unwrap()
    else {
        unreachable!()
    };
    // 5. Set generator.[[AsyncGeneratorState]] to executing.
    async_generator_state.replace(AsyncGeneratorState::Executing(queue));

    // 2. Let genContext be generator.[[AsyncGeneratorContext]].
    // 3. Let callerContext be the running execution context.
    // 4. Suspend callerContext.
    // 6. Push genContext onto the execution context stack; genContext is now
    //    the running execution context.
    agent.execution_context_stack.push(execution_context);

    let scoped_generator = generator.scope(agent, gc.nogc());

    // 7. Resume the suspended evaluation of genContext using completion as the
    //    result of the operation that suspended it. Let result be the
    //    Completion Record returned by the resumed computation.
    let execution_result = match vm_or_args {
        VmOrArguments::Arguments(args) => {
            Vm::execute(agent, executable, Some(&args), gc.reborrow())
        }
        VmOrArguments::Vm(vm) => {
            let AsyncGeneratorRequestCompletion::Ok(value) = completion else {
                unreachable!()
            };
            vm.resume(agent, executable, value, gc.reborrow())
        }
    };
    // 8. Assert: result is never an abrupt completion.
    let execution_context = agent.execution_context_stack.pop().unwrap();

    // 9. Assert: When we return here, genContext has already been removed from
    //    the execution context stack and callerContext is the currently
    //    running execution context.
    match execution_result {
        ExecutionResult::Return(result) => {
            let generator = scoped_generator.get(agent).bind(gc.nogc());
            // AsyncGeneratorStart step 4:
            // g. Set acGenerator.[[AsyncGeneratorState]] to draining-queue.
            // let async_generator_state = &mut agent[generator].async_generator_state;
            let async_generator_state = &mut agent[generator].async_generator_state;
            let AsyncGeneratorState::Executing(mut queue) = async_generator_state.take().unwrap()
            else {
                unreachable!()
            };
            // i. If result is a return completion, set result to NormalCompletion(result.[[Value]]).
            // j. Perform AsyncGeneratorCompleteStep(acGenerator, result, true).
            assert!(!queue.is_empty());
            let next = queue.pop_front().unwrap();
            async_generator_state.replace(AsyncGeneratorState::DrainingQueue(queue));
            // 4. Let promiseCapability be next.[[Capability]].
            let promise_capability = next.capability;
            // i. Let iteratorResult be CreateIteratorResultObject(value, done).
            let iterator_result = create_iter_result_object(agent, result, true, gc.nogc());
            // d. Perform ! Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »).
            promise_capability.resolve(agent, iterator_result.into_value(), gc.reborrow());
            // k. Perform AsyncGeneratorDrainQueue(acGenerator).
            let generator = scoped_generator.get(agent).bind(gc.nogc());
            async_generator_drain_queue(agent, generator.unbind(), gc.reborrow());
            // l. Return undefined.
        }
        ExecutionResult::Throw(err) => {
            let generator = scoped_generator.get(agent).bind(gc.nogc());
            // AsyncGeneratorStart step 4:
            // g. Set acGenerator.[[AsyncGeneratorState]] to draining-queue.
            let async_generator_state = &mut agent[generator].async_generator_state;
            let AsyncGeneratorState::Executing(mut queue) = async_generator_state.take().unwrap()
            else {
                unreachable!()
            };
            // j. Perform AsyncGeneratorCompleteStep(acGenerator, result, true).
            // Assert: generator.[[AsyncGeneratorQueue]] is not empty.
            assert!(!queue.is_empty());
            // 2. Let next be the first element of generator.[[AsyncGeneratorQueue]].
            // 3. Remove the first element from generator.[[AsyncGeneratorQueue]].
            let next = queue.pop_front().unwrap();
            let emptied_queue = queue.is_empty();
            if emptied_queue {
                async_generator_state.replace(AsyncGeneratorState::Completed);
            } else {
                async_generator_state.replace(AsyncGeneratorState::DrainingQueue(queue));
            }
            // 4. Let promiseCapability be next.[[Capability]].
            let promise_capability = next.capability;
            // 5. Let value be completion.[[Value]].
            let value = err.value();
            // 6. If completion is a throw completion, then
            // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « value »).
            promise_capability.reject(agent, value);
            // k. Perform AsyncGeneratorDrainQueue(acGenerator).
            if !emptied_queue {
                async_generator_drain_queue(agent, generator.unbind(), gc);
            }
            // l. Return undefined.
        }
        ExecutionResult::Yield { vm, yielded_value } => {
            todo!()
        }
        ExecutionResult::Await { .. } => todo!(),
    }
    // 10. Return unused.
}

/// ### [27.6.3.9 AsyncGeneratorAwaitReturn ( generator )](https://tc39.es/ecma262/#sec-asyncgeneratorawaitreturn)
///
/// The abstract operation AsyncGeneratorAwaitReturn takes argument generator
/// (an AsyncGenerator) and returns unused.
fn async_generator_await_return(agent: &mut Agent, generator: AsyncGenerator, mut gc: GcScope) {
    // 1. Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    assert!(generator.is_draining_queue(agent));
    // 2. Let queue be generator.[[AsyncGeneratorQueue]].
    // 3. Assert: queue is not empty.
    assert!(!generator.queue_is_empty(agent));
    // 4. Let next be the first element of queue.
    let next = generator.peek_first(agent);
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
    let _promise = Promise::resolve(agent, value, gc.reborrow());
    // 11. Let fulfilledClosure be a new Abstract Closure with parameters (value) that captures generator and performs the following steps when called:
    //         a. Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    //         b. Let result be NormalCompletion(value).
    //         c. Perform AsyncGeneratorCompleteStep(generator, result, true).
    //         d. Perform AsyncGeneratorDrainQueue(generator).
    //         e. Return undefined.
    // 12. Let onFulfilled be CreateBuiltinFunction(fulfilledClosure, 1, "", « »).
    // 13. Let rejectedClosure be a new Abstract Closure with parameters (reason) that captures generator and performs the following steps when called:
    //         a. Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    //         b. Let result be ThrowCompletion(reason).
    //         c. Perform AsyncGeneratorCompleteStep(generator, result, true).
    //         d. Perform AsyncGeneratorDrainQueue(generator).
    //         e. Return undefined.
    // 14. Let onRejected be CreateBuiltinFunction(rejectedClosure, 1, "", « »).
    // 15. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
    // 16. Return unused.
}

/// ### [27.6.3.10 AsyncGeneratorDrainQueue ( generator )](https://tc39.es/ecma262/#sec-asyncgeneratordrainqueue)
///
/// The abstract operation AsyncGeneratorDrainQueue takes argument generator
/// (an AsyncGenerator) and returns unused. It drains the generator's
/// AsyncGeneratorQueue until it encounters an AsyncGeneratorRequest which
/// holds a return completion.
fn async_generator_drain_queue(agent: &mut Agent, generator: AsyncGenerator, mut gc: GcScope) {
    // Assert: generator.[[AsyncGeneratorState]] is draining-queue.
    // 2. Let queue be generator.[[AsyncGeneratorQueue]].
    let Some(AsyncGeneratorState::DrainingQueue(queue)) =
        &mut agent[generator].async_generator_state
    else {
        unreachable!()
    };
    // 3. If queue is empty, then
    if queue.is_empty() {
        // a. Set generator.[[AsyncGeneratorState]] to completed.
        agent[generator]
            .async_generator_state
            .replace(AsyncGeneratorState::Completed);
        // b. Return unused.
        return;
    }

    // 4. Let done be false.
    let mut done = false;
    // 5. Repeat, while done is false,
    while !done {
        // a. Let next be the first element of queue.
        let Some(AsyncGeneratorState::DrainingQueue(queue)) =
            &mut agent[generator].async_generator_state
        else {
            unreachable!()
        };
        let next = queue.pop_front().unwrap();
        // b. Let completion be Completion(next.[[Completion]]).
        let completion = next.completion;
        // c. If completion is a return completion, then
        if let AsyncGeneratorRequestCompletion::Return(_) = completion {
            // i. Perform AsyncGeneratorAwaitReturn(generator).
            async_generator_await_return(agent, generator, gc.reborrow());
            // ii. Set done to true.
            done = true;
        } else {
            // d. Else,
            // i. If completion is a normal completion, then
            if let AsyncGeneratorRequestCompletion::Ok(_) = completion {
                // 1. Set completion to NormalCompletion(undefined).
            }
            // ii. Perform AsyncGeneratorCompleteStep(generator, completion, true).
            async_generator_complete_step(agent, generator, completion, true, None, gc.reborrow());
            // iii. If queue is empty, then
            let Some(AsyncGeneratorState::DrainingQueue(queue)) =
                &mut agent[generator].async_generator_state
            else {
                unreachable!()
            };
            if queue.is_empty() {
                // 1. Set generator.[[AsyncGeneratorState]] to completed.
                agent[generator]
                    .async_generator_state
                    .replace(AsyncGeneratorState::Completed);
                // 2. Set done to true.
                done = true
            }
        }
    }

    // 6. Return unused.
}
