// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;
use core::ops::ControlFlow;

use ahash::AHasher;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, ArrayHeap, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter,
        BuiltinIntrinsicConstructor, ExceptionType, Function, IteratorRecord, JsResult, Map,
        MapHeapData, MapPrototypeSet, Object, PropertyKey, ProtoIntrinsics, Realm, String,
        TryError, TryGetResult, Value, builders::BuiltinFunctionBuilder, call_function,
        canonicalize_keyed_collection_key, create_array_from_list, get, get_iterator, get_method,
        group_by_collection, handle_try_get_result, if_abrupt_close_iterator, is_callable,
        iterator_close_with_error, iterator_step_value, ordinary_create_from_constructor,
        same_value, throw_not_callable, try_get,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::{
        ArenaAccess, CreateHeapData, Heap, IntrinsicConstructorIndexes, PrimitiveHeap,
        WellKnownSymbolIndexes,
    },
};

pub(crate) struct MapConstructor;
impl Builtin for MapConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for MapConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Map;
}
struct MapGroupBy;
impl Builtin for MapGroupBy {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapConstructor::group_by);
    const LENGTH: u8 = 2;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.groupBy;
}
struct MapGetSpecies;
impl Builtin for MapGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapConstructor::get_species);
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for MapGetSpecies {}

impl MapConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _: Value<'static>,
        arguments: ArgumentsList<'_, 'static>,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        let nogc = gc.nogc();
        let iterable = arguments.get(0);
        let no_iterable = iterable.is_undefined() || iterable.is_null();
        crate::engine::bind!(let new_target = new_target, gc);

        // If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Constructor Map requires 'new'",
                gc.into_nogc(),
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%Map.prototype%", « [[MapData]] »).
        // 4. If iterable is either undefined or null, return map.
        if no_iterable {
            return Ok(ordinary_create_from_constructor(
                agent,
                new_target,
                ProtoIntrinsics::Map,
                gc,
            )?
            .into());
        }
        let iterable = iterable.scope(agent, nogc);
        let mut map = Map::try_from(ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::Map,
            gc.reborrow(),
        )?)
        .unwrap();
        // 3. Set map.[[MapData]] to a new empty List.
        // Note
        // If the parameter iterable is present, it is expected to be an
        // object that implements an @@iterator method that returns an
        // iterator object that produces a two element array-like object
        // whose first element is a value that will be used as a Map key
        // and whose second element is the value to associate with that
        // key.

        // 5. Let adder be ? Get(map, "set").
        let adder = try_get(
            agent,
            map,
            BUILTIN_STRING_MEMORY.set.to_property_key(),
            None,
            gc.nogc(),
        );
        let adder = match adder {
            ControlFlow::Continue(TryGetResult::Unset) => Value::Undefined,
            ControlFlow::Continue(TryGetResult::Value(v)) => v,
            ControlFlow::Break(TryError::Err(e)) => {
                return Err(e);
            }
            _ => {
                let scoped_map = map.scope(agent, gc.nogc());
                let adder = handle_try_get_result(
                    agent,
                    map,
                    BUILTIN_STRING_MEMORY.set.to_property_key(),
                    adder,
                    gc.reborrow(),
                )?;
                let gc = gc.nogc();
                // SAFETY: not shared.
                map = unsafe { scoped_map.take(agent).local() };
                adder
            }
        };
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        let Some(adder) = is_callable(adder, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Map.prototype.set is not callable",
                gc.into_nogc(),
            ));
        };
        // 7. Return ? AddEntriesFromIterable(map, iterable, adder).
        add_entries_from_iterable_map_constructor(
            agent,
            map,
            iterable.get(agent).local(),
            adder,
            gc,
        )
        .map(|result| result.into())
    }

    /// ### [24.1.2.1 Map.groupBy ( items, callback )](https://tc39.es/ecma262/#sec-map.groupby)
    fn group_by<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList<'_, 'static>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        crate::engine::bind!(let items = arguments.get(0), gc);
        crate::engine::bind!(let callback_fn = arguments.get(1), gc);
        // 1. Let groups be ? GroupBy(items, callback, collection).
        let groups = group_by_collection(agent, items, callback_fn, gc.reborrow())?;
        // 2. Let map be ! Construct(%Map%).
        let gc = gc.into_nogc();
        crate::engine::bind!(let groups = groups, gc);
        let map_data = MapHeapData::with_capacity(groups.len());
        crate::engine::bind!(let map = agent.heap.create(map_data), gc);

        // 3. For each Record { [[Key]], [[Elements]] } g of groups, do
        let keys_and_elements = groups
            .into_collection_keyed_iter()
            .map(|(key, elements)| {
                // a. Let elements be CreateArrayFromList(g.[[Elements]]).
                let elements = create_array_from_list(agent, &elements, gc);
                (key, elements)
            })
            .collect::<Vec<_>>();

        let Heap {
            maps,
            bigints,
            numbers,
            strings,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

        let (map_data, keys, values) = map.get_map_data_mut(maps, &primitive_heap);
        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            value.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };

        for (key, elements) in keys_and_elements {
            let key_hash = hasher(key);
            let entry = map_data.entry(
                key_hash,
                |hash_equal_index| {
                    let found_key = keys[*hash_equal_index as usize].unwrap();
                    found_key == key || same_value(&primitive_heap, found_key, key)
                },
                |index_to_hash| hasher(keys[*index_to_hash as usize].unwrap()),
            );
            match entry {
                hashbrown::hash_table::Entry::Occupied(occupied) => {
                    let index = *occupied.get();
                    values[index as usize] = Some(elements.into());
                }
                hashbrown::hash_table::Entry::Vacant(vacant) => {
                    // b. Let entry be the Record { [[Key]]: g.[[Key]], [[Value]]: elements }.
                    // c. Append entry to map.[[MapData]].
                    let index = u32::try_from(values.len()).unwrap();
                    vacant.insert(index);
                    keys.push(Some(key));
                    values.push(Some(elements.into()));
                }
            }
        }
        // 4. Return map
        Ok(map.into())
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList<'_, 'static>,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let map_prototype = intrinsics.map_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<MapConstructor>(agent, realm)
            .with_property_capacity(3)
            .with_builtin_function_property::<MapGroupBy>()
            .with_prototype_property(map_prototype.into())
            .with_builtin_function_getter_property::<MapGetSpecies>()
            .build();
    }
}

