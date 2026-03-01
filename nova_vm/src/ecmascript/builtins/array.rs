// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [10.4.2 Array Exotic Objects](https://tc39.es/ecma262/#sec-array-exotic-objects)

mod abstract_operations;
mod data;

pub(crate) use abstract_operations::*;
pub(crate) use data::*;

use core::ops::RangeInclusive;
use std::collections::{TryReserveError, hash_map::Entry};

use crate::{
    ecmascript::{
        Agent, BUILTIN_STRING_MEMORY, Caches, Function, InternalMethods, InternalSlots, JsResult,
        Object, OrdinaryObject, PropertyDescriptor, PropertyKey, ProtoIntrinsics, TryError,
        TryGetResult, TryHasResult, TryResult, Value, call_function, create_array_from_list,
        js_result_into_try, object_handle, ordinary_define_own_property, same_value, unwrap_try,
    },
    engine::{Bindable, GcScope, NoGcScope},
    heap::{
        ArenaAccessSoA, ArenaAccessSoAMut, BaseIndex, CompactionLists, CreateHeapData,
        ElementArrays, ElementDescriptor, ElementStorageMut, ElementStorageRef, ElementsVector,
        Heap, HeapIndexHandle, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues,
        arena_vec_access,
    },
};

use ahash::AHashMap;
use soavec::SoAVec;

use super::{
    PropertyLookupCache, ordinary_delete, ordinary_get, ordinary_get_own_property,
    ordinary_has_property, ordinary_try_get, ordinary_try_has_property,
};

