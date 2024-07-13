// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{construct, get, get_function_realm},
            testing_and_comparison::{is_array, is_constructor, same_value},
            type_conversion::{to_number, to_uint32},
        },
        builtins::ArgumentsList,
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{IntoObject, Number, Object, PropertyDescriptor, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{indexes::ArrayIndex, Heap, WellKnownSymbolIndexes},
};

use super::{data::SealableElementsVector, Array, ArrayHeapData};

/// ### [10.4.2.2 ArrayCreate ( length \[ , proto \] )](https://tc39.es/ecma262/#sec-arraycreate)
///
/// The abstract operation ArrayCreate takes argument length (a non-negative
/// integer) and optional argument proto (an Object) and returns either a
/// normal completion containing an Array exotic object or a throw completion.
/// It is used to specify the creation of new Arrays.
pub fn array_create(
    agent: &mut Agent,
    length: usize,
    capacity: usize,
    proto: Option<Object>,
) -> JsResult<Array> {
    // 1. If length > 2**32 - 1, throw a RangeError exception.
    if length > (2usize.pow(32) - 1) {
        return Err(agent.throw_exception(ExceptionType::RangeError, "invalid array length"));
    }
    // 2. If proto is not present, set proto to %Array.prototype%.
    let object_index = if let Some(proto) = proto {
        if proto
            == agent
                .current_realm()
                .intrinsics()
                .array_prototype()
                .into_object()
        {
            None
        } else {
            Some(agent.heap.create_object_with_prototype(proto, &[]))
        }
    } else {
        None
    };
    // 3. Let A be MakeBasicObject(« [[Prototype]], [[Extensible]] »).
    // 5. Set A.[[DefineOwnProperty]] as specified in 10.4.2.1.
    let mut elements = agent
        .heap
        .elements
        .allocate_elements_with_capacity(capacity);
    elements.len = length as u32;
    let data = ArrayHeapData {
        // 4. Set A.[[Prototype]] to proto.
        object_index,
        elements: SealableElementsVector::from_elements_vector(elements),
    };
    agent.heap.arrays.push(Some(data));

    // 7. Return A.
    Ok(Array(ArrayIndex::last(&agent.heap.arrays)))
}

/// ### [10.4.2.3 ArraySpeciesCreate ( originalArray, length )](https://tc39.es/ecma262/#sec-arrayspeciescreate)
///
/// The abstract operation ArraySpeciesCreate takes arguments originalArray (an
/// Object) and length (a non-negative integer) and returns either a normal
/// completion containing an Object or a throw completion. It is used to
/// specify the creation of a new Array or similar object using a constructor
/// function that is derived from originalArray. It does not enforce that the
/// constructor function returns an Array.
///
/// > Note: If originalArray was created using the standard built-in Array
/// > constructor for a realm that is not the realm of the running execution
/// > context, then a new Array is created using the realm of the running
/// > execution context. This maintains compatibility with Web browsers that
/// > have historically had that behaviour for the Array.prototype methods
/// > that now are defined using ArraySpeciesCreate.
pub(crate) fn array_species_create(
    agent: &mut Agent,
    original_array: Object,
    length: usize,
) -> JsResult<Object> {
    // 1. Let isArray be ? IsArray(originalArray).
    let original_is_array = is_array(agent, original_array.into_value())?;
    // 2. If isArray is false, return ? ArrayCreate(length).
    if !original_is_array {
        let new_array = array_create(agent, length, length, None)?;
        return Ok(new_array.into_object());
    }
    // 3. Let C be ? Get(originalArray, "constructor").
    let mut c = get(
        agent,
        original_array,
        BUILTIN_STRING_MEMORY.constructor.into(),
    )?;
    // 4. If IsConstructor(C) is true, then
    if let Some(c_func) = is_constructor(agent, c) {
        // a. Let thisRealm be the current Realm Record.
        let this_realm = agent.current_realm_id();
        // b. Let realmC be ? GetFunctionRealm(C).
        let realm_c = get_function_realm(agent, c_func)?;
        // c. If thisRealm and realmC are not the same Realm Record, then
        if this_realm != realm_c {
            // i. If SameValue(C, realmC.[[Intrinsics]].[[%Array%]]) is true, set C to undefined.
            if same_value(agent, c, agent.get_realm(realm_c).intrinsics().array()) {
                c = Value::Undefined;
            }
        }
    }
    // 5. If C is an Object, then
    if let Ok(c_obj) = Object::try_from(c) {
        // a. Set C to ? Get(C, @@species).
        c = get(agent, c_obj, WellKnownSymbolIndexes::Species.into())?;
        // b. If C is null, set C to undefined.
        if c.is_null() {
            c = Value::Undefined;
        }
    }
    // 6. If C is undefined, return ? ArrayCreate(length).
    if c.is_undefined() {
        let new_array = array_create(agent, length, length, None)?;
        return Ok(new_array.into_object());
    }
    // 7. If IsConstructor(C) is false, throw a TypeError exception.
    let Some(c) = is_constructor(agent, c) else {
        return Err(agent.throw_exception(ExceptionType::TypeError, "Not a constructor"));
    };
    // 8. Return ? Construct(C, « 𝔽(length) »).
    let length = Value::from_f64(agent, length as f64);
    construct(agent, c, Some(ArgumentsList(&[length])), None)
}