/// ### [24.1.1.2 AddEntriesFromIterable ( target, iterable, adder )](https://tc39.es/ecma262/#sec-add-entries-from-iterable)
///
/// #### Unspecified specialization
///
/// This is a specialization for the `new Map()` use case.
pub(crate) fn add_entries_from_iterable_map_constructor<'a>(
    agent: &mut Agent,
    target: Map,
    iterable: Value,
    adder: Function,
    mut gc: GcScope,
) -> JsResult<'a, Map<'a>> {
    let nogc = gc.nogc();
    let mut target = target;
    let mut iterable = iterable;
    let mut adder = adder;
    if let Function::BuiltinFunction(bf) = adder
        && bf.get(agent).local().behaviour == MapPrototypeSet::BEHAVIOUR
    {
        // Normal Map.prototype.set
        if let Value::Array(arr_iterable) = iterable {
            let scoped_target = target.scope(agent, nogc);
            let scoped_iterable = arr_iterable.scope(agent, nogc);
            let scoped_adder = bf.scope(agent, nogc);
            let using_iterator = get_method(
                agent,
                arr_iterable.into(),
                WellKnownSymbolIndexes::Iterator.into(),
                gc.reborrow(),
            )?;
            target = scoped_target.get(agent).local();
            if using_iterator
                == Some(
                    agent
                        .current_realm_record()
                        .intrinsics()
                        .array_prototype_values()
                        .into(),
                )
            {
                crate::engine::bind!(let arr_iterable = scoped_iterable.get(agent).local(), gc);
                let Heap {
                    elements,
                    arrays,
                    bigints,
                    numbers,
                    strings,
                    maps,
                    ..
                } = &mut agent.heap;
                let array_heap = ArrayHeap::new(elements, arrays);

                let arr_elements = arr_iterable.get_elements(&array_heap);
                // Iterable uses the normal Array iterator of this realm.
                if arr_elements.is_empty() {
                    // Array iterator does not iterate empty arrays.
                    return Ok(scoped_target.get(agent).local());
                }
                if arr_elements.is_trivial(&array_heap)
                    && arr_elements
                        .get_storage(&array_heap)
                        .values
                        .iter()
                        .all(|entry| {
                            if let Some(Value::Array(entry)) = *entry {
                                entry.get_elements(&array_heap).len() == 2
                                    && entry.is_trivial(&array_heap)
                                    && entry.is_dense(&array_heap)
                            } else {
                                false
                            }
                        })
                {
                    // Trivial, dense array of trivial, dense arrays of two elements.
                    let gc = gc.nogc();
                    let length = arr_elements.len();
                    let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
                    let (map_data, keys, values) = target.get_map_data_mut(maps, &primitive_heap);

                    let length = length as usize;
                    keys.reserve(length);
                    values.reserve(length);
                    // Note: The Map is empty at this point, we don't need the hasher function.
                    assert!(map_data.is_empty());
                    map_data.reserve(length, |_| 0);
                    let hasher = |value: Value| {
                        let mut hasher = AHasher::default();
                        value.hash(&primitive_heap, &mut hasher);
                        hasher.finish()
                    };
                    for entry in arr_iterable.as_slice(&array_heap).iter() {
                        let Some(Value::Array(entry)) = *entry else {
                            unreachable!()
                        };
                        let slice = entry.as_slice(&array_heap);
                        let key = canonicalize_keyed_collection_key(numbers, slice[0].unwrap());
                        let key_hash = hasher(key);
                        crate::engine::bind!(let value = slice[1].unwrap(), gc);
                        let next_index = keys.len() as u32;
                        let entry = map_data.entry(
                            key_hash,
                            |hash_equal_index| keys[*hash_equal_index as usize].unwrap() == key,
                            |index_to_hash| hasher(keys[*index_to_hash as usize].unwrap()),
                        );
                        match entry {
                            hashbrown::hash_table::Entry::Occupied(occupied) => {
                                // We have duplicates in the array. Latter
                                // ones overwrite earlier ones.
                                let index = *occupied.get();
                                values[index as usize] = Some(value);
                            }
                            hashbrown::hash_table::Entry::Vacant(vacant) => {
                                vacant.insert(next_index);
                                keys.push(Some(key));
                                values.push(Some(value));
                            }
                        }
                    }
                    return Ok(scoped_target.get(agent).local());
                }
            }
            let gc = gc.nogc();
            iterable = scoped_iterable.get(agent).local();
            adder = scoped_adder.get(agent).local();
        }
    }

    add_entries_from_iterable(agent, target, iterable, adder, gc)
}

