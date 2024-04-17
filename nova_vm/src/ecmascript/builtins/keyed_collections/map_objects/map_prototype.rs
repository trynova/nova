use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct MapPrototype;

struct MapPrototypeClear;
impl Builtin for MapPrototypeClear {
    const NAME: String = BUILTIN_STRING_MEMORY.clear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::clear);
}
struct MapPrototypeDelete;
impl Builtin for MapPrototypeDelete {
    const NAME: String = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::delete);
}
struct MapPrototypeEntries;
impl Builtin for MapPrototypeEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::entries);
}
impl BuiltinIntrinsic for MapPrototypeEntries {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::MapPrototypeEntries;
}
struct MapPrototypeForEach;
impl Builtin for MapPrototypeForEach {
    const NAME: String = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::for_each);
}
struct MapPrototypeGet;
impl Builtin for MapPrototypeGet {
    const NAME: String = BUILTIN_STRING_MEMORY.get;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::get);
}
struct MapPrototypeHas;
impl Builtin for MapPrototypeHas {
    const NAME: String = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::has);
}
struct MapPrototypeKeys;
impl Builtin for MapPrototypeKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::keys);
}
struct MapPrototypeSet;
impl Builtin for MapPrototypeSet {
    const NAME: String = BUILTIN_STRING_MEMORY.set;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::set);
}
struct MapPrototypeGetSize;
impl Builtin for MapPrototypeGetSize {
    const NAME: String = BUILTIN_STRING_MEMORY.get_size;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::get_size);
}
impl BuiltinGetter for MapPrototypeGetSize {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.size.to_property_key();
}
struct MapPrototypeValues;
impl Builtin for MapPrototypeValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::values);
}

impl MapPrototype {
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

    fn get(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn has(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn keys(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
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
        let this = intrinsics.map_prototype();
        let map_constructor = intrinsics.map();

        let mut map_prototype_values: Option<Value> = None;

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(13)
            .with_builtin_function_property::<MapPrototypeClear>()
            .with_constructor_property(map_constructor)
            .with_builtin_function_property::<MapPrototypeDelete>()
            .with_builtin_intrinsic_function_property::<MapPrototypeEntries>()
            .with_builtin_function_property::<MapPrototypeForEach>()
            .with_builtin_function_property::<MapPrototypeGet>()
            .with_builtin_function_property::<MapPrototypeHas>()
            .with_builtin_function_property::<MapPrototypeKeys>()
            .with_builtin_function_property::<MapPrototypeSet>()
            .with_builtin_function_getter_property::<MapPrototypeGetSize>()
            .with_property(|builder| {
                builder
                    .with_key(MapPrototypeValues::NAME.into())
                    .with_value_creator(|agent| {
                        let value = BuiltinFunctionBuilder::new::<MapPrototypeValues>(agent, realm)
                            .build()
                            .into_value();
                        map_prototype_values = Some(value);
                        value
                    })
                    .with_enumerable(MapPrototypeValues::ENUMERABLE)
                    .with_configurable(MapPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(map_prototype_values.unwrap())
                    .with_enumerable(MapPrototypeValues::ENUMERABLE)
                    .with_configurable(MapPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Map.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
