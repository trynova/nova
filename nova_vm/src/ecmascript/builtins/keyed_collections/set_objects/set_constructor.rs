// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;

use ahash::AHasher;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                IteratorRecord, get_iterator, if_abrupt_close_iterator, iterator_step_value,
            },
            operations_on_objects::{call_function, get, throw_not_callable},
            testing_and_comparison::is_callable,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
            array::ArrayHeap,
            keyed_collections::map_objects::map_prototype::canonicalize_keyed_collection_key,
            ordinary::ordinary_create_from_constructor, set::Set,
        },
        execution::{Agent, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, Function, Object, PropertyKey, String, Value},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::{Heap, IntrinsicConstructorIndexes, PrimitiveHeap, WellKnownSymbolIndexes},
};

pub(crate) struct SetConstructor;
impl Builtin for SetConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Set;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for SetConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Set;
}
struct SetGetSpecies;
impl Builtin for SetGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetConstructor::get_species);
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for SetGetSpecies {}

impl SetConstructor {
    /// ### [24.2.2.1 Set ( \[ iterable \] )](https://tc39.es/ecma262/#sec-set-iterable)
    fn constructor<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let iterable = arguments.get(0).bind(nogc);
        let new_target = new_target.bind(nogc);
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot call Set as a function",
                gc.into_nogc(),
            ));
        };
        // 2. Let set be ? OrdinaryCreateFromConstructor(NewTarget, "%Set.prototype%", « [[SetData]] »).
        let new_target = Function::try_from(new_target).unwrap();
        // 4. If iterable is either undefined or null, return set.
        if iterable.is_undefined() || iterable.is_null() {
            return ordinary_create_from_constructor(
                agent,
                new_target.unbind(),
                ProtoIntrinsics::Set,
                gc,
            )
            .map(|o| o.into());
        }
        let scoped_iterable = iterable.scope(agent, nogc);
        let set = Set::try_from(
            ordinary_create_from_constructor(
                agent,
                new_target.unbind(),
                ProtoIntrinsics::Set,
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc()),
        )
        .unwrap()
        .unbind()
        .bind(gc.nogc());
        let scoped_set = set.scope(agent, gc.nogc());
        // 3. Set set.[[SetData]] to a new empty List.

        // 5. Let adder be ? Get(set, "add").
        let adder = get(
            agent,
            set.into().unbind(),
            BUILTIN_STRING_MEMORY.add.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        let Some(adder) = is_callable(adder.unbind(), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid adder function",
                gc.into_nogc(),
            ));
        };
        let adder = adder.scope(agent, gc.nogc());
        if let Value::Array(iterable) = scoped_iterable.get(agent) {
            let iterable = iterable.bind(gc.nogc());
            if iterable.is_trivial(agent) && iterable.is_trivially_iterable(agent, gc.nogc()) {
                // Accessorless, holeless array with standard Array values
                // iterator. We can fast-path this.
                let set = scoped_set.get(agent).bind(gc.nogc());
                let Value::Array(iterable) = scoped_iterable.get(agent).bind(gc.nogc()) else {
                    unreachable!()
                };
                let Heap {
                    elements,
                    arrays,
                    bigints,
                    numbers,
                    strings,
                    sets,
                    ..
                } = &mut agent.heap;
                let array_heap = ArrayHeap::new(elements, arrays);
                let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

                let mut set_heap_data = set.get_direct_mut(sets);
                let values = &mut set_heap_data.values;
                let set_data = set_heap_data.set_data.get_mut();

                let hasher = |value: Value| {
                    let mut hasher = AHasher::default();
                    value.hash(&primitive_heap, &mut hasher);
                    hasher.finish()
                };

                let iterable_elements = iterable.get_elements(&array_heap);
                let iterable_length = iterable_elements.len() as usize;
                values.reserve(iterable_length);
                // Note: There should be no items in the set data. Hence the
                // hasher function should never be called.
                assert!(set_data.is_empty());
                set_data.reserve(iterable_length, |_| unreachable!());
                iterable_elements
                    .get_storage(&array_heap)
                    .values
                    .iter()
                    .for_each(|value| {
                        let value = canonicalize_keyed_collection_key(numbers, value.unwrap());
                        let value_hash = hasher(value);
                        let next_index = values.len() as u32;
                        let entry = set_data.entry(
                            value_hash,
                            |hash_equal_index| values[*hash_equal_index as usize].unwrap() == value,
                            |index_to_hash| hasher(values[*index_to_hash as usize].unwrap()),
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
                                values.push(Some(value));
                            }
                        }
                    });
                return Ok(set.into().unbind());
            }
        }
        // 7. Let iteratorRecord be ? GetIterator(iterable, SYNC).
        let Some(IteratorRecord {
            iterator,
            next_method,
        }) = get_iterator(agent, scoped_iterable.get(agent), false, gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_iterator_record()
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };

        let iterator = iterator.scope(agent, gc.nogc());
        let next_method = next_method.scope(agent, gc.nogc());

        // 8. Repeat,
        loop {
            // a. Let next be ? IteratorStepValue(iteratorRecord).
            let next = iterator_step_value(
                agent,
                IteratorRecord {
                    iterator: iterator.get(agent),
                    next_method: next_method.get(agent),
                },
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. If next is DONE, return set.
            let Some(next) = next else {
                return Ok(scoped_set.get(agent).into());
            };
            // c. Let status be Completion(Call(adder, set, « next »)).
            let status = call_function(
                agent,
                adder.get(agent),
                scoped_set.get(agent).into(),
                Some(ArgumentsList::from_mut_slice(&mut [next.unbind()])),
                gc.reborrow(),
            );
            // d. IfAbruptCloseIterator(status, iteratorRecord).
            let iterator_record = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let _ = if_abrupt_close_iterator!(agent, status, iterator_record, gc);
        }
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(this_value.unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let set_prototype = intrinsics.set_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<SetConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype_property(set_prototype.into())
            .with_builtin_function_getter_property::<SetGetSpecies>()
            .build();
    }
}
