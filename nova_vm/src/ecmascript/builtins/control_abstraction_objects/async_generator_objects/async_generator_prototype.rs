// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::create_iter_result_object;
use crate::ecmascript::builtins::promise_objects::promise_abstract_operations::promise_capability_records::{if_abrupt_reject_promise, PromiseCapability};
use crate::ecmascript::execution::agent::ExceptionType;
use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

use super::async_generator_abstract_operations::{
    async_generator_enqueue, async_generator_resume, async_generator_validate,
};
use super::{AsyncGenerator, AsyncGeneratorRequestCompletion};

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
    fn next(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope,
    ) -> JsResult<Value> {
        let value = arguments.get(0);
        // 1. Let generator be the this value.
        let generator = this_value;
        // 2. Let promiseCapability be ! NewPromiseCapability(%Promise%).
        let promise_capability = PromiseCapability::new(agent);
        // 3. Let result be Completion(AsyncGeneratorValidate(generator, empty)).
        let result = async_generator_validate(agent, generator, (), gc.nogc());
        // 4. IfAbruptRejectPromise(result, promiseCapability).
        let generator = if_abrupt_reject_promise(agent, result, promise_capability)?;
        // 5. Let state be generator.[[AsyncGeneratorState]].
        let state = agent[generator].async_generator_state.as_ref().unwrap();
        // 6. If state is completed, then
        if state.is_completed() {
            println!("Completed");
            // a. Let iteratorResult be CreateIteratorResultObject(undefined, true).
            let iterator_result =
                create_iter_result_object(agent, Value::Undefined, true, gc.nogc());
            // b. Perform ! Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »).
            promise_capability.resolve(agent, iterator_result.into_value(), gc);
            // c. Return promiseCapability.[[Promise]].
            return Ok(promise_capability.promise().into_value());
        }
        let state_is_suspended = state.is_suspended();
        let state_is_executing_or_draining = state.is_active();
        // 7. Let completion be NormalCompletion(value).
        let completion = AsyncGeneratorRequestCompletion::Ok(value);
        // 8. Perform AsyncGeneratorEnqueue(generator, completion, promiseCapability).
        println!("Enqueue");
        async_generator_enqueue(agent, generator, completion, promise_capability);
        // 9. If state is either suspended-start or suspended-yield, then
        if state_is_suspended {
            // a. Perform AsyncGeneratorResume(generator, completion).
            async_generator_resume(agent, generator.unbind(), completion, gc);
        } else {
            // 10. Else,
            // a. Assert: state is either executing or draining-queue.
            assert!(state_is_executing_or_draining);
        }
        // 11. Return promiseCapability.[[Promise]].
        Ok(promise_capability.promise().into_value())
    }

    fn r#return(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        todo!()
    }

    fn throw(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
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
