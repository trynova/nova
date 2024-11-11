// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct FinalizationRegistryPrototype;

struct FinalizationRegistryPrototypeRegister;
impl Builtin for FinalizationRegistryPrototypeRegister {
    const NAME: String = BUILTIN_STRING_MEMORY.register;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(FinalizationRegistryPrototype::register);
}
struct FinalizationRegistryPrototypeUnregister;
impl Builtin for FinalizationRegistryPrototypeUnregister {
    const NAME: String = BUILTIN_STRING_MEMORY.unregister;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(FinalizationRegistryPrototype::unregister);
}

impl FinalizationRegistryPrototype {
    fn register(
        _agent: &mut Agent,
        _gc: GcScope<'_, '_>,

        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn unregister(
        _agent: &mut Agent,
        _gc: GcScope<'_, '_>,

        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.finalization_registry_prototype();
        let finalization_registry_constructor = intrinsics.finalization_registry();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(4)
            .with_prototype(object_prototype)
            .with_constructor_property(finalization_registry_constructor)
            .with_builtin_function_property::<FinalizationRegistryPrototypeRegister>()
            .with_builtin_function_property::<FinalizationRegistryPrototypeUnregister>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.FinalizationRegistry.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
