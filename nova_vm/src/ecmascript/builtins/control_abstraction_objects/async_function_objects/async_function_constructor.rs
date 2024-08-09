// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct AsyncFunctionConstructor;
impl Builtin for AsyncFunctionConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.AsyncFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(AsyncFunctionConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for AsyncFunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::AsyncFunction;
}

impl AsyncFunctionConstructor {
    fn behaviour<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
        _new_target: Option<Object<'gen>>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let async_function_prototype = intrinsics.async_function_prototype();
        let function_constructor = intrinsics.function();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AsyncFunctionConstructor>(agent, realm)
            .with_prototype(function_constructor.into_object())
            .with_property_capacity(1)
            .with_prototype_property(async_function_prototype.into_object())
            .build();
    }
}
