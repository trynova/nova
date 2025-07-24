// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### 10.4.2 Array Exotic Objects
//!
//! https://tc39.es/ecma262/#sec-array-exotic-objects

pub(crate) mod abstract_operations;
mod data;

use core::ops::{Index, IndexMut, RangeInclusive};
use std::collections::hash_map::Entry;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, create_array_from_list, try_get_object_method},
            testing_and_comparison::same_value,
        },
        builtins::{
            array::abstract_operations::{array_set_length, array_try_set_length},
            ordinary::ordinary_define_own_property,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            BUILTIN_STRING_MEMORY, Function, InternalMethods, InternalSlots, IntoFunction,
            IntoObject, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, Value,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        unwrap_try,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WellKnownSymbolIndexes, WorkQueues,
        element_array::{
            ElementArrays, ElementDescriptor, ElementStorageMut, ElementStorageRef, ElementsVector,
        },
        indexes::ArrayIndex,
    },
};

use ahash::AHashMap;
pub use data::ArrayHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array<'a>(ArrayIndex<'a>);

pub(crate) static ARRAY_INDEX_RANGE: RangeInclusive<i64> = 0..=(i64::pow(2, 32) - 2);

impl<'a> Array<'a> {
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
    pub fn from_slice(agent: &mut Agent, elements: &[Value], gc: NoGcScope<'a, '_>) -> Self {
        create_array_from_list(agent, elements, gc)
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn len(&self, agent: &impl Index<Array<'a>, Output = ArrayHeapData<'static>>) -> u32 {
        agent[*self].elements.len()
    }

    pub fn length_writable(
        &self,
        agent: &impl Index<Array<'a>, Output = ArrayHeapData<'static>>,
    ) -> bool {
        agent[*self].elements.len_writable
    }

    pub fn is_empty(&self, agent: &impl Index<Array<'a>, Output = ArrayHeapData<'static>>) -> bool {
        agent[*self].elements.is_empty()
    }

    /// An array is dense if it contains no holes or getters.
    ///
    /// A dense array's properties can be accessed without calling into
    /// JavaScript. This does not necessarily mean that all the slots in the
    /// array contain a Value; some may be None but those slots are setters
    /// without a matching getter and accessing them returns `undefined`.
    pub(crate) fn is_dense(self, agent: &impl ArrayHeapIndexable<'a>) -> bool {
        agent[self].elements.is_dense(agent)
    }

    /// An array is simple if it contains no element accessor descriptors.
    pub(crate) fn is_simple(self, agent: &impl ArrayHeapIndexable<'a>) -> bool {
        agent[self].elements.is_simple(agent)
    }

    /// An array is trivial if it contains no element descriptors.
    pub(crate) fn is_trivial(self, agent: &impl ArrayHeapIndexable<'a>) -> bool {
        agent[self].elements.is_trivial(agent)
    }

    /// Returns the `value` as an Array if it is one `method` is
    /// `%Array.prototype.values%`.
    pub(crate) fn is_iterable_array(
        agent: &mut Agent,
        value: Value<'a>,
        method: Function<'a>,
    ) -> Option<Self> {
        match value {
            Value::Array(array)
                if method
                    == agent
                        .current_realm_record()
                        .intrinsics()
                        .array_prototype_values()
                        .into_function() =>
            {
                Some(array)
            }
            _ => None,
        }
    }

    /// Returns true if it is trivially iterable, ie. it contains no element
    /// accessor descriptors and uses the Array intrinsic itrator method.
    pub(crate) fn is_trivially_iterable(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> bool {
        if !self.is_dense(agent) {
            // Contains holes or getters, so cannot be iterated without looking
            // into the prototype chain or calling getters.
            false
        } else {
            let TryResult::Continue(Ok(Some(iterator_method))) = try_get_object_method(
                agent,
                self.into_object(),
                PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
                gc,
            ) else {
                // Can't get iterator method without calling a getter or Proxy
                // method; or getting the method threw an error which we ignore
                // here; or there is no iterator method, which will throw an
                // error later.
                return false;
            };

            // We got a proper iterator method; but is it the intrinsic Array
            // values iterator method?
            iterator_method
                == agent
                    .current_realm_record()
                    .intrinsics()
                    .array_prototype_values()
                    .into_function()
        }
    }

    // This method creates a "shallow clone" of the elements of a simple array (no descriptors).
    // If array is not simple, this cloned array will do some odd things (e.g. getter/setter indexes become holes)
    pub(crate) fn to_cloned(self, agent: &mut Agent) -> Self {
        let Heap {
            arrays, elements, ..
        } = &mut agent.heap;
        let cloned_elements = elements.shallow_clone(&arrays[self].elements);
        let data = ArrayHeapData {
            object_index: None,
            elements: cloned_elements,
        };
        agent.heap.create(data)
    }

    #[inline]
    fn try_get_backing<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
        if let Some(object_index) = self.get_backing_object(agent) {
            // If backing object exists, then we might have properties there
            object_index.try_get(agent, property_key, receiver, gc)
        } else {
            // If backing object doesn't exist, then we might still have
            // properties in the prototype.
            self.internal_prototype(agent)
                .unwrap()
                .try_get(agent, property_key, receiver, gc)
        }
    }

    #[inline]
    fn internal_get_backing<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let property_key = property_key.bind(gc.nogc());
        if let Some(object_index) = self.get_backing_object(agent) {
            // If backing object exists, then we might have properties there
            object_index.internal_get(agent, property_key.unbind(), receiver, gc)
        } else {
            // If backing object doesn't exist, then we might still have
            // properties in the prototype.
            self.internal_prototype(agent).unwrap().internal_get(
                agent,
                property_key.unbind(),
                receiver,
                gc,
            )
        }
    }

