// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;

use ahash::AHasher;

use crate::engine::context::{Bindable, GcScope};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                get_iterator, if_abrupt_close_iterator, iterator_step_value,
            },
            operations_on_objects::{call_function, get, get_method},
            testing_and_comparison::is_callable,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            array::ArrayHeap,
            ordinary::ordinary_create_from_constructor,
            set::{data::SetData, Set},
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
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let iterable = arguments.get(0).bind(nogc);
        let new_target = new_target.bind(nogc);
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot call Set as a function",
                gc.nogc(),
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
            .map(|o| o.into_value());
        }
        let scoped_iterable = iterable.scope(agent, nogc);
        let set = Set::try_from(ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::Set,
            gc.reborrow(),
        )?)
        .unwrap()
        .unbind()
        .bind(gc.nogc());
        let scoped_set = set.scope(agent, gc.nogc());
        // 3. Set set.[[SetData]] to a new empty List.

        // 5. Let adder be ? Get(set, "add").
        let adder = get(
            agent,
            set.into_object().unbind(),
            BUILTIN_STRING_MEMORY.add.into(),
            gc.reborrow(),
        )?;
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        let Some(adder) = is_callable(adder.unbind(), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid adder function",
                gc.nogc(),
            ));
        };
        let adder = adder.scope(agent, gc.nogc());
        if let Value::Array(iterable) = scoped_iterable.get(agent) {
            let iterable = iterable.bind(gc.nogc());
            if iterable.is_trivial(agent)
                && iterable.is_dense(agent)
                && get_method(
                    agent,
                    iterable.unbind().into_value(),
                    PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
                    gc.reborrow(),
                )? == Some(
                    agent
                        .current_realm()
                        .intrinsics()
                        .array_prototype_values()
                        .into_function(),
                )
            {
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

                let SetData {
                    values, set_data, ..
                } = &mut sets[set].borrow_mut(&primitive_heap);
                let set_data = set_data.get_mut();

                let hasher = |value: Value| {
                    let mut hasher = AHasher::default();
                    value.hash(&primitive_heap, &mut hasher);
                    hasher.finish()
                };

                let iterable_length = iterable.len(&array_heap) as usize;
                values.reserve(iterable_length);
                // Note: There should be no items in the set data. Hence the
                // hasher function should never be called.
                assert!(set_data.is_empty());
                set_data.reserve(iterable_length, |_| unreachable!());
                iterable.as_slice(&array_heap).iter().for_each(|value| {
                    let value = value.unwrap();
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
                            values.push(Some(value.unbind()));
                        }
                    }
                });
                return Ok(set.into_value().unbind());
            }
        }
        // 7. Let iteratorRecord be ? GetIterator(iterable, SYNC).
        let mut iterator_record =
            get_iterator(agent, scoped_iterable.get(agent), false, gc.reborrow())?;
        // 8. Repeat,
        loop {
            // a. Let next be ? IteratorStepValue(iteratorRecord).
            let next = iterator_step_value(agent, &mut iterator_record, gc.reborrow())?;
            // b. If next is DONE, return set.
            let Some(next) = next else {
                return Ok(scoped_set.get(agent).into_value());
            };
            // c. Let status be Completion(Call(adder, set, « next »)).
            let status = call_function(
                agent,
                adder.get(agent),
                scoped_set.get(agent).into_value(),
                Some(ArgumentsList(&[next.unbind()])),
                gc.reborrow(),
            );
            // d. IfAbruptCloseIterator(status, iteratorRecord).
            let _ = if_abrupt_close_iterator(
                agent,
                status.map(|v| v.unbind()),
                &iterator_record,
                gc.reborrow(),
            )?;
        }
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
        let set_prototype = intrinsics.set_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<SetConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype_property(set_prototype.into_object())
            .with_builtin_function_getter_property::<SetGetSpecies>()
            .build();
    }
}
