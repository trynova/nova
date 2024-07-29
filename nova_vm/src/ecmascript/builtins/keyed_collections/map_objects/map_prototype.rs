// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::call_function,
            testing_and_comparison::{is_callable, same_value},
        },
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{map::Map, ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct MapPrototype;

struct MapPrototypeClear;
impl Builtin for MapPrototypeClear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.clear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::clear);
}
struct MapPrototypeDelete;
impl Builtin for MapPrototypeDelete {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.delete;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::delete);
}
struct MapPrototypeEntries;
impl Builtin for MapPrototypeEntries {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::entries);
}
impl BuiltinIntrinsic for MapPrototypeEntries {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::MapPrototypeEntries;
}
struct MapPrototypeForEach;
impl Builtin for MapPrototypeForEach {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::for_each);
}
struct MapPrototypeGet;
impl Builtin for MapPrototypeGet {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::get);
}
struct MapPrototypeHas;
impl Builtin for MapPrototypeHas {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::has);
}
struct MapPrototypeKeys;
impl Builtin for MapPrototypeKeys {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::keys);
}
pub(super) struct MapPrototypeSet;
impl Builtin for MapPrototypeSet {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.set;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::set);
}
struct MapPrototypeGetSize;
impl Builtin for MapPrototypeGetSize {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_size;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.size.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::get_size);
}
impl BuiltinGetter for MapPrototypeGetSize {}
struct MapPrototypeValues;
impl Builtin for MapPrototypeValues {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapPrototype::values);
}