    #[inline]
    pub(crate) fn as_slice(self, arena: &impl ArrayHeapIndexable<'a>) -> &[Option<Value<'a>>] {
        &arena.as_ref()[&arena[self].elements]
    }

    #[inline]
    pub(crate) fn as_mut_slice(self, agent: &mut Agent) -> &mut [Option<Value<'static>>] {
        let elements = agent[self].elements;
        &mut agent[&elements]
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Array<'_> {
    type Of<'a> = Array<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<ArrayIndex<'a>> for Array<'a> {
    fn from(value: ArrayIndex<'a>) -> Self {
        Array(value)
    }
}

impl<'a> From<Array<'a>> for Object<'a> {
    fn from(value: Array) -> Self {
        Self::Array(value.unbind())
    }
}

impl<'a> From<Array<'a>> for Value<'a> {
    fn from(value: Array<'a>) -> Self {
        Self::Array(value)
    }
}

impl<'a> TryFrom<Value<'a>> for Array<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Array(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for Array<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::Array(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for Array<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Array;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
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
            if prototype
                == Some(
                    agent
                        .current_realm_record()
                        .intrinsics()
                        .array_prototype()
                        .into_object(),
                )
            {
                return;
            }
            // Create array base object with custom prototype
            self.create_backing_object(agent)
                .internal_set_prototype(agent, prototype);
        }
    }
}

