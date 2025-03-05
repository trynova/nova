// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;

use ahash::AHasher;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{get, get_method, try_get},
            testing_and_comparison::is_callable,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
            array::ArrayHeap,
            keyed_collections::map_objects::{
                map_constructor::add_entries_from_iterable,
                map_prototype::canonicalize_keyed_collection_key,
            },
            ordinary::ordinary_create_from_constructor,
            weak_map::{WeakMap, data::WeakMapData},
        },
        execution::{Agent, JsResult, ProtoIntrinsics, RealmIdentifier, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoFunction, IntoObject, IntoValue, Object, String,
            Value,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::{Heap, IntrinsicConstructorIndexes, PrimitiveHeap, WellKnownSymbolIndexes},
};

use super::weak_map_prototype::WeakMapPrototypeSet;

pub(crate) struct WeakMapConstructor;
impl Builtin for WeakMapConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakMap;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}

impl BuiltinIntrinsicConstructor for WeakMapConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakMap;
}

impl WeakMapConstructor {
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
                "Constructor WeakMap requires 'new'",
                gc.nogc(),
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%WeakMap.prototype%", « [[WeakMapData]] »).
        // 4. If iterable is either undefined or null, return map.
        if no_iterable {
            return Ok(ordinary_create_from_constructor(
                agent,
                new_target.unbind(),
                ProtoIntrinsics::WeakMap,
                gc,
            )?
            .into_value());
        }
        let iterable = iterable.scope(agent, nogc);
        let mut map = WeakMap::try_from(ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::WeakMap,
            gc.reborrow(),
        )?)
        .unwrap()
        .unbind()
        .bind(gc.nogc());
        // Note
        // If the parameter iterable is present, it is expected to be an
        // object that implements an @@iterator method that returns an
        // iterator object that produces a two element array-like object
        // whose first element is a value that will be used as a WeakMap key
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
            adder
        };
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        let Some(adder) = is_callable(adder, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "WeakMap.prototype.set is not callable",
                gc.nogc(),
            ));
        };
        // 7. Return ? AddEntriesFromIterable(map, iterable, adder).
        add_entries_from_iterable_weak_map_constructor(
            agent,
            map.unbind(),
            iterable.get(agent),
            adder.unbind(),
            gc,
        )
        .map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let weak_map_prototype = intrinsics.weak_map_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakMapConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_map_prototype.into_object())
            .build();
    }
}

/// ### [24.1.1.2 AddEntriesFromIterable ( target, iterable, adder )](https://tc39.es/ecma262/#sec-add-entries-from-iterable)
///
/// #### Unspecified specialization
///
/// This is a specialization for the `new WeakMap()` use case.
pub fn add_entries_from_iterable_weak_map_constructor<'a>(
    agent: &mut Agent,
    target: WeakMap,
    iterable: Value,
    adder: Function,
    mut gc: GcScope<'a, '_>,
) -> JsResult<WeakMap<'a>> {
    let mut target = target.bind(gc.nogc());
    let mut iterable = iterable.bind(gc.nogc());
    let mut adder = adder.bind(gc.nogc());
    if let Function::BuiltinFunction(bf) = adder {
        if agent[bf].behaviour == WeakMapPrototypeSet::BEHAVIOUR {
            // Normal WeakMap.prototype.set
            if let Value::Array(arr_iterable) = iterable {
                let scoped_target = target.scope(agent, gc.nogc());
                let scoped_iterable = arr_iterable.scope(agent, gc.nogc());
                let scoped_adder = bf.scope(agent, gc.nogc());
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
                        weak_maps,
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
                        let WeakMapData {
                            keys,
                            values,
                            weak_map_data,
                            ..
                        } = weak_maps[target].borrow_mut(&primitive_heap);
                        let map_data = weak_map_data.get_mut();

                        let length = length as usize;
                        keys.reserve(length);
                        values.reserve(length);
                        // Note: The WeakMap is empty at this point, we don't need the hasher function.
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

    Ok(WeakMap::try_from(add_entries_from_iterable(
        agent,
        target.into_object().unbind(),
        iterable.unbind(),
        adder.unbind(),
        gc,
    )?)
    .unwrap())
}
