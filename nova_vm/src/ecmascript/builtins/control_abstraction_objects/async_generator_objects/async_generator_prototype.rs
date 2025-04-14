// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::create_iter_result_object;
use crate::ecmascript::builtins::async_generator_objects::AsyncGeneratorState;
use crate::ecmascript::builtins::promise_objects::promise_abstract_operations::promise_capability_records::{if_abrupt_reject_promise, PromiseCapability};
use crate::ecmascript::execution::agent::JsError;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::rootable::Scopable;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, Realm},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

use super::AsyncGeneratorRequestCompletion;
use super::async_generator_abstract_operations::{
    async_generator_await_return, async_generator_enqueue, async_generator_resume,
    async_generator_validate,
};

pub(crate) struct AsyncGeneratorPrototype;

struct AsyncGeneratorPrototypeNext;
impl Builtin for AsyncGeneratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncGeneratorPrototype::next);
}
struct AsyncGeneratorPrototypeReturn;
impl Builtin for AsyncGeneratorPrototypeReturn {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.r#return;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncGeneratorPrototype::r#return);
}
struct AsyncGeneratorPrototypeThrow;
impl Builtin for AsyncGeneratorPrototypeThrow {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.throw;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncGeneratorPrototype::throw);
}

impl AsyncGeneratorPrototype {
    /// ### [27.6.1.2 %AsyncGeneratorPrototype%.next ( value )](https://tc39.es/ecma262/#sec-asyncgenerator-prototype-next)
    fn next<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let value = arguments.get(0).bind(gc.nogc());
        // 1. Let generator be the this value.
        let generator = this_value.bind(gc.nogc());
        // 2. Let promiseCapability be ! NewPromiseCapability(%Promise%).
        let promise_capability = PromiseCapability::new(agent, gc.nogc());
        let promise = promise_capability.promise().scope(agent, gc.nogc());
        // 3. Let result be Completion(AsyncGeneratorValidate(generator, empty)).
        let result = async_generator_validate(agent, generator, (), gc.nogc());
        // 4. IfAbruptRejectPromise(result, promiseCapability).
        let generator =
            match if_abrupt_reject_promise(agent, result, promise_capability.clone(), gc.nogc()) {
                Ok(g) => g,
                Err(p) => {
                    return Ok(p.into_value().unbind());
                }
            };
        // 5. Let state be generator.[[AsyncGeneratorState]].
        let state = agent[generator].async_generator_state.as_ref().unwrap();
        // 6. If state is completed, then
        if state.is_completed() {
            // a. Let iteratorResult be CreateIteratorResultObject(undefined, true).
            let iterator_result =
                create_iter_result_object(agent, Value::Undefined, true, gc.nogc());
            // b. Perform ! Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »).
            promise_capability
                .unbind()
                .resolve(agent, iterator_result.into_value().unbind(), gc);
            // c. Return promiseCapability.[[Promise]].
            // SAFETY: Promise has not been shared.
            return Ok(unsafe { promise.take(agent).into_value() });
        }
        let state_is_suspended = state.is_suspended();
        let state_is_executing_or_draining = state.is_active();
        // 7. Let completion be NormalCompletion(value).
        let completion = AsyncGeneratorRequestCompletion::Ok(value);
        // 8. Perform AsyncGeneratorEnqueue(generator, completion, promiseCapability).
        async_generator_enqueue(agent, generator, completion, promise_capability);
        // 9. If state is either suspended-start or suspended-yield, then
        if state_is_suspended {
            // a. Perform AsyncGeneratorResume(generator, completion).
            async_generator_resume(agent, generator.unbind(), completion.unbind(), gc);
        } else {
            // 10. Else,
            // a. Assert: state is either executing or draining-queue.
            assert!(state_is_executing_or_draining);
        }
        // 11. Return promiseCapability.[[Promise]].
        Ok(unsafe { promise.take(agent).into_value() })
    }

    /// ### [27.6.1.3 %AsyncGeneratorPrototype%.return ( value )](https://tc39.es/ecma262/#sec-asyncgenerator-prototype-return)
    fn r#return<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let value = arguments.get(0).bind(gc.nogc());
        // 1. Let generator be the this value.
        let generator = this_value.bind(gc.nogc());
        // 2. Let promiseCapability be ! NewPromiseCapability(%Promise%).
        let promise_capability = PromiseCapability::new(agent, gc.nogc());
        let promise = promise_capability.promise().bind(gc.nogc());
        // 3. Let result be Completion(AsyncGeneratorValidate(generator, empty)).
        let result = async_generator_validate(agent, generator, (), gc.nogc());
        // 4. IfAbruptRejectPromise(result, promiseCapability).
        let generator =
            match if_abrupt_reject_promise(agent, result, promise_capability.clone(), gc.nogc()) {
                Ok(g) => g,
                Err(p) => {
                    return Ok(p.into_value().unbind());
                }
            };
        // 5. Let completion be ReturnCompletion(value).
        let completion = AsyncGeneratorRequestCompletion::Return(value);
        // 6. Perform AsyncGeneratorEnqueue(generator, completion, promiseCapability).
        async_generator_enqueue(agent, generator, completion, promise_capability);
        // 7. Let state be generator.[[AsyncGeneratorState]].
        if generator.is_suspended_start(agent) || generator.is_completed(agent) {
            // 8. If state is either suspended-start or completed, then
            let promise = promise.scope(agent, gc.nogc());
            // a. Set generator.[[AsyncGeneratorState]] to draining-queue.
            generator.transition_to_draining_queue(agent);
            // b. Perform AsyncGeneratorAwaitReturn(generator).
            let generator = generator.scope(agent, gc.nogc());
            async_generator_await_return(agent, generator, gc.reborrow());
            // 11. Return promiseCapability.[[Promise]].
            Ok(promise.get(agent).into_value())
        } else if generator.is_suspended_yield(agent) {
            // 9. Else if state is suspended-yield, then
            let promise = promise.scope(agent, gc.nogc());
            // a. Perform AsyncGeneratorResume(generator, completion).
            async_generator_resume(
                agent,
                generator.unbind(),
                completion.unbind(),
                gc.reborrow(),
            );
            // 11. Return promiseCapability.[[Promise]].
            Ok(promise.get(agent).into_value())
        } else {
            // 10. Else,
            // a. Assert: state is either executing or draining-queue.
            assert!(generator.is_draining_queue(agent) || generator.is_executing(agent));
            // 11. Return promiseCapability.[[Promise]].
            Ok(promise.into_value().unbind())
        }
    }

    /// ### [27.6.1.4 %AsyncGeneratorPrototype%.throw ( exception )](https://tc39.es/ecma262/#sec-asyncgenerator-prototype-throw)
    fn throw<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let exception = arguments.get(0).bind(gc.nogc());
        // 1. Let generator be the this value.
        let generator = this_value.bind(gc.nogc());
        // 2. Let promiseCapability be ! NewPromiseCapability(%Promise%).
        let promise_capability = PromiseCapability::new(agent, gc.nogc());
        let mut promise = promise_capability.promise().bind(gc.nogc());
        // 3. Let result be Completion(AsyncGeneratorValidate(generator, empty)).
        let result = async_generator_validate(agent, generator, (), gc.nogc());
        // 4. IfAbruptRejectPromise(result, promiseCapability).
        let generator =
            match if_abrupt_reject_promise(agent, result, promise_capability.clone(), gc.nogc()) {
                Ok(g) => g,
                Err(p) => {
                    return Ok(p.into_value().unbind());
                }
            };
        // 5. Let state be generator.[[AsyncGeneratorState]].
        // 6. If state is suspended-start, then
        let mut completed = false;
        if generator.is_suspended_start(agent) {
            // a. Set generator.[[AsyncGeneratorState]] to completed.
            agent[generator].async_generator_state = Some(AsyncGeneratorState::Completed);
            // b. Set state to completed.
            completed = true;
        }
        // 7. If state is completed, then
        if completed || generator.is_completed(agent) {
            // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « exception »).
            promise_capability.reject(agent, exception, gc.nogc());
            // b. Return promiseCapability.[[Promise]].
            return Ok(promise.into_value().unbind());
        }
        // 8. Let completion be ThrowCompletion(exception).
        let completion =
            AsyncGeneratorRequestCompletion::Err(JsError::new(exception.unbind())).bind(gc.nogc());
        // 9. Perform AsyncGeneratorEnqueue(generator, completion, promiseCapability).
        async_generator_enqueue(agent, generator, completion, promise_capability);
        // 10. If state is suspended-yield, then
        if generator.is_suspended_yield(agent) {
            // a. Perform AsyncGeneratorResume(generator, completion).
            let scoped_promise = promise.scope(agent, gc.nogc());
            async_generator_resume(
                agent,
                generator.unbind(),
                completion.unbind(),
                gc.reborrow(),
            );
            promise = scoped_promise.get(agent).bind(gc.nogc());
        } else {
            // 11. Else,
            // a. Assert: state is either executing or draining-queue.
            assert!(generator.is_executing(agent) || generator.is_draining_queue(agent));
        }
        // 12. Return promiseCapability.[[Promise]].
        Ok(promise.into_value().unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let async_iterator_prototype = intrinsics.async_iterator_prototype();
        let async_generator_function_prototype = intrinsics.async_generator_function_prototype();
        let this = intrinsics.async_generator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(async_iterator_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.constructor.into())
                    .with_value_readonly(async_generator_function_prototype.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_builtin_function_property::<AsyncGeneratorPrototypeNext>()
            .with_builtin_function_property::<AsyncGeneratorPrototypeReturn>()
            .with_builtin_function_property::<AsyncGeneratorPrototypeThrow>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.AsyncGenerator.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
