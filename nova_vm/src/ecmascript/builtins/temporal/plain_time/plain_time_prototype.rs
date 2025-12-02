use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        execution::{Agent, Realm},
        types::BUILTIN_STRING_MEMORY,
    },
    engine::context::NoGcScope,
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct TemporalPlainTimePrototype;
impl TemporalPlainTimePrototype {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_plain_time_prototype();
        let object_prototype = intrinsics.object_prototype();
        let plain_time_constructor = intrinsics.temporal_plain_time();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(object_prototype)
            .with_constructor_property(plain_time_constructor)
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal_PlainTime.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