/// ### [10.4.2.4 ArraySetLength ( A, Desc )](https://tc39.es/ecma262/#sec-arraysetlength)
///
/// The abstract operation ArraySetLength takes arguments A (an Array) and Desc (a Property Descriptor) and returns either a normal completion containing a Boolean or a throw completion.
pub fn array_set_length(agent: &mut Agent, a: Array, desc: PropertyDescriptor) -> JsResult<bool> {
    // 1. If Desc does not have a [[Value]] field, then
    let Some(desc_value) = desc.value else {
        // a. Return ! OrdinaryDefineOwnProperty(A, "length", Desc).
        if !desc.has_fields() {
            return Ok(true);
        }
        if desc.configurable == Some(true) || desc.enumerable == Some(true) {
            return Ok(false);
        }
        if !desc.is_generic_descriptor() && desc.is_accessor_descriptor() {
            return Ok(false);
        }
        if !agent[a].elements.len_writable {
            // Length is already frozen.
            if desc.writable == Some(true) {
                return Ok(false);
            }
        } else if desc.writable == Some(false) {
            agent[a].elements.len_writable = false;
        }
        return Ok(true);
    };
    // 2. Let newLenDesc be a copy of Desc.
    // 13. If newLenDesc does not have a [[Writable]] field or newLenDesc.[[Writable]] is true, then
    // a. Let newLenDesc.[[Writable]] be true
    let new_len_writable = desc.writable.unwrap_or(true);
    // NOTE: Setting the [[Writable]] attribute to false is deferred in case any elements cannot be deleted.
    // 3. Let newLen be ? ToUint32(Desc.[[Value]]).
    let new_len = to_uint32(agent, desc_value)?;
    // 4. Let numberLen be ? ToNumber(Desc.[[Value]]).
    let number_len = to_number(agent, desc_value)?;
    // 5. If SameValueZero(newLen, numberLen) is false, throw a RangeError exception.
    if !Number::same_value_zero(agent, number_len, new_len.into()) {
        return Err(agent.throw_exception(ExceptionType::RangeError, "invalid array length"));
    }
    // 6. Set newLenDesc.[[Value]] to newLen.
    // 7. Let oldLenDesc be OrdinaryGetOwnProperty(A, "length").
    let Heap {
        arrays, elements, ..
    } = &mut agent.heap;
    let array_heap_data = &mut arrays[a];
    // 10. Let oldLen be oldLenDesc.[[Value]].
    let (old_len, old_len_writable) = (
        array_heap_data.elements.len(),
        array_heap_data.elements.len_writable,
    );
    // 12. If oldLenDesc.[[Writable]] is false, return false.
    if !old_len_writable {
        return Ok(false);
    }
    // Optimization: check OrdinaryDefineOwnProperty conditions for failing early on.
    if desc.configurable == Some(true) || desc.enumerable == Some(true) {
        // 16. If succeeded is false, return false.
        return Ok(false);
    }
    // 11. If newLen ≥ oldLen, then
    if new_len >= old_len {
        // a. Return ! OrdinaryDefineOwnProperty(A, "length", newLenDesc).
        array_heap_data.elements.reserve(elements, new_len);
        array_heap_data.elements.len = new_len;
        array_heap_data.elements.len_writable = new_len_writable;
        return Ok(true);
    }
    // 15. Let succeeded be ! OrdinaryDefineOwnProperty(A, "length", newLenDesc).
    let old_elements = array_heap_data.elements;
    array_heap_data.elements.len = new_len;
    // 17. For each own property key P of A such that P is an array index and ! ToUint32(P) ≥ newLen, in descending numeric index order, do
    debug_assert!(old_len > new_len);
    for i in new_len + 1..old_len {
        // a. Let deleteSucceeded be ! A.[[Delete]](P).
        let elements = &mut elements[old_elements];
        // TODO: Handle unwritable properties and property descriptors.
        *elements.get_mut(i as usize).unwrap() = None;
        let delete_succeeded = true;
        // b. If deleteSucceeded is false, then
        if !delete_succeeded {
            let array_heap_data = &mut arrays[a];
            // i. Set newLenDesc.[[Value]] to ! ToUint32(P) + 1𝔽.
            array_heap_data.elements.len = i + 1;
            // ii. If newWritable is false, set newLenDesc.[[Writable]] to false.
            array_heap_data.elements.len_writable &= new_len_writable;
            // iii. Perform ! OrdinaryDefineOwnProperty(A, "length", newLenDesc).
            // iv. Return false.
            return Ok(false);
        }
    }
    // 18. If newWritable is false, then
    if !new_len_writable {
        // a. Set succeeded to ! OrdinaryDefineOwnProperty(A, "length", PropertyDescriptor { [[Writable]]: false }).
        // b. Assert: succeeded is true.
        let array_heap_data = &mut arrays[a];
        array_heap_data.elements.len_writable &= new_len_writable;
    }
    // 19. Return true.
    Ok(true)
}