/// ### [10.4.2 Array Exotic Objects](https://tc39.es/ecma262/#sec-array-exotic-objects)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Array<'a>(BaseIndex<'a, ArrayHeapData<'static>>);
object_handle!(Array);
arena_vec_access!(soa: Array, 'a, ArrayHeapData, arrays, ArrayHeapDataRef, ArrayHeapDataMut);

pub(crate) static ARRAY_INDEX_RANGE: RangeInclusive<i64> = 0..=(i64::pow(2, 32) - 2);

impl<'a> Array<'a> {
    /// Allocate a new Array in the Agent heap with 0 capacity.
    pub fn new(agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Self {
        agent
            .heap
            .create(ArrayHeapData {
                object_index: None,
                elements: ElementsVector::EMPTY,
            })
            .bind(gc)
    }

    /// Get a reference to the next Array that will be allocated.
    ///
    /// # Safety
    ///
    /// You should not use this unless you're about to allocate the next Array
    /// very soon but need to generate references to it ahead of time.
    /// Effectively, this is only useful in Realm creation for getting the
    /// `%Array.prototype%` intrinsic.
    pub(crate) unsafe fn next_array(agent: &Agent) -> Self {
        Array(BaseIndex::from_index_u32(agent.heap.arrays.len()))
    }

    /// Allocate a new Array in the Agent heap with 0 capacity.
    pub fn with_capacity(
        agent: &mut Agent,
        capacity: u32,
        gc: NoGcScope<'a, '_>,
    ) -> Result<Self, TryReserveError> {
        let elements = agent
            .heap
            .elements
            .allocate_elements_with_length(capacity as usize)?;
        Ok(agent
            .heap
            .create(ArrayHeapData {
                object_index: None,
                elements,
            })
            .bind(gc))
    }

    /// Push a Value into this Array.
    ///
    /// > Note: this should only be used in places where the next index is
    /// > populated using code such as `! CreateDataPropertyOrThrow(A, "0", S)`,
    /// > ie. when the operation is known to be infallible.
    pub fn push(self, agent: &mut Agent, value: Value<'a>) -> Result<(), TryReserveError> {
        agent
            .heap
            .arrays
            .get_mut(self.0.get_index_u32())
            .unwrap()
            .elements
            .push(&mut agent.heap.elements, Some(value), None)
    }

    /// Reserve space for `additional` Values in the Array.
    pub fn reserve(self, agent: &mut Agent, additional: u32) -> Result<(), TryReserveError> {
        let Heap {
            arrays, elements, ..
        } = &mut agent.heap;
        let elems = self.get_elements_mut(arrays);
        elems.reserve(elements, elems.len().saturating_add(additional))
    }

    pub(crate) fn get_elements(
        self,
        agent: &impl AsRef<SoAVec<ArrayHeapData<'static>>>,
    ) -> &ElementsVector<'a> {
        agent
            .as_ref()
            .get(self.0.get_index_u32())
            .expect("Invalid Array reference")
            .elements
    }

    pub(crate) fn get_elements_mut(
        self,
        agent: &mut impl AsMut<SoAVec<ArrayHeapData<'static>>>,
    ) -> &mut ElementsVector<'static> {
        agent
            .as_mut()
            .get_mut(self.0.get_index_u32())
            .expect("Invalid Array reference")
            .elements
    }

    /// Creates a new array with the given elements.
    ///
    /// This is equal to the [CreateArrayFromList](https://tc39.es/ecma262/#sec-createarrayfromlist)
    /// abstract operation.
    #[inline]
    pub fn from_slice(agent: &mut Agent, elements: &[Value], gc: NoGcScope<'a, '_>) -> Self {
        create_array_from_list(agent, elements, gc)
    }

    /// Get the length property value of the Array.
    pub fn len(self, agent: &Agent) -> u32 {
        self.get_elements(agent).len()
    }

    /// ### Safety
    ///
    /// `len` must be less or equal to the capacity of the Array. Also note
    /// that uninitialised entries in the Array are allowed and are guaranteed
    /// to appear as JavaScript Array "holes", but this guarantee may be
    /// removed in the future.
    pub unsafe fn set_len(self, agent: &mut Agent, len: u32) {
        let elems = self.get_elements_mut(agent);
        debug_assert!(elems.len_writable);
        elems.len = len;
    }

    pub(crate) fn length_writable(self, agent: &Agent) -> bool {
        self.get_elements(agent).len_writable
    }

    pub(crate) fn set_length_readonly(self, agent: &mut Agent) {
        self.get_elements_mut(agent).len_writable = false;
    }

    /// Returns `true` if the Array length is 0.
    pub fn is_empty(self, agent: &Agent) -> bool {
        self.get_elements(agent).is_empty()
    }

    /// An array is dense if it contains no holes or getters.
    ///
    /// A dense array's properties can be accessed without calling into
    /// JavaScript. This does not necessarily mean that all the slots in the
    /// array contain a Value; some may be None but those slots are setters
    /// without a matching getter and accessing them returns `undefined`.
    pub(crate) fn is_dense(self, agent: &impl ArrayHeapAccess<'a>) -> bool {
        self.get_elements(agent).is_dense(agent)
    }

    /// An array is simple if it contains no element accessor descriptors.
    pub(crate) fn is_simple(self, agent: &impl ArrayHeapAccess<'a>) -> bool {
        self.get_elements(agent).is_simple(agent)
    }

    /// An array is trivial if it contains no element descriptors.
    pub(crate) fn is_trivial(self, agent: &impl ArrayHeapAccess<'a>) -> bool {
        self.get_elements(agent).is_trivial(agent)
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
                        .into() =>
            {
                Some(array)
            }
            _ => None,
        }
    }

    /// Returns true if it is trivially iterable, ie. it contains no element
    /// accessor descriptors and uses the Array intrinsic itrator method.
    #[cfg(any(feature = "set", feature = "weak-refs"))]
    pub(crate) fn is_trivially_iterable(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> bool {
        use crate::ecmascript::abstract_operations::try_get_object_method;
        use crate::heap::WellKnownSymbols;
        if !self.is_dense(agent) {
            // Contains holes or getters, so cannot be iterated without looking
            // into the prototype chain or calling getters.
            false
        } else {
            let TryResult::Continue(Some(iterator_method)) = try_get_object_method(
                agent,
                self.into(),
                PropertyKey::Symbol(WellKnownSymbols::Iterator.into()),
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
                    .into()
        }
    }

    // This method creates a "shallow clone" of the elements of a simple array (no descriptors).
    // If array is not simple, this cloned array will do some odd things (e.g. getter/setter indexes become holes)
    pub(crate) fn to_cloned(self, agent: &mut Agent) -> Self {
        let Heap {
            arrays, elements, ..
        } = &mut agent.heap;
        let cloned_elements = elements.shallow_clone(self.get_elements(arrays));
        let data = ArrayHeapData {
            object_index: None,
            elements: cloned_elements,
        };
        agent.heap.create(data)
    }

    #[inline]
    pub(crate) fn as_slice(self, arena: &impl ArrayHeapAccess<'a>) -> &[Option<Value<'a>>] {
        let elvec = self.get_elements(arena);
        let elements: &ElementArrays = arena.as_ref();
        elements.get_values(elvec)
    }

    #[inline]
    pub(crate) fn as_mut_slice(self, agent: &mut Agent) -> &mut [Option<Value<'static>>] {
        let elvec = self.get_elements(&agent.heap.arrays);
        let elements = &mut agent.heap.elements;
        elements.get_values_mut(elvec)
    }

    pub(crate) fn get_storage(self, arena: &impl ArrayHeapAccess<'a>) -> ElementStorageRef<'_, 'a> {
        self.get_elements(arena).get_storage(arena)
    }

    pub(crate) fn get_storage_mut(self, agent: &mut Agent) -> ElementStorageMut<'_> {
        self.get_elements(&agent.heap.arrays)
            .get_storage_mut(&mut agent.heap.elements)
    }
}

