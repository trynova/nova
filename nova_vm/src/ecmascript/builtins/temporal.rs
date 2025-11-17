// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod duration;
pub mod error;
pub mod instant;
pub mod options;
pub mod plain_time;

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        execution::{Agent, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoValue},
    },
    engine::context::NoGcScope,
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct Temporal;

impl Temporal {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.temporal();

        let instant_constructor = intrinsics.temporal_instant();
        let plain_time_constructor = intrinsics.temporal_plain_time();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(object_prototype)
            // 1.2.1 Temporal.Instant ( . . . )
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.Instant.into())
                    .with_value(instant_constructor.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            // 1.2.2 Temporal.PlainDateTime ( . . . )
            // 1.2.3 Temporal.PlainDate ( . . . )
            // 1.2.4 Temporal.PlainTime ( . . . )
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.PlainTime.into())
                    .with_value(plain_time_constructor.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            // 1.2.5 Temporal.PlainYearMonth ( . . . )
            // 1.2.6 Temporal.PlainMonthDay ( . . . )
            // 1.2.7 Temporal.Duration ( . . . )
            // 1.2.8 Temporal.ZonedDateTime ( . . . )
            // 1.3.1 Temporal.Now
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
