use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoObject, Object, String, Value},
    },
    engine::context::{GcScope, NoGcScope},
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct TemporalDurationConstructor;

impl Builtin for TemporalDurationConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Duration;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(TemporalDurationConstructor::constructor);
}
impl BuiltinIntrinsicConstructor for TemporalDurationConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalDuration;
}

impl TemporalDurationConstructor {
    fn constructor<'gc>(
        _agent: &mut Agent,
        _: Value,
        _args: ArgumentsList,
        _new_target: Option<Object>,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _gc: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let duration_prototype = intrinsics.temporal_duration_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TemporalDurationConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(duration_prototype.into_object())
        .build();
    }
}
