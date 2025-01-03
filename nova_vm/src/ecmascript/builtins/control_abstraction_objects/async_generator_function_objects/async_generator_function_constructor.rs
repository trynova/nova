// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct AsyncGeneratorFunctionConstructor;
impl Builtin for AsyncGeneratorFunctionConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.AsyncGeneratorFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for AsyncGeneratorFunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::AsyncGeneratorFunction;
}

impl AsyncGeneratorFunctionConstructor {
    fn constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let function_constructor = intrinsics.function();
        let async_generator_function_prototype = intrinsics.async_generator_function_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AsyncGeneratorFunctionConstructor>(
            agent, realm,
        )
        .with_prototype(function_constructor.into_object())
        .with_property_capacity(1)
        .with_prototype_property(async_generator_function_prototype.into_object())
        .build();
    }
}