impl<'a> InternalMethods<'a> for Array<'a> {
    fn try_prevent_extensions(self, agent: &mut Agent, _gc: NoGcScope) -> TryResult<bool> {
        self.internal_set_extensible(agent, false);
        TryResult::Continue(true)
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                if let Some(backing_object) = self.get_backing_object(agent) {
                    return TryResult::Continue(unwrap_try(backing_object.try_get_own_property(
                        agent,
                        property_key,
                        gc,
                    )));
                } else {
                    return TryResult::Continue(None);
                }
            }
            // ARRAY_INDEX_RANGE guarantees were in u32 area.
            let index = index as u32;
            let elements = &agent[self].elements;
            let length = elements.len();
            if index >= length {
                // Out of bounds
                return TryResult::Continue(None);
            }
            let ElementStorageRef {
                values,
                descriptors,
            } = elements.get_storage(agent);
            // We checked that we're within the vector bounds.
            let value = values[index as usize].bind(gc);
            let descriptor = descriptors.and_then(|d| d.get(&index));
            return if value.is_none() && descriptor.is_none() {
                TryResult::Continue(None)
            } else {
                TryResult::Continue(Some(ElementDescriptor::to_property_descriptor(
                    descriptor.cloned(),
                    value,
                )))
            };
        }
        let length_key = PropertyKey::from(BUILTIN_STRING_MEMORY.length);
        let array_data = agent[self];
        if property_key == length_key {
            TryResult::Continue(Some(PropertyDescriptor {
                value: Some(array_data.elements.len().into()),
                writable: Some(array_data.elements.len_writable),
                configurable: Some(false),
                enumerable: Some(false),
                ..Default::default()
            }))
        } else if let Some(backing_object) = array_data.object_index {
            TryResult::Continue(unwrap_try(backing_object.try_get_own_property(
                agent,
                property_key,
                gc,
            )))
        } else {
            TryResult::Continue(None)
        }
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            array_try_set_length(agent, self, property_descriptor)
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                let backing_object = self
                    .get_backing_object(agent)
                    .unwrap_or_else(|| self.create_backing_object(agent));
                return TryResult::Continue(ordinary_define_own_property(
                    agent,
                    backing_object,
                    property_key,
                    property_descriptor,
                    gc,
                ));
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
                    return TryResult::Continue(false);
                }
                let Heap {
                    elements,
                    arrays,
                    alloc_counter,
                    ..
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
                *alloc_counter += core::mem::size_of::<Option<Value>>();
                if element_descriptor.is_some() {
                    *alloc_counter += core::mem::size_of::<(u32, ElementDescriptor)>();
                }
                array_heap_data
                    .elements
                    .push(elements, value, element_descriptor);
                // j. If index ≥ length, then
                // i. Set lengthDesc.[[Value]] to index + 1𝔽.
                // This should've already been handled by the push.
                debug_assert_eq!(agent[self].elements.len(), index + 1);
                // iii. Assert: succeeded is true.
                TryResult::Continue(true)
            } else {
                // h. Let succeeded be ! OrdinaryDefineOwnProperty(A, P, Desc).
                TryResult::Continue(ordinary_define_own_property_for_array(
                    agent,
                    elements,
                    index,
                    property_descriptor,
                    gc,
                ))
            }
        } else {
            let backing_object = self
                .get_backing_object(agent)
                .unwrap_or_else(|| self.create_backing_object(agent));
            TryResult::Continue(ordinary_define_own_property(
                agent,
                backing_object,
                property_key,
                property_descriptor,
                gc,
            ))
        }
    }

    fn internal_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        let property_descriptor = property_descriptor.bind(gc.nogc());
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            array_set_length(agent, self, property_descriptor.unbind(), gc)
        } else {
            Ok(unwrap_try(self.try_define_own_property(
                agent,
                property_key.unbind(),
                property_descriptor.unbind(),
                gc.into_nogc(),
            )))
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        let has_own = unwrap_try(self.try_get_own_property(agent, property_key, gc));
        if has_own.is_some() {
            return TryResult::Continue(true);
        }

        // 3. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = unwrap_try(self.try_get_prototype_of(agent, gc));

        // 4. If parent is not null, then
        if let Some(parent) = parent {
            // a. Return ? parent.[[HasProperty]](P).
            return parent.try_has_property(agent, property_key, gc);
        }

        // 5. Return false.
        TryResult::Continue(false)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        // Note: GetOwnProperty cannot fail in Array.
        let has_own =
            unwrap_try(self.try_get_own_property(agent, property_key.unbind(), gc.nogc()));
        if has_own.is_some() {
            return Ok(true);
        }

        // 3. Let parent be ? O.[[GetPrototypeOf]]().
        // Note: GetPrototypeOf cannot fail in Array.
        let parent = unwrap_try(self.try_get_prototype_of(agent, gc.nogc()));

        // 4. If parent is not null, then
        if let Some(parent) = parent {
            // a. Return ? parent.[[HasProperty]](P).
            return parent
                .unbind()
                .internal_has_property(agent, property_key.unbind(), gc);
        }

        // 5. Return false.
        Ok(false)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            TryResult::Continue(self.len(agent).into())
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                // Negative indexes and indexes over 2^32 - 2 go into backing store
                return self.try_get_backing(agent, property_key, receiver, gc);
            }
            let index = index as u32;
            let elements = agent[self].elements;
            if index >= elements.len() {
                // Indexes below 2^32 but above length are necessarily not
                // defined: If they were, then the length would be larger.
                // Hence, we look in the prototype.
                return if let Some(prototype) = self.internal_prototype(agent) {
                    prototype.try_get(agent, property_key, receiver, gc)
                } else {
                    TryResult::Continue(Value::Undefined)
                };
            }
            // Index has been checked to be between 0 <= idx < len; indexing should never fail.
            let element = agent[&elements][index as usize];
            if let Some(element) = element {
                TryResult::Continue(element)
            } else {
                let ElementStorageRef { descriptors, .. } =
                    agent.heap.elements.get_element_storage(&elements);
                if let Some(descriptors) = descriptors
                    && let Some(descriptor) = descriptors.get(&index)
                    && descriptor.has_getter()
                {
                    // 7. Return ? Call(getter, Receiver).
                    // return call_function(agent, getter, receiver, None, gc);
                    return TryResult::Break(());
                }
                if let Some(prototype) = self.internal_prototype(agent) {
                    prototype.try_get(agent, property_key, receiver, gc)
                } else {
                    TryResult::Continue(Value::Undefined)
                }
            }
        } else {
            self.try_get_backing(agent, property_key, receiver, gc)
        }
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let property_key = property_key.bind(gc.nogc());
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(self.len(agent).into())
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                // Negative indexes and indexes over 2^32 - 2 go into backing store
                return self.internal_get_backing(agent, property_key.unbind(), receiver, gc);
            }
            let index = index as u32;
            let elements = agent[self].elements;
            if index >= elements.len() {
                // Indexes below 2^32 but above length are necessarily not
                // defined: If they were, then the length would be larger.
                // Hence, we look in the prototype.
                return if let Some(prototype) = self.internal_prototype(agent) {
                    prototype.internal_get(agent, property_key.unbind(), receiver, gc)
                } else {
                    Ok(Value::Undefined)
                };
            }
            // Index has been checked to be between 0 <= idx < len; indexing should never fail.
            let element = agent[&elements][index as usize];
            if let Some(element) = element {
                Ok(element)
            } else {
                let ElementStorageRef { descriptors, .. } =
                    agent.heap.elements.get_element_storage(&elements);
                if let Some(descriptors) = descriptors
                    && let Some(descriptor) = descriptors.get(&index)
                    && let Some(getter) = descriptor.getter_function(gc.nogc())
                {
                    // 7. Return ? Call(getter, Receiver).
                    return call_function(agent, getter.unbind(), receiver, None, gc);
                }
                if let Some(prototype) = self.internal_prototype(agent) {
                    prototype.internal_get(agent, property_key.unbind(), receiver, gc)
                } else {
                    Ok(Value::Undefined)
                }
            }
        } else {
            self.internal_get_backing(agent, property_key.unbind(), receiver, gc)
        }
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            TryResult::Continue(false)
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !ARRAY_INDEX_RANGE.contains(&index) {
                return TryResult::Continue(
                    self.get_backing_object(agent)
                        .map(|object_index| {
                            unwrap_try(object_index.try_delete(agent, property_key, gc))
                        })
                        .unwrap_or(true),
                );
            }
            let index = index as u32;
            let elements = agent[self].elements;
            if index >= elements.len() {
                return TryResult::Continue(true);
            }
            let ElementStorageMut {
                values,
                descriptors,
            } = agent.heap.elements.get_element_storage_mut(&elements);
            if let Entry::Occupied(mut descriptors) = descriptors {
                let descriptors = descriptors.get_mut();
                if let Some(descriptor) = descriptors.get(&index) {
                    if !descriptor.is_configurable() {
                        // Unconfigurable property.
                        return TryResult::Continue(false);
                    }
                    descriptors.remove(&index);
                }
            }
            // Index has been checked to be between 0 <= idx < len; indexing should never fail.
            values[index as usize] = None;
            TryResult::Continue(true)
        } else {
            TryResult::Continue(
                self.get_backing_object(agent)
                    .map(|object_index| {
                        unwrap_try(object_index.try_delete(agent, property_key, gc))
                    })
                    .unwrap_or(true),
            )
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        let backing_keys = if let Some(backing_object) = self.get_backing_object(agent) {
            unwrap_try(backing_object.try_own_property_keys(agent, gc))
        } else {
            Default::default()
        };
        let elements = &agent[self].elements;
        let mut keys = Vec::with_capacity(elements.len() as usize + 1 + backing_keys.len());

        let elements_data = &agent[elements];

        for (index, value) in elements_data.iter().enumerate() {
            if value.is_some() {
                keys.push(PropertyKey::Integer((index as u32).into()))
            }
        }

        keys.push(BUILTIN_STRING_MEMORY.length.to_property_key());
        keys.extend(backing_keys);

        TryResult::Continue(keys)
    }
}

