// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, ErrorConstructor, ExceptionType, InternalMethods, JsResult,
        Object, PropertyDescriptor, Realm, String, Value, builders::BuiltinFunctionBuilder,
        create_array_from_scoped_list, get_iterator, iterator_to_list, throw_not_callable,
        unwrap_try,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::IntrinsicConstructorIndexes,
};

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
        let message = arguments.get(1).bind(gc.nogc());
        let options = arguments.get(2).bind(gc.nogc());
        let o = ErrorConstructor::base_constructor(
            agent,
            ExceptionType::AggregateError,
            ArgumentsList::from_mut_slice(&mut [message.unbind(), options.unbind()]),
            new_target,
            gc.reborrow(),
        )
        .unbind()?
        .scope(agent, gc.nogc());
        // 5. Let errorsList be ? IteratorToList(? GetIterator(errors, sync)).
        let Some(iterator_record) = get_iterator(agent, errors.get(agent), false, gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_iterator_record()
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let errors_list =
            iterator_to_list(agent, iterator_record.unbind(), gc.reborrow()).unbind()?;
        let gc = gc.into_nogc();
        let o = unsafe { o.take(agent) }.bind(gc);
        // 6. Perform ! DefinePropertyOrThrow(O, "errors", PropertyDescriptor {
        // [[Configurable]]: true,
        // [[Enumerable]]: false,
        // [[Writable]]: true,
        // [[Value]]: CreateArrayFromList(errorsList)
        let property_descriptor = PropertyDescriptor::non_enumerable_data_descriptor(
            create_array_from_scoped_list(agent, errors_list, gc),
        );
        unwrap_try(o.try_define_own_property(
            agent,
            BUILTIN_STRING_MEMORY.errors.into(),
            property_descriptor,
            None,
            gc,
        ));
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
