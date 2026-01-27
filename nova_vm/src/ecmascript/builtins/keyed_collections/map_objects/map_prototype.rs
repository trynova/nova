// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;

use ahash::AHasher;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter,
        BuiltinIntrinsic, CollectionIteratorKind, ExceptionType, HeapNumber, JsResult, Map,
        MapIterator, PropertyKey, Realm, String, Value, builders::OrdinaryObjectBuilder,
        call_function, is_callable, same_value,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable},
    heap::{
        ArenaAccess, ArenaAccessSoA, Heap, IntrinsicFunctionIndexes, PrimitiveHeap,
        WellKnownSymbolIndexes,
    },
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
pub(crate) struct MapPrototypeSet;
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
    fn clear<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value, gc)?;
        // 3. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        // a. Set p.[[Key]] to EMPTY.
        // b. Set p.[[Value]] to EMPTY.
        m.clear(agent);
        // 4. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.1.3.3 Map.prototype.delete ( key )](https://tc39.es/ecma262/#sec-map.prototype.delete)
    ///
    /// > #### Note
    /// > The value EMPTY is used as a specification device to indicate that an
    /// > entry has been deleted. Actual implementations may take other actions
    /// > such as physically removing the entry from internal data structures.
    fn delete<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let key = arguments.get(0).bind(gc);
        let m = require_map_data_internal_slot(agent, this_value, gc)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(numbers, key);
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        let key_hash = {
            let mut hasher = AHasher::default();
            key.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let (map_data, keys, values) = m.get_map_data_mut(maps, &primitive_heap);

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

    /// ### [24.1.3.4 Map.prototype.entries ( )](https://tc39.es/ecma262/#sec-map.prototype.entries)
    fn entries<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let M be the this value.
        // 2. Return ? CreateMapIterator(M, KEY+VALUE).

        // 24.1.5.1 CreateMapIterator ( map, kind )
        // 1. Perform ? RequireInternalSlot(map, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value, gc)?;
        Ok(MapIterator::from_map(agent, m, CollectionIteratorKind::KeyAndValue).into())
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
    fn for_each<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).bind(nogc);
        let this_arg = arguments.get(1).bind(nogc);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let mut m = require_map_data_internal_slot(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn, nogc) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function parameter is not callable",
                gc.into_nogc(),
            ));
        };

        // 4. Let entries be M.[[MapData]].
        // 5. Let numEntries be the number of elements in entries.
        let mut num_entries = m.entries_len(agent);

        let this_arg = this_arg.scope(agent, nogc);
        let callback_fn = callback_fn.scope(agent, nogc);
        let scoped_m = m.scope(agent, nogc);

        // 6. Let index be 0.
        let mut index = 0;
        // 7. Repeat, while index < numEntries,
        while index < num_entries {
            // a. Let e be entries[index].
            let (keys, values) = m.get_entries(agent);
            let entry_index = index;
            // b. Set index to index + 1.
            index += 1;
            let k = keys[entry_index as usize];
            // c. If e.[[Key]] is not EMPTY, then
            if let Some(k) = k {
                let v = values[entry_index as usize].unwrap();
                // i. Perform ? Call(callbackfn, thisArg, « e.[[Value]], e.[[Key]], M »).
                call_function(
                    agent,
                    callback_fn.get(agent),
                    this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        v.unbind(),
                        k.unbind(),
                        m.unbind().into(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?;
                // ii. NOTE: The number of elements in entries may have
                //     increased during execution of callbackfn.
                // iii. Set numEntries to the number of elements in entries.
                m = scoped_m.get(agent).bind(gc.nogc());
                num_entries = m.entries_len(agent);
            }
        }
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.1.3.6 Map.prototype.get ( key )](https://tc39.es/ecma262/#sec-map.prototype.get)
    fn get<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let key = arguments.get(0).bind(gc);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value, gc)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(numbers, key);
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        let key_hash = {
            let mut hasher = AHasher::default();
            key.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };

        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let (map_data, keys, values) = m.get_map_data_mut(maps, &primitive_heap);

        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, return p.[[Value]].
        let found = map_data.find(key_hash, |hash_equal_index| {
            let found_key = keys[*hash_equal_index as usize].unwrap();
            // Quick check: Equal keys have the same value.
            found_key == key || same_value(&primitive_heap, found_key, key)
        });
        if let Some(index) = found {
            Ok(values[*index as usize].unwrap().unbind().bind(gc))
        } else {
            // 5. Return undefined.
            Ok(Value::Undefined)
        }
    }

    /// ### [24.1.3.7 Map.prototype.has ( key )](https://tc39.es/ecma262/#sec-map.prototype.has)
    fn has<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let key = arguments.get(0).bind(gc);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value, gc)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(numbers, key);
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        let key_hash = {
            let mut hasher = AHasher::default();
            key.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        let (map_data, keys, _) = m.get_map_data(maps, &primitive_heap);

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

    /// ### [24.1.3.8 Map.prototype.keys ( )](https://tc39.es/ecma262/#sec-map.prototype.keys)
    fn keys<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let M be the this value.
        // 2. Return ? CreateMapIterator(M, KEY).

        // 24.1.5.1 CreateMapIterator ( map, kind )
        // 1. Perform ? RequireInternalSlot(map, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value, gc)?;
        Ok(MapIterator::from_map(agent, m, CollectionIteratorKind::Key).into())
    }

    /// ### [24.1.3.9 Map.prototype.set ( key, value )](https://tc39.es/ecma262/#sec-map.prototype.set)
    fn set<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let key = arguments.get(0).bind(gc);
        let value = arguments.get(1).bind(gc);
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value, gc)?;

        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        let (map_data, keys, values) = m.get_map_data_mut(maps, &primitive_heap);

        // 3. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(numbers, key);
        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            value.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };
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
                values[index as usize] = Some(value.unbind());
                // ii. Return M.
            }
            hashbrown::hash_table::Entry::Vacant(vacant) => {
                // 5. Let p be the Record { [[Key]]: key, [[Value]]: value }.
                // 6. Append p to M.[[MapData]].
                let index = u32::try_from(values.len()).unwrap();
                vacant.insert(index);
                keys.push(Some(key.unbind()));
                values.push(Some(value.unbind()));
            }
        }
        // 7. Return M.
        Ok(m.into())
    }

    /// ### [24.1.3.10 get Map.prototype.size](https://tc39.es/ecma262/#sec-get-map.prototype.size)
    fn get_size<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let m = require_map_data_internal_slot(agent, this_value, gc)?;
        let count = m.get(agent).size();
        Ok(count.into())
    }

    /// ### [24.1.3.11 Map.prototype.values ( )](https://tc39.es/ecma262/#sec-map.prototype.values)
    fn values<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let M be the this value.
        // 2. Return ? CreateMapIterator(M, VALUE).

        // 24.1.5.1 CreateMapIterator ( map, kind )
        // 1. Perform ? RequireInternalSlot(map, [[MapData]]).
        let m = require_map_data_internal_slot(agent, this_value, gc)?;
        Ok(MapIterator::from_map(agent, m, CollectionIteratorKind::Value).into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
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
                    .with_value(map_prototype_entries.into())
                    .with_enumerable(MapPrototypeValues::ENUMERABLE)
                    .with_configurable(MapPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Map.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[inline(always)]
fn require_map_data_internal_slot<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Map<'a>> {
    match value {
        Value::Map(map) => Ok(map.bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Object is not a Map",
            gc,
        )),
    }
}

#[inline(always)]
/// ### [24.5.1 CanonicalizeKeyedCollectionKey ( key )](https://tc39.es/ecma262/#sec-canonicalizekeyedcollectionkey)
/// The abstract operation CanonicalizeKeyedCollectionKey takes argument key
/// (an ECMAScript language value) and returns an ECMAScript language value.
pub(crate) fn canonicalize_keyed_collection_key<'gc, T>(agent: &T, key: Value<'gc>) -> Value<'gc>
where
    HeapNumber<'gc>: ArenaAccess<T, Output = f64>,
{
    // 1. If key is -0𝔽, return +0𝔽.
    if let Value::SmallF64(key) = key {
        // Note: Only f32 should hold -0.
        if key.into_f64() == -0.0 {
            return 0.into();
        }
    } else if let Value::Number(key) = key {
        debug_assert_ne!(*key.get(agent), -0.0, "HeapNumber should never be -0.0");
    }
    // 2. Return key.
    key
}
