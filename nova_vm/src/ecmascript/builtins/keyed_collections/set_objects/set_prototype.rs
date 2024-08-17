// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::hash::Hasher;

use ahash::AHasher;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::call_function,
            testing_and_comparison::{is_callable, same_value},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            keyed_collections::map_objects::map_prototype::canonicalize_keyed_collection_key,
            set::{data::SetHeapData, Set},
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct SetPrototype;

struct SetPrototypeAdd;
impl Builtin for SetPrototypeAdd {
    const NAME: String = BUILTIN_STRING_MEMORY.add;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::add);
}
struct SetPrototypeClear;
impl Builtin for SetPrototypeClear {
    const NAME: String = BUILTIN_STRING_MEMORY.clear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::clear);
}
struct SetPrototypeDelete;
impl Builtin for SetPrototypeDelete {
    const NAME: String = BUILTIN_STRING_MEMORY.delete;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::delete);
}
struct SetPrototypeEntries;
impl Builtin for SetPrototypeEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::entries);
}
struct SetPrototypeForEach;
impl Builtin for SetPrototypeForEach {
    const NAME: String = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::for_each);
}
struct SetPrototypeHas;
impl Builtin for SetPrototypeHas {
    const NAME: String = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::has);
}
struct SetPrototypeKeys;
impl Builtin for SetPrototypeKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::keys);
}
struct SetPrototypeGetSize;
impl Builtin for SetPrototypeGetSize {
    const NAME: String = BUILTIN_STRING_MEMORY.get_size;
    const KEY: Option<PropertyKey> = Some(BUILTIN_STRING_MEMORY.size.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::get_size);
}
impl BuiltinGetter for SetPrototypeGetSize {}
struct SetPrototypeValues;
impl Builtin for SetPrototypeValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetPrototype::values);
}
impl BuiltinIntrinsic for SetPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::SetPrototypeValues;
}

impl SetPrototype {
    /// #### [24.2.4.1 Set.prototype.add ( value )](https://tc39.es/ecma262/#sec-set.prototype.add)
    fn add(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value)?;
        // 3. Set value to CanonicalizeKeyedCollectionKey(value).
        let value = canonicalize_keyed_collection_key(agent, arguments.get(0));

