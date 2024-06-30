//! ### 10.4.2 Array Exotic Objects
//!
//! https://tc39.es/ecma262/#sec-array-exotic-objects

pub(crate) mod abstract_operations;
mod data;

use std::ops::{Index, IndexMut, RangeInclusive};

use super::{array_set_length, ordinary::ordinary_define_own_property};
use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            PropertyDescriptor, PropertyKey, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        element_array::{ElementDescriptor, ElementsVector},
        indexes::ArrayIndex,
        CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

pub use data::{ArrayHeapData, SealableElementsVector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array(ArrayIndex);

static ARRAY_INDEX_RANGE: RangeInclusive<i64> = 0..=(i64::pow(2, 32) - 2);

impl Array {
    /// # Do not use this
    /// This is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        Self(ArrayIndex::from_u32_index(0))
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn len(&self, agent: &Agent) -> u32 {
        agent[*self].elements.len()
    }

    pub fn is_dense(self, agent: &Agent) -> bool {
        agent[self].elements.is_dense(agent)
    }

    /// An array is simple if it contains no element accessor descriptors.
    pub(crate) fn is_simple(self, agent: &Agent) -> bool {
        agent[self].elements.is_simple(agent)
    }

    /// An array is trivial if it contains no element descriptors.
    pub(crate) fn is_trivial(self, agent: &Agent) -> bool {
        agent[self].elements.is_trivial(agent)
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
    pub(crate) fn as_slice(self, agent: &Agent) -> &[Option<Value>] {
        let elements = agent[self].elements;
        &agent[elements]
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
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
        let prototype = Some(
            agent
                .current_realm()
                .intrinsics()
                .array_prototype()
                .into_object(),
        );
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype,
            keys: ElementsVector::default(),
            values: ElementsVector::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
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
            let elements = agent[self].elements;
            let elements = &agent[elements];
            if let Some(value) = elements.get(index as usize) {
                return Ok(value.map(|value| PropertyDescriptor {
                    value: Some(value),
                    ..Default::default()
                }));
            }
            return Ok(None);
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
                let (element_descriptor, value) =
                    ElementDescriptor::from_property_descriptor(&property_descriptor.into());
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
                // TODO: Handle property descriptors
                *agent[elements].get_mut(index as usize).unwrap() = property_descriptor.value;
                // i. If succeeded is false, return false.
                if false {
                    Ok(false)
                } else {
                    // k. Return true.
                    Ok(true)
                }
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
            let elements = agent[self].elements;
            if index >= elements.len() as i64 {
                // Indexes below 2^32 but above length are necessarily not
                // defined: If they were, then the length would be larger.
                // Hence, we look in the prototype.
                return if let Some(prototype) = self.internal_prototype(agent) {
                    prototype.internal_get(agent, property_key, receiver)
                } else {
                    Ok(Value::Undefined)
                };
            }
            let elements = &agent[elements];
            // Index has been checked to be between 0 <= idx < len; unwrapping should never fail.
            let element = *elements.get(index as usize).unwrap();
            if let Some(element) = element {
                Ok(element)
            } else {
                // TODO: getters
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
            let elements = agent[self].elements;
            if index >= elements.len() as i64 {
                return Ok(true);
            }
            let elements = &mut agent[elements];
            // TODO: Handle unwritable properties
            // Index has been checked to be between 0 <= idx < len; unwrapping should never fail.
            *elements.get_mut(index as usize).unwrap() = None;
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
        let idx = self.0.into_u32_index();
        self.0 = ArrayIndex::from_u32_index(idx - compactions.arrays.get_shift_for_index(idx));
    }
}