impl<'a> InternalSlots<'a> for Array<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Array;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        let ArrayHeapDataMut {
            elements: elems,
            object_index: backing_object,
        } = self.get_mut(agent);
        elems.len_writable = value;
        if let Some(object_index) = backing_object {
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
                        .into(),
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
    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        _gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        self.internal_set_extensible(agent, false);
        TryResult::Continue(true)
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        let array = self.bind(gc);
        if let Some(index) = property_key.into_u32() {
            let elements = array.get_elements(agent);
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
                    descriptor, value,
                )))
            };
        }
        if property_key == BUILTIN_STRING_MEMORY.length.into() {
            let elems = array.get_elements(agent);
            TryResult::Continue(Some(PropertyDescriptor {
                value: Some(elems.len().into()),
                writable: Some(elems.len_writable),
                configurable: Some(false),
                enumerable: Some(false),
                ..Default::default()
            }))
        } else if let Some(backing_object) = array.get_backing_object(agent) {
            TryResult::Continue(
                ordinary_get_own_property(
                    agent,
                    self.into(),
                    backing_object,
                    property_key,
                    cache,
                    gc,
                )
                .bind(gc),
            )
        } else {
            TryResult::Continue(None)
        }
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            array_try_set_length(agent, self, property_descriptor, gc)
        } else if let Some(index) = property_key.into_u32() {
            // Let lengthDesc be OrdinaryGetOwnProperty(A, "length").
            // b. Assert: IsDataDescriptor(lengthDesc) is true.
            // c. Assert: lengthDesc.[[Configurable]] is false.
            // d. Let length be lengthDesc.[[Value]].
            let elements = self.get_elements(agent);
            let length = elements.len();
            let length_writable = elements.len_writable;
            // e. Assert: length is a non-negative integral Number.
            // f. Let index be ! ToUint32(P).
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
                let ArrayHeapDataMut {
                    elements: elems,
                    object_index: backing_object,
                } = self.get_mut(arrays);
                if elems.reserve(elements, index + 1).is_err() {
                    return TryError::GcError.into();
                }
                let mut value = property_descriptor.value;
                let element_descriptor =
                    ElementDescriptor::from_property_descriptor(property_descriptor);
                if element_descriptor.is_none_or(|d| d.is_data_descriptor()) {
                    value = Some(value.unwrap_or(Value::Undefined));
                }
                if index > length {
                    // Elements backing store should be filled with Nones already
                    elems.len = index;
                }
                // ii. Set succeeded to ! OrdinaryDefineOwnProperty(A, "length", lengthDesc).
                *alloc_counter += core::mem::size_of::<Option<Value>>();
                if element_descriptor.is_some() {
                    *alloc_counter += core::mem::size_of::<(u32, ElementDescriptor)>();
                }
                if let Err(err) = elems.push(elements, value, element_descriptor) {
                    return agent.throw_allocation_exception(err, gc).into();
                };
                // j. If index ≥ length, then
                // i. Set lengthDesc.[[Value]] to index + 1𝔽.
                // This should've already been handled by the push.
                debug_assert_eq!(elems.len(), index + 1);
                if let Some(shape) = backing_object.map(|o| o.object_shape(agent))
                    && shape.is_intrinsic(agent)
                {
                    // We set a value on an intrinsic object, we have to
                    // invalidate caches.
                    Caches::invalidate_caches_on_intrinsic_shape_property_addition(
                        agent,
                        self.into(),
                        shape,
                        index.into(),
                        u32::MAX,
                        gc,
                    );
                }
                // iii. Assert: succeeded is true.
                TryResult::Continue(true)
            } else {
                // h. Let succeeded be ! OrdinaryDefineOwnProperty(A, P, Desc).
                TryResult::Continue(ordinary_define_own_property_for_array(
                    agent,
                    self,
                    *elements,
                    index,
                    property_descriptor,
                    gc,
                ))
            }
        } else {
            let backing_object = self
                .get_backing_object(agent)
                .unwrap_or_else(|| self.create_backing_object(agent));
            js_result_into_try(ordinary_define_own_property(
                agent,
                self.into(),
                backing_object,
                property_key,
                property_descriptor,
                cache,
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
                None,
                gc.into_nogc(),
            )))
        }
    }

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        let array = self.bind(gc);
        if property_key == BUILTIN_STRING_MEMORY.length.into() {
            return TryHasResult::Custom(u32::MAX, array.into()).into();
        } else if let Some(index) = property_key.into_u32() {
            // Within possible Array bounds: the data is found in the Array
            // elements storage.
            let values = array.as_slice(agent);
            if index < values.len() as u32 {
                // Within the Array slice: first check values as checking
                // descriptors requires a hash calculation.
                if values[index as usize].is_some() {
                    return TryHasResult::Custom(index, array.into()).into();
                }
                // No value at this index; we have to check descriptors.
                let ElementStorageRef {
                    values: _,
                    descriptors,
                } = array.get_storage(agent);
                if let Some(d) = descriptors
                    && d.contains_key(&index)
                {
                    // Indeed, found a descriptor at this index. It must be an
                    // accessor, otherwise it should have a value as well.
                    debug_assert!(d.get(&index).unwrap().is_accessor_descriptor());
                    return TryHasResult::Custom(index, array.into()).into();
                }
            }
            // Overindexing, or no value or descriptor at this index: we have
            // to check the prototype chain.
        }
        ordinary_try_has_property(
            agent,
            array.into(),
            array.get_backing_object(agent),
            property_key,
            cache,
            gc,
        )
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        if property_key == BUILTIN_STRING_MEMORY.length.into() {
            return Ok(true);
        } else if let Some(index) = property_key.into_u32() {
            // Within possible Array bounds: the data is found in the Array
            // elements storage.
            let values = self.as_slice(agent);
            if index < values.len() as u32 {
                // Within the Array slice: first check values as checking
                // descriptors requires a hash calculation.
                if values[index as usize].is_some() {
                    return Ok(true);
                }
                // No value at this index; we have to check descriptors.
                let ElementStorageRef {
                    values: _,
                    descriptors,
                } = self.get_storage(agent);
                if let Some(d) = descriptors
                    && d.contains_key(&index)
                {
                    // Indeed, found a descriptor at this index. It must be an
                    // accessor, otherwise it should have a value as well.
                    debug_assert!(d.get(&index).unwrap().is_accessor_descriptor());
                    return Ok(true);
                }
            }
            // Overindexing, or no value or descriptor at this index: we have
            // to check the prototype chain.
        } else {
            // Looking up a property that would be stored in the backing
            // object; see if the backing object has something for us.
            if let Some(backing_object) = self.get_backing_object(agent) {
                // Note: this looks up in the prototype chain as well, so we
                // don't need to fall-through if this returns false or such.
                return ordinary_has_property(agent, self.into(), backing_object, property_key, gc);
            }
        }
        // Data is not found in the array or its backing object (or one does
        // not exist); we should look into the prototype chain.

        // 3. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = self.internal_prototype(agent);

        // 4. If parent is not null, then
        if let Some(parent) = parent {
            // a. Return ? parent.[[HasProperty]](P).
            return parent.internal_has_property(agent, property_key, gc);
        }

        // 5. Return false.
        Ok(false)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        let array = self.bind(gc);
        let property_key = property_key.bind(gc);
        let receiver = receiver.bind(gc);
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            return TryGetResult::Value(array.len(agent).into()).into();
        } else if let Some(index) = property_key.into_u32() {
            let values = array.as_slice(agent);
            if index < values.len() as u32 {
                // Index has been checked to be between 0 <= idx < len;
                // indexing should never fail.
                let element = values[index as usize];
                if let Some(element) = element {
                    return TryGetResult::Value(element).into();
                }
                // No value at this index; this might be a getter or setter.
                let ElementStorageRef { descriptors, .. } = array.get_storage(agent);
                if let Some(descriptors) = descriptors
                    && let Some(descriptor) = descriptors.get(&index)
                {
                    return if let Some(getter) = descriptor.getter_function(gc) {
                        // 7. Return ? Call(getter, Receiver).
                        // return call_function(agent, getter, receiver, None, gc);
                        TryGetResult::Get(getter).into()
                    } else {
                        // Accessor with no getter.
                        debug_assert!(descriptor.is_accessor_descriptor());
                        TryGetResult::Value(Value::Undefined).into()
                    };
                }
                // Hole! We must look into the prototype chain!
            }
        }
        ordinary_try_get(
            agent,
            self.into(),
            self.get_backing_object(agent),
            property_key,
            receiver,
            cache,
            gc,
        )
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let array = self.bind(nogc);
        let property_key = property_key.bind(nogc);
        let receiver = receiver.bind(nogc);
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            return Ok(array.len(agent).into());
        } else if let Some(index) = property_key.into_u32() {
            let values = array.as_slice(agent);
            if index < values.len() as u32 {
                // Index has been checked to be between 0 <= idx < len;
                // indexing should never fail.
                let element = values[index as usize];
                if let Some(element) = element {
                    return Ok(element.unbind());
                }
                // No value at this index; this might be a getter or setter.
                let ElementStorageRef { descriptors, .. } = array.get_storage(agent);
                if let Some(descriptors) = descriptors
                    && let Some(descriptor) = descriptors.get(&index)
                {
                    return if let Some(getter) = descriptor.getter_function(nogc) {
                        // 7. Return ? Call(getter, Receiver).
                        call_function(agent, getter.unbind(), receiver.unbind(), None, gc)
                    } else {
                        // Accessor with no getter.
                        debug_assert!(descriptor.is_accessor_descriptor());
                        Ok(Value::Undefined)
                    };
                }
                // Hole! We must look into the prototype chain!
            }
        } else {
            // Looking up a property that would be stored in the backing
            // object; see if the backing object has something for us.
            if let Some(backing_object) = array.get_backing_object(agent) {
                // Note: this looks up in the prototype chain as well, so we
                // don't need to fall-through if this returns false or such.
                return ordinary_get(
                    agent,
                    backing_object.unbind(),
                    property_key.unbind(),
                    receiver.unbind(),
                    gc,
                );
            }
        }
        // 3. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = array.internal_prototype(agent);

        // 4. If parent is not null, then
        if let Some(parent) = parent {
            // a. Return ? parent.[[HasProperty]](P).
            return parent.internal_get(agent, property_key.unbind(), receiver.unbind(), gc);
        }
        Ok(Value::Undefined)
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            TryResult::Continue(false)
        } else if let Some(index) = property_key.into_u32() {
            let elements = self.get_elements(&agent.heap.arrays);
            if index >= elements.len() {
                return TryResult::Continue(true);
            }
            let ElementStorageMut {
                values,
                descriptors,
            } = agent.heap.elements.get_element_storage_mut(elements);
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
            // Index has been checked to be between 0 <= idx < len;
            // indexing should never fail.
            values[index as usize] = None;
            TryResult::Continue(true)
        } else {
            TryResult::Continue(
                self.get_backing_object(agent)
                    .map(|backing_object| {
                        ordinary_delete(agent, self.into(), backing_object, property_key, gc)
                    })
                    .unwrap_or(true),
            )
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        let backing_keys = if let Some(backing_object) = self.get_backing_object(agent) {
            unwrap_try(backing_object.try_own_property_keys(agent, gc))
        } else {
            Default::default()
        };
        let ElementStorageRef {
            values,
            descriptors,
        } = self.get_elements(agent).get_storage(agent);
        let mut keys = Vec::with_capacity(values.len() + 1 + backing_keys.len());

        for (index, value) in values.iter().enumerate() {
            let index = index as u32;
            if value.is_some() || descriptors.is_some_and(|d| d.contains_key(&index)) {
                keys.push(index.into());
            }
        }

        keys.push(BUILTIN_STRING_MEMORY.length.to_property_key());
        keys.extend(backing_keys);

        TryResult::Continue(keys)
    }
}

