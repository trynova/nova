// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod internal_methods;
mod internal_slots;
mod into_object;
mod property_key;
mod property_key_set;
mod property_key_vec;
mod property_storage;

use core::hash::Hash;
use std::{
    collections::{TryReserveError, hash_map::Entry},
    ops::ControlFlow,
};

#[cfg(feature = "date")]
use super::value::DATE_DISCRIMINANT;
#[cfg(feature = "proposal-float16array")]
use super::value::FLOAT_16_ARRAY_DISCRIMINANT;
#[cfg(feature = "regexp")]
use super::value::REGEXP_DISCRIMINANT;
#[cfg(feature = "shared-array-buffer")]
use super::value::SHARED_ARRAY_BUFFER_DISCRIMINANT;
#[cfg(feature = "array-buffer")]
use super::value::{
    ARRAY_BUFFER_DISCRIMINANT, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
    DATA_VIEW_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
    INT_8_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
    UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
    UINT_32_ARRAY_DISCRIMINANT,
};
#[cfg(feature = "weak-refs")]
use super::value::{WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT};
use super::{
    Function, Value,
    value::{
        ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
        ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT, ASYNC_GENERATOR_DISCRIMINANT,
        BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        ECMASCRIPT_FUNCTION_DISCRIMINANT, EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT,
        FINALIZATION_REGISTRY_DISCRIMINANT, GENERATOR_DISCRIMINANT, MAP_DISCRIMINANT,
        MAP_ITERATOR_DISCRIMINANT, MODULE_DISCRIMINANT, OBJECT_DISCRIMINANT,
        PRIMITIVE_OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT,
        STRING_ITERATOR_DISCRIMINANT,
    },
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExp;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
#[cfg(feature = "set")]
use crate::ecmascript::{
    builtins::{
        keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
    },
    types::{SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT},
};
#[cfg(feature = "array-buffer")]
use crate::{
    ecmascript::builtins::{ArrayBuffer, data_view::DataView, typed_array::TypedArray},
    engine::context::NoGcScope,
    heap::indexes::TypedArrayIndex,
};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::call_function,
        builtins::{
            ArgumentsList, Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
            async_generator_objects::AsyncGenerator,
            bound_function::BoundFunction,
            control_abstraction_objects::{
                generator_objects::Generator,
                promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction,
            },
            embedder_object::EmbedderObject,
            error::Error,
            finalization_registry::FinalizationRegistry,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator,
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIterator,
            map::Map,
            module::Module,
            ordinary::{
                caches::{PropertyLookupCache, PropertyOffset},
                ordinary_object_create_with_intrinsics,
                shape::{ObjectShape, ObjectShapeRecord},
            },
            primitive_objects::PrimitiveObject,
            promise::Promise,
            proxy::Proxy,
            text_processing::string_objects::string_iterator_objects::StringIterator,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{IntoValue, PropertyDescriptor},
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        ObjectEntry, WorkQueues,
        element_array::{
            ElementDescriptor, ElementStorageMut, ElementStorageRef, ElementStorageUninit,
            ElementsVector, PropertyStorageMut, PropertyStorageRef,
        },
        indexes::ObjectIndex,
    },
};

use ahash::AHashMap;
pub use data::ObjectHeapData;
pub use internal_methods::{
    GetCachedResult, InternalMethods, NoCache, SetCachedProps, SetCachedResult,
};
pub use internal_slots::InternalSlots;
pub use into_object::IntoObject;
pub use property_key::PropertyKey;
pub use property_key_set::PropertyKeySet;
pub(crate) use property_key_vec::ScopedPropertyKey;
pub use property_storage::PropertyStorage;

/// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
///
/// In Nova
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Object<'a> {
    Object(OrdinaryObject<'a>) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction<'a>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'a>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'a>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject<'a>) = PRIMITIVE_OBJECT_DISCRIMINANT,
    Arguments(OrdinaryObject<'a>) = ARGUMENTS_DISCRIMINANT,
    Array(Array<'a>) = ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'a>) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'a>) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "date")]
    Date(Date<'a>) = DATE_DISCRIMINANT,
    Error(Error<'a>) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistry<'a>) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(Map<'a>) = MAP_DISCRIMINANT,
    Promise(Promise<'a>) = PROMISE_DISCRIMINANT,
    Proxy(Proxy<'a>) = PROXY_DISCRIMINANT,
    #[cfg(feature = "regexp")]
    RegExp(RegExp<'a>) = REGEXP_DISCRIMINANT,
    #[cfg(feature = "set")]
    Set(Set<'a>) = SET_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'a>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'a>) = WEAK_MAP_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'a>) = WEAK_REF_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'a>) = WEAK_SET_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int8Array(TypedArrayIndex<'a>) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(TypedArrayIndex<'a>) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(TypedArrayIndex<'a>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(TypedArrayIndex<'a>) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(TypedArrayIndex<'a>) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(TypedArrayIndex<'a>) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(TypedArrayIndex<'a>) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(TypedArrayIndex<'a>) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(TypedArrayIndex<'a>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(TypedArrayIndex<'a>) = FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(TypedArrayIndex<'a>) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(TypedArrayIndex<'a>) = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncGenerator(AsyncGenerator<'a>) = ASYNC_GENERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator<'a>) = ARRAY_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'a>) = SET_ITERATOR_DISCRIMINANT,
    MapIterator(MapIterator<'a>) = MAP_ITERATOR_DISCRIMINANT,
    StringIterator(StringIterator<'a>) = STRING_ITERATOR_DISCRIMINANT,
    Generator(Generator<'a>) = GENERATOR_DISCRIMINANT,
    Module(Module<'a>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject<'a>) = EMBEDDER_OBJECT_DISCRIMINANT,
}

impl Object<'_> {
    /// Returns true if this Object is a Module.
    pub fn is_module(self) -> bool {
        matches!(self, Object::Module(_))
    }

    /// Returns true if this Object is a Proxy.
    pub fn is_proxy(self) -> bool {
        matches!(self, Object::Proxy(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrdinaryObject<'a>(pub(crate) ObjectIndex<'a>);

impl<'a> OrdinaryObject<'a> {
    pub(crate) const fn _def() -> Self {
        Self(ObjectIndex::from_u32_index(0))
    }
    pub(crate) const fn new(value: ObjectIndex<'a>) -> Self {
        Self(value)
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// Returns true if the Object has no properties.
    pub(crate) fn is_empty(self, agent: &Agent) -> bool {
        agent[self].is_empty()
    }

    /// Returns the number of properties in the object.
    pub(crate) fn len(self, agent: &Agent) -> u32 {
        agent[self].len()
    }

    pub(crate) fn reserve(self, agent: &mut Agent, new_len: u32) -> Result<(), TryReserveError> {
        agent.heap.objects[self].reserve(&mut agent.heap.elements, new_len)
    }

    pub fn create_empty_object(agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Self {
        let Object::Object(ordinary) =
            ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None, gc)
        else {
            unreachable!()
        };
        ordinary
    }

    /// Turn an OrdinaryObject's Object Shape into an intrinsic.
    ///
    /// For objects with an intrinsic shape, this is a no-op.
    pub(crate) fn make_intrinsic(self, agent: &mut Agent) {
        let shape = self.object_shape(agent);
        if shape.is_intrinsic(agent) {
            // Already an intrinsic shape, nothing to do.
            return;
        }
        let new_shape = shape.make_intrinsic(agent);
        agent[self].set_shape(new_shape);
    }

    pub(crate) unsafe fn try_set_property_by_offset<'gc>(
        self,
        agent: &mut Agent,
        offset: u16,
        value: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<Function<'gc>, bool> {
        let data = self.get_elements_storage_mut(agent);
        match data.descriptors {
            Entry::Occupied(e) => {
                let e = e.into_mut();
                let offset = offset as u32;
                let d = e.get(&offset);
                if let Some(d) = d
                    && !(d.is_data_descriptor() && d.is_writable().unwrap())
                {
                    // Either unwritable data descriptor, or an accessor
                    // descriptor.
                    if let Some(setter) = d.setter_function(gc) {
                        ControlFlow::Break(setter)
                    } else {
                        ControlFlow::Continue(false)
                    }
                } else {
                    data.values[offset as usize] = Some(value.unbind());
                    ControlFlow::Continue(true)
                }
            }
            Entry::Vacant(_) => {
                // No descriptors: pure WEC data properties.
                data.values[offset as usize] = Some(value.unbind());
                ControlFlow::Continue(true)
            }
        }
    }

    pub(crate) unsafe fn call_property_getter_by_offset<'gc>(
        self,
        agent: &mut Agent,
        offset: u16,
        this_value: Object,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let data = self.get_elements_storage(agent);
        debug_assert!(data.values[offset as usize].is_none());
        let getter = data
            .descriptors
            .and_then(|d| d.get(&(offset as u32)).unwrap().getter_function(gc.nogc()))
            .unwrap();
        call_function(agent, getter.unbind(), this_value.into_value(), None, gc)
    }

    pub(crate) fn get_property_storage<'b>(self, agent: &'b Agent) -> PropertyStorageRef<'b, 'a> {
        let Heap {
            object_shapes,
            elements,
            objects,
            ..
        } = &agent.heap;
        let data = &objects[self];
        let shape = data.get_shape();
        let keys = shape.keys(object_shapes, elements);
        let elements = data.get_storage(elements);
        debug_assert_eq!(keys.len(), elements.values.len());
        PropertyStorageRef::from_keys_and_elements(keys, elements)
    }

    pub(crate) fn get_property_storage_mut<'b>(
        self,
        agent: &'b mut Agent,
    ) -> Option<PropertyStorageMut<'b, 'a>> {
        let Heap {
            object_shapes,
            elements,
            objects,
            ..
        } = &mut agent.heap;
        let data = &objects[self];
        let shape = data.get_shape();
        elements.get_property_storage_mut_raw(
            shape.get_keys(object_shapes),
            shape.get_cap(object_shapes),
            data.get_values(),
            data.get_cap(),
            data.len(),
        )
    }

    pub(crate) fn get_elements_storage<'b>(self, agent: &'b Agent) -> ElementStorageRef<'b, 'a> {
        let Heap {
            elements, objects, ..
        } = &agent.heap;
        let data = &objects[self];
        elements.get_element_storage_raw(data.get_values(), data.get_cap(), data.len())
    }

    pub(crate) fn get_elements_storage_mut<'b>(
        self,
        agent: &'b mut Agent,
    ) -> ElementStorageMut<'b> {
        let Heap {
            elements, objects, ..
        } = &mut agent.heap;
        let data = &objects[self];
        elements.get_element_storage_mut_raw(data.get_values(), data.get_cap(), data.len())
    }

    pub(crate) fn get_elements_storage_uninit<'b>(
        self,
        agent: &'b mut Agent,
    ) -> ElementStorageUninit<'b> {
        let Heap {
            elements, objects, ..
        } = &mut agent.heap;
        let data = &objects[self];
        elements.get_element_storage_uninit_raw(data.get_values(), data.get_cap())
    }

    fn create_object_internal(
        agent: &mut Agent,
        shape: ObjectShape<'a>,
        entries: &[ObjectEntry<'a>],
    ) -> Self {
        let nontrivial_entry_count = entries.iter().filter(|p| !p.is_trivial()).count();
        agent.heap.alloc_counter += core::mem::size_of::<Option<Value>>() * entries.len()
            + if nontrivial_entry_count > 0 {
                core::mem::size_of::<Option<AHashMap<u32, ElementDescriptor<'static>>>>()
                    + core::mem::size_of::<(u32, ElementDescriptor<'static>)>()
                        * nontrivial_entry_count
            } else {
                0
            };
        let ElementsVector {
            elements_index: values,
            cap,
            len,
            len_writable: extensible,
        } = agent
            .heap
            .elements
            .allocate_object_property_storage_from_entries_slice(entries)
            .expect("Failed to create object");
        agent
            .heap
            .create(ObjectHeapData::new(shape, values, cap, len, extensible))
    }

    pub fn create_object(
        agent: &mut Agent,
        prototype: Option<Object<'a>>,
        entries: &[ObjectEntry<'a>],
    ) -> Self {
        let base_shape = ObjectShape::get_shape_for_prototype(agent, prototype);
        let mut shape = base_shape;
        for e in entries {
            shape = shape.get_child_shape(agent, e.key);
        }
        Self::create_object_internal(agent, shape, entries)
    }

    pub(crate) fn create_object_with_shape_and_data_properties(
        agent: &mut Agent,
        shape: ObjectShape<'a>,
        values: &[Value<'a>],
    ) -> Self {
        // SAFETY: Option<Value> uses a niche in Value enum at discriminant 0.
        let values = unsafe { core::mem::transmute::<&[Value<'a>], &[Option<Value<'a>>]>(values) };
        let ElementsVector {
            elements_index: values,
            cap,
            len,
            len_writable: extensible,
        } = agent
            .heap
            .elements
            .allocate_property_storage(values, None)
            .expect("Failed to create object");
        agent
            .heap
            .create(ObjectHeapData::new(shape, values, cap, len, extensible))
    }

    /// Creates a new "intrinsic" object. An intrinsic object owns its Object
    /// Shape uniquely and thus any changes to the object properties mutate the
    /// Shape directly.
    pub(crate) fn create_intrinsic_object(
        agent: &mut Agent,
        prototype: Option<Object<'a>>,
        entries: &[ObjectEntry<'a>],
    ) -> Self {
        let properties_count = entries.len();
        let (cap, index) = agent
            .heap
            .elements
            // Note: intrinsics should always allocate a keys storage.
            .allocate_keys_with_capacity(properties_count.max(1));
        let keys_memory = agent.heap.elements.get_keys_uninit_raw(cap, index);
        for (slot, key) in keys_memory.iter_mut().zip(entries.iter().map(|e| e.key)) {
            *slot = Some(key.unbind());
        }
        let shape = agent.heap.create(ObjectShapeRecord::create(
            prototype,
            index,
            cap,
            properties_count,
        ));
        Self::create_object_internal(agent, shape, entries)
    }

    /// Attempts to make this ordinary Object a copy of some source ordinary
    /// Object. This only succeeds if the current Object is empty, the
    /// prototypes of the two Objects match, and all own keys in the source
    /// Object are enumerable keys (descriptor is enumerable, key is not symbol
    /// or private), and the source Object contains no getters.
    ///
    /// Returns true if the copy succeeded. If the copy does not succeed, no
    /// changes to either object are performed and the function returns false.
    pub(crate) fn try_copy_from_object(self, agent: &mut Agent, source: OrdinaryObject) -> bool {
        if source.is_empty(agent) {
            // It's always possible to copy an empty object.
            return true;
        }
        if !self.is_empty(agent)
            || source.internal_prototype(agent) != self.internal_prototype(agent)
            || source.object_shape(agent).is_intrinsic(agent)
        {
            // Our own object is not empty, our prototypes don't match, or the
            // source object is an intrinsic object; can't perform the copy.
            // Note: for intrinsic objects the problem is that their Object
            // Shape is considered uniquely owned by the intrinsic object
            // itself.
            return false;
        }
        // Copying properties from one ordinary object to another and they
        // have the same prototype: we may be able to perform a trivial copy.
        let PropertyStorageRef {
            keys, descriptors, ..
        } = source.unbind().get_property_storage(agent);
        if descriptors.is_some_and(|d| d.iter().any(|(_, d)| !d.is_enumerable() || d.has_getter()))
            || keys.iter().any(|k| k.is_symbol() || k.is_private_name())
        {
            // Found a non-enumerable property, a getter, or a non-enumerable
            // key. Cannot perform the copy.
            return false;
        }
        // All properties in the source object are enumerable, none are
        // getters, and no key is a symbol or private name: the shape of the
        // source and the self objects will be identical after this operation.
        let len = keys.len() as u32;
        let source_shape = agent[source].get_shape();
        if self.reserve(agent, len).is_err() {
            return false;
        };
        let ElementStorageUninit {
            values: target_values,
            ..
        } = self.get_elements_storage_uninit(agent);
        // Descriptors are now set, then just copy the values over.
        let target_values = target_values as *mut [Option<Value<'static>>];
        let PropertyStorageRef {
            values: source_values,
            ..
        } = source.unbind().get_property_storage(agent);
        // SAFETY: self and source are two distinct objects (otherwise we'd
        // have returned early from self not being empty, or source being
        // empty). Thus, the values slices here are distinct from one another.
        let target_values = unsafe { &mut *target_values };
        target_values.copy_from_slice(source_values);
        agent[self].set_len(len);
        agent[self].set_shape(source_shape);
        true
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Object<'_> {
    type Of<'a> = Object<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for OrdinaryObject<'_> {
    type Of<'a> = OrdinaryObject<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<OrdinaryObject<'a>> for Object<'a> {
    fn from(value: OrdinaryObject<'a>) -> Self {
        Self::Object(value)
    }
}

impl<'a> From<ObjectIndex<'a>> for OrdinaryObject<'a> {
    fn from(value: ObjectIndex<'a>) -> Self {
        OrdinaryObject(value)
    }
}

impl<'a> From<OrdinaryObject<'a>> for Value<'a> {
    fn from(value: OrdinaryObject<'a>) -> Self {
        Self::Object(value)
    }
}

impl<'a> TryFrom<Value<'a>> for OrdinaryObject<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for OrdinaryObject<'a> {
    type Error = ();

    #[inline]
    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for OrdinaryObject<'a> {
    #[inline(always)]
    fn get_backing_object(self, _: &Agent) -> Option<OrdinaryObject<'static>> {
        Some(self.unbind())
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!();
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!();
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        agent[self].get_shape()
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        agent[self].get_extensible()
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        agent[self].set_extensible(value)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        agent[self].get_prototype(agent)
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        let original_shape = agent[self].get_shape();
        if original_shape.get_prototype(agent) == prototype {
            return;
        }
        let new_shape = original_shape.get_shape_with_prototype(agent, prototype);
        agent[self].set_shape(new_shape);
    }
}

impl<'a> From<ObjectIndex<'a>> for Object<'a> {
    fn from(value: ObjectIndex<'a>) -> Self {
        let value: OrdinaryObject<'a> = value.into();
        Object::Object(value.unbind())
    }
}

impl<'a> From<BoundFunction<'a>> for Object<'a> {
    fn from(value: BoundFunction) -> Self {
        Object::BoundFunction(value.unbind())
    }
}

impl<'a> From<Object<'a>> for Value<'a> {
    fn from(value: Object<'a>) -> Self {
        match value {
            Object::Object(data) => Value::Object(data.unbind()),
            Object::BoundFunction(data) => Value::BoundFunction(data.unbind()),
            Object::BuiltinFunction(data) => Value::BuiltinFunction(data.unbind()),
            Object::ECMAScriptFunction(data) => Value::ECMAScriptFunction(data.unbind()),
            Object::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            Object::BuiltinConstructorFunction(data) => {
                Value::BuiltinConstructorFunction(data.unbind())
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data.unbind())
            }
            Object::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data.unbind()),
            Object::Arguments(data) => Value::Arguments(data.unbind()),
            Object::Array(data) => Value::Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => Value::ArrayBuffer(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => Value::DataView(data.unbind()),
            #[cfg(feature = "date")]
            Object::Date(data) => Value::Date(data.unbind()),
            Object::Error(data) => Value::Error(data.unbind()),
            Object::FinalizationRegistry(data) => Value::FinalizationRegistry(data.unbind()),
            Object::Map(data) => Value::Map(data.unbind()),
            Object::Promise(data) => Value::Promise(data.unbind()),
            Object::Proxy(data) => Value::Proxy(data.unbind()),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => Value::RegExp(data.unbind()),
            #[cfg(feature = "set")]
            Object::Set(data) => Value::Set(data.unbind()),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => Value::SharedArrayBuffer(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => Value::WeakMap(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => Value::WeakRef(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => Value::WeakSet(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => Value::Int8Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => Value::Uint8Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => Value::Uint8ClampedArray(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => Value::Int16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => Value::Uint16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => Value::Int32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => Value::Uint32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => Value::BigInt64Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => Value::BigUint64Array(data.unbind()),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => Value::Float16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => Value::Float32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => Value::Float64Array(data.unbind()),
            Object::AsyncFromSyncIterator => Value::AsyncFromSyncIterator,
            Object::AsyncGenerator(data) => Value::AsyncGenerator(data),
            Object::ArrayIterator(data) => Value::ArrayIterator(data.unbind()),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => Value::SetIterator(data.unbind()),
            Object::MapIterator(data) => Value::MapIterator(data.unbind()),
            Object::StringIterator(data) => Value::StringIterator(data.unbind()),
            Object::Generator(data) => Value::Generator(data.unbind()),
            Object::Module(data) => Value::Module(data.unbind()),
            Object::EmbedderObject(data) => Value::EmbedderObject(data.unbind()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for Object<'a> {
    type Error = ();
    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        match value {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::String(_)
            | Value::SmallString(_)
            | Value::Symbol(_)
            | Value::Number(_)
            | Value::Integer(_)
            | Value::SmallF64(_)
            | Value::BigInt(_)
            | Value::SmallBigInt(_) => Err(()),
            Value::Object(x) => Ok(Object::from(x)),
            Value::Array(x) => Ok(Object::from(x)),
            #[cfg(feature = "date")]
            Value::Date(x) => Ok(Object::Date(x)),
            Value::Error(x) => Ok(Object::from(x)),
            Value::BoundFunction(x) => Ok(Object::from(x)),
            Value::BuiltinFunction(x) => Ok(Object::from(x)),
            Value::ECMAScriptFunction(x) => Ok(Object::from(x)),
            Value::BuiltinGeneratorFunction => Ok(Object::BuiltinGeneratorFunction),
            Value::BuiltinConstructorFunction(data) => Ok(Object::BuiltinConstructorFunction(data)),
            Value::BuiltinPromiseResolvingFunction(data) => {
                Ok(Object::BuiltinPromiseResolvingFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Object::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Object::BuiltinProxyRevokerFunction),
            Value::PrimitiveObject(data) => Ok(Object::PrimitiveObject(data)),
            Value::Arguments(data) => Ok(Object::Arguments(data)),
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(idx) => Ok(Object::ArrayBuffer(idx)),
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => Ok(Object::DataView(data)),
            Value::FinalizationRegistry(data) => Ok(Object::FinalizationRegistry(data)),
            Value::Map(data) => Ok(Object::Map(data)),
            Value::Promise(data) => Ok(Object::Promise(data)),
            Value::Proxy(data) => Ok(Object::Proxy(data)),
            #[cfg(feature = "regexp")]
            Value::RegExp(idx) => Ok(Object::RegExp(idx)),
            #[cfg(feature = "set")]
            Value::Set(data) => Ok(Object::Set(data)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => Ok(Object::SharedArrayBuffer(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => Ok(Object::WeakMap(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => Ok(Object::WeakRef(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => Ok(Object::WeakSet(data)),
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => Ok(Object::Int8Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => Ok(Object::Uint8Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => Ok(Object::Uint8ClampedArray(data)),
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => Ok(Object::Int16Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => Ok(Object::Uint16Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => Ok(Object::Int32Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => Ok(Object::Uint32Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => Ok(Object::BigInt64Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => Ok(Object::BigUint64Array(data)),
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(data) => Ok(Object::Float16Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => Ok(Object::Float32Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => Ok(Object::Float64Array(data)),
            Value::AsyncFromSyncIterator => Ok(Object::AsyncFromSyncIterator),
            Value::AsyncGenerator(data) => Ok(Object::AsyncGenerator(data)),
            Value::ArrayIterator(data) => Ok(Object::ArrayIterator(data)),
            #[cfg(feature = "set")]
            Value::SetIterator(data) => Ok(Object::SetIterator(data)),
            Value::MapIterator(data) => Ok(Object::MapIterator(data)),
            Value::StringIterator(data) => Ok(Object::StringIterator(data)),
            Value::Generator(data) => Ok(Object::Generator(data)),
            Value::Module(data) => Ok(Object::Module(data)),
            Value::EmbedderObject(data) => Ok(Object::EmbedderObject(data)),
        }
    }
}

impl<'a> OrdinaryObject<'a> {
    pub fn property_storage(self) -> PropertyStorage<'a> {
        PropertyStorage::new(self)
    }
}

impl Hash for Object<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Object::Object(data) => data.get_index().hash(state),
            Object::BoundFunction(data) => data.get_index().hash(state),
            Object::BuiltinFunction(data) => data.get_index().hash(state),
            Object::ECMAScriptFunction(data) => data.get_index().hash(state),
            Object::BuiltinGeneratorFunction => {}
            Object::BuiltinConstructorFunction(data) => data.get_index().hash(state),
            Object::BuiltinPromiseResolvingFunction(data) => data.get_index().hash(state),
            Object::BuiltinPromiseCollectorFunction => {}
            Object::BuiltinProxyRevokerFunction => {}
            Object::PrimitiveObject(data) => data.get_index().hash(state),
            Object::Arguments(data) => data.get_index().hash(state),
            Object::Array(data) => data.get_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.get_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.get_index().hash(state),
            #[cfg(feature = "date")]
            Object::Date(data) => data.get_index().hash(state),
            Object::Error(data) => data.get_index().hash(state),
            Object::FinalizationRegistry(data) => data.get_index().hash(state),
            Object::Map(data) => data.get_index().hash(state),
            Object::Promise(data) => data.get_index().hash(state),
            Object::Proxy(data) => data.get_index().hash(state),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.get_index().hash(state),
            #[cfg(feature = "set")]
            Object::Set(data) => data.get_index().hash(state),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.get_index().hash(state),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.get_index().hash(state),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.get_index().hash(state),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.get_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => data.into_index().hash(state),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => data.into_index().hash(state),
            Object::AsyncFromSyncIterator => {}
            Object::AsyncGenerator(data) => data.get_index().hash(state),
            Object::ArrayIterator(data) => data.get_index().hash(state),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.get_index().hash(state),
            Object::MapIterator(data) => data.get_index().hash(state),
            Object::StringIterator(data) => data.get_index().hash(state),
            Object::Generator(data) => data.get_index().hash(state),
            Object::Module(data) => data.get_index().hash(state),
            Object::EmbedderObject(data) => data.get_index().hash(state),
        }
    }
}

impl<'a> InternalSlots<'a> for Object<'a> {
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        match self {
            Object::Object(data) => data.get_backing_object(agent),
            Object::Array(data) => data.get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.get_backing_object(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.get_backing_object(agent),
            Object::Error(data) => data.get_backing_object(agent),
            Object::BoundFunction(data) => data.get_backing_object(agent),
            Object::BuiltinFunction(data) => data.get_backing_object(agent),
            Object::ECMAScriptFunction(data) => data.get_backing_object(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.get_backing_object(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.get_backing_object(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.get_backing_object(agent),
            Object::Arguments(data) => data.get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.get_backing_object(agent),
            Object::FinalizationRegistry(data) => data.get_backing_object(agent),
            Object::Map(data) => data.get_backing_object(agent),
            Object::Promise(data) => data.get_backing_object(agent),
            Object::Proxy(data) => data.get_backing_object(agent),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.get_backing_object(agent),
            #[cfg(feature = "set")]
            Object::Set(data) => data.get_backing_object(agent),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.get_backing_object(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.get_backing_object(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.get_backing_object(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).get_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).get_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).get_backing_object(agent)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).get_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).get_backing_object(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.get_backing_object(agent),
            Object::ArrayIterator(data) => data.get_backing_object(agent),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.get_backing_object(agent),
            Object::MapIterator(data) => data.get_backing_object(agent),
            Object::StringIterator(data) => data.get_backing_object(agent),
            Object::Generator(data) => data.get_backing_object(agent),
            Object::Module(data) => data.get_backing_object(agent),
            Object::EmbedderObject(data) => data.get_backing_object(agent),
        }
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("Object should not try to set its backing object");
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!("Object should not try to create its backing object");
    }

    fn get_or_create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        match self {
            Object::Object(data) => data.get_or_create_backing_object(agent),
            Object::Array(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.get_or_create_backing_object(agent),
            Object::Error(data) => data.get_or_create_backing_object(agent),
            Object::BoundFunction(data) => data.get_or_create_backing_object(agent),
            Object::BuiltinFunction(data) => data.get_or_create_backing_object(agent),
            Object::ECMAScriptFunction(data) => data.get_or_create_backing_object(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.get_or_create_backing_object(agent),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.get_or_create_backing_object(agent)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.get_or_create_backing_object(agent),
            Object::Arguments(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.get_or_create_backing_object(agent),
            Object::FinalizationRegistry(data) => data.get_or_create_backing_object(agent),
            Object::Map(data) => data.get_or_create_backing_object(agent),
            Object::Promise(data) => data.get_or_create_backing_object(agent),
            Object::Proxy(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "set")]
            Object::Set(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).get_or_create_backing_object(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).get_or_create_backing_object(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.get_or_create_backing_object(agent),
            Object::ArrayIterator(data) => data.get_or_create_backing_object(agent),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.get_or_create_backing_object(agent),
            Object::MapIterator(data) => data.get_or_create_backing_object(agent),
            Object::StringIterator(data) => data.get_or_create_backing_object(agent),
            Object::Generator(data) => data.get_or_create_backing_object(agent),
            Object::Module(data) => data.get_or_create_backing_object(agent),
            Object::EmbedderObject(data) => data.get_or_create_backing_object(agent),
        }
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        match self {
            Object::Object(data) => data.object_shape(agent),
            Object::Array(data) => data.object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.object_shape(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.object_shape(agent),
            Object::Error(data) => data.object_shape(agent),
            Object::BoundFunction(data) => data.object_shape(agent),
            Object::BuiltinFunction(data) => data.object_shape(agent),
            Object::ECMAScriptFunction(data) => data.object_shape(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.object_shape(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.object_shape(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.object_shape(agent),
            Object::Arguments(data) => data.object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.object_shape(agent),
            Object::FinalizationRegistry(data) => data.object_shape(agent),
            Object::Map(data) => data.object_shape(agent),
            Object::Promise(data) => data.object_shape(agent),
            Object::Proxy(data) => data.object_shape(agent),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.object_shape(agent),
            #[cfg(feature = "set")]
            Object::Set(data) => data.object_shape(agent),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.object_shape(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.object_shape(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.object_shape(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).object_shape(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data).object_shape(agent),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).object_shape(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).object_shape(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.object_shape(agent),
            Object::ArrayIterator(data) => data.object_shape(agent),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.object_shape(agent),
            Object::MapIterator(data) => data.object_shape(agent),
            Object::StringIterator(data) => data.object_shape(agent),
            Object::Generator(data) => data.object_shape(agent),
            Object::Module(data) => data.object_shape(agent),
            Object::EmbedderObject(data) => data.object_shape(agent),
        }
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        match self {
            Object::Object(data) => data.internal_extensible(agent),
            Object::Array(data) => data.internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_extensible(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_extensible(agent),
            Object::Error(data) => data.internal_extensible(agent),
            Object::BoundFunction(data) => data.internal_extensible(agent),
            Object::BuiltinFunction(data) => data.internal_extensible(agent),
            Object::ECMAScriptFunction(data) => data.internal_extensible(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_extensible(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_extensible(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_extensible(agent),
            Object::Arguments(data) => data.internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_extensible(agent),
            Object::FinalizationRegistry(data) => data.internal_extensible(agent),
            Object::Map(data) => data.internal_extensible(agent),
            Object::Promise(data) => data.internal_extensible(agent),
            Object::Proxy(data) => data.internal_extensible(agent),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_extensible(agent),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_extensible(agent),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_extensible(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_extensible(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_extensible(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_extensible(agent)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_extensible(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_extensible(agent),
            Object::ArrayIterator(data) => data.internal_extensible(agent),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_extensible(agent),
            Object::MapIterator(data) => data.internal_extensible(agent),
            Object::StringIterator(data) => data.internal_extensible(agent),
            Object::Generator(data) => data.internal_extensible(agent),
            Object::Module(data) => data.internal_extensible(agent),
            Object::EmbedderObject(data) => data.internal_extensible(agent),
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        match self {
            Object::Object(data) => data.internal_set_extensible(agent, value),
            Object::Array(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set_extensible(agent, value),
            Object::Error(data) => data.internal_set_extensible(agent, value),
            Object::BoundFunction(data) => data.internal_set_extensible(agent, value),
            Object::BuiltinFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::ECMAScriptFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_extensible(agent, value)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_extensible(agent, value),
            Object::Arguments(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set_extensible(agent, value),
            Object::FinalizationRegistry(data) => data.internal_set_extensible(agent, value),
            Object::Map(data) => data.internal_set_extensible(agent, value),
            Object::Promise(data) => data.internal_set_extensible(agent, value),
            Object::Proxy(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_extensible(agent, value)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_set_extensible(agent, value),
            Object::ArrayIterator(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_set_extensible(agent, value),
            Object::MapIterator(data) => data.internal_set_extensible(agent, value),
            Object::Generator(data) => data.internal_set_extensible(agent, value),
            Object::StringIterator(data) => data.internal_set_extensible(agent, value),
            Object::Module(data) => data.internal_set_extensible(agent, value),
            Object::EmbedderObject(data) => data.internal_set_extensible(agent, value),
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        match self {
            Object::Object(data) => data.internal_prototype(agent),
            Object::Array(data) => data.internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_prototype(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_prototype(agent),
            Object::Error(data) => data.internal_prototype(agent),
            Object::BoundFunction(data) => data.internal_prototype(agent),
            Object::BuiltinFunction(data) => data.internal_prototype(agent),
            Object::ECMAScriptFunction(data) => data.internal_prototype(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_prototype(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_prototype(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_prototype(agent),
            Object::Arguments(data) => data.internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_prototype(agent),
            Object::FinalizationRegistry(data) => data.internal_prototype(agent),
            Object::Map(data) => data.internal_prototype(agent),
            Object::Promise(data) => data.internal_prototype(agent),
            Object::Proxy(data) => data.internal_prototype(agent),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_prototype(agent),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_prototype(agent),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_prototype(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_prototype(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_prototype(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_prototype(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_prototype(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_prototype(agent)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_prototype(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_prototype(agent),
            Object::ArrayIterator(data) => data.internal_prototype(agent),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_prototype(agent),
            Object::MapIterator(data) => data.internal_prototype(agent),
            Object::StringIterator(data) => data.internal_prototype(agent),
            Object::Generator(data) => data.internal_prototype(agent),
            Object::Module(data) => data.internal_prototype(agent),
            Object::EmbedderObject(data) => data.internal_prototype(agent),
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        match self {
            Object::Object(data) => data.internal_set_prototype(agent, prototype),
            Object::Array(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set_prototype(agent, prototype),
            Object::Error(data) => data.internal_set_prototype(agent, prototype),
            Object::BoundFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::BuiltinFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::ECMAScriptFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set_prototype(agent, prototype)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_prototype(agent, prototype)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_prototype(agent, prototype),
            Object::Arguments(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set_prototype(agent, prototype),
            Object::FinalizationRegistry(data) => data.internal_set_prototype(agent, prototype),
            Object::Map(data) => data.internal_set_prototype(agent, prototype),
            Object::Promise(data) => data.internal_set_prototype(agent, prototype),
            Object::Proxy(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_prototype(agent, prototype)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_set_prototype(agent, prototype),
            Object::ArrayIterator(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::MapIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::StringIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::Generator(data) => data.internal_set_prototype(agent, prototype),
            Object::Module(data) => data.internal_set_prototype(agent, prototype),
            Object::EmbedderObject(data) => data.internal_set_prototype(agent, prototype),
        }
    }
}

impl<'a> InternalMethods<'a> for Object<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<Object<'gc>>> {
        match self {
            Object::Object(data) => data.try_get_prototype_of(agent, gc),
            Object::Array(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_get_prototype_of(agent, gc),
            Object::Error(data) => data.try_get_prototype_of(agent, gc),
            Object::BoundFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_get_prototype_of(agent, gc),
            Object::Arguments(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_get_prototype_of(agent, gc),
            Object::FinalizationRegistry(data) => data.try_get_prototype_of(agent, gc),
            Object::Map(data) => data.try_get_prototype_of(agent, gc),
            Object::Promise(data) => data.try_get_prototype_of(agent, gc),
            Object::Proxy(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_get_prototype_of(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_get_prototype_of(agent, gc),
            Object::ArrayIterator(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_get_prototype_of(agent, gc),
            Object::MapIterator(data) => data.try_get_prototype_of(agent, gc),
            Object::StringIterator(data) => data.try_get_prototype_of(agent, gc),
            Object::Generator(data) => data.try_get_prototype_of(agent, gc),
            Object::Module(data) => data.try_get_prototype_of(agent, gc),
            Object::EmbedderObject(data) => data.try_get_prototype_of(agent, gc),
        }
    }

    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Object<'gc>>> {
        match self {
            Object::Object(data) => data.internal_get_prototype_of(agent, gc),
            Object::Array(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get_prototype_of(agent, gc),
            Object::Error(data) => data.internal_get_prototype_of(agent, gc),
            Object::BoundFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::BuiltinFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get_prototype_of(agent, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get_prototype_of(agent, gc),
            Object::Arguments(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get_prototype_of(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_get_prototype_of(agent, gc),
            Object::Map(data) => data.internal_get_prototype_of(agent, gc),
            Object::Promise(data) => data.internal_get_prototype_of(agent, gc),
            Object::Proxy(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_prototype_of(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_get_prototype_of(agent, gc),
            Object::ArrayIterator(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_get_prototype_of(agent, gc),
            Object::MapIterator(data) => data.internal_get_prototype_of(agent, gc),
            Object::StringIterator(data) => data.internal_get_prototype_of(agent, gc),
            Object::Generator(data) => data.internal_get_prototype_of(agent, gc),
            Object::Module(data) => data.internal_get_prototype_of(agent, gc),
            Object::EmbedderObject(data) => data.internal_get_prototype_of(agent, gc),
        }
    }

    fn try_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Array(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Error(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::BoundFunction(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::BuiltinFunction(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::ECMAScriptFunction(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Arguments(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::FinalizationRegistry(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Map(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Promise(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Proxy(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::ArrayIterator(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::MapIterator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::StringIterator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Generator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Module(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::EmbedderObject(data) => data.try_set_prototype_of(agent, prototype, gc),
        }
    }

    fn internal_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Object::Object(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Array(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Error(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::BoundFunction(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::BuiltinFunction(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::ECMAScriptFunction(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Arguments(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::Map(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Promise(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Proxy(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::ArrayIterator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::MapIterator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::StringIterator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Generator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Module(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::EmbedderObject(data) => data.internal_set_prototype_of(agent, prototype, gc),
        }
    }

    fn try_is_extensible(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_is_extensible(agent, gc),
            Object::Array(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_is_extensible(agent, gc),
            Object::Error(data) => data.try_is_extensible(agent, gc),
            Object::BoundFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinFunction(data) => data.try_is_extensible(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_is_extensible(agent, gc),
            Object::Arguments(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_is_extensible(agent, gc),
            Object::FinalizationRegistry(data) => data.try_is_extensible(agent, gc),
            Object::Map(data) => data.try_is_extensible(agent, gc),
            Object::Promise(data) => data.try_is_extensible(agent, gc),
            Object::Proxy(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_is_extensible(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_is_extensible(agent, gc),
            Object::ArrayIterator(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_is_extensible(agent, gc),
            Object::MapIterator(data) => data.try_is_extensible(agent, gc),
            Object::StringIterator(data) => data.try_is_extensible(agent, gc),
            Object::Generator(data) => data.try_is_extensible(agent, gc),
            Object::Module(data) => data.try_is_extensible(agent, gc),
            Object::EmbedderObject(data) => data.try_is_extensible(agent, gc),
        }
    }

    fn internal_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Object::Object(data) => data.internal_is_extensible(agent, gc),
            Object::Array(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_is_extensible(agent, gc),
            Object::Error(data) => data.internal_is_extensible(agent, gc),
            Object::BoundFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinFunction(data) => data.internal_is_extensible(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_is_extensible(agent, gc),
            Object::Arguments(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_is_extensible(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_is_extensible(agent, gc),
            Object::Map(data) => data.internal_is_extensible(agent, gc),
            Object::Promise(data) => data.internal_is_extensible(agent, gc),
            Object::Proxy(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_is_extensible(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_is_extensible(agent, gc),
            Object::ArrayIterator(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_is_extensible(agent, gc),
            Object::MapIterator(data) => data.internal_is_extensible(agent, gc),
            Object::StringIterator(data) => data.internal_is_extensible(agent, gc),
            Object::Generator(data) => data.internal_is_extensible(agent, gc),
            Object::Module(data) => data.internal_is_extensible(agent, gc),
            Object::EmbedderObject(data) => data.internal_is_extensible(agent, gc),
        }
    }

    fn try_prevent_extensions(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_prevent_extensions(agent, gc),
            Object::Array(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_prevent_extensions(agent, gc),
            Object::Error(data) => data.try_prevent_extensions(agent, gc),
            Object::BoundFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_prevent_extensions(agent, gc),
            Object::Arguments(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_prevent_extensions(agent, gc),
            Object::FinalizationRegistry(data) => data.try_prevent_extensions(agent, gc),
            Object::Map(data) => data.try_prevent_extensions(agent, gc),
            Object::Promise(data) => data.try_prevent_extensions(agent, gc),
            Object::Proxy(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_prevent_extensions(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_prevent_extensions(agent, gc),
            Object::ArrayIterator(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_prevent_extensions(agent, gc),
            Object::MapIterator(data) => data.try_prevent_extensions(agent, gc),
            Object::StringIterator(data) => data.try_prevent_extensions(agent, gc),
            Object::Generator(data) => data.try_prevent_extensions(agent, gc),
            Object::Module(data) => data.try_prevent_extensions(agent, gc),
            Object::EmbedderObject(data) => data.try_prevent_extensions(agent, gc),
        }
    }

    fn internal_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Object::Object(data) => data.internal_prevent_extensions(agent, gc),
            Object::Array(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_prevent_extensions(agent, gc),
            Object::Error(data) => data.internal_prevent_extensions(agent, gc),
            Object::BoundFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::BuiltinFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_prevent_extensions(agent, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_prevent_extensions(agent, gc),
            Object::Arguments(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_prevent_extensions(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_prevent_extensions(agent, gc),
            Object::Map(data) => data.internal_prevent_extensions(agent, gc),
            Object::Promise(data) => data.internal_prevent_extensions(agent, gc),
            Object::Proxy(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_prevent_extensions(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_prevent_extensions(agent, gc),
            Object::ArrayIterator(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_prevent_extensions(agent, gc),
            Object::MapIterator(data) => data.internal_prevent_extensions(agent, gc),
            Object::StringIterator(data) => data.internal_prevent_extensions(agent, gc),
            Object::Generator(data) => data.internal_prevent_extensions(agent, gc),
            Object::Module(data) => data.internal_prevent_extensions(agent, gc),
            Object::EmbedderObject(data) => data.internal_prevent_extensions(agent, gc),
        }
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        match self {
            Object::Object(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Array(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Error(data) => data.try_get_own_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.try_get_own_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.try_get_own_property(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.try_get_own_property(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Arguments(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_get_own_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => {
                data.try_get_own_property(agent, property_key, gc)
            }
            Object::Map(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Promise(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Proxy(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_get_own_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::ArrayIterator(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::MapIterator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::StringIterator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Generator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Module(data) => data.try_get_own_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.try_get_own_property(agent, property_key, gc),
        }
    }

    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
        match self {
            Object::Object(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Array(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Error(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::Arguments(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::Map(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Promise(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Proxy(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_own_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::ArrayIterator(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::MapIterator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::StringIterator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Generator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Module(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.internal_get_own_property(agent, property_key, gc),
        }
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Array(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "date")]
            Object::Date(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Error(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BoundFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Arguments(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::FinalizationRegistry(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Map(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Promise(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Proxy(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::Set(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .try_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data)
                .try_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::ArrayIterator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::SetIterator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::MapIterator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::StringIterator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Generator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Module(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::EmbedderObject(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
        }
    }

    fn internal_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Object::Object(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Array(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "date")]
            Object::Date(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Error(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BoundFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Arguments(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::FinalizationRegistry(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Map(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Promise(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Proxy(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::Set(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::ArrayIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::SetIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::MapIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::StringIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Generator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Module(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::EmbedderObject(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_has_property(agent, property_key, gc),
            Object::Array(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_has_property(agent, property_key, gc),
            Object::Error(data) => data.try_has_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.try_has_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.try_has_property(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.try_has_property(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_has_property(agent, property_key, gc),
            Object::Arguments(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_has_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => data.try_has_property(agent, property_key, gc),
            Object::Map(data) => data.try_has_property(agent, property_key, gc),
            Object::Promise(data) => data.try_has_property(agent, property_key, gc),
            Object::Proxy(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_has_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_has_property(agent, property_key, gc),
            Object::ArrayIterator(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_has_property(agent, property_key, gc),
            Object::MapIterator(data) => data.try_has_property(agent, property_key, gc),
            Object::StringIterator(data) => data.try_has_property(agent, property_key, gc),
            Object::Generator(data) => data.try_has_property(agent, property_key, gc),
            Object::Module(data) => data.try_has_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.try_has_property(agent, property_key, gc),
        }
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Object::Object(data) => data.internal_has_property(agent, property_key, gc),
            Object::Array(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_has_property(agent, property_key, gc),
            Object::Error(data) => data.internal_has_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.internal_has_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.internal_has_property(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.internal_has_property(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_has_property(agent, property_key, gc),
            Object::Arguments(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_has_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_has_property(agent, property_key, gc)
            }
            Object::Map(data) => data.internal_has_property(agent, property_key, gc),
            Object::Promise(data) => data.internal_has_property(agent, property_key, gc),
            Object::Proxy(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_has_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_has_property(agent, property_key, gc),
            Object::ArrayIterator(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_has_property(agent, property_key, gc),
            Object::MapIterator(data) => data.internal_has_property(agent, property_key, gc),
            Object::StringIterator(data) => data.internal_has_property(agent, property_key, gc),
            Object::Generator(data) => data.internal_has_property(agent, property_key, gc),
            Object::Module(data) => data.internal_has_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.internal_has_property(agent, property_key, gc),
        }
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
        match self {
            Object::Object(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Array(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Error(data) => data.try_get(agent, property_key, receiver, gc),
            Object::BoundFunction(data) => data.try_get(agent, property_key, receiver, gc),
            Object::BuiltinFunction(data) => data.try_get(agent, property_key, receiver, gc),
            Object::ECMAScriptFunction(data) => data.try_get(agent, property_key, receiver, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Arguments(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_get(agent, property_key, receiver, gc),
            Object::FinalizationRegistry(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Map(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Promise(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Proxy(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_get(agent, property_key, receiver, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::ArrayIterator(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::MapIterator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::StringIterator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Generator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Module(data) => data.try_get(agent, property_key, receiver, gc),
            Object::EmbedderObject(data) => data.try_get(agent, property_key, receiver, gc),
        }
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        match self {
            Object::Object(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Array(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Error(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::BoundFunction(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::BuiltinFunction(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::ECMAScriptFunction(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Arguments(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::Map(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Promise(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Proxy(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get(agent, property_key, receiver, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::ArrayIterator(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::MapIterator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::StringIterator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Generator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Module(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::EmbedderObject(data) => data.internal_get(agent, property_key, receiver, gc),
        }
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Array(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Error(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::BoundFunction(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::BuiltinFunction(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::ECMAScriptFunction(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Arguments(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::FinalizationRegistry(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::Map(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Promise(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Proxy(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data).try_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::ArrayIterator(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::MapIterator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::StringIterator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Generator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Module(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::EmbedderObject(data) => data.try_set(agent, property_key, value, receiver, gc),
        }
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Object::Object(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Array(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Error(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::BoundFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::Arguments(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::Map(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Promise(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Proxy(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::ArrayIterator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "set")]
            Object::SetIterator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::MapIterator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::StringIterator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::Generator(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Module(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::EmbedderObject(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
        }
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_delete(agent, property_key, gc),
            Object::Array(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_delete(agent, property_key, gc),
            Object::Error(data) => data.try_delete(agent, property_key, gc),
            Object::BoundFunction(data) => data.try_delete(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.try_delete(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.try_delete(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_delete(agent, property_key, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_delete(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_delete(agent, property_key, gc),
            Object::Arguments(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_delete(agent, property_key, gc),
            Object::FinalizationRegistry(data) => data.try_delete(agent, property_key, gc),
            Object::Map(data) => data.try_delete(agent, property_key, gc),
            Object::Promise(data) => data.try_delete(agent, property_key, gc),
            Object::Proxy(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_delete(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_delete(agent, property_key, gc),
            Object::ArrayIterator(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_delete(agent, property_key, gc),
            Object::MapIterator(data) => data.try_delete(agent, property_key, gc),
            Object::StringIterator(data) => data.try_delete(agent, property_key, gc),
            Object::Generator(data) => data.try_delete(agent, property_key, gc),
            Object::Module(data) => data.try_delete(agent, property_key, gc),
            Object::EmbedderObject(data) => data.try_delete(agent, property_key, gc),
        }
    }

    fn internal_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Object::Object(data) => data.internal_delete(agent, property_key, gc),
            Object::Array(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_delete(agent, property_key, gc),
            Object::Error(data) => data.internal_delete(agent, property_key, gc),
            Object::BoundFunction(data) => data.internal_delete(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.internal_delete(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.internal_delete(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_delete(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_delete(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_delete(agent, property_key, gc),
            Object::Arguments(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_delete(agent, property_key, gc),
            Object::FinalizationRegistry(data) => data.internal_delete(agent, property_key, gc),
            Object::Map(data) => data.internal_delete(agent, property_key, gc),
            Object::Promise(data) => data.internal_delete(agent, property_key, gc),
            Object::Proxy(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_delete(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_delete(agent, property_key, gc),
            Object::ArrayIterator(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_delete(agent, property_key, gc),
            Object::MapIterator(data) => data.internal_delete(agent, property_key, gc),
            Object::StringIterator(data) => data.internal_delete(agent, property_key, gc),
            Object::Generator(data) => data.internal_delete(agent, property_key, gc),
            Object::Module(data) => data.internal_delete(agent, property_key, gc),
            Object::EmbedderObject(data) => data.internal_delete(agent, property_key, gc),
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        match self {
            Object::Object(data) => data.try_own_property_keys(agent, gc),
            Object::Array(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_own_property_keys(agent, gc),
            Object::Error(data) => data.try_own_property_keys(agent, gc),
            Object::BoundFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinFunction(data) => data.try_own_property_keys(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_own_property_keys(agent, gc),
            Object::Arguments(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_own_property_keys(agent, gc),
            Object::FinalizationRegistry(data) => data.try_own_property_keys(agent, gc),
            Object::Map(data) => data.try_own_property_keys(agent, gc),
            Object::Promise(data) => data.try_own_property_keys(agent, gc),
            Object::Proxy(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_own_property_keys(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.try_own_property_keys(agent, gc),
            Object::ArrayIterator(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_own_property_keys(agent, gc),
            Object::MapIterator(data) => data.try_own_property_keys(agent, gc),
            Object::StringIterator(data) => data.try_own_property_keys(agent, gc),
            Object::Generator(data) => data.try_own_property_keys(agent, gc),
            Object::Module(data) => data.try_own_property_keys(agent, gc),
            Object::EmbedderObject(data) => data.try_own_property_keys(agent, gc),
        }
    }

    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
        match self {
            Object::Object(data) => data.internal_own_property_keys(agent, gc),
            Object::Array(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_own_property_keys(agent, gc),
            Object::Error(data) => data.internal_own_property_keys(agent, gc),
            Object::BoundFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::BuiltinFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_own_property_keys(agent, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_own_property_keys(agent, gc),
            Object::Arguments(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_own_property_keys(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_own_property_keys(agent, gc),
            Object::Map(data) => data.internal_own_property_keys(agent, gc),
            Object::Promise(data) => data.internal_own_property_keys(agent, gc),
            Object::Proxy(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_own_property_keys(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.internal_own_property_keys(agent, gc),
            Object::ArrayIterator(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_own_property_keys(agent, gc),
            Object::MapIterator(data) => data.internal_own_property_keys(agent, gc),
            Object::StringIterator(data) => data.internal_own_property_keys(agent, gc),
            Object::Generator(data) => data.internal_own_property_keys(agent, gc),
            Object::Module(data) => data.internal_own_property_keys(agent, gc),
            Object::EmbedderObject(data) => data.internal_own_property_keys(agent, gc),
        }
    }

    fn get_cached<'gc>(
        self,
        agent: &mut Agent,
        p: PropertyKey,
        cache: PropertyLookupCache,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<GetCachedResult<'gc>, NoCache> {
        match self {
            Object::Object(data) => data.get_cached(agent, p, cache, gc),
            Object::Array(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.get_cached(agent, p, cache, gc),
            Object::Error(data) => data.get_cached(agent, p, cache, gc),
            Object::BoundFunction(data) => data.get_cached(agent, p, cache, gc),
            Object::BuiltinFunction(data) => data.get_cached(agent, p, cache, gc),
            Object::ECMAScriptFunction(data) => data.get_cached(agent, p, cache, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.get_cached(agent, p, cache, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.get_cached(agent, p, cache, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.get_cached(agent, p, cache, gc),
            Object::Arguments(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.get_cached(agent, p, cache, gc),
            Object::FinalizationRegistry(data) => data.get_cached(agent, p, cache, gc),
            Object::Map(data) => data.get_cached(agent, p, cache, gc),
            Object::Promise(data) => data.get_cached(agent, p, cache, gc),
            Object::Proxy(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).get_cached(agent, p, cache, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).get_cached(agent, p, cache, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).get_cached(agent, p, cache, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.get_cached(agent, p, cache, gc),
            Object::ArrayIterator(data) => data.get_cached(agent, p, cache, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.get_cached(agent, p, cache, gc),
            Object::MapIterator(data) => data.get_cached(agent, p, cache, gc),
            Object::StringIterator(data) => data.get_cached(agent, p, cache, gc),
            Object::Generator(data) => data.get_cached(agent, p, cache, gc),
            Object::Module(data) => data.get_cached(agent, p, cache, gc),
            Object::EmbedderObject(data) => data.get_cached(agent, p, cache, gc),
        }
    }

    fn set_cached<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
        match self {
            Object::Object(data) => data.set_cached(agent, props, gc),
            Object::Array(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.set_cached(agent, props, gc),
            Object::Error(data) => data.set_cached(agent, props, gc),
            Object::BoundFunction(data) => data.set_cached(agent, props, gc),
            Object::BuiltinFunction(data) => data.set_cached(agent, props, gc),
            Object::ECMAScriptFunction(data) => data.set_cached(agent, props, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.set_cached(agent, props, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.set_cached(agent, props, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.set_cached(agent, props, gc),
            Object::Arguments(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.set_cached(agent, props, gc),
            Object::FinalizationRegistry(data) => data.set_cached(agent, props, gc),
            Object::Map(data) => data.set_cached(agent, props, gc),
            Object::Promise(data) => data.set_cached(agent, props, gc),
            Object::Proxy(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).set_cached(agent, props, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).set_cached(agent, props, gc),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).set_cached(agent, props, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).set_cached(agent, props, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).set_cached(agent, props, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).set_cached(agent, props, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).set_cached(agent, props, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.set_cached(agent, props, gc),
            Object::ArrayIterator(data) => data.set_cached(agent, props, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.set_cached(agent, props, gc),
            Object::MapIterator(data) => data.set_cached(agent, props, gc),
            Object::StringIterator(data) => data.set_cached(agent, props, gc),
            Object::Generator(data) => data.set_cached(agent, props, gc),
            Object::Module(data) => data.set_cached(agent, props, gc),
            Object::EmbedderObject(data) => data.set_cached(agent, props, gc),
        }
    }

    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<GetCachedResult<'gc>, NoCache> {
        match self {
            Object::Object(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::Array(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::Error(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::BoundFunction(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::BuiltinFunction(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::ECMAScriptFunction(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.get_own_property_at_offset(agent, offset, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.get_own_property_at_offset(agent, offset, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::Arguments(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::FinalizationRegistry(data) => {
                data.get_own_property_at_offset(agent, offset, gc)
            }
            Object::Map(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::Promise(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::Proxy(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).get_own_property_at_offset(agent, offset, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::ArrayIterator(data) => data.get_own_property_at_offset(agent, offset, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::MapIterator(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::StringIterator(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::Generator(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::Module(data) => data.get_own_property_at_offset(agent, offset, gc),
            Object::EmbedderObject(data) => data.get_own_property_at_offset(agent, offset, gc),
        }
    }

    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
        match self {
            Object::Object(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Array(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Error(data) => data.set_at_offset(agent, props, offset, gc),
            Object::BoundFunction(data) => data.set_at_offset(agent, props, offset, gc),
            Object::BuiltinFunction(data) => data.set_at_offset(agent, props, offset, gc),
            Object::ECMAScriptFunction(data) => data.set_at_offset(agent, props, offset, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.set_at_offset(agent, props, offset, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.set_at_offset(agent, props, offset, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Arguments(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.set_at_offset(agent, props, offset, gc),
            Object::FinalizationRegistry(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Map(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Promise(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Proxy(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).set_at_offset(agent, props, offset, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).set_at_offset(agent, props, offset, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncGenerator(data) => data.set_at_offset(agent, props, offset, gc),
            Object::ArrayIterator(data) => data.set_at_offset(agent, props, offset, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.set_at_offset(agent, props, offset, gc),
            Object::MapIterator(data) => data.set_at_offset(agent, props, offset, gc),
            Object::StringIterator(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Generator(data) => data.set_at_offset(agent, props, offset, gc),
            Object::Module(data) => data.set_at_offset(agent, props, offset, gc),
            Object::EmbedderObject(data) => data.set_at_offset(agent, props, offset, gc),
        }
    }

    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        match self {
            Object::BoundFunction(data) => data.internal_call(agent, this_value, arguments, gc),
            Object::BuiltinFunction(data) => data.internal_call(agent, this_value, arguments, gc),
            Object::ECMAScriptFunction(data) => {
                data.internal_call(agent, this_value, arguments, gc)
            }
            Object::EmbedderObject(_) => todo!(),
            _ => unreachable!(),
        }
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        match self {
            Object::BoundFunction(data) => {
                data.internal_construct(agent, arguments, new_target, gc)
            }
            Object::BuiltinFunction(data) => {
                data.internal_construct(agent, arguments, new_target, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_construct(agent, arguments, new_target, gc)
            }
            _ => unreachable!(),
        }
    }
}

impl HeapMarkAndSweep for Object<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Self::Object(data) => data.mark_values(queues),
            Self::Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "date")]
            Self::Date(data) => data.mark_values(queues),
            Self::Error(data) => data.mark_values(queues),
            Self::BoundFunction(data) => data.mark_values(queues),
            Self::BuiltinFunction(data) => data.mark_values(queues),
            Self::ECMAScriptFunction(data) => data.mark_values(queues),
            Self::BuiltinGeneratorFunction => todo!(),
            Self::BuiltinConstructorFunction(data) => data.mark_values(queues),
            Self::BuiltinPromiseResolvingFunction(data) => data.mark_values(queues),
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::PrimitiveObject(data) => data.mark_values(queues),
            Self::Arguments(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data) => data.mark_values(queues),
            Self::FinalizationRegistry(data) => data.mark_values(queues),
            Self::Map(data) => data.mark_values(queues),
            Self::Promise(data) => data.mark_values(queues),
            Self::Proxy(data) => data.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExp(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
            Self::Set(data) => data.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(data) => data.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(data) => data.mark_values(queues),
            Self::AsyncFromSyncIterator => todo!(),
            Self::AsyncGenerator(data) => data.mark_values(queues),
            Self::ArrayIterator(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
            Self::SetIterator(data) => data.mark_values(queues),
            Self::MapIterator(data) => data.mark_values(queues),
            Self::StringIterator(data) => data.mark_values(queues),
            Self::Generator(data) => data.mark_values(queues),
            Self::Module(data) => data.mark_values(queues),
            Self::EmbedderObject(data) => data.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::Object(data) => data.sweep_values(compactions),
            Self::BoundFunction(data) => data.sweep_values(compactions),
            Self::BuiltinFunction(data) => data.sweep_values(compactions),
            Self::ECMAScriptFunction(data) => data.sweep_values(compactions),
            Self::BuiltinGeneratorFunction => todo!(),
            Self::BuiltinConstructorFunction(data) => data.sweep_values(compactions),
            Self::BuiltinPromiseResolvingFunction(data) => data.sweep_values(compactions),
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::PrimitiveObject(data) => data.sweep_values(compactions),
            Self::Arguments(data) => data.sweep_values(compactions),
            Self::Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data) => data.sweep_values(compactions),
            #[cfg(feature = "date")]
            Self::Date(data) => data.sweep_values(compactions),
            Self::Error(data) => data.sweep_values(compactions),
            Self::FinalizationRegistry(data) => data.sweep_values(compactions),
            Self::Map(data) => data.sweep_values(compactions),
            Self::Promise(data) => data.sweep_values(compactions),
            Self::Proxy(data) => data.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExp(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::Set(data) => data.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(data) => data.sweep_values(compactions),
            Self::AsyncFromSyncIterator => todo!(),
            Self::AsyncGenerator(data) => data.sweep_values(compactions),
            Self::ArrayIterator(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::SetIterator(data) => data.sweep_values(compactions),
            Self::MapIterator(data) => data.sweep_values(compactions),
            Self::StringIterator(data) => data.sweep_values(compactions),
            Self::Generator(data) => data.sweep_values(compactions),
            Self::Module(data) => data.sweep_values(compactions),
            Self::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}

impl HeapSweepWeakReference for Object<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        match self {
            Self::Object(data) => data.sweep_weak_reference(compactions).map(Self::Object),
            Self::BoundFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BoundFunction),
            Self::BuiltinFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinFunction),
            Self::ECMAScriptFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::ECMAScriptFunction),
            Self::BuiltinGeneratorFunction => Some(Self::BuiltinGeneratorFunction),
            Self::BuiltinConstructorFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinConstructorFunction),
            Self::BuiltinPromiseResolvingFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinPromiseResolvingFunction),
            Self::BuiltinPromiseCollectorFunction => Some(Self::BuiltinPromiseCollectorFunction),
            Self::BuiltinProxyRevokerFunction => Some(Self::BuiltinProxyRevokerFunction),
            Self::PrimitiveObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::PrimitiveObject),
            Self::Arguments(data) => data.sweep_weak_reference(compactions).map(Self::Arguments),
            Self::Array(data) => data.sweep_weak_reference(compactions).map(Self::Array),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::ArrayBuffer),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data) => data.sweep_weak_reference(compactions).map(Self::DataView),
            #[cfg(feature = "date")]
            Self::Date(data) => data.sweep_weak_reference(compactions).map(Self::Date),
            Self::Error(data) => data.sweep_weak_reference(compactions).map(Self::Error),
            Self::FinalizationRegistry(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::FinalizationRegistry),
            Self::Map(data) => data.sweep_weak_reference(compactions).map(Self::Map),
            Self::Promise(data) => data.sweep_weak_reference(compactions).map(Self::Promise),
            Self::Proxy(data) => data.sweep_weak_reference(compactions).map(Self::Proxy),
            #[cfg(feature = "regexp")]
            Self::RegExp(data) => data.sweep_weak_reference(compactions).map(Self::RegExp),
            #[cfg(feature = "set")]
            Self::Set(data) => data.sweep_weak_reference(compactions).map(Self::Set),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::SharedArrayBuffer),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.sweep_weak_reference(compactions).map(Self::WeakMap),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.sweep_weak_reference(compactions).map(Self::WeakRef),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.sweep_weak_reference(compactions).map(Self::WeakSet),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(data) => data.sweep_weak_reference(compactions).map(Self::Int8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(data) => data.sweep_weak_reference(compactions).map(Self::Uint8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Uint8ClampedArray),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(data) => data.sweep_weak_reference(compactions).map(Self::Int16Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Uint16Array),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(data) => data.sweep_weak_reference(compactions).map(Self::Int32Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Uint32Array),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BigInt64Array),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BigUint64Array),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Float16Array),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Float32Array),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Float64Array),
            Self::AsyncFromSyncIterator => Some(Self::AsyncFromSyncIterator),
            Self::AsyncGenerator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::AsyncGenerator),
            Self::ArrayIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::ArrayIterator),
            #[cfg(feature = "set")]
            Self::SetIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::SetIterator),
            Self::MapIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::MapIterator),
            Self::StringIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::StringIterator),
            Self::Generator(data) => data.sweep_weak_reference(compactions).map(Self::Generator),
            Self::Module(data) => data.sweep_weak_reference(compactions).map(Self::Module),
            Self::EmbedderObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::EmbedderObject),
        }
    }
}

impl<'a> CreateHeapData<ObjectHeapData<'a>, OrdinaryObject<'a>> for Heap {
    fn create(&mut self, data: ObjectHeapData<'a>) -> OrdinaryObject<'a> {
        self.objects.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<ObjectHeapData<'static>>>();
        OrdinaryObject(ObjectIndex::last(&self.objects))
    }
}

impl TryFrom<HeapRootData> for OrdinaryObject<'_> {
    type Error = ();

    fn try_from(value: HeapRootData) -> Result<Self, ()> {
        if let HeapRootData::Object(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl TryFrom<HeapRootData> for Object<'_> {
    type Error = ();

    fn try_from(value: HeapRootData) -> Result<Self, ()> {
        match value {
            HeapRootData::Empty
            | HeapRootData::String(_)
            | HeapRootData::Symbol(_)
            | HeapRootData::Number(_)
            | HeapRootData::BigInt(_) => Err(()),
            HeapRootData::Object(ordinary_object) => Ok(Self::Object(ordinary_object)),
            HeapRootData::BoundFunction(bound_function) => Ok(Self::BoundFunction(bound_function)),
            HeapRootData::BuiltinFunction(builtin_function) => {
                Ok(Self::BuiltinFunction(builtin_function))
            }
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                Ok(Self::ECMAScriptFunction(ecmascript_function))
            }
            HeapRootData::BuiltinGeneratorFunction => Ok(Self::BuiltinGeneratorFunction),
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => Ok(
                Self::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Ok(Self::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function,
                ))
            }
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Ok(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            HeapRootData::PrimitiveObject(primitive_object) => {
                Ok(Self::PrimitiveObject(primitive_object))
            }
            HeapRootData::Arguments(ordinary_object) => Ok(Self::Arguments(ordinary_object)),
            HeapRootData::Array(array) => Ok(Self::Array(array)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(array_buffer) => Ok(Self::ArrayBuffer(array_buffer)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(data_view) => Ok(Self::DataView(data_view)),
            #[cfg(feature = "date")]
            HeapRootData::Date(date) => Ok(Self::Date(date)),
            HeapRootData::Error(error) => Ok(Self::Error(error)),
            HeapRootData::FinalizationRegistry(finalization_registry) => {
                Ok(Self::FinalizationRegistry(finalization_registry))
            }
            HeapRootData::Map(map) => Ok(Self::Map(map)),
            HeapRootData::Promise(promise) => Ok(Self::Promise(promise)),
            HeapRootData::Proxy(proxy) => Ok(Self::Proxy(proxy)),
            #[cfg(feature = "regexp")]
            HeapRootData::RegExp(reg_exp) => Ok(Self::RegExp(reg_exp)),
            #[cfg(feature = "set")]
            HeapRootData::Set(set) => Ok(Self::Set(set)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(shared_array_buffer) => {
                Ok(Self::SharedArrayBuffer(shared_array_buffer))
            }
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => Ok(Self::WeakMap(weak_map)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => Ok(Self::WeakRef(weak_ref)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => Ok(Self::WeakSet(weak_set)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(base_index) => Ok(Self::Int8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(base_index) => Ok(Self::Uint8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(base_index) => Ok(Self::Uint8ClampedArray(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(base_index) => Ok(Self::Int16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(base_index) => Ok(Self::Uint16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(base_index) => Ok(Self::Int32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(base_index) => Ok(Self::Uint32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(base_index) => Ok(Self::BigInt64Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(base_index) => Ok(Self::BigUint64Array(base_index)),
            #[cfg(feature = "proposal-float16array")]
            HeapRootData::Float16Array(base_index) => Ok(Self::Float16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(base_index) => Ok(Self::Float32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(base_index) => Ok(Self::Float64Array(base_index)),
            HeapRootData::AsyncFromSyncIterator => Ok(Self::AsyncFromSyncIterator),
            HeapRootData::AsyncGenerator(r#gen) => Ok(Self::AsyncGenerator(r#gen)),
            HeapRootData::ArrayIterator(array_iterator) => Ok(Self::ArrayIterator(array_iterator)),
            #[cfg(feature = "set")]
            HeapRootData::SetIterator(set_iterator) => Ok(Self::SetIterator(set_iterator)),
            HeapRootData::MapIterator(map_iterator) => Ok(Self::MapIterator(map_iterator)),
            HeapRootData::StringIterator(map_iterator) => Ok(Self::StringIterator(map_iterator)),
            HeapRootData::Generator(generator) => Ok(Self::Generator(generator)),
            HeapRootData::Module(module) => Ok(Self::Module(module)),
            HeapRootData::EmbedderObject(embedder_object) => {
                Ok(Self::EmbedderObject(embedder_object))
            }
            HeapRootData::AwaitReaction(_)
            | HeapRootData::PromiseReaction(_)
            | HeapRootData::Executable(_)
            | HeapRootData::Realm(_)
            | HeapRootData::Script(_)
            | HeapRootData::SourceCode(_)
            | HeapRootData::SourceTextModule(_)
            | HeapRootData::DeclarativeEnvironment(_)
            | HeapRootData::FunctionEnvironment(_)
            | HeapRootData::GlobalEnvironment(_)
            | HeapRootData::ModuleEnvironment(_)
            | HeapRootData::ObjectEnvironment(_)
            | HeapRootData::PrivateEnvironment(_) => Err(()),
        }
    }
}
