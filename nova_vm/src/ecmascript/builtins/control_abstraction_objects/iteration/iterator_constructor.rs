// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
            ordinary::ordinary_create_from_constructor,
        },
        execution::{Agent, JsResult, ProtoIntrinsics, RealmIdentifier, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, Function, IntoObject, Object, String, Value},
    },
    engine::context::{Bindable, GcScope},
    heap::IntrinsicConstructorIndexes,
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

impl IteratorConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. If NewTarget is either undefined or the active function object, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator Constructor requires 'new'",
                gc.nogc(),
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

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier<'static>) {
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
