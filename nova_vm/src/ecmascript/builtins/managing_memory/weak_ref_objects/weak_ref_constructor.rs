// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoObject, Object, String, Value},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct WeakRefConstructor;
impl Builtin for WeakRefConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakRef;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for WeakRefConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakRef;
}

impl WeakRefConstructor {
    fn constructor<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let weak_ref_prototype = intrinsics.weak_ref_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakRefConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_ref_prototype.into_object())
            .build();
    }
}
