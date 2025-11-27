// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;

use ahash::AHasher;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::call_function,
            testing_and_comparison::{is_callable, same_value},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            keyed_collections::{
                map_objects::map_prototype::canonicalize_keyed_collection_key,
                set_objects::set_iterator_objects::set_iterator::SetIterator,
            },
            set::Set,
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, Number, PropertyKey, String, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{Heap, IntrinsicFunctionIndexes, PrimitiveHeap, WellKnownSymbolIndexes},
};

pub(crate) struct SetPrototype;

struct SetPrototypeAdd;
impl Builtin for SetPrototypeAdd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.add;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::add);
}
struct SetPrototypeClear;
impl Builtin for SetPrototypeClear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.clear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::clear);
}
struct SetPrototypeDelete;
impl Builtin for SetPrototypeDelete {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.delete;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::delete);
}
struct SetPrototypeEntries;
impl Builtin for SetPrototypeEntries {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::entries);
}
struct SetPrototypeForEach;
impl Builtin for SetPrototypeForEach {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::for_each);
}
struct SetPrototypeHas;
impl Builtin for SetPrototypeHas {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::has);
}
struct SetPrototypeGetSize;
impl Builtin for SetPrototypeGetSize {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_size;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.size.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::get_size);
}
impl BuiltinGetter for SetPrototypeGetSize {}
struct SetPrototypeValues;
impl Builtin for SetPrototypeValues {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::values);
}
impl BuiltinIntrinsic for SetPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::SetPrototypeValues;
}

