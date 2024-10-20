// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### 10.4.2 Array Exotic Objects
//!
//! https://tc39.es/ecma262/#sec-array-exotic-objects

pub(crate) mod abstract_operations;
mod data;

use std::ops::{Index, IndexMut, RangeInclusive};

use super::{array_set_length, ordinary::ordinary_define_own_property};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, create_array_from_list},
            testing_and_comparison::same_value,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyDescriptor, PropertyKey, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        element_array::{ElementArrays, ElementDescriptor},
        indexes::ArrayIndex,
        CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

pub use data::{ArrayHeapData, SealableElementsVector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array(ArrayIndex);

pub(crate) static ARRAY_INDEX_RANGE: RangeInclusive<i64> = 0..=(i64::pow(2, 32) - 2);

impl Array {
    /// # Do not use this
    /// This is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        Self(ArrayIndex::from_u32_index(0))
    }

    /// Creates a new array with the given elements.
    ///
    /// This is equal to the [CreateArrayFromList](https://tc39.es/ecma262/#sec-createarrayfromlist)
    /// abstract operation.
    #[inline]
    pub fn from_slice(agent: &mut Agent, elements: &[Value]) -> Self {
        create_array_from_list(agent, elements)
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn len(&self, agent: &impl Index<Array, Output = ArrayHeapData>) -> u32 {
        agent[*self].elements.len()
    }

    pub fn is_empty(&self, agent: &impl Index<Array, Output = ArrayHeapData>) -> bool {
        agent[*self].elements.len() == 0
    }

    pub(crate) fn is_dense(self, agent: &impl ArrayHeapIndexable) -> bool {
        agent[self].elements.is_dense(agent)
    }

    /// An array is simple if it contains no element accessor descriptors.
    pub(crate) fn is_simple(self, agent: &impl ArrayHeapIndexable) -> bool {
        agent[self].elements.is_simple(agent)
    }

    /// An array is trivial if it contains no element descriptors.
    pub(crate) fn is_trivial(self, agent: &impl ArrayHeapIndexable) -> bool {
        agent[self].elements.is_trivial(agent)
    }

    #[inline]
    pub(crate) fn shallow_clone(self, agent: &mut Agent) -> Result<Array, ()> {
        let elements = agent[self].elements;
        let object_index = self.get_backing_object(agent);
        let cloned_elements = agent.heap.elements.shallow_clone(elements.into());
        let data = ArrayHeapData {
            object_index,
            elements: cloned_elements,
        };
        agent.heap.arrays.push(Some(data));
        Ok(Array(ArrayIndex::last(&agent.heap.arrays)))
    }

    #[inline]
    fn internal_get_backing(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if let Some(object_index) = self.get_backing_object(agent) {
            // If backing object exists, then we might have properties there
            object_index.internal_get(agent, property_key, receiver)
        } else {
            // If backing object doesn't exist, then we might still have
            // properties in the prototype.
            self.internal_prototype(agent)
                .unwrap()
                .internal_get(agent, property_key, receiver)
        }
    }

    #[inline]
    pub(crate) fn as_slice(self, arena: &impl ArrayHeapIndexable) -> &[Option<Value>] {
        let elements = arena[self].elements;
        &arena.as_ref()[elements]
    }

    #[inline]
    pub(crate) fn as_mut_slice(self, agent: &mut Agent) -> &mut [Option<Value>] {
        let elements = agent[self].elements;
        &mut agent[elements]
    }
}

impl IntoValue for Array {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Array {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<ArrayIndex> for Array {
    fn from(value: ArrayIndex) -> Self {
        Array(value)
    }
}

impl From<Array> for Object {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

impl From<Array> for Value {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

impl TryFrom<Value> for Array {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for Array {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::Array(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl InternalSlots for Array {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Array;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        agent[self].elements.len_writable = value;
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_extensible(agent, value)
        } else if !value {
            self.create_backing_object(agent)
                .internal_set_extensible(agent, value);
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_prototype(agent, prototype)
        } else {
            // 1. Let current be O.[[Prototype]].
            let current = agent.current_realm().intrinsics().array_prototype();
            if prototype == Some(current.into_object()) {
                return;
            }
            // Create array base object with custom prototype
            self.create_backing_object(agent)
                .internal_set_prototype(agent, prototype);
        }
    }
}

impl InternalMethods for Array {
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                if let Some(backing_object) = self.get_backing_object(agent) {
                    return backing_object.internal_get_own_property(agent, property_key);
                } else {
                    return Ok(None);
                }
            }
            // ARRAY_INDEX_RANGE guarantees were in u32 area.
            let index = index as u32;
            let elements = agent[self].elements;
            let length = elements.len();
            if index >= length {
                // Out of bounds
                return Ok(None);
            }
            let elements = elements.into();
            let index = index as usize;
            // We checked that we're within the vector bounds.
            let value = *agent.heap.elements.get(elements).get(index).unwrap();
            let descriptor = agent.heap.elements.get_descriptor(elements, index);
            return if value.is_none() && descriptor.is_none() {
                Ok(None)
            } else {
                Ok(Some(ElementDescriptor::to_property_descriptor(
                    descriptor, value,
                )))
            };
        }
        let length_key = PropertyKey::from(BUILTIN_STRING_MEMORY.length);
        let array_data = agent[self];
        if property_key == length_key {
            Ok(Some(PropertyDescriptor {
                value: Some(array_data.elements.len().into()),
                writable: Some(array_data.elements.len_writable),
                configurable: Some(false),
                enumerable: Some(false),
                ..Default::default()
            }))
        } else if let Some(backing_object) = array_data.object_index {
            backing_object.internal_get_own_property(agent, property_key)
        } else {
            Ok(None)
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            array_set_length(agent, self, property_descriptor)
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                let backing_object = self
                    .get_backing_object(agent)
                    .unwrap_or_else(|| self.create_backing_object(agent))
                    .into_object();
                return ordinary_define_own_property(
                    agent,
                    backing_object,
                    property_key,
                    property_descriptor,
                );
            }
            // Let lengthDesc be OrdinaryGetOwnProperty(A, "length").
            // b. Assert: IsDataDescriptor(lengthDesc) is true.
            // c. Assert: lengthDesc.[[Configurable]] is false.
            // d. Let length be lengthDesc.[[Value]].
            let elements = agent[self].elements;
            let length = elements.len();
            let length_writable = elements.len_writable;
            // e. Assert: length is a non-negative integral Number.
            // f. Let index be ! ToUint32(P).
            let index = index as u32;
            if index >= length {
                // g. If index ≥ length and lengthDesc.[[Writable]] is false, return false.
                if !length_writable {
                    return Ok(false);
                }
                let Heap {
                    elements, arrays, ..
                } = &mut agent.heap;
                let array_heap_data = &mut arrays[self];
                array_heap_data.elements.reserve(elements, index + 1);
                let value = property_descriptor.value;
                let element_descriptor =
                    ElementDescriptor::from_property_descriptor(property_descriptor);
                if index > length {
                    // Elements backing store should be filled with Nones already
                    array_heap_data.elements.len = index;
                }
                // ii. Set succeeded to ! OrdinaryDefineOwnProperty(A, "length", lengthDesc).
                array_heap_data
                    .elements
                    .push(elements, value, element_descriptor);
                // j. If index ≥ length, then
                // i. Set lengthDesc.[[Value]] to index + 1𝔽.
                // This should've already been handled by the push.
                debug_assert_eq!(agent[self].elements.len(), index + 1);
                // iii. Assert: succeeded is true.
                Ok(true)
            } else {
                // h. Let succeeded be ! OrdinaryDefineOwnProperty(A, P, Desc).
                return Ok(ordinary_define_own_property_for_array(
                    agent,
                    elements,
                    index,
                    property_descriptor,
                ));
            }
        } else {
            let backing_object = self
                .get_backing_object(agent)
                .unwrap_or_else(|| self.create_backing_object(agent))
                .into_object();
            ordinary_define_own_property(agent, backing_object, property_key, property_descriptor)
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        let has_own = self.internal_get_own_property(agent, property_key)?;
        if has_own.is_some() {
            return Ok(true);
        }

        // 3. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = self.internal_get_prototype_of(agent)?;

        // 4. If parent is not null, then
        if let Some(parent) = parent {
            // a. Return ? parent.[[HasProperty]](P).
            return parent.internal_has_property(agent, property_key);
        }

        // 5. Return false.
        Ok(false)
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(self.len(agent).into())
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                // Negative indexes and indexes over 2^32 - 2 go into backing store
                return self.internal_get_backing(agent, property_key, receiver);
            }
            let index = index as u32;
            let elements = agent[self].elements;
            if index >= elements.len() {
                // Indexes below 2^32 but above length are necessarily not
                // defined: If they were, then the length would be larger.
                // Hence, we look in the prototype.
                return if let Some(prototype) = self.internal_prototype(agent) {
                    prototype.internal_get(agent, property_key, receiver)
                } else {
                    Ok(Value::Undefined)
                };
            }
            // Index has been checked to be between 0 <= idx < len; indexing should never fail.
            let element = agent[elements][index as usize];
            if let Some(element) = element {
                Ok(element)
            } else {
                let (descriptors, _) = agent
                    .heap
                    .elements
                    .get_descriptors_and_slice(elements.into());
                if let Some(descriptors) = descriptors {
                    if let Some(descriptor) = descriptors.get(&index) {
                        if let Some(getter) = descriptor.getter_function() {
                            // 7. Return ? Call(getter, Receiver).
                            return call_function(agent, getter, receiver, None);
                        }
                    }
                }
                if let Some(prototype) = self.internal_prototype(agent) {
                    prototype.internal_get(agent, property_key, receiver)
                } else {
                    Ok(Value::Undefined)
                }
            }
        } else {
            self.internal_get_backing(agent, property_key, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(true)
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                return self
                    .get_backing_object(agent)
                    .map_or(Ok(true), |object_index| {
                        object_index.internal_delete(agent, property_key)
                    });
            }
            let index = index as u32;
            let elements = agent[self].elements;
            if index >= elements.len() {
                return Ok(true);
            }
            let (descriptors, slice) = agent
                .heap
                .elements
                .get_descriptors_and_slice_mut(elements.into());
            if let Some(descriptors) = descriptors {
                if let Some(descriptor) = descriptors.get(&index) {
                    if !descriptor.is_configurable() {
                        // Unconfigurable property.
                        return Ok(false);
                    }
                    descriptors.remove(&index);
                }
            }
            // Index has been checked to be between 0 <= idx < len; indexing should never fail.
            slice[index as usize] = None;
            Ok(true)
        } else {
            self.get_backing_object(agent)
                .map_or(Ok(true), |object_index| {
                    object_index.internal_delete(agent, property_key)
                })
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        let backing_keys = if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_own_property_keys(agent)?
        } else {
            Default::default()
        };
        let elements = agent[self].elements;
        let mut keys = Vec::with_capacity(elements.len() as usize + backing_keys.len());

        let elements_data = &agent[elements];

        for (index, value) in elements_data.iter().enumerate() {
            if value.is_some() {
                keys.push(PropertyKey::Integer((index as u32).into()))
            }
        }

        keys.extend(backing_keys);

        Ok(keys)
    }
}

impl Index<Array> for Agent {
    type Output = ArrayHeapData;

    fn index(&self, index: Array) -> &Self::Output {
        &self.heap.arrays[index]
    }
}

impl IndexMut<Array> for Agent {
    fn index_mut(&mut self, index: Array) -> &mut Self::Output {
        &mut self.heap.arrays[index]
    }
}

impl Index<Array> for Vec<Option<ArrayHeapData>> {
    type Output = ArrayHeapData;

    fn index(&self, index: Array) -> &Self::Output {
        self.get(index.get_index())
            .expect("Array out of bounds")
            .as_ref()
            .expect("Array slot empty")
    }
}

impl IndexMut<Array> for Vec<Option<ArrayHeapData>> {
    fn index_mut(&mut self, index: Array) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Array out of bounds")
            .as_mut()
            .expect("Array slot empty")
    }
}

