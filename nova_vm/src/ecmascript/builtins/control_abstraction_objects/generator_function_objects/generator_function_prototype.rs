// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{Agent, BUILTIN_STRING_MEMORY, OrdinaryObjectBuilder, Realm},
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct GeneratorFunctionPrototype;

impl GeneratorFunctionPrototype {
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let function_prototype = intrinsics.function_prototype();
        let generator_prototype = intrinsics.generator_prototype();
        let this = intrinsics.generator_function_prototype();
        let generator_function_constructor = intrinsics.generator_function();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(function_prototype)
            .with_property(|builder| {
                builder
                    .with_value_readonly(generator_function_constructor.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .with_key(BUILTIN_STRING_MEMORY.constructor.into())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.prototype.into())
                    .with_value_readonly(generator_prototype.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.GeneratorFunction.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
