use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct SetPrototype;

struct SetPrototypeAdd;
impl Builtin for SetPrototypeAdd {
    const NAME: String = BUILTIN_STRING_MEMORY.add;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::add);
}
struct SetPrototypeClear;
impl Builtin for SetPrototypeClear {
    const NAME: String = BUILTIN_STRING_MEMORY.clear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::clear);
}
struct SetPrototypeDelete;
impl Builtin for SetPrototypeDelete {
    const NAME: String = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::delete);
}
struct SetPrototypeEntries;
impl Builtin for SetPrototypeEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::entries);
}
struct SetPrototypeForEach;
impl Builtin for SetPrototypeForEach {
    const NAME: String = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::for_each);
}
struct SetPrototypeHas;
impl Builtin for SetPrototypeHas {
    const NAME: String = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::has);
}
struct SetPrototypeKeys;
impl Builtin for SetPrototypeKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::keys);
}
struct SetPrototypeGetSize;
impl Builtin for SetPrototypeGetSize {
    const NAME: String = BUILTIN_STRING_MEMORY.get_size;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::get_size);
}
impl BuiltinGetter for SetPrototypeGetSize {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.size.to_property_key();
}
struct SetPrototypeValues;
impl Builtin for SetPrototypeValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::values);
}
impl BuiltinIntrinsic for SetPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::SetPrototypeValues;
}

impl SetPrototype {
    fn add(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn clear(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn entries(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn delete(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn for_each(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn has(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn keys(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_size(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn values(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.set_prototype();
        let set_constructor = intrinsics.set();
        let set_prototype_values = intrinsics.set_prototype_values();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(12)
            .with_builtin_function_property::<SetPrototypeAdd>()
            .with_builtin_function_property::<SetPrototypeClear>()
            .with_constructor_property(set_constructor)
            .with_builtin_function_property::<SetPrototypeDelete>()
            .with_builtin_function_property::<SetPrototypeEntries>()
            .with_builtin_function_property::<SetPrototypeForEach>()
            .with_builtin_function_property::<SetPrototypeHas>()
            .with_builtin_function_property::<SetPrototypeKeys>()
            .with_builtin_function_getter_property::<SetPrototypeGetSize>()
            .with_builtin_intrinsic_function_property::<SetPrototypeValues>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(set_prototype_values.into_value())
                    .with_enumerable(SetPrototypeValues::ENUMERABLE)
                    .with_configurable(SetPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Set.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
