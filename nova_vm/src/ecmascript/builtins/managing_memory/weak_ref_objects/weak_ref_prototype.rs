// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct WeakRefPrototype;

struct WeakRefPrototypeDeref;
impl Builtin for WeakRefPrototypeDeref {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.deref;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakRefPrototype::deref);
}

impl WeakRefPrototype {
    fn deref(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.weak_ref_prototype();
        let weak_ref_constructor = intrinsics.weak_ref();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(object_prototype)
            .with_constructor_property(weak_ref_constructor)
            .with_builtin_function_property::<WeakRefPrototypeDeref>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.WeakRef.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
