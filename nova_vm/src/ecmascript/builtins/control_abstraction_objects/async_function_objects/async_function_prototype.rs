// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        execution::{Agent, Realm},
        types::BUILTIN_STRING_MEMORY,
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct AsyncFunctionPrototype;

impl AsyncFunctionPrototype {
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let function_prototype = intrinsics.function_prototype();
        let this = intrinsics.async_function_prototype();
        let async_function_constructor = intrinsics.async_function();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_prototype(function_prototype)
            .with_property_capacity(2)
            .with_constructor_property(async_function_constructor)
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.AsyncFunction.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
