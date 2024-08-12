// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                get_iterator, if_abrupt_close_iterator, iterator_close, iterator_step_value,
            },
            operations_on_objects::{call_function, get, get_method},
            testing_and_comparison::is_callable,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            keyed_collections::map_objects::map_prototype::{
                canonicalize_keyed_collection_key, MapPrototypeSet,
            },
            map::{data::MapHeapData, Map},
            ordinary::ordinary_create_from_constructor,
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            Function, IntoFunction, IntoObject, IntoValue, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct MapConstructor;
impl Builtin for MapConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(MapConstructor::behaviour);
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
    const KEY: Option<PropertyKey> = Some(WellKnownSymbolIndexes::Species.to_property_key());
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for MapGetSpecies {}

impl MapConstructor {
    fn behaviour<'gen>(
        agent: &mut Agent<'gen>,
        _: Value<'gen>,
        arguments: ArgumentsList<'_, 'gen>,
        new_target: Option<Object<'gen>>,
    ) -> JsResult<'gen, Value<'gen>> {
        // If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Constructor Map requires 'new'",
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%Map.prototype%", « [[MapData]] »).
        let map = Map::try_from(ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::Map,
        )?)
        .unwrap();
        // 3. Set map.[[MapData]] to a new empty List.
        let iterable = arguments.get(0);
        // 4. If iterable is either undefined or null, return map.
        if iterable.is_undefined() || iterable.is_null() {
            Ok(map.into_value())
        } else {
            // Note
            // If the parameter iterable is present, it is expected to be an
            // object that implements an @@iterator method that returns an
            // iterator object that produces a two element array-like object
            // whose first element is a value that will be used as a Map key
            // and whose second element is the value to associate with that
            // key.

            // 5. Let adder be ? Get(map, "set").
            let adder = get(
                agent,
                map.into_object(),
                BUILTIN_STRING_MEMORY.set.to_property_key(),
            )?;
            // 6. If IsCallable(adder) is false, throw a TypeError exception.
            let Some(adder) = is_callable(adder) else {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Map.prototype.set is not callable",
                ));
            };
            // 7. Return ? AddEntriesFromIterable(map, iterable, adder).
            add_entries_from_iterable_map_constructor(agent, map, iterable, adder)
                .map(|result| result.into_value())
        }
    }

    fn group_by<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_species(_: &mut Agent<'gen>, this_value: Value<'gen>, _: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
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
pub fn add_entries_from_iterable_map_constructor(
    agent: &mut Agent<'gen>,
    target: Map,
    iterable: Value<'gen>,
    adder: Function<'gen>,
) -> JsResult<'gen, Map> {
    if let Function::BuiltinFunction(adder) = adder {
        if agent[adder].behaviour == MapPrototypeSet::BEHAVIOUR {
            // Normal Map.prototype.set
            if let Value::Array(iterable) = iterable {
                let using_iterator = get_method(
                    agent,
                    iterable.into_value(),
                    WellKnownSymbolIndexes::Iterator.into(),
                )?;
                if using_iterator
                    == Some(
                        agent
                            .current_realm()
                            .intrinsics()
                            .array_prototype_values()
                            .into_function(),
                    )
                {
                    // Iterable uses the normal Array iterator of this realm.
                    if iterable.len(agent) == 0 {
                        // Array iterator does not iterate empty arrays.
                        return Ok(target);
                    }
                    if iterable.is_trivial(agent)
                        && iterable.as_slice(agent).iter().all(|entry| {
                            if let Some(Value::Array(entry)) = *entry {
                                entry.len(agent) == 2
                                    && entry.is_trivial(agent)
                                    && entry.is_dense(agent)
                            } else {
                                false
                            }
                        })
                    {
                        // Trivial, dense array of trivial, dense arrays of two elements.
                        let length = iterable.len(agent);
                        // SAFETY: None of the other Agent borrows can
                        // invalidate the MapHeapData borrow here.
                        // It's thus safe to keep this borrow alive
                        // while we iterate the entries.
                        let data = unsafe {
                            std::mem::transmute::<&mut MapHeapData, &'static mut MapHeapData>(
                                &mut agent[target],
                            )
                        };
                        data.keys.reserve(length as usize);
                        data.values.reserve(length as usize);
                        for entry in iterable.as_slice(agent).iter() {
                            let Some(Value::Array(entry)) = *entry else {
                                unreachable!()
                            };
                            let slice = entry.as_slice(agent);
                            let key = slice[0].unwrap();
                            let value = slice[1].unwrap();
                            data.keys
                                .push(Some(canonicalize_keyed_collection_key(agent, key)));
                            data.values.push(Some(value));
                        }
                        return Ok(target);
                    }
                }
            }
        }
    }

    add_entries_from_iterable_map_constructor(agent, target, iterable, adder)
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
pub(crate) fn add_entries_from_iterable(
    agent: &mut Agent<'gen>,
    target: Object<'gen>,
    iterable: Value<'gen>,
    adder: Function<'gen>,
) -> JsResult<'gen, Object<'gen>> {
    // 1. Let iteratorRecord be ? GetIterator(iterable, SYNC).
    let mut iterator_record = get_iterator(agent, iterable, false)?;
    // 2. Repeat,
    loop {
        // a. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(agent, &mut iterator_record)?;
        // b. If next is DONE, return target.
        let Some(next) = next else {
            return Ok(target);
        };
        // c. If next is not an Object, then
        let Ok(next) = Object::try_from(next) else {
            // i. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid iterator next return value",
            );
            // ii. Return ? IteratorClose(iteratorRecord, error).
            return iterator_close(agent, &iterator_record, Err(error));
        };
        // d. Let k be Completion(Get(next, "0")).
        let k = get(agent, next, 0.into());
        // e. IfAbruptCloseIterator(k, iteratorRecord).
        let k = if_abrupt_close_iterator(agent, k, &iterator_record)?;
        // f. Let v be Completion(Get(next, "1")).
        let v = get(agent, next, 1.into());
        // g. IfAbruptCloseIterator(v, iteratorRecord).
        let v = if_abrupt_close_iterator(agent, v, &iterator_record)?;
        // h. Let status be Completion(Call(adder, target, « k, v »)).
        let status = call_function(
            agent,
            adder,
            target.into_value(),
            Some(ArgumentsList(&[k, v])),
        );
        let _ = if_abrupt_close_iterator(agent, status, &iterator_record)?;
    }
}
