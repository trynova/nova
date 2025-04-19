// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoObject, Object, String, Value},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct WeakMapConstructor;
impl Builtin for WeakMapConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakMap;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for WeakMapConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakMap;
}

impl WeakMapConstructor {
    fn constructor<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let weak_map_prototype = intrinsics.weak_map_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakMapConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_map_prototype.into_object())
            .build();
    }
}
