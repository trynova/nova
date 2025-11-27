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

pub(crate) struct TemporalPlainTimeConstructor;

impl Builtin for TemporalPlainTimeConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainTime;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(TemporalPlainTimeConstructor::constructor);
}

impl BuiltinIntrinsicConstructor for TemporalPlainTimeConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalPlainTime;
}

impl TemporalPlainTimeConstructor {
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
        let plain_time_prototype = intrinsics.temporal_plain_time_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TemporalPlainTimeConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(plain_time_prototype.into_object())
        .build();
    }
}
