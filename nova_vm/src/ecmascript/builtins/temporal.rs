// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.


use temporal_rs::{Instant, Duration, /* etc */};

use core::f64::consts;
use std::thread::Builder;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_big_int, to_number, to_number_primitive, to_uint32},
        builders::{self, ordinary_object_builder::OrdinaryObjectBuilder},
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{agent, Agent, JsResult, Realm},
        types::{IntoValue, Number, Primitive, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::WellKnownSymbolIndexes,
};


pub(crate) struct TemporalObject;


impl TemporalObject {
    pub fn create_intrinsic(
        agent: &mut Agent,
        realm: Realm<'static>,
        gc: NoGcScope,
    ) { 
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.temporal();

        let builders = OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(1)
            .with_prototype(object_prototype)
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



/* 
struct TemporalPlainDateTime;
impl Builtin for TemporalPlainDateTime {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainDateTime;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalObject::PlainDateTime);
}

struct TemporalPlainDate;
impl Builtin for TemporalPlainDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainDate;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalObject::PlainDate);
}

struct TemporalPlainTime;
impl Builtin for TemporalPlainTime {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainTime;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalObject::PlainTime);
}

struct TemporalPlainYearMonth;
impl Builtin for TemporalPlainYearMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainYearMonth;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalObject::PlainYearMonth);
}

struct TemporalPlainMonthDay;
impl Builtin for TemporalPlainMonthDay {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainMonthDay;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalObject::PlainMonthDay);
}

struct TemporalDuration;
impl Builtin for TemporalDuration {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Duration;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalObject::Duration);
}

struct TemporalZonedDateTime;
impl Builtin for TemporalZonedDateTime {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.ZonedDateTime;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalObject::ZonedDateTime);
}*/