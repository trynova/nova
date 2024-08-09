// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct WeakRefConstructor;
impl Builtin for WeakRefConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakRef;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(WeakRefConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for WeakRefConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakRef;
}

impl WeakRefConstructor {
    fn behaviour<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
        _new_target: Option<Object<'gen>>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let weak_ref_prototype = intrinsics.weak_ref_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakRefConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_ref_prototype.into_object())
            .build();
    }
}