impl Index<Array<'_>> for Agent {
    type Output = ArrayHeapData<'static>;

    fn index(&self, index: Array) -> &Self::Output {
        &self.heap.arrays[index]
    }
}

impl IndexMut<Array<'_>> for Agent {
    fn index_mut(&mut self, index: Array) -> &mut Self::Output {
        &mut self.heap.arrays[index]
    }
}

impl Index<Array<'_>> for Vec<Option<ArrayHeapData<'static>>> {
    type Output = ArrayHeapData<'static>;

    fn index(&self, index: Array) -> &Self::Output {
        self.get(index.get_index())
            .expect("Array out of bounds")
            .as_ref()
            .expect("Array slot empty")
    }
}

impl IndexMut<Array<'_>> for Vec<Option<ArrayHeapData<'static>>> {
    fn index_mut(&mut self, index: Array) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Array out of bounds")
            .as_mut()
            .expect("Array slot empty")
    }
}

impl Rootable for Array<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Array(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Array(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<ArrayHeapData<'a>, Array<'a>> for Heap {
    fn create(&mut self, data: ArrayHeapData<'a>) -> Array<'a> {
        self.arrays.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<ArrayHeapData<'static>>>();
        Array::from(ArrayIndex::last(&self.arrays))
    }
}

impl HeapMarkAndSweep for Array<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.arrays.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.arrays.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Array<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.arrays.shift_weak_index(self.0).map(Self)
    }
}

fn ordinary_define_own_property_for_array(
    agent: &mut Agent,
    elements: ElementsVector,
    index: u32,
    descriptor: PropertyDescriptor,
    gc: NoGcScope,
) -> bool {
    let descriptor_value = descriptor.value;

    let ElementStorageRef {
        values,
        descriptors,
    } = agent.heap.elements.get_element_storage(&elements);
    let current_value = values[index as usize];
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
            let elem_descriptor = ElementDescriptor::from_accessor_descriptor(descriptor);
            insert_element_descriptor(agent, &elements, index, None, elem_descriptor);
        }
        // d. Else,
        else {
            // i. Create an own data property named P of object O whose [[Value]], [[Writable]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            insert_data_descriptor(
                agent,
                &elements,
                index,
                Some(descriptor_value.unwrap_or(Value::Undefined)),
                ElementDescriptor::from_data_descriptor(descriptor),
            );
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
    let current_enumerable = current_descriptor.is_none_or(|c| c.is_enumerable());
    let current_configurable = current_descriptor.is_none_or(|c| c.is_configurable());
    let current_is_data_descriptor = current_descriptor.is_some_and(|c| c.is_data_descriptor());
    let current_is_accessor_descriptor =
        current_descriptor.is_some_and(|c| c.is_accessor_descriptor());
    let current_getter = current_descriptor.and_then(|c| c.getter_function(gc));
    let current_setter = current_descriptor.and_then(|c| c.setter_function(gc));

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
            .is_some_and(|enumerable| enumerable != current_enumerable)
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
            if let Some(desc_get) = descriptor.get
                && current_getter != Some(desc_get)
            {
                return false;
            }

            // ii. If Desc has a [[Set]] field and SameValue(Desc.[[Set]], current.[[Set]]) is
            //     false, return false.
            if let Some(desc_set) = descriptor.set
                && current_setter != Some(desc_set)
            {
                return false;
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
            if let Some(desc_value) = descriptor.value
                && !same_value(agent, desc_value, current_value.unwrap())
            {
                return false;
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
        let elem_descriptor = ElementDescriptor::from_accessor_descriptor_fields(
            descriptor.get,
            descriptor.set,
            enumerable,
            configurable,
        );
        insert_element_descriptor(agent, &elements, index, None, elem_descriptor);
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
        mutate_element_descriptor(
            agent,
            &elements,
            index,
            Some(descriptor.value.unwrap_or(Value::Undefined)),
            ElementDescriptor::new_with_wec(
                descriptor.writable.unwrap_or(false),
                enumerable,
                configurable,
            ),
        );
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
        let elem_descriptor = ElementDescriptor::from_property_descriptor(descriptor);
        mutate_data_descriptor(agent, &elements, index, result_value, elem_descriptor);
    }

    true
}

fn mutate_data_descriptor(
    agent: &mut Agent,
    elements: &ElementsVector,
    index: u32,
    descriptor_value: Option<Value>,
    elem_descriptor: Option<ElementDescriptor>,
) {
    if let Some(descriptor) = elem_descriptor {
        insert_element_descriptor(agent, elements, index, descriptor_value, descriptor);
    } else {
        let ElementStorageMut {
            values,
            descriptors,
        } = agent.heap.elements.get_element_storage_mut(elements);
        values[index as usize] = descriptor_value.unbind();
        if let Entry::Occupied(mut descriptors) = descriptors {
            let descriptors = descriptors.get_mut();
            descriptors.remove(&index);
        }
    }
}

fn mutate_element_descriptor(
    agent: &mut Agent,
    elements: &ElementsVector,
    index: u32,
    descriptor_value: Option<Value>,
    elem_descriptor: Option<ElementDescriptor>,
) {
    if let Some(descriptor) = elem_descriptor {
        insert_element_descriptor(agent, elements, index, descriptor_value, descriptor);
    } else if let ElementStorageMut {
        descriptors: Entry::Occupied(mut descriptors),
        ..
    } = agent.heap.elements.get_element_storage_mut(elements)
    {
        let descriptors = descriptors.get_mut();
        descriptors.remove(&index);
    }
}

fn insert_data_descriptor(
    agent: &mut Agent,
    elements: &ElementsVector,
    index: u32,
    descriptor_value: Option<Value>,
    elem_descriptor: Option<ElementDescriptor>,
) {
    if let Some(descriptor) = elem_descriptor {
        insert_element_descriptor(agent, elements, index, descriptor_value, descriptor);
    } else {
        agent.heap.alloc_counter += core::mem::size_of::<Option<Value>>();
        agent[elements][index as usize] =
            Some(descriptor_value.unwrap_or(Value::Undefined).unbind());
    }
}

fn insert_element_descriptor(
    agent: &mut Agent,
    elements: &ElementsVector,
    index: u32,
    descriptor_value: Option<Value>,
    descriptor: ElementDescriptor,
) {
    let ElementStorageMut {
        values,
        descriptors,
    } = agent.heap.elements.get_element_storage_mut(elements);
    values[index as usize] = descriptor_value.unbind();
    match descriptors {
        Entry::Occupied(e) => {
            let descriptors = e.into_mut();
            let inserted = descriptors.insert(index, descriptor.unbind()).is_none();
            if inserted {
                agent.heap.alloc_counter += core::mem::size_of::<(u32, ElementDescriptor)>();
            }
        }
        Entry::Vacant(vacant_entry) => {
            agent.heap.alloc_counter += core::mem::size_of::<(u32, ElementDescriptor)>();
            let mut descriptors = AHashMap::with_capacity(1);
            descriptors.insert(index, descriptor.unbind());
            vacant_entry.insert(descriptors);
        }
    }
}

/// A partial view to the Agent's Heap that allows accessing array heap data.
pub(crate) struct ArrayHeap<'a> {
    elements: &'a ElementArrays,
    arrays: &'a Vec<Option<ArrayHeapData<'static>>>,
}

impl ArrayHeap<'_> {
    pub(crate) fn new<'a>(
        elements: &'a ElementArrays,
        arrays: &'a Vec<Option<ArrayHeapData<'static>>>,
    ) -> ArrayHeap<'a> {
        ArrayHeap { elements, arrays }
    }
}

impl Index<Array<'_>> for ArrayHeap<'_> {
    type Output = ArrayHeapData<'static>;

    fn index(&self, index: Array) -> &ArrayHeapData<'static> {
        self.arrays.index(index)
    }
}

impl AsRef<ElementArrays> for ArrayHeap<'_> {
    fn as_ref(&self) -> &ElementArrays {
        self.elements
    }
}

/// Helper trait for array indexing.
pub(crate) trait ArrayHeapIndexable<'a>:
    Index<Array<'a>, Output = ArrayHeapData<'static>> + AsRef<ElementArrays>
{
}
impl ArrayHeapIndexable<'_> for ArrayHeap<'_> {}
impl ArrayHeapIndexable<'_> for Agent {}
