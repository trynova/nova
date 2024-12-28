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

pub(crate) struct WeakSetConstructor;
impl Builtin for WeakSetConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakSet;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(WeakSetConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for WeakSetConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakSet;
}

impl WeakSetConstructor {
    fn behaviour(
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
        let weak_set_prototype = intrinsics.weak_set_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakSetConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_set_prototype.into_object())
            .build();
    }
}
