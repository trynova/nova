// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;

use ahash::AHasher;

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::if_abrupt_close_iterator;
use crate::ecmascript::abstract_operations::operations_on_objects::try_get;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::TryResult;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{get_iterator, iterator_close, iterator_step_value},
            operations_on_objects::{call_function, get, get_method},
            testing_and_comparison::is_callable,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            array::ArrayHeap,
            keyed_collections::map_objects::map_prototype::{
                canonicalize_keyed_collection_key, MapPrototypeSet,
            },
            map::{data::MapData, Map},
            ordinary::ordinary_create_from_constructor,
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            Function, IntoFunction, IntoObject, IntoValue, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{Heap, IntrinsicConstructorIndexes, PrimitiveHeap, WellKnownSymbolIndexes},
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
    const LENGTH: u8 = 0;
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
        _: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let iterable = arguments.get(0).bind(nogc);
        let no_iterable = iterable.is_undefined() || iterable.is_null();
        let new_target = new_target.bind(nogc);

        // If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Constructor Map requires 'new'",
                gc.nogc(),
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%Map.prototype%", « [[MapData]] »).
        // 4. If iterable is either undefined or null, return map.
        if no_iterable {
            return Ok(ordinary_create_from_constructor(
                agent,
                new_target.unbind(),
                ProtoIntrinsics::Map,
                gc,
            )?
            .into_value());
        }
        let iterable = iterable.scope(agent, nogc);
        let mut map = Map::try_from(ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::Map,
            gc.reborrow(),
        )?)
        .unwrap()
        .unbind()
        .bind(gc.nogc());
        // 3. Set map.[[MapData]] to a new empty List.
        // Note
        // If the parameter iterable is present, it is expected to be an
        // object that implements an @@iterator method that returns an
        // iterator object that produces a two element array-like object
        // whose first element is a value that will be used as a Map key
        // and whose second element is the value to associate with that
        // key.

        // 5. Let adder be ? Get(map, "set").
        let adder = if let TryResult::Continue(adder) = try_get(
            agent,
            map.into_object().unbind(),
            BUILTIN_STRING_MEMORY.set.to_property_key(),
            gc.nogc(),
        ) {
            adder
        } else {
            let scoped_map = map.scope(agent, gc.nogc());
            let adder = get(
                agent,
                map.into_object().unbind(),
                BUILTIN_STRING_MEMORY.set.to_property_key(),
                gc.reborrow(),
            )?
            .unbind();
            let gc = gc.nogc();
            map = scoped_map.get(agent).bind(gc);
            adder.bind(gc)
        };
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        let Some(adder) = is_callable(adder, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Map.prototype.set is not callable",
                gc.nogc(),
            ));
        };
        // 7. Return ? AddEntriesFromIterable(map, iterable, adder).
        add_entries_from_iterable_map_constructor(
            agent,
            map.unbind(),
            iterable.get(agent),
            adder.unbind(),
            gc,
        )
        .map(|result| result.into_value())
    }

    fn group_by<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        Ok(this_value.unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let map_prototype = intrinsics.map_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<MapConstructor>(agent, realm)
            .with_property_capacity(3)
            .with_builtin_function_property::<MapGroupBy>()
            .with_prototype_property(map_prototype.into_object())
            .with_builtin_function_getter_property::<MapGetSpecies>()
            .build();
    }
}

