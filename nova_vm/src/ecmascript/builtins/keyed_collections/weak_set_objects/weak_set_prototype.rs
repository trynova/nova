// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::types::IntoValue;
use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct WeakSetPrototype;

struct WeakSetPrototypeAdd;
impl Builtin for WeakSetPrototypeAdd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.add;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakSetPrototype::add);
}
struct WeakSetPrototypeDelete;
impl Builtin for WeakSetPrototypeDelete {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.delete;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakSetPrototype::delete);
}
struct WeakSetPrototypeHas;
impl Builtin for WeakSetPrototypeHas {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakSetPrototype::has);
}

impl WeakSetPrototype {
    fn add<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("WeakSet.prototype.add", gc.into_nogc()))
    }

    fn delete<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("WeakSet.prototype.delete", gc.into_nogc()))
    }

    fn has<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("WeakSet.prototype.has", gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.weak_set_prototype();
        let weak_set_constructor = intrinsics.weak_set();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<WeakSetPrototypeAdd>()
            .with_constructor_property(weak_set_constructor)
            .with_builtin_function_property::<WeakSetPrototypeDelete>()
            .with_builtin_function_property::<WeakSetPrototypeHas>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.WeakMap.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
