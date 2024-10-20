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

pub(crate) struct FinalizationRegistryConstructor;
impl Builtin for FinalizationRegistryConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.FinalizationRegistry;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(FinalizationRegistryConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for FinalizationRegistryConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::FinalizationRegistry;
}

impl FinalizationRegistryConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _gc: GcScope<'_, '_>,

        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let finalization_registry_prototype = intrinsics.finalization_registry_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<FinalizationRegistryConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(finalization_registry_prototype.into_object())
        .build();
    }
}
