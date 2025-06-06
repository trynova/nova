// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::setter_that_ignores_prototype_properties,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
            BuiltinSetter, ordinary::ordinary_create_from_constructor,
        },
        execution::{Agent, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, PropertyKey, String,
            Value,
        },
    },
    engine::context::{Bindable, GcScope},
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct IteratorConstructor;
impl Builtin for IteratorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Iterator;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(IteratorConstructor::constructor);
}
impl BuiltinIntrinsicConstructor for IteratorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Iterator;
}

struct IteratorConstructorToStringTag;
impl Builtin for IteratorConstructorToStringTag {
    const NAME: String<'static> = String::EMPTY_STRING;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::ToStringTag.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorConstructor::get_to_string_tag);
}
impl BuiltinGetter for IteratorConstructorToStringTag {
    const GETTER_NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_toStringTag_;
}
impl BuiltinSetter for IteratorConstructorToStringTag {
    const SETTER_NAME: String<'static> = BUILTIN_STRING_MEMORY.set__Symbol_toStringTag_;

    const SETTER_BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorConstructor::set_to_string_tag);
}

impl IteratorConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. If NewTarget is either undefined or the active function object, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator Constructor requires 'new'",
                gc.into_nogc(),
            ));
        };
        if new_target
            == agent
                .running_execution_context()
                .function
                .unwrap()
                .into_object()
        {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator constructor can't be used directly",
                gc.into_nogc(),
            ));
        }

        // 2. Return ? OrdinaryCreateFromConstructor(NewTarget, "%Iterator.prototype%").
        ordinary_create_from_constructor(
            agent,
            Function::try_from(new_target.unbind()).unwrap(),
            ProtoIntrinsics::Iterator,
            gc,
        )
        .map(Into::into)
    }

    fn get_to_string_tag<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return "Iterator".
        Ok(BUILTIN_STRING_MEMORY.Iterator.into_value())
    }

    fn set_to_string_tag<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let v = arguments.get(0);
        // 1. Perform ? SetterThatIgnoresPrototypeProperties(
        setter_that_ignores_prototype_properties(
            agent,
            // this value,
            this_value,
            // %Iterator.prototype%,
            agent
                .current_realm_record()
                .intrinsics()
                .iterator_prototype()
                .into_object(),
            // "constructor",
            BUILTIN_STRING_MEMORY.constructor.to_property_key(),
            // v
            v,
            gc,
        )?;
        // ).
        // 2. Return undefined.
        Ok(Value::Undefined)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let iterator_prototype = intrinsics.iterator_prototype();
        let function_prototype = intrinsics.function_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<IteratorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(function_prototype.into_object())
            .with_prototype_property(iterator_prototype.into_object())
            .build();
    }
}
