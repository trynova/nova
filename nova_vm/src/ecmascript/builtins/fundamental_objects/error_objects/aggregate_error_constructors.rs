// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{get_iterator, iterator_to_list},
            operations_on_objects::{create_array_from_list, define_property_or_throw},
            type_conversion::to_string,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            error::Error, ordinary::ordinary_create_from_constructor, ArgumentsList, Behaviour,
            Builtin, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            Function, IntoObject, IntoValue, Object, PropertyDescriptor, PropertyKey, String,
            Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::IntrinsicConstructorIndexes,
};

use super::error_constructor::get_error_cause;

pub(crate) struct AggregateErrorConstructor;
impl Builtin for AggregateErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.AggregateError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for AggregateErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::AggregateError;
}

impl<'gc> AggregateErrorConstructor {
    fn constructor(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let errors = arguments.get(0);
        let message = arguments.get(1);
        let options = arguments.get(2);
        // 1. If NewTarget is undefined, let newTarget be the active function object; else let newTarget be NewTarget.
        let new_target = new_target.map_or_else(
            || agent.running_execution_context().function.unwrap(),
            |new_target| Function::try_from(new_target).unwrap(),
        );
        // 2. Let O be ? OrdinaryCreateFromConstructor(newTarget, "%AggregateError.prototype%", « [[ErrorData]] »).
        let o = ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::AggregateError,
            gc.reborrow(),
        )?;
        let o = Error::try_from(o.unbind()).unwrap();
        // 3. If message is not undefined, then
        let message = if !message.is_undefined() {
            // a. Let msg be ? ToString(message).
            Some(
                to_string(agent, message, gc.reborrow())?
                    .unbind()
                    .scope(agent, gc.nogc()),
            )
        } else {
            None
        };
        // 4. Perform ? InstallErrorCause(O, options).
        let cause = get_error_cause(agent, options, gc.reborrow())?;
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let message: Option<String<'_>> = message.map(|message| message.get(agent));
        let heap_data = &mut agent[o];
        heap_data.kind = ExceptionType::Error;
        heap_data.message = message;
        heap_data.cause = cause.map(|c| c.unbind());
        // 5. Let errorsList be ? IteratorToList(? GetIterator(errors, sync)).
        let iterator_record = get_iterator(agent, errors.unbind(), false, gc.reborrow())?;
        let errors_list = iterator_to_list(agent, &iterator_record, gc.reborrow())?;
        // 6. Perform ! DefinePropertyOrThrow(O, "errors", PropertyDescriptor {
        let property_descriptor = PropertyDescriptor {
            // [[Configurable]]: true,
            configurable: Some(true),
            // [[Enumerable]]: false,
            enumerable: Some(false),
            // [[Writable]]: true,
            writable: Some(true),
            // [[Value]]: CreateArrayFromList(errorsList)
            value: Some(create_array_from_list(agent, &errors_list, gc.nogc()).into_value()),
            ..Default::default()
        };
        define_property_or_throw(
            agent,
            o.unbind(),
            PropertyKey::from(BUILTIN_STRING_MEMORY.errors),
            property_descriptor,
            gc.reborrow(),
        )?;
        // }).
        // 7. Return O.
        Ok(o.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let aggregate_error_prototype = intrinsics.aggregate_error_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AggregateErrorConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(aggregate_error_prototype.into_object())
        .build();
    }
}