impl SetPrototype {
    /// ### [24.2.4.1 Set.prototype.add ( value )](https://tc39.es/ecma262/#sec-set.prototype.add)
    fn add<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        let value = arguments.get(0).bind(gc);
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value, gc)?;

        let Heap {
            bigints,
            numbers,
            strings,
            sets,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

        // 3. Set value to CanonicalizeKeyedCollectionKey(value).
        let value = canonicalize_keyed_collection_key(numbers, value);

        let set_heap_data = s.get_direct_mut(sets);
        let values = set_heap_data.values;
        let set_data = set_heap_data.set_data.get_mut();
        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            value.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };

        let value_hash = hasher(value);

        // 4. For each element e of S.[[SetData]], do
        // a. If e is not empty and SameValue(e, value) is true, then
        if let hashbrown::hash_table::Entry::Vacant(entry) = set_data.entry(
            value_hash,
            |hash_equal_index| {
                let found_value = values[*hash_equal_index as usize].unwrap();
                // Quick check: Equal values have the same value.
                found_value == value || same_value(&primitive_heap, found_value, value)
            },
            |index_to_hash| hasher(values[*index_to_hash as usize].unwrap()),
        ) {
            // 5. Append value to S.[[SetData]].
            let index = u32::try_from(values.len()).unwrap();
            entry.insert(index);
            values.push(Some(value));
        }
        // i. Return S.
        // 6. Return S.
        Ok(s.into_value())
    }

    /// ### [24.2.4.2 Set.prototype.clear ( )](https://tc39.es/ecma262/#sec-set.prototype.clear)
    ///
    /// > NOTE: The existing \[\[SetData]] List is preserved because there may
    /// > be existing Set Iterator objects that are suspended midway through
    /// > iterating over that List.
    fn clear<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value, gc)?;
        // 3. For each element e of S.[[SetData]], do
        // a. Replace the element of S.[[SetData]] whose value is e with an
        // element whose value is EMPTY.
        let data = s.get_mut(agent);
        data.set_data.borrow_mut().clear();
        data.values.clear();
        // 4. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.2.4.4 Set.prototype.delete ( value )](https://tc39.es/ecma262/#sec-set.prototype.delete)
    ///
    /// > NOTE: The value EMPTY is used as a specification device to indicate
    /// > that an entry has been deleted. Actual implementations may take other
    /// > actions such as physically removing the entry from internal data
    /// > structures.
    fn delete<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        let value = arguments.get(0).bind(gc);
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value, gc)?;

        let Heap {
            bigints,
            numbers,
            strings,
            sets,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);

        // 3. Set value to CanonicalizeKeyedCollectionKey(value).
        let value = canonicalize_keyed_collection_key(numbers, value);
        let mut hasher = AHasher::default();
        let value_hash = {
            value.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };
        let set_heap_data = s.get_direct_mut(sets);
        let values = set_heap_data.values;
        let set_data = set_heap_data.set_data.get_mut();
        // 4. For each element e of S.[[SetData]], do
        if let Ok(entry) = set_data.find_entry(value_hash, |hash_equal_index| {
            let found_value = values[*hash_equal_index as usize].unwrap();
            // Quick check: Equal keys have the same value.
            found_value == value || same_value(&primitive_heap, found_value, value)
        }) {
            // a. If e is not EMPTY and SameValue(e, value) is true, then
            let index = *entry.get() as usize;
            // i. Replace the element of S.[[SetData]] whose value is e with
            // an element whose value is EMPTY.
            values[index] = None;
            let _ = entry.remove();
            // ii. Return true.
            Ok(true.into())
        } else {
            // 5. Return false.
            Ok(false.into())
        }
    }

    /// ### [24.2.4.6 Set.prototype.entries ( )](https://tc39.es/ecma262/#sec-set.prototype.entries)
    ///
    /// > NOTE: For iteration purposes, a Set appears similar to a Map where
    /// > each entry has the same value for its key and value.
    fn entries<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Let S be the this value.
        // 2. Return ? CreateSetIterator(S, KEY+VALUE).

        // 24.2.6.1 CreateSetIterator ( set, kind )
        // 1. Perform ? RequireInternalSlot(set, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value, gc)?;
        Ok(SetIterator::from_set(agent, s, CollectionIteratorKind::KeyAndValue).into_value())
    }

    /// ### [24.2.4.7 Set.prototype.forEach ( callbackfn \[ , thisArg \] )](https://tc39.es/ecma262/#sec-set.prototype.foreach)
    ///
    /// > NOTE: `callbackfn` should be a function that accepts three arguments.
    /// > **forEach** calls `callbackfn` once for each value present in the Set
    /// > object, in value insertion order. `callbackfn` is called only for
    /// > values of the Set which actually exist; it is not called for keys
    /// > that have been deleted from the set.
    /// >
    /// > If a `thisArg` parameter is provided, it will be used as the **this**
    /// > value for each invocation of `callbackfn`. If it is not provided,
    /// > **undefined** is used instead.
    /// >
    /// > `callbackfn` is called with three arguments: the first two arguments
    /// > are a value contained in the Set. The same value is passed for both
    /// > arguments. The Set object being traversed is passed as the third
    /// > argument.
    /// >
    /// > The `callbackfn` is called with three arguments to be consistent with
    /// > the call back functions used by **forEach** methods for Map and
    /// > Array. For Sets, each item value is considered to be both the key and
    /// > the value.
    /// >
    /// > **forEach** does not directly mutate the object on which it is called
    /// > but the object may be mutated by the calls to `callbackfn`.
    /// >
    /// > Each value is normally visited only once. However, a value will be
    /// > revisited if it is deleted after it has been visited and then
    /// > re-added before the **forEach** call completes. Values that are
    /// > deleted after the call to **forEach** begins and before being visited
    /// > are not visited unless the value is added again before the
    /// > **forEach** call completes. New values added after the call to
    /// > **forEach** begins are visited.
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
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let mut s = require_set_data_internal_slot(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn, nogc) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
                gc.into_nogc(),
            ));
        };
        // 4. Let entries be S.[[SetData]].
        // 5. Let numEntries be the number of elements in entries.
        // Note: We must use the values vector length, not the size. The size
        // does not contain empty slots.
        let mut num_entries = s.get(agent).values.len() as u32;

        let callback_fn = callback_fn.scope(agent, nogc);
        let scoped_s = s.scope(agent, nogc);
        let scoped_this_arg = this_arg.scope(agent, nogc);

        // 6. Let index be 0.
        let mut index = 0;
        // 7. Repeat, while index < numEntries,
        while index < num_entries {
            // a. Let e be entries[index].
            let e = s.get(agent).values[index as usize];
            // b. Set index to index + 1.
            index += 1;
            // c. If e is not EMPTY, then
            if let Some(e) = e {
                // i. Perform ? Call(callbackfn, thisArg, « e, e, S »).
                call_function(
                    agent,
                    callback_fn.get(agent),
                    scoped_this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        e.unbind(),
                        e.unbind(),
                        s.into_value().unbind(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?;
                // ii. NOTE: The number of elements in entries may have increased during execution of callbackfn.
                // iii. Set numEntries to the number of elements in entries.
                s = scoped_s.get(agent).bind(gc.nogc());
                num_entries = s.get(agent).values.len() as u32;
            }
        }
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.2.4.8 Set.prototype.has ( value )](https://tc39.es/ecma262/#sec-set.prototype.has)
    fn has<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        let value = arguments.get(0).bind(gc);
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value, gc)?;

        let Heap {
            bigints,
            numbers,
            strings,
            sets,
            ..
        } = &agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        let set_heap_data = s.get_direct(sets);
        let values = set_heap_data.values;
        let set_data = set_heap_data.set_data.borrow();

        // 3. Set value to CanonicalizeKeyedCollectionKey(value).
        let value = canonicalize_keyed_collection_key(&primitive_heap, value);
        let mut hasher = AHasher::default();
        let value_hash = {
            value.hash(&primitive_heap, &mut hasher);
            hasher.finish()
        };
        // 4. For each element e of S.[[SetData]], do
        // a. If e is not EMPTY and SameValue(e, value) is true, return true.
        let found = set_data
            .find(value_hash, |hash_equal_index| {
                let found_value = values[*hash_equal_index as usize].unwrap();
                // Quick check: Equal values have the same value.
                found_value == value || same_value(&primitive_heap, found_value, value)
            })
            .is_some();
        // 5. Return false.
        Ok(found.into())
    }

    /// ### [24.2.4.14 get Set.prototype.size](https://tc39.es/ecma262/#sec-get-set.prototype.size)
    ///
    /// Set.prototype.size is an accessor property whose set accessor function
    /// is undefined.
    fn get_size<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value, gc)?;
        // 3. Let size be SetDataSize(S.[[SetData]]).
        let size = s.get(agent).set_data.borrow().len() as u32;
        // 4. Return 𝔽(size).
        Ok(Number::from(size).into_value())
    }

    /// ### [24.2.4.17 Set.prototype.values ( )](https://tc39.es/ecma262/#sec-set.prototype.values)
    fn values<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Let S be the this value.
        // 2. Return ? CreateSetIterator(S, VALUE).

        // 24.2.6.1 CreateSetIterator ( set, kind )
        // 1. Perform ? RequireInternalSlot(set, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value, gc)?;
        Ok(SetIterator::from_set(agent, s, CollectionIteratorKind::Value).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.set_prototype();
        let set_constructor = intrinsics.set();
        let set_prototype_values = intrinsics.set_prototype_values();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(12)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<SetPrototypeAdd>()
            .with_builtin_function_property::<SetPrototypeClear>()
            .with_constructor_property(set_constructor)
            .with_builtin_function_property::<SetPrototypeDelete>()
            .with_builtin_function_property::<SetPrototypeEntries>()
            .with_builtin_function_property::<SetPrototypeForEach>()
            .with_builtin_function_property::<SetPrototypeHas>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.keys.to_property_key())
                    .with_value(set_prototype_values.into_value())
                    .with_enumerable(SetPrototypeValues::ENUMERABLE)
                    .with_configurable(SetPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_builtin_function_getter_property::<SetPrototypeGetSize>()
            .with_builtin_intrinsic_function_property::<SetPrototypeValues>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(set_prototype_values.into_value())
                    .with_enumerable(SetPrototypeValues::ENUMERABLE)
                    .with_configurable(SetPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Set.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[inline(always)]
fn require_set_data_internal_slot<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Set<'a>> {
    match value {
        Value::Set(map) => Ok(map.bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Object is not a Set",
            gc,
        )),
    }
}