impl<'a> CreateHeapData<ArrayHeapData<'a>, Array<'a>> for Heap {
    fn create(&mut self, data: ArrayHeapData<'a>) -> Array<'a> {
        let i = self.arrays.len();
        self.arrays
            .push(data.unbind())
            .expect("Failed to allocate Array");
        self.alloc_counter += core::mem::size_of::<ArrayHeapData<'static>>();
        Array(BaseIndex::from_index_u32(i))
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

/// Helper to invalidate property lookup caches associated with an index when
/// an intrinsic Array is mutated.
fn invalidate_array_index_caches(agent: &mut Agent, array: Array, index: u32, gc: NoGcScope) {
    if let Some(shape) = array
        .get_backing_object(agent)
        .map(|o| o.object_shape(agent))
        && shape.is_intrinsic(agent)
    {
        // We set a value on an intrinsic object, we have to
        // invalidate caches.
        Caches::invalidate_caches_on_intrinsic_shape_property_addition(
            agent,
            array.into(),
            shape,
            index.into(),
            u32::MAX,
            gc,
        );
    }
}

fn ordinary_define_own_property_for_array(
    agent: &mut Agent,
    array: Array,
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
            invalidate_array_index_caches(agent, array, index, gc);
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
            invalidate_array_index_caches(agent, array, index, gc);
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
    let current_is_data_descriptor = current_descriptor.is_none_or(|c| c.is_data_descriptor());
    let current_is_accessor_descriptor =
        current_descriptor.is_some_and(|c| c.is_accessor_descriptor());
    let current_getter = current_descriptor.and_then(|c| c.getter_function(gc));
    let current_setter = current_descriptor.and_then(|c| c.setter_function(gc));

    // 5. If current.[[Configurable]] is false, then
    if !current_configurable {
        // a. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
        if descriptor.configurable == Some(true) {
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
            // i. If Desc has a [[Get]] field and
            //    SameValue(Desc.[[Get]], current.[[Get]]) is false,
            if descriptor.get.is_some_and(|get| get != current_getter) {
                return false;
            }

            // ii. If Desc has a [[Set]] field and
            //     SameValue(Desc.[[Set]], current.[[Set]]) is false,
            if descriptor.set.is_some_and(|set| set != current_setter) {
                return false;
            }
        }
        // e. Else if current.[[Writable]] is false, then
        else if !current_writable.unwrap() {
            // i. If Desc has a [[Writable]] field and Desc.[[Writable]] is
            //    true,
            if descriptor.writable == Some(true) {
                return false;
            }

            // ii. If Desc has a [[Value]] field and
            //     SameValue(Desc.[[Value]], current.[[Value]]) is false,
            if descriptor
                .value
                .is_some_and(|value| !same_value(agent, value, current_value.unwrap()))
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
            descriptor.get.flatten(),
            descriptor.set.flatten(),
            enumerable,
            configurable,
        );
        insert_element_descriptor(agent, &elements, index, None, elem_descriptor);
        invalidate_array_index_caches(agent, array, index, gc);
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
        // i. For each field of Desc, set the corresponding attribute of the
        //    property named P of object O to the value of the field.
        let mut descriptor = descriptor;
        let result_value = descriptor.value.or(current_value);
        descriptor.writable = descriptor.writable.or(current_writable);
        descriptor.get = descriptor.get.or(current_getter.map(Some));
        descriptor.set = descriptor.set.or(current_setter.map(Some));
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
            let descs = descriptors.get_mut();
            descs.remove(&index);
            if descs.is_empty() {
                descriptors.remove();
            }
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
        let descs = descriptors.get_mut();
        descs.remove(&index);
        if descs.is_empty() {
            descriptors.remove();
        }
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
        agent.heap.elements.get_values_mut(elements)[index as usize] =
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
    elements: &'a mut ElementArrays,
    arrays: &'a mut SoAVec<ArrayHeapData<'static>>,
}

impl ArrayHeap<'_> {
    #[inline(always)]
    pub(crate) fn new<'a>(
        elements: &'a mut ElementArrays,
        arrays: &'a mut SoAVec<ArrayHeapData<'static>>,
    ) -> ArrayHeap<'a> {
        ArrayHeap { elements, arrays }
    }
}

impl AsRef<SoAVec<ArrayHeapData<'static>>> for ArrayHeap<'_> {
    #[inline(always)]
    fn as_ref(&self) -> &SoAVec<ArrayHeapData<'static>> {
        self.arrays
    }
}

impl AsRef<ElementArrays> for ArrayHeap<'_> {
    #[inline(always)]
    fn as_ref(&self) -> &ElementArrays {
        self.elements
    }
}

impl AsMut<SoAVec<ArrayHeapData<'static>>> for ArrayHeap<'_> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut SoAVec<ArrayHeapData<'static>> {
        self.arrays
    }
}

impl AsMut<ElementArrays> for ArrayHeap<'_> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut ElementArrays {
        self.elements
    }
}

/// Helper trait for array indexing.
pub(crate) trait ArrayHeapAccess<'a>:
    AsRef<SoAVec<ArrayHeapData<'static>>>
    + AsRef<ElementArrays>
    + AsMut<SoAVec<ArrayHeapData<'static>>>
    + AsMut<ElementArrays>
{
}
impl ArrayHeapAccess<'_> for ArrayHeap<'_> {}
impl ArrayHeapAccess<'_> for Agent {}
