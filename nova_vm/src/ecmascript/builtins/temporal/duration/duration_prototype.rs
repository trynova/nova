// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{Agent, BUILTIN_STRING_MEMORY, Realm, builders::OrdinaryObjectBuilder},
    engine::NoGcScope,
    heap::WellKnownSymbols,
};

pub(crate) struct TemporalDurationPrototype;
impl TemporalDurationPrototype {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_duration_prototype();
        let object_prototype = intrinsics.object_prototype();
        let duration_constructor = intrinsics.temporal_duration();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(object_prototype)
            .with_constructor_property(duration_constructor)
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbols::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal_Duration.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
