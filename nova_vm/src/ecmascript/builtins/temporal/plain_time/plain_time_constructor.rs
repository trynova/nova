// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinIntrinsicConstructor, String},
    heap::IntrinsicConstructorIndexes,
};

/// Constructor function object for %Temporal.PlainTime%.
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
        agent: &mut Agent,
        _: Value,
        _args: ArgumentsList,
        _new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Temporal.PlainTime", gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _gc: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let plain_time_prototype = intrinsics.temporal_plain_time_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TemporalPlainTimeConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(plain_time_prototype.into())
        .build();
    }
}