impl CreateHeapData<ArrayHeapData, Array> for Heap {
    fn create(&mut self, data: ArrayHeapData) -> Array {
        self.arrays.push(Some(data));
        Array::from(ArrayIndex::last(&self.arrays))
    }
}

impl HeapMarkAndSweep for Array {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.arrays.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.arrays.shift_index(&mut self.0);
    }
}

fn ordinary_define_own_property_for_array(
    agent: &mut Agent,
    elements: SealableElementsVector,
    index: u32,
    descriptor: PropertyDescriptor,
) -> bool {
    let descriptor_value = descriptor.value;

    let (descriptors, slice) = agent
        .heap
        .elements
        .get_descriptors_and_slice(elements.into());
    let current_value = slice[index as usize];
    let current_descriptor = {
        let descriptor = descriptors.and_then(|descriptors| descriptors.get(&index).copied());
        if current_value.is_some() && descriptor.is_none() {
            Some(ElementDescriptor::WritableEnumerableConfigurableData)
        } else {
            descriptor
        }
    };

    // 2. If current is undefined, then
    if current_descriptor.is_none() && current_value.is_none() {
        // Hole

        // a. If extensible is false, return false.
        if !elements.writable() {
            return false;
        }

        // c. If IsAccessorDescriptor(Desc) is true, then
        if descriptor.is_accessor_descriptor() {
            // i. Create an own accessor property named P of object O whose [[Get]], [[Set]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            let (descriptors, _) = agent
                .heap
                .elements
                .get_descriptors_and_slice_mut(elements.into());
            let elem_descriptor = ElementDescriptor::from_property_descriptor(descriptor).unwrap();
            if let Some(descriptors) = descriptors {
                descriptors.insert(index, elem_descriptor);
            } else {
                agent.heap.elements.set_descriptor(
                    elements.into(),
                    index as usize,
                    Some(elem_descriptor),
                )
            }
        }
        // d. Else,
        else {
            // i. Create an own data property named P of object O whose [[Value]], [[Writable]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            let (descriptors, slice) = agent
                .heap
                .elements
                .get_descriptors_and_slice_mut(elements.into());
            slice[index as usize] = Some(descriptor_value.unwrap_or(Value::Undefined));
            let elem_descriptor = ElementDescriptor::from_property_descriptor(descriptor);
            if let Some(descriptor) = elem_descriptor {
                if let Some(descriptors) = descriptors {
                    descriptors.insert(index, descriptor);
                } else {
                    agent.heap.elements.set_descriptor(
                        elements.into(),
                        index as usize,
                        Some(descriptor),
                    )
                }
            }
        }

        // e. Return true.
        return true;
    };

    // 4. If Desc does not have any fields, return true.
    if !descriptor.has_fields() {
        return true;
    }

    // If current descriptor doesn't exist, then its a default data descriptor
    // with WEC all true.
    let current_writable = current_descriptor.map_or(Some(true), |c| c.is_writable());
    let current_enumerable = current_descriptor.map_or(true, |c| c.is_enumerable());
    let current_configurable = current_descriptor.map_or(true, |c| c.is_configurable());
    let current_is_data_descriptor = current_descriptor.map_or(false, |c| c.is_data_descriptor());
    let current_is_accessor_descriptor =
        current_descriptor.map_or(false, |c| c.is_accessor_descriptor());
    let current_getter = current_descriptor.and_then(|c| c.getter_function());
    let current_setter = current_descriptor.and_then(|c| c.setter_function());

    // 5. If current.[[Configurable]] is false, then
    if !current_configurable {
        // a. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
        if let Some(true) = descriptor.configurable {
            return false;
        }

        // b. If Desc has an [[Enumerable]] field and SameValue(Desc.[[Enumerable]], current.[[Enumerable]])
        //    is false, return false.
        if descriptor
            .enumerable
            .map_or(false, |enumerable| enumerable != current_enumerable)
        {
            return false;
        }

        // c. If IsGenericDescriptor(Desc) is false and SameValue(IsAccessorDescriptor(Desc), IsAccessorDescriptor(current))
        //    is false, return false.
        if !descriptor.is_generic_descriptor()
            && descriptor.is_accessor_descriptor() != current_is_accessor_descriptor
        {
            return false;
        }

        // d. If IsAccessorDescriptor(current) is true, then
        if current_is_accessor_descriptor {
            // i. If Desc has a [[Get]] field and SameValue(Desc.[[Get]], current.[[Get]]) is false,
            //    return false.
            if let Some(desc_get) = descriptor.get {
                if current_getter.map_or(true, |current_getter| desc_get != current_getter) {
                    return false;
                }
            }

            // ii. If Desc has a [[Set]] field and SameValue(Desc.[[Set]], current.[[Set]]) is
            //     false, return false.
            if let Some(desc_set) = descriptor.set {
                if current_setter.map_or(true, |current_setter| desc_set != current_setter) {
                    return false;
                }
            }
        }
        // e. Else if current.[[Writable]] is false, then
        else if !current_writable.unwrap() {
            // i. If Desc has a [[Writable]] field and Desc.[[Writable]] is true, return false.
            if let Some(true) = descriptor.writable {
                return false;
            }

            // ii. If Desc has a [[Value]] field and SameValue(Desc.[[Value]], current.[[Value]])
            //     is false, return false.
            if let Some(desc_value) = descriptor.value {
                if !same_value(agent, desc_value, current_value.unwrap()) {
                    return false;
                }
            }
        }
    }
    // a. If IsDataDescriptor(current) is true and IsAccessorDescriptor(Desc) is true, then
    if current_is_data_descriptor && descriptor.is_accessor_descriptor() {
        // i. If Desc has a [[Configurable]] field, let configurable be Desc.[[Configurable]];
        //    else let configurable be current.[[Configurable]].
        let configurable = descriptor.configurable.unwrap_or(current_configurable);

        // ii. If Desc has a [[Enumerable]] field, let enumerable be Desc.[[Enumerable]]; else
        //     let enumerable be current.[[Enumerable]].
        let enumerable = descriptor.enumerable.unwrap_or(current_enumerable);

        // iii. Replace the property named P of object O with an accessor property whose
        //      [[Configurable]] and [[Enumerable]] attributes are set to configurable and
        //      enumerable, respectively, and whose [[Get]] and [[Set]] attributes are set to
        //      the value of the corresponding field in Desc if Desc has that field, or to the
        //      attribute's default value otherwise.
        let new_descriptor = match (descriptor.get, descriptor.set) {
            (None, None) => unreachable!(),
            (None, Some(set)) => ElementDescriptor::new_with_set_ec(set, enumerable, configurable),
            (Some(get), None) => ElementDescriptor::new_with_get_ec(get, enumerable, configurable),
            (Some(get), Some(set)) => {
                ElementDescriptor::new_with_get_set_ec(get, set, enumerable, configurable)
            }
        };
        let (descriptors, slice) = agent
            .heap
            .elements
            .get_descriptors_and_slice_mut(elements.into());
        slice[index as usize] = None;
        if let Some(descriptors) = descriptors {
            descriptors.insert(index, new_descriptor);
        } else {
            agent.heap.elements.set_descriptor(
                elements.into(),
                index as usize,
                Some(new_descriptor),
            )
        }
    }
    // b. Else if IsAccessorDescriptor(current) is true and IsDataDescriptor(Desc) is true, then
    else if current_is_accessor_descriptor && descriptor.is_data_descriptor() {
        // i. If Desc has a [[Configurable]] field, let configurable be Desc.[[Configurable]];
        //    else let configurable be current.[[Configurable]].
        let configurable = descriptor.configurable.unwrap_or(current_configurable);

        // ii. If Desc has a [[Enumerable]] field, let enumerable be Desc.[[Enumerable]]; else
        //     let enumerable be current.[[Enumerable]].
        let enumerable = descriptor.enumerable.unwrap_or(current_enumerable);

        // iii. Replace the property named P of object O with a data property whose
        //      [[Configurable]] and [[Enumerable]] attributes are set to configurable and
        //      enumerable, respectively, and whose [[Value]] and [[Writable]] attributes are
        //      set to the value of the corresponding field in Desc if Desc has that field, or
        //      to the attribute's default value otherwise.
        // try object.propertyStorage().set(property_key, PropertyDescriptor{
        //     .value = descriptor.value or else .undefined,
        //     .writable = descriptor.writable or else false,
        //     .enumerable = enumerable,
        //     .configurable = configurable,
        // });
        let (descriptors, slice) = agent
            .heap
            .elements
            .get_descriptors_and_slice_mut(elements.into());
        if let Some(elem_descriptor) = ElementDescriptor::new_with_wec(
            descriptor.writable.unwrap_or(false),
            enumerable,
            configurable,
        ) {
            descriptors.unwrap().insert(index, elem_descriptor);
        } else {
            descriptors.unwrap().remove(&index);
        }
        slice[index as usize] = Some(descriptor.value.unwrap_or(Value::Undefined));
    }
    // c. Else,
    else {
        // i. For each field of Desc, set the corresponding attribute of the property named P
        //    of object O to the value of the field.
        let mut descriptor = descriptor;
        let result_value = descriptor.value.or(current_value);
        descriptor.writable = descriptor.writable.or(current_writable);
        descriptor.get = descriptor.get.or(current_getter);
        descriptor.set = descriptor.set.or(current_setter);
        descriptor.enumerable = Some(descriptor.enumerable.unwrap_or(current_enumerable));
        descriptor.configurable = Some(descriptor.configurable.unwrap_or(current_configurable));
        let (descriptors, slice) = agent
            .heap
            .elements
            .get_descriptors_and_slice_mut(elements.into());
        slice[index as usize] = result_value;
        if let Some(elem_descriptor) = ElementDescriptor::from_property_descriptor(descriptor) {
            if let Some(descriptors) = descriptors {
                descriptors.insert(index, elem_descriptor);
            } else {
                agent.heap.elements.set_descriptor(
                    elements.into(),
                    index as usize,
                    Some(elem_descriptor),
                )
            }
        } else if let Some(descriptors) = descriptors {
            descriptors.remove(&index);
        }
    }

    true
}

/// A partial view to the Agent's Heap that allows accessing array heap data.
pub(crate) struct ArrayHeap<'a> {
    elements: &'a ElementArrays,
    arrays: &'a Vec<Option<ArrayHeapData>>,
}

impl ArrayHeap<'_> {
    pub(crate) fn new<'a>(
        elements: &'a ElementArrays,
        arrays: &'a Vec<Option<ArrayHeapData>>,
    ) -> ArrayHeap<'a> {
        ArrayHeap { elements, arrays }
    }
}

impl Index<Array> for ArrayHeap<'_> {
    type Output = ArrayHeapData;

    fn index(&self, index: Array) -> &ArrayHeapData {
        self.arrays.index(index)
    }
}

impl AsRef<ElementArrays> for ArrayHeap<'_> {
    fn as_ref(&self) -> &ElementArrays {
        self.elements
    }
}

/// Helper trait for array indexing.
pub(crate) trait ArrayHeapIndexable:
    Index<Array, Output = ArrayHeapData> + AsRef<ElementArrays>
{
}
impl ArrayHeapIndexable for ArrayHeap<'_> {}
impl ArrayHeapIndexable for Agent {}