/// ### [24.1.1.2 AddEntriesFromIterable ( target, iterable, adder )](https://tc39.es/ecma262/#sec-add-entries-from-iterable)
///
/// The abstract operation AddEntriesFromIterable takes arguments target (an
/// Object), iterable (an ECMAScript language value, but not undefined or
/// null), and adder (a function object) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. adder will
/// be invoked, with target as the receiver.
///
/// > NOTE: The parameter iterable is expected to be an object that implements
/// > an @@iterator method that returns an iterator object that produces a two
/// > element array-like object whose first element is a value that will be used
/// > as a Map key and whose second element is the value to associate with that
/// > key.
pub(crate) fn add_entries_from_iterable<'a, T: Into<Object<'a>> + TryFrom<Object<'a>>>(
    agent: &mut Agent,
    target: T,
    iterable: Value,
    adder: Function,
    mut gc: GcScope,
) -> JsResult<'a, T> {
    let nogc = gc.nogc();
    let target: Object = target.into();
    let target = target.scope(agent, nogc);
    crate::engine::bind!(let iterable = iterable, gc);
    let adder = adder.scope(agent, nogc);
    // 1. Let iteratorRecord be ? GetIterator(iterable, SYNC).
    let Some(IteratorRecord {
        iterator,
        next_method,
        ..
    }) = get_iterator(agent, iterable, false, gc.reborrow())?.into_iterator_record()
    else {
        return Err(throw_not_callable(agent, gc.into_nogc()));
    };

    let iterator = iterator.scope(agent, gc.nogc());
    let next_method = next_method.scope(agent, gc.nogc());

    // 2. Repeat,
    loop {
        // a. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(
            agent,
            IteratorRecord {
                iterator: iterator.get(agent).local(),
                next_method: next_method.get(agent).local(),
            },
            gc.reborrow(),
        )?;
        // b. If next is DONE, return target.
        let Some(next) = next else {
            // SAFETY: not shared.
            let target = unsafe { target.take(agent).local() };
            // SAFETY: passed in type is still the same type.
            let target = unsafe { T::try_from(target).unwrap_unchecked() };
            return Ok(target);
        };
        // c. If next is not an Object, then
        let Ok(next) = Object::try_from(next) else {
            // i. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid iterator next return value",
                gc.nogc(),
            );
            // ii. Return ? IteratorClose(iteratorRecord, error).
            return Err(iterator_close_with_error(
                agent,
                iterator.get(agent).local(),
                error,
                gc,
            ));
        };
        let next = next;
        let scoped_next = next.scope(agent, gc.nogc());
        // d. Let k be Completion(Get(next, "0")).
        let k = get(agent, next, 0.into(), gc.reborrow());
        // e. IfAbruptCloseIterator(k, iteratorRecord).
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent).local(),
            next_method: next_method.get(agent).local(),
        };
        let k = if_abrupt_close_iterator!(agent, k, iterator_record, gc).scope(agent, gc.nogc());
        // f. Let v be Completion(Get(next, "1")).
        let v = get(
            agent,
            scoped_next.get(agent).local(),
            1.into(),
            gc.reborrow(),
        );
        // g. IfAbruptCloseIterator(v, iteratorRecord).
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent).local(),
            next_method: next_method.get(agent).local(),
        };
        let v = if_abrupt_close_iterator!(agent, v, iterator_record, gc);
        // h. Let status be Completion(Call(adder, target, « k, v »)).
        let status = call_function(
            agent,
            adder.get(agent).local(),
            target.get(agent).local().into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                k.get(agent).local(),
                v,
            ])),
            gc.reborrow(),
        );
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent).local(),
            next_method: next_method.get(agent).local(),
        };
        // i. IfAbruptCloseIterator(status, iteratorRecord).
        let _ = if_abrupt_close_iterator!(agent, status, iterator_record, gc);
    }
}
