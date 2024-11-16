// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{hash::Hasher, ops::Index};

use ahash::AHasher;

use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::call_function,
            testing_and_comparison::{is_callable, same_value},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIterator,
            map::{data::MapData, Map},
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{HeapNumber, IntoValue, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{Heap, IntrinsicFunctionIndexes, PrimitiveHeap, WellKnownSymbolIndexes},
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
    const KEY: Option<PropertyKey> = Some(BUILTIN_STRING_MEMORY.size.to_property_key());
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
    fn clear(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;
        // 3. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        // a. Set p.[[Key]] to EMPTY.
        // b. Set p.[[Value]] to EMPTY.
        agent[m].clear();
        // 4. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.1.3.3 Map.prototype.delete ( key )](https://tc39.es/ecma262/#sec-map.prototype.delete)
    ///
    /// > #### Note
    /// > The value EMPTY is used as a specification device to indicate that an
    /// > entry has been deleted. Actual implementations may take other actions
    /// > such as physically removing the entry from internal data structures.
    fn delete(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(numbers, arguments.get(0));
        let key_hash = {
            let mut hasher = AHasher::default();
            key.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let MapData {
            keys,
            values,
            map_data,
            ..
        } = maps[m].borrow_mut(&primitive_heap);
        let map_data = map_data.get_mut();

        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, then
        if let Ok(entry) = map_data.find_entry(key_hash, |hash_equal_index| {
            let found_key = keys[*hash_equal_index as usize].unwrap();
            // Quick check: Equal keys have the same value.
            found_key == key || same_value(&primitive_heap, found_key, key)
        }) {
            let index = *entry.get() as usize;
            let _ = entry.remove();
            // i. Set p.[[Key]] to EMPTY.
            keys[index] = None;
            // ii. Set p.[[Value]] to EMPTY.
            values[index] = None;
            // iii. Return true.
            Ok(true.into())
        } else {
            // 5. Return false.
            Ok(false.into())
        }
    }

    fn entries(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let M be the this value.
        // 2. Return ? CreateMapIterator(M, KEY+VALUE).

        // 24.1.5.1 CreateMapIterator ( map, kind )
        // 1. Perform ? RequireInternalSlot(map, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;
        Ok(MapIterator::from_map(agent, m, CollectionIteratorKind::KeyAndValue).into_value())
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
    fn for_each(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let callback_fn = arguments.get(0);
        let this_arg = arguments.get(1);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                *gc,
                ExceptionType::TypeError,
                "Callback function parameter is not callable",
            ));
        };
        // 4. Let entries be M.[[MapData]].
        // 5. Let numEntries be the number of elements in entries.
        let mut num_entries = agent[m].values().len();
        // 6. Let index be 0.
        let mut index = 0;
        // 7. Repeat, while index < numEntries,
        while index < num_entries {
            // a. Let e be entries[index].
            let data = &agent[m];
            let entry_index = index;
            // b. Set index to index + 1.
            index += 1;
            let k = data.keys()[entry_index];
            // c. If e.[[Key]] is not EMPTY, then
            if let Some(k) = k {
                let v = data.values()[entry_index].unwrap();
                // i. Perform ? Call(callbackfn, thisArg, « e.[[Value]], e.[[Key]], M »).
                call_function(
                    agent,
                    gc.reborrow(),
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[v, k, m.into_value()])),
                )?;
                // ii. NOTE: The number of elements in entries may have increased during execution of callbackfn.
                // iii. Set numEntries to the number of elements in entries.
                num_entries = agent[m].values().len();
            }
        }
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.1.3.6 Map.prototype.get ( key )](https://tc39.es/ecma262/#sec-map.prototype.get)
    fn get(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(agent, arguments.get(0));
        let key_hash = {
            let mut hasher = AHasher::default();
            key.hash(agent, &mut hasher);
            hasher.finish()
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let MapData {
            keys,
            values,
            map_data,
            ..
        } = &maps[m].borrow(&primitive_heap);
        let map_data = map_data.borrow();

        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, return p.[[Value]].
        let found = map_data.find(key_hash, |hash_equal_index| {
            let found_key = keys[*hash_equal_index as usize].unwrap();
            // Quick check: Equal keys have the same value.
            found_key == key || same_value(agent, found_key, key)
        });
        if let Some(index) = found {
            Ok(values[*index as usize].unwrap())
        } else {
            // 5. Return undefined.
            Ok(Value::Undefined)
        }
    }

    /// ### [24.1.3.7 Map.prototype.has ( key )](https://tc39.es/ecma262/#sec-map.prototype.has)
    fn has(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(numbers, arguments.get(0));
        let key_hash = {
            let mut hasher = AHasher::default();
            key.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let MapData { keys, map_data, .. } = &mut maps[m].borrow_mut(&primitive_heap);
        let map_data = map_data.get_mut();

        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, return true.
        // 5. Return false.
        let found = map_data
            .find(key_hash, |hash_equal_index| {
                let found_key = keys[*hash_equal_index as usize].unwrap();
                // Quick check: Equal keys have the same value.
                found_key == key || same_value(&primitive_heap, found_key, key)
            })
            .is_some();
        Ok(found.into())
    }

    fn keys(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let M be the this value.
        // 2. Return ? CreateMapIterator(M, KEY).

        // 24.1.5.1 CreateMapIterator ( map, kind )
        // 1. Perform ? RequireInternalSlot(map, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;
        Ok(MapIterator::from_map(agent, m, CollectionIteratorKind::Key).into_value())
    }

    /// ### [24.1.3.9 Map.prototype.set ( key, value )](https://tc39.es/ecma262/#sec-map.prototype.set)
    fn set(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let value = arguments.get(1);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

        let MapData {
            keys,
            values,
            map_data,
            ..
        } = &mut maps[m].borrow_mut(&primitive_heap);
        let map_data = map_data.get_mut();

        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            value.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(numbers, arguments.get(0));
        let key_hash = hasher(key);
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, then
        let entry = map_data.entry(
            key_hash,
            |hash_equal_index| {
                let found_key = keys[*hash_equal_index as usize].unwrap();
                // Quick check: Equal keys have the same value.
                found_key == key || same_value(&primitive_heap, found_key, key)
            },
            |index_to_hash| hasher(keys[*index_to_hash as usize].unwrap()),
        );
        match entry {
            hashbrown::hash_table::Entry::Occupied(occupied) => {
                let index = *occupied.get();
                // i. Set p.[[Value]] to value.
                values[index as usize] = Some(value);
                // ii. Return M.
            }
            hashbrown::hash_table::Entry::Vacant(vacant) => {
                // 5. Let p be the Record { [[Key]]: key, [[Value]]: value }.
                // 6. Append p to M.[[MapData]].
                let index = u32::try_from(values.len()).unwrap();
                vacant.insert(index);
                keys.push(Some(key));
                values.push(Some(value));
            }
        }
        // 7. Return M.
        Ok(m.into_value())
    }

    fn get_size(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;
        let count = agent[m].size();
        Ok(count.into())
    }

    fn values(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let M be the this value.
        // 2. Return ? CreateMapIterator(M, VALUE).

        // 24.1.5.1 CreateMapIterator ( map, kind )
        // 1. Perform ? RequireInternalSlot(map, [[MapData]]).
        let m = require_map_data_internal_slot(agent, *gc, this_value)?;
        Ok(MapIterator::from_map(agent, m, CollectionIteratorKind::Value).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.map_prototype();
        let map_constructor = intrinsics.map();
        let map_prototype_entries = intrinsics.map_prototype_entries();

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
            .with_builtin_function_property::<MapPrototypeValues>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(map_prototype_entries.into_value())
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
fn require_map_data_internal_slot(agent: &mut Agent, gc: NoGcScope, value: Value) -> JsResult<Map> {
    match value {
        Value::Map(map) => Ok(map),
        _ => Err(agent.throw_exception_with_static_message(
            gc,
            ExceptionType::TypeError,
            "Object is not a Map",
        )),
    }
}

#[inline(always)]
/// ### [24.5.1 CanonicalizeKeyedCollectionKey ( key )](https://tc39.es/ecma262/#sec-canonicalizekeyedcollectionkey)
/// The abstract operation CanonicalizeKeyedCollectionKey takes argument key
/// (an ECMAScript language value) and returns an ECMAScript language value.
pub(crate) fn canonicalize_keyed_collection_key(
    agent: &impl Index<HeapNumber, Output = f64>,
    key: Value,
) -> Value {
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
