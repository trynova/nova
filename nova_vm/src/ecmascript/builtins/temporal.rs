// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod duration;
pub mod instant;

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        execution::{Agent, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoValue},
    },
    engine::context::NoGcScope,
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct TemporalObject;

impl TemporalObject {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.temporal();

        let temporal_instant_constructor = intrinsics.temporal_instant();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(object_prototype)
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.Instant.into())
                    .with_value(temporal_instant_constructor.into_value())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .build();
    }
}
