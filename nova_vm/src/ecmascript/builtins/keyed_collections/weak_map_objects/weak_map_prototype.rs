use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct WeakMapPrototype;

struct WeakMapPrototypeDelete;
impl Builtin for WeakMapPrototypeDelete {
    const NAME: String = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::delete);
}
struct WeakMapPrototypeGet;
impl Builtin for WeakMapPrototypeGet {
    const NAME: String = BUILTIN_STRING_MEMORY.get;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::get);
}
struct WeakMapPrototypeHas;
impl Builtin for WeakMapPrototypeHas {
    const NAME: String = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::has);
}
struct WeakMapPrototypeSet;
impl Builtin for WeakMapPrototypeSet {
    const NAME: String = BUILTIN_STRING_MEMORY.set;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::set);
}

impl WeakMapPrototype {
    fn delete(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn has(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.weak_map_prototype();
        let weak_map_constructor = intrinsics.weak_map();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(6)
            .with_constructor_property(weak_map_constructor)
            .with_builtin_function_property::<WeakMapPrototypeDelete>()
            .with_builtin_function_property::<WeakMapPrototypeGet>()
            .with_builtin_function_property::<WeakMapPrototypeHas>()
            .with_builtin_function_property::<WeakMapPrototypeSet>()
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
