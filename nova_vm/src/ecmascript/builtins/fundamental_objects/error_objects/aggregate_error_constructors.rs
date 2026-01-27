// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, builders::BuiltinFunctionBuilder,
        BuiltinIntrinsicConstructor, Error, ExceptionType, Function, JsResult, Object,
        PropertyDescriptor, PropertyKey, ProtoIntrinsics, Realm, String, Value,
        create_array_from_scoped_list, define_property_or_throw, get_iterator, iterator_to_list,
        ordinary_create_from_constructor, throw_not_callable, to_string,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::{ArenaAccessMut, IntrinsicConstructorIndexes},
};

use super::error_constructor::get_error_cause;

pub(crate) struct AggregateErrorConstructor;
impl Builtin for AggregateErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.AggregateError;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for AggregateErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::AggregateError;
}

impl AggregateErrorConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let errors = arguments.get(0).scope(agent, gc.nogc());
        let message = arguments.get(1).scope(agent, gc.nogc());
        let options = arguments.get(2);
        // 1. If NewTarget is undefined, let newTarget be the active function object; else let newTarget be NewTarget.
        let new_target = new_target.map_or_else(
            || agent.running_execution_context().function.unwrap(),
            |new_target| Function::try_from(new_target).unwrap(),
        );
        // 2. Let O be ? OrdinaryCreateFromConstructor(newTarget, "%AggregateError.prototype%", « [[ErrorData]] »).
        let o = ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::AggregateError,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let o = Error::try_from(o.unbind()).unwrap();
        // 3. If message is not undefined, then
        let message = message.get(agent).bind(gc.nogc());
        let message = if !message.is_undefined() {
            // a. Let msg be ? ToString(message).
            Some(
                to_string(agent, message.unbind(), gc.reborrow())
                    .unbind()?
                    .scope(agent, gc.nogc()),
            )
        } else {
            None
        };
        // 4. Perform ? InstallErrorCause(O, options).
        let cause = get_error_cause(agent, options, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let message = message.map(|message| message.get(agent).bind(gc.nogc()));
        let heap_data = o.get_mut(agent);
        heap_data.kind = ExceptionType::AggregateError;
        heap_data.message = message.unbind();
        heap_data.cause = cause.unbind();
        // 5. Let errorsList be ? IteratorToList(? GetIterator(errors, sync)).
        let Some(iterator_record) = get_iterator(agent, errors.get(agent), false, gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_iterator_record()
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let errors_list = iterator_to_list(agent, iterator_record.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 6. Perform ! DefinePropertyOrThrow(O, "errors", PropertyDescriptor {
        let property_descriptor = PropertyDescriptor {
            // [[Configurable]]: true,
            configurable: Some(true),
            // [[Enumerable]]: false,
            enumerable: Some(false),
            // [[Writable]]: true,
            writable: Some(true),
            // [[Value]]: CreateArrayFromList(errorsList)
            value: Some(
                create_array_from_scoped_list(agent, errors_list, gc.nogc())
                    .unbind()
                    .into(),
            ),
            ..Default::default()
        };
        define_property_or_throw(
            agent,
            o.unbind(),
            PropertyKey::from(BUILTIN_STRING_MEMORY.errors),
            property_descriptor,
            gc.reborrow(),
        )
        .unbind()?;
        // }).
        // 7. Return O.
        Ok(o.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let error_constructor = intrinsics.error();
        let aggregate_error_prototype = intrinsics.aggregate_error_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AggregateErrorConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype(error_constructor)
        .with_prototype_property(aggregate_error_prototype.into())
        .build();
    }
}