        // SAFETY: Borrow for hashing a Value and comparing values only
        // requires access to string, number, and bigint data. It will never
        // access Map data.
        let SetHeapData {
            values, set_data, ..
        } = unsafe {
            std::mem::transmute::<&mut SetHeapData, &'static mut SetHeapData>(&mut agent[s])
        };
        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            value.hash(agent, &mut hasher);
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
                found_value == value || same_value(agent, found_value, value)
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
    /// > #### Note
    /// > The existing \[\[SetData]] List is preserved because there may be
    /// > existing Set Iterator objects that are suspended midway through
    /// > iterating over that List.
    fn clear(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value)?;
        let SetHeapData {
            values, set_data, ..
        } = &mut agent[s];
        // 3. For each element e of S.[[SetData]], do
        // a. Replace the element of S.[[SetData]] whose value is e with an
        // element whose value is EMPTY.
        values.fill(None);
        set_data.clear();
        // 4. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.2.4.4 Set.prototype.delete ( value )](https://tc39.es/ecma262/#sec-set.prototype.delete)
    ///
    /// > #### Note
    /// >
    /// > The value EMPTY is used as a specification device to indicate that an
    /// > entry has been deleted. Actual implementations may take other actions
    /// > such as physically removing the entry from internal data structures.
    fn delete(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value)?;
        // 3. Set value to CanonicalizeKeyedCollectionKey(value).
        let value = canonicalize_keyed_collection_key(agent, arguments.get(0));
        let mut hasher = AHasher::default();
        let value_hash = {
            value.hash(agent, &mut hasher);
            hasher.finish()
        };
        // SAFETY: Borrow for hashing a Value and comparing values only
        // requires access to string, number, and bigint data. It will never
        // access Map data.
        let SetHeapData {
            values, set_data, ..
        } = unsafe {
            std::mem::transmute::<&mut SetHeapData, &'static mut SetHeapData>(&mut agent[s])
        };
        // 4. For each element e of S.[[SetData]], do
        if let Ok(entry) = set_data.find_entry(value_hash, |hash_equal_index| {
            let found_value = values[*hash_equal_index as usize].unwrap();
            // Quick check: Equal keys have the same value.
            found_value == value || same_value(agent, found_value, value)
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

    fn entries(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    /// ### [24.2.4.7 Set.prototype.forEach ( callbackfn \[ , thisArg \] )](https://tc39.es/ecma262/#sec-set.prototype.foreach)
    ///
    /// > #### Note
    /// > `callbackfn` should be a function that accepts three arguments.
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
    fn for_each(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let callback_fn = arguments.get(0);
        let this_arg = arguments.get(1);
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
            ));
        };
        // 4. Let entries be S.[[SetData]].
        // 5. Let numEntries be the number of elements in entries.
        let mut num_entries = agent[s].values.len();
        // 6. Let index be 0.
        let mut index = 0;
        // 7. Repeat, while index < numEntries,
        while index < num_entries {
            // a. Let e be entries[index].
            let e = agent[s].values[index];
            // b. Set index to index + 1.
            index += 1;
            // c. If e is not EMPTY, then
            if let Some(e) = e {
                // i. Perform ? Call(callbackfn, thisArg, « e, e, S »).
                call_function(
                    agent,
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[e, e, s.into_value()])),
                )?;
                // ii. NOTE: The number of elements in entries may have increased during execution of callbackfn.
                // iii. Set numEntries to the number of elements in entries.
                num_entries = agent[s].values.len();
            }
        }
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [24.2.4.8 Set.prototype.has ( value )](https://tc39.es/ecma262/#sec-set.prototype.has)
    fn has(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value)?;
        // 3. Set value to CanonicalizeKeyedCollectionKey(value).
        let value = canonicalize_keyed_collection_key(agent, arguments.get(0));
        let mut hasher = AHasher::default();
        let value_hash = {
            value.hash(agent, &mut hasher);
            hasher.finish()
        };
        let data = &agent[s];
        // 4. For each element e of S.[[SetData]], do
        // a. If e is not EMPTY and SameValue(e, value) is true, return true.
        let found = data
            .set_data
            .find(value_hash, |hash_equal_index| {
                let found_value = data.values[*hash_equal_index as usize].unwrap();
                // Quick check: Equal values have the same value.
                found_value == value || same_value(agent, found_value, value)
            })
            .is_some();
        // 5. Return false.
        Ok(found.into())
    }

    fn keys(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    /// ### [24.2.4.14 get Set.prototype.size](https://tc39.es/ecma262/#sec-get-set.prototype.size)
    ///
    /// Set.prototype.size is an accessor property whose set accessor function
    /// is undefined.
    fn get_size(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let s = require_set_data_internal_slot(agent, this_value)?;
        // 3. Let size be SetDataSize(S.[[SetData]]).
        let size = set_data_size(&agent[s]);
        // 4. Return 𝔽(size).
        Ok(Number::from(size).into_value())
    }

    fn values(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
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
            .with_builtin_function_property::<SetPrototypeKeys>()
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
fn require_set_data_internal_slot(agent: &mut Agent, value: Value) -> JsResult<Set> {
    match value {
        Value::Set(map) => Ok(map),
        _ => Err(agent
            .throw_exception_with_static_message(ExceptionType::TypeError, "Object is not a Set")),
    }
}

/// ### [24.2.1.5 SetDataSize ( setData )](https://tc39.es/ecma262/#sec-setdatasize)
///
/// The abstract operation SetDataSize takes argument setData (a List of either
/// ECMAScript language values or EMPTY) and returns a non-negative integer.
#[inline(always)]
fn set_data_size(set_data: &SetHeapData) -> u32 {
    // 1. Let count be 0.
    // 2. For each element e of setData, do
    // a. If e is not EMPTY, set count to count + 1.
    // 3. Return count.
    set_data.set_data.len() as u32
}