impl MapPrototype {
    /// ### [24.1.3.1 Map.prototype.clear ( )](https://tc39.es/ecma262/#sec-map.prototype.clear)
    ///
    /// > #### Note
    /// > The existing \[\[MapData]] List is preserved because there may be
    /// > existing Map Iterator objects that are suspended midway through
    /// > iterating over that List.
    fn clear<'gen>(agent: &mut Agent<'gen>, this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value)?;
        // 3. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let data = &mut agent[m];
        // a. Set p.[[Key]] to EMPTY.
        // b. Set p.[[Value]] to EMPTY.
        data.keys.fill(None);
        data.values.fill(None);
        // 4. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.1.3.3 Map.prototype.delete ( key )](https://tc39.es/ecma262/#sec-map.prototype.delete)
    ///
    /// > #### Note
    /// > The value EMPTY is used as a specification device to indicate that an
    /// > entry has been deleted. Actual implementations may take other actions
    /// > such as physically removing the entry from internal data structures.
    fn delete<'gen>(agent: &mut Agent<'gen>, this_value: Value<'gen>, arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value)?;
        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(agent, arguments.get(0));
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let data = &agent[m];
        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, then
        let found = data
            .keys
            .iter()
            .enumerate()
            .find(|&(_, p_key)| {
                p_key.map_or(false, |p_key| p_key == key || same_value(agent, p_key, key))
            })
            .map(|(index, _)| index);
        if let Some(index) = found {
            let data = &mut agent[m];
            // i. Set p.[[Key]] to EMPTY.
            data.keys[index] = None;
            // ii. Set p.[[Value]] to EMPTY.
            data.values[index] = None;
            // iii. Return true.
            Ok(true.into())
        } else {
            // 5. Return false.
            Ok(false.into())
        }
    }

    fn entries<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    /// ### [24.1.3.5 Map.prototype.forEach ( callbackfn \[ , thisArg \] )](https://tc39.es/ecma262/#sec-map.prototype.foreach)
    ///
    /// > #### Note
    /// > `callbackfn` should be a function that accepts three arguments.
    /// > `forEach` calls `callbackfn` once for each key/value pair present in
    /// > the Map, in key insertion order. `callbackfn` is called only for keys
    /// > of the Map which actually exist; it is not called for keys that have
    /// > been deleted from the Map.
    /// >
    /// > If a `thisArg` parameter is provided, it will be used as the this
    /// > value for each invocation of `callbackfn`. If it is not provided,
    /// > undefined is used instead.
    /// >
    /// > `callbackfn` is called with three arguments: the value of the item,
    /// > the key of the item, and the Map being traversed.
    /// >
    /// > `forEach` does not directly mutate the object on which it is called
    /// > but the object may be mutated by the calls to `callbackfn`. Each
    /// > entry of a map's \[\[MapData]] is only visited once. New keys added
    /// > after the call to `forEach` begins are visited. A key will be
    /// > revisited if it is deleted after it has been visited and then
    /// > re-added before the `forEach` call completes. Keys that are deleted
    /// > after the call to `forEach` begins and before being visited are not
    /// > visited unless the key is added again before the `forEach` call
    /// > completes.
    fn for_each<'gen>(agent: &mut Agent<'gen>, this_value: Value<'gen>, arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        let callback_fn = arguments.get(0);
        let this_arg = arguments.get(1);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function parameter is not callable",
            ));
        };
        // 4. Let entries be M.[[MapData]].
        // 5. Let numEntries be the number of elements in entries.
        let mut num_entries = agent[m].keys.len();
        // 6. Let index be 0.
        let mut index = 0;
        // 7. Repeat, while index < numEntries,
        while index < num_entries {
            // a. Let e be entries[index].
            let data = &agent[m];
            let entry_index = index;
            // b. Set index to index + 1.
            index += 1;
            let k = data.keys[entry_index];
            // c. If e.[[Key]] is not EMPTY, then
            if let Some(k) = k {
                let v = data.values[entry_index].unwrap();
                // i. Perform ? Call(callbackfn, thisArg, « e.[[Value]], e.[[Key]], M »).
                call_function(
                    agent,
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[v, k, m.into_value()])),
                )?;
                // ii. NOTE: The number of elements in entries may have increased during execution of callbackfn.
                // iii. Set numEntries to the number of elements in entries.
                num_entries = agent[m].keys.len();
            }
        }
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.1.3.6 Map.prototype.get ( key )](https://tc39.es/ecma262/#sec-map.prototype.get)
    fn get<'gen>(agent: &mut Agent<'gen>, this_value: Value<'gen>, arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value)?;
        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(agent, arguments.get(0));
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let data = &agent[m];
        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, return p.[[Value]].
        let found = data
            .keys
            .iter()
            .enumerate()
            .find(|&(_, p_key)| {
                p_key.map_or(false, |p_key| p_key == key || same_value(agent, p_key, key))
            })
            .map(|(index, _)| index);
        if let Some(index) = found {
            Ok(data.values[index].unwrap())
        } else {
            // 5. Return undefined.
            Ok(Value::Undefined)
        }
    }

    /// ### [24.1.3.7 Map.prototype.has ( key )](https://tc39.es/ecma262/#sec-map.prototype.has)
    fn has<'gen>(agent: &mut Agent<'gen>, this_value: Value<'gen>, arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value)?;
        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(agent, arguments.get(0));
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let data = &agent[m];
        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, return true.
        // 5. Return false.
        let found = data.keys.iter().any(|&p_key| {
            p_key.map_or(false, |p_key| p_key == key || same_value(agent, p_key, key))
        });
        Ok(found.into())
    }

    fn keys<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    /// ### [24.1.3.9 Map.prototype.set ( key, value )](https://tc39.es/ecma262/#sec-map.prototype.set)
    fn set<'gen>(agent: &mut Agent<'gen>, this_value: Value<'gen>, arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        let value = arguments.get(1);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value)?;
        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(agent, arguments.get(0));
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let data = &agent[m];
        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, then
        let found = data
            .keys
            .iter()
            .enumerate()
            .find(|&(_, p_key)| {
                p_key.map_or(false, |p_key| p_key == key || same_value(agent, p_key, key))
            })
            .map(|(index, _)| index);
        let data = &mut agent[m];
        if let Some(index) = found {
            // i. Set p.[[Value]] to value.
            data.values[index] = Some(value);
        } else {
            // 5. Let p be the Record { [[Key]]: key, [[Value]]: value }.
            // 6. Append p to M.[[MapData]].
            data.keys.push(Some(key));
            data.values.push(Some(value));
        }
        // 7. Return M.
        Ok(m.into_value())
    }

    fn get_size<'gen>(agent: &mut Agent<'gen>, this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        let m = require_map_data_internal_slot(agent, this_value)?;
        let count = agent[m].keys.iter().filter(|key| key.is_some()).count() as u32;
        Ok(count.into())
    }

    fn values<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.map_prototype();
        let map_constructor = intrinsics.map();

        let mut map_prototype_values: Option<Value<'gen>> = None;

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(13)
            .with_prototype(object_prototype)
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

#[inline(always)]
fn require_map_data_internal_slot<'gen>(agent: &mut Agent<'gen>, value: Value<'gen>) -> JsResult<'gen, Map> {
    match value {
        Value::Map(map) => Ok(map),
        _ => Err(agent
            .throw_exception_with_static_message(ExceptionType::TypeError, "Object is not a Map")),
    }
}

#[inline(always)]
/// ### [24.5.1 CanonicalizeKeyedCollectionKey ( key )](https://tc39.es/ecma262/#sec-canonicalizekeyedcollectionkey)
/// The abstract operation CanonicalizeKeyedCollectionKey takes argument key
/// (an ECMAScript language value) and returns an ECMAScript language value.
pub(crate) fn canonicalize_keyed_collection_key(agent: &Agent<'gen>, key: Value<'gen>) -> Value<'gen> {
    // 1. If key is -0𝔽, return +0𝔽.
    if let Value::SmallF64(key) = key {
        // Note: Only f32 should hold -0.
        if key.into_f64() == -0.0 {
            return 0.into();
        }
    } else if let Value::Number(key) = key {
        debug_assert_ne!(agent[key], -0.0, "HeapNumber should never be -0.0");
    }
    // 2. Return key.
    key
}
