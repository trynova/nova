// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::{Agent, BUILTIN_STRING_MEMORY, builders::OrdinaryObjectBuilder, Realm, String};

pub(crate) struct AggregateErrorPrototype;
impl AggregateErrorPrototype {
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let aggregate_constructor = intrinsics.aggregate_error();
        let this = intrinsics.aggregate_error_prototype();
        let error_prototype = intrinsics.error_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_prototype(error_prototype)
            .with_property_capacity(3)
            .with_constructor_property(aggregate_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.AggregateError.into())
                    .build()
            })
            .build();
    }
}