/// ### [24.1.1.2 AddEntriesFromIterable ( target, iterable, adder )](https://tc39.es/ecma262/#sec-add-entries-from-iterable)
///
/// #### Unspecified specialization
///
/// This is a specialization for the `new Map()` use case.
pub fn add_entries_from_iterable_map_constructor<'a>(
    agent: &mut Agent,
    target: Map,
    iterable: Value,
    adder: Function,
    mut gc: GcScope<'a, '_>,
) -> JsResult<Map<'a>> {
    let nogc = gc.nogc();
    let mut target = target.bind(nogc);
    let mut iterable = iterable.bind(nogc);
    let mut adder = adder.bind(nogc);
    if let Function::BuiltinFunction(bf) = adder {
        if agent[bf].behaviour == MapPrototypeSet::BEHAVIOUR {
            // Normal Map.prototype.set
            if let Value::Array(arr_iterable) = iterable {
                let scoped_target = target.scope(agent, nogc);
                let scoped_iterable = arr_iterable.scope(agent, nogc);
                let scoped_adder = bf.scope(agent, nogc);
                let using_iterator = get_method(
                    agent,
                    arr_iterable.into_value().unbind(),
                    WellKnownSymbolIndexes::Iterator.into(),
                    gc.reborrow(),
                )?
                .map(|f| f.unbind())
                .map(|f| f.bind(gc.nogc()));
                target = scoped_target.get(agent).bind(gc.nogc());
                if using_iterator
                    == Some(
                        agent
                            .current_realm()
                            .intrinsics()
                            .array_prototype_values()
                            .into_function(),
                    )
                {
                    let arr_iterable = scoped_iterable.get(agent).bind(gc.nogc());
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
                    let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

                    // Iterable uses the normal Array iterator of this realm.
                    if arr_iterable.len(&array_heap) == 0 {
                        // Array iterator does not iterate empty arrays.
                        return Ok(scoped_target.get(agent).bind(gc.into_nogc()));
                    }
                    if arr_iterable.is_trivial(&array_heap)
                        && arr_iterable.as_slice(&array_heap).iter().all(|entry| {
                            if let Some(Value::Array(entry)) = *entry {
                                entry.len(&array_heap) == 2
                                    && entry.is_trivial(&array_heap)
                                    && entry.is_dense(&array_heap)
                            } else {
                                false
                            }
                        })
                    {
                        // Trivial, dense array of trivial, dense arrays of two elements.
                        let target = target.unbind();
                        let arr_iterable = arr_iterable.unbind();
                        let gc = gc.into_nogc();
                        let target = target.bind(gc);
                        let arr_iterable = arr_iterable.bind(gc);
                        let length = arr_iterable.len(&array_heap);
                        let MapData {
                            keys,
                            values,
                            map_data,
                            ..
                        } = maps[target].borrow_mut(&primitive_heap);
                        let map_data = map_data.get_mut();

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
                            let key = canonicalize_keyed_collection_key(
                                numbers,
                                slice[0].unwrap().bind(gc),
                            );
                            let key_hash = hasher(key);
                            let value = slice[1].unwrap().bind(gc);
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
                                    values[index as usize] = Some(value.unbind());
                                }
                                hashbrown::hash_table::Entry::Vacant(vacant) => {
                                    vacant.insert(next_index);
                                    keys.push(Some(key.unbind()));
                                    values.push(Some(value.unbind()));
                                }
                            }
                        }
                        return Ok(scoped_target.get(agent).bind(gc));
                    }
                }
                let gc = gc.nogc();
                iterable = scoped_iterable.get(agent).bind(gc).into_value();
                adder = scoped_adder.get(agent).bind(gc).into_function();
            }
        }
    }

    add_entries_from_iterable(
        agent,
        target.unbind(),
        iterable.unbind(),
        adder.unbind(),
        gc,
    )
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
pub(crate) fn add_entries_from_iterable<'a>(
    agent: &mut Agent,
    target: Map,
    iterable: Value,
    adder: Function,
    mut gc: GcScope<'a, '_>,
) -> JsResult<Map<'a>> {
    let nogc = gc.nogc();
    let target = target.bind(nogc).scope(agent, nogc);
    let iterable = iterable.bind(nogc);
    let adder = adder.bind(nogc).scope(agent, nogc);
    // 1. Let iteratorRecord be ? GetIterator(iterable, SYNC).
    let mut iterator_record = get_iterator(agent, iterable.unbind(), false, gc.reborrow())?;
    // 2. Repeat,
    loop {
        // a. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(agent, &mut iterator_record, gc.reborrow())?;
        // b. If next is DONE, return target.
        let Some(next) = next else {
            return Ok(target.get(agent).bind(gc.into_nogc()));
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
            return iterator_close(agent, &iterator_record, Err(error), gc.reborrow());
        };
        let next = next.unbind().bind(gc.nogc());
        let scoped_next = next.scope(agent, gc.nogc());
        // d. Let k be Completion(Get(next, "0")).
        let k = get(agent, next.unbind(), 0.into(), gc.reborrow());
        // e. IfAbruptCloseIterator(k, iteratorRecord).
        let k = if_abrupt_close_iterator!(agent, k, iterator_record, gc).scope(agent, gc.nogc());
        // f. Let v be Completion(Get(next, "1")).
        let v = get(agent, scoped_next.get(agent), 1.into(), gc.reborrow());
        // g. IfAbruptCloseIterator(v, iteratorRecord).
        let v = if_abrupt_close_iterator!(agent, v, iterator_record, gc);
        // h. Let status be Completion(Call(adder, target, « k, v »)).
        let status = call_function(
            agent,
            adder.get(agent),
            target.get(agent).into_value(),
            Some(ArgumentsList(&[k.get(agent), v.unbind()])),
            gc.reborrow(),
        );
        let _ = if_abrupt_close_iterator!(agent, status, iterator_record, gc);
    }
}
