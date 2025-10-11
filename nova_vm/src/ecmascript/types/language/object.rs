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
use std::collections::TryReserveError;

#[cfg(feature = "date")]
use super::value::DATE_DISCRIMINANT;
#[cfg(feature = "weak-refs")]
use super::value::{WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT};
use super::{
    Function, Value,
    value::{
        ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
        ASYNC_GENERATOR_DISCRIMINANT, BOUND_FUNCTION_DISCRIMINANT,
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT, BUILTIN_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
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
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::{builtins::typed_array::Float16Array, types::FLOAT_16_ARRAY_DISCRIMINANT};
#[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
use crate::ecmascript::{
    builtins::typed_array::SharedFloat16Array, types::SHARED_FLOAT_16_ARRAY_DISCRIMINANT,
};
#[cfg(feature = "array-buffer")]
use crate::ecmascript::{
    builtins::{
        ArrayBuffer,
        data_view::DataView,
        typed_array::{
            BigInt64Array, BigUint64Array, Float32Array, Float64Array, Int8Array, Int16Array,
            Int32Array, Uint8Array, Uint8ClampedArray, Uint16Array, Uint32Array,
        },
    },
    types::{
        ARRAY_BUFFER_DISCRIMINANT, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
        DATA_VIEW_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
        INT_8_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
        UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
        UINT_32_ARRAY_DISCRIMINANT,
    },
};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::{
    builtins::{
        data_view::SharedDataView,
        shared_array_buffer::SharedArrayBuffer,
        typed_array::{
            SharedBigInt64Array, SharedBigUint64Array, SharedFloat32Array, SharedFloat64Array,
            SharedInt8Array, SharedInt16Array, SharedInt32Array, SharedUint8Array,
            SharedUint8ClampedArray, SharedUint16Array, SharedUint32Array,
        },
    },
    types::{
        SHARED_ARRAY_BUFFER_DISCRIMINANT, SHARED_BIGINT_64_ARRAY_DISCRIMINANT,
        SHARED_BIGUINT_64_ARRAY_DISCRIMINANT, SHARED_DATA_VIEW_DISCRIMINANT,
        SHARED_FLOAT_32_ARRAY_DISCRIMINANT, SHARED_FLOAT_64_ARRAY_DISCRIMINANT,
        SHARED_INT_8_ARRAY_DISCRIMINANT, SHARED_INT_16_ARRAY_DISCRIMINANT,
        SHARED_INT_32_ARRAY_DISCRIMINANT, SHARED_UINT_8_ARRAY_DISCRIMINANT,
        SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT, SHARED_UINT_16_ARRAY_DISCRIMINANT,
        SHARED_UINT_32_ARRAY_DISCRIMINANT,
    },
};
#[cfg(feature = "set")]
use crate::ecmascript::{
    builtins::{
        keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
    },
    types::{SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT},
};
#[cfg(feature = "regexp")]
use crate::ecmascript::{
    builtins::{
        regexp::RegExp,
        text_processing::regexp_objects::regexp_string_iterator_objects::RegExpStringIterator,
    },
    types::{REGEXP_DISCRIMINANT, REGEXP_STRING_ITERATOR_DISCRIMINANT},
};
use crate::{
    ecmascript::{
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
            promise_objects::promise_abstract_operations::promise_finally_functions::BuiltinPromiseFinallyFunction,
            proxy::Proxy,
            text_processing::string_objects::string_iterator_objects::StringIterator,
        },
        execution::{Agent, JsResult, ProtoIntrinsics, agent::TryResult},
        types::PropertyDescriptor,
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        IntrinsicConstructorIndexes, IntrinsicObjectIndexes, IntrinsicPrimitiveObjectIndexes,
        ObjectEntry, WorkQueues,
        element_array::{
            ElementDescriptor, ElementStorageMut, ElementStorageRef, ElementStorageUninit,
            ElementsVector, PropertyStorageMut, PropertyStorageRef,
        },
        indexes::BaseIndex,
    },
};

use ahash::AHashMap;
pub(crate) use data::ObjectRecord;
pub use internal_methods::*;
pub use internal_slots::InternalSlots;
pub use into_object::IntoObject;
pub use property_key::PropertyKey;
pub use property_key_set::PropertyKeySet;
#[cfg(feature = "json")]
pub(crate) use property_key_vec::ScopedPropertyKey;
pub use property_storage::PropertyStorage;

/// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
///
/// In Nova
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Object<'a> {
    Object(OrdinaryObject<'a>) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction<'a>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'a>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'a>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseFinallyFunction(BuiltinPromiseFinallyFunction<'a>) =
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject<'a>) = PRIMITIVE_OBJECT_DISCRIMINANT,
    Arguments(OrdinaryObject<'a>) = ARGUMENTS_DISCRIMINANT,
    Array(Array<'a>) = ARRAY_DISCRIMINANT,
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
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'a>) = WEAK_MAP_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'a>) = WEAK_REF_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'a>) = WEAK_SET_DISCRIMINANT,

    /// ### [25.1 ArrayBuffer Objects](https://tc39.es/ecma262/#sec-arraybuffer-objects)
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'a>) = ARRAY_BUFFER_DISCRIMINANT,
    /// ### [25.3 DataView Objects](https://tc39.es/ecma262/#sec-dataview-objects)
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'a>) = DATA_VIEW_DISCRIMINANT,
    // ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
    #[cfg(feature = "array-buffer")]
    Int8Array(Int8Array<'a>) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(Uint8Array<'a>) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(Uint8ClampedArray<'a>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(Int16Array<'a>) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(Uint16Array<'a>) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(Int32Array<'a>) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(Uint32Array<'a>) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(BigInt64Array<'a>) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(BigUint64Array<'a>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(Float16Array<'a>) = FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(Float32Array<'a>) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(Float64Array<'a>) = FLOAT_64_ARRAY_DISCRIMINANT,

    /// ### [25.2 SharedArrayBuffer Objects](https://tc39.es/ecma262/#sec-sharedarraybuffer-objects)
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'a>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    /// ### [25.3 DataView Objects](https://tc39.es/ecma262/#sec-dataview-objects)
    ///
    /// A variant of DataView Objects viewing a SharedArrayBuffer.
    #[cfg(feature = "shared-array-buffer")]
    SharedDataView(SharedDataView<'a>) = SHARED_DATA_VIEW_DISCRIMINANT,
    // ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
    //
    // Variants of TypedArray Objects viewing a SharedArrayBuffer.
    #[cfg(feature = "shared-array-buffer")]
    SharedInt8Array(SharedInt8Array<'a>) = SHARED_INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8Array(SharedUint8Array<'a>) = SHARED_UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8ClampedArray(SharedUint8ClampedArray<'a>) = SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt16Array(SharedInt16Array<'a>) = SHARED_INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint16Array(SharedUint16Array<'a>) = SHARED_UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt32Array(SharedInt32Array<'a>) = SHARED_INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint32Array(SharedUint32Array<'a>) = SHARED_UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedBigInt64Array(SharedBigInt64Array<'a>) = SHARED_BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedBigUint64Array(SharedBigUint64Array<'a>) = SHARED_BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
    SharedFloat16Array(SharedFloat16Array<'a>) = SHARED_FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat32Array(SharedFloat32Array<'a>) = SHARED_FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat64Array(SharedFloat64Array<'a>) = SHARED_FLOAT_64_ARRAY_DISCRIMINANT,

    AsyncGenerator(AsyncGenerator<'a>) = ASYNC_GENERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator<'a>) = ARRAY_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'a>) = SET_ITERATOR_DISCRIMINANT,
    MapIterator(MapIterator<'a>) = MAP_ITERATOR_DISCRIMINANT,
    StringIterator(StringIterator<'a>) = STRING_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "regexp")]
    RegExpStringIterator(RegExpStringIterator<'a>) = REGEXP_STRING_ITERATOR_DISCRIMINANT,
    Generator(Generator<'a>) = GENERATOR_DISCRIMINANT,
    Module(Module<'a>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject<'a>) = EMBEDDER_OBJECT_DISCRIMINANT,
}

bindable_handle!(Object);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrdinaryObject<'a>(BaseIndex<'a, ObjectRecord<'static>>);

bindable_handle!(OrdinaryObject);

impl<'a> OrdinaryObject<'a> {
    /// Allocate a a new blank OrdinaryObject and return its reference.
    ///
    /// The new OrdinaryObject is conceptually equivalent to a
    /// `{ __proto__: null }` object.
    pub(crate) fn new_uninitialised(agent: &mut Agent) -> Self {
        agent.heap.objects.push(ObjectRecord::BLANK);
        OrdinaryObject(BaseIndex::last_t(&agent.heap.objects))
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    pub(crate) fn get(self, agent: &Agent) -> &ObjectRecord<'a> {
        self.get_direct(&agent.heap.objects)
    }

    #[inline(always)]
    pub(crate) fn get_mut(self, agent: &mut Agent) -> &mut ObjectRecord<'a> {
        self.get_direct_mut(&mut agent.heap.objects)
    }

    #[inline(always)]
    pub(crate) fn get_direct<'o>(self, objects: &'o [ObjectRecord<'a>]) -> &'o ObjectRecord<'a> {
        &objects[self.get_index()]
    }

    #[inline(always)]
    pub(crate) fn get_direct_mut<'o>(
        self,
        objects: &'o mut [ObjectRecord<'static>],
    ) -> &'o mut ObjectRecord<'a> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<&mut ObjectRecord<'static>, &mut ObjectRecord<'a>>(
                &mut objects[self.get_index()],
            )
        }
    }

    /// Returns true if the Object has no properties.
    pub(crate) fn is_empty(self, agent: &Agent) -> bool {
        self.get(agent).is_empty(agent)
    }

    /// Returns the number of properties in the object.
    pub(crate) fn len(self, agent: &Agent) -> u32 {
        self.get(agent).len(agent)
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
        self.get_mut(agent).set_shape(new_shape);
    }

    pub(crate) fn get_property_storage<'b>(self, agent: &'b Agent) -> PropertyStorageRef<'b, 'a> {
        let Heap {
            object_shapes,
            elements,
            objects,
            ..
        } = &agent.heap;
        let data = self.get_direct(objects);
        let shape = data.get_shape();
        let keys = shape.keys(object_shapes, elements);
        let elements = data.get_storage(elements, object_shapes);
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
        let data = self.get_direct(objects);
        let shape = data.get_shape();
        elements.get_property_storage_mut_raw(
            shape.keys_index(object_shapes),
            shape.keys_capacity(object_shapes),
            data.get_values(),
            shape.values_capacity(object_shapes),
            shape.len(object_shapes),
        )
    }

    /// Get the elements backing storage of the Object as an ElementsVector.
    ///
    /// The Object owns this backing storage uniquely. Performing mutations on
    /// the ElementsVector needs to be done carefully to avoid causing
    /// "undefined behaviour" in JavaScript.
    pub(crate) fn get_elements_vector(self, agent: &Agent) -> ElementsVector<'a> {
        let data = self.get(agent);
        let shape = data.get_shape();
        let elements_index = data.get_values();
        let len_writable = shape.extensible();
        let cap = shape.values_capacity(agent);
        let len = shape.len(agent);
        ElementsVector {
            elements_index,
            cap,
            len,
            len_writable,
        }
    }

    pub(crate) fn get_elements_storage<'b>(self, agent: &'b Agent) -> ElementStorageRef<'b, 'a> {
        let Heap {
            elements,
            objects,
            object_shapes,
            ..
        } = &agent.heap;
        let data = self.get_direct(objects);
        elements.get_element_storage_raw(
            data.get_values(),
            data.values_capacity(object_shapes),
            data.len(object_shapes),
        )
    }

    pub(crate) fn get_elements_storage_mut<'b>(
        self,
        agent: &'b mut Agent,
    ) -> ElementStorageMut<'b> {
        let Heap {
            elements,
            objects,
            object_shapes,
            ..
        } = &mut agent.heap;
        let data = self.get_direct(objects);
        elements.get_element_storage_mut_raw(
            data.get_values(),
            data.values_capacity(object_shapes),
            data.len(object_shapes),
        )
    }

    pub(crate) fn get_elements_storage_uninit<'b>(
        self,
        agent: &'b mut Agent,
    ) -> ElementStorageUninit<'b> {
        let Heap {
            elements,
            objects,
            object_shapes,
            ..
        } = &mut agent.heap;
        let data = self.get_direct(objects);
        elements
            .get_element_storage_uninit_raw(data.get_values(), data.values_capacity(object_shapes))
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
            len_writable: _,
        } = agent
            .heap
            .elements
            .allocate_object_property_storage_from_entries_slice(entries)
            .expect("Failed to create object");
        assert_eq!(len, shape.len(agent));
        assert_eq!(cap.capacity(), shape.values_capacity(agent).capacity());
        agent.heap.create(ObjectRecord::new(shape, values))
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
            len_writable: _,
        } = agent
            .heap
            .elements
            .allocate_property_storage(values, None)
            .expect("Failed to create object");
        assert_eq!(cap, shape.values_capacity(agent));
        assert_eq!(len, shape.len(agent));
        agent.heap.create(ObjectRecord::new(shape, values))
    }

    pub(crate) fn create_object_with_shape(
        agent: &mut Agent,
        shape: ObjectShape<'a>,
    ) -> Result<Self, TryReserveError> {
        let ElementsVector {
            elements_index: values,
            cap,
            len: _,
            len_writable: _,
        } = agent
            .heap
            .elements
            .allocate_elements_with_capacity(shape.values_capacity(agent))?;
        assert_eq!(cap, shape.values_capacity(agent));
        Ok(agent.heap.create(ObjectRecord::new(shape, values)))
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
            .allocate_keys_with_capacity(properties_count);
        let cap = cap.make_intrinsic();
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
        let mut source_shape = self.get(agent).get_shape();
        // Note: our source object can be frozen but we should not become
        // frozen just by copying the source properties.
        source_shape.set_extensible(true);
        let elements_vector = agent
            .heap
            .elements
            .shallow_clone(&source.get_elements_vector(agent));
        let data = self.get_mut(agent);
        data.set_shape(source_shape);
        data.set_values(elements_vector.elements_index.unbind());
        true
    }
}

impl IntrinsicObjectIndexes {
    pub(crate) const fn get_backing_object<'a>(
        self,
        base: BaseIndex<'a, ObjectRecord<'static>>,
    ) -> OrdinaryObject<'a> {
        OrdinaryObject(BaseIndex::from_u32_index(
            self as u32 + base.into_u32_index() + Self::OBJECT_INDEX_OFFSET,
        ))
    }
}

impl IntrinsicConstructorIndexes {
    pub(crate) const fn get_backing_object<'a>(
        self,
        base: BaseIndex<'a, ObjectRecord<'static>>,
    ) -> OrdinaryObject<'a> {
        OrdinaryObject(BaseIndex::from_u32_index(
            self as u32 + base.into_u32_index() + Self::OBJECT_INDEX_OFFSET,
        ))
    }
}

impl IntrinsicPrimitiveObjectIndexes {
    pub(crate) const fn get_backing_object<'a>(
        self,
        base: BaseIndex<'a, ObjectRecord<'static>>,
    ) -> OrdinaryObject<'a> {
        OrdinaryObject(BaseIndex::from_u32_index(
            self as u32 + base.into_u32_index() + Self::OBJECT_INDEX_OFFSET,
        ))
    }
}

impl<'a> From<OrdinaryObject<'a>> for Object<'a> {
    fn from(value: OrdinaryObject<'a>) -> Self {
        Self::Object(value)
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
        self.get(agent).get_shape().unbind()
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        self.get(agent).get_extensible()
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        self.get_mut(agent).set_extensible(value)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        self.get(agent).get_prototype(agent).unbind()
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        let original_shape = self.get(agent).get_shape();
        if original_shape.get_prototype(agent) == prototype {
            return;
        }
        let new_shape = original_shape.get_shape_with_prototype(agent, prototype);
        self.get_mut(agent).set_shape(new_shape);
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
            Object::Object(data) => Self::Object(data),
            Object::BoundFunction(data) => Self::BoundFunction(data),
            Object::BuiltinFunction(data) => Self::BuiltinFunction(data),
            Object::ECMAScriptFunction(data) => Self::ECMAScriptFunction(data),
            Object::BuiltinConstructorFunction(data) => Self::BuiltinConstructorFunction(data),
            Object::BuiltinPromiseResolvingFunction(data) => {
                Self::BuiltinPromiseResolvingFunction(data)
            }
            Object::BuiltinPromiseFinallyFunction(data) => {
                Self::BuiltinPromiseFinallyFunction(data)
            }
            Object::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(data) => Self::PrimitiveObject(data),
            Object::Arguments(data) => Self::Arguments(data),
            Object::Array(data) => Self::Array(data),
            #[cfg(feature = "date")]
            Object::Date(data) => Self::Date(data),
            Object::Error(data) => Self::Error(data),
            Object::FinalizationRegistry(data) => Self::FinalizationRegistry(data),
            Object::Map(data) => Self::Map(data),
            Object::Promise(data) => Self::Promise(data),
            Object::Proxy(data) => Self::Proxy(data),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => Self::RegExp(data),
            #[cfg(feature = "set")]
            Object::Set(data) => Self::Set(data),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => Self::WeakMap(data),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => Self::WeakRef(data),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => Self::WeakSet(data),

            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(ab) => Self::ArrayBuffer(ab),
            #[cfg(feature = "array-buffer")]
            Object::DataView(dv) => Self::DataView(dv),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(ta) => Self::Int8Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(ta) => Self::Uint8Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(ta) => Self::Uint8ClampedArray(ta),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(ta) => Self::Int16Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(ta) => Self::Uint16Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(ta) => Self::Int32Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(ta) => Self::Uint32Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(ta) => Self::BigInt64Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(ta) => Self::BigUint64Array(ta),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(ta) => Self::Float16Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(ta) => Self::Float32Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(ta) => Self::Float64Array(ta),

            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(sab) => Self::SharedArrayBuffer(sab),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedDataView(sdv) => Self::SharedDataView(sdv),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedInt8Array(sta) => Self::SharedInt8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint8Array(sta) => Self::SharedUint8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint8ClampedArray(sta) => Self::SharedUint8ClampedArray(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedInt16Array(sta) => Self::SharedInt16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint16Array(sta) => Self::SharedUint16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedInt32Array(sta) => Self::SharedInt32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint32Array(sta) => Self::SharedUint32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedBigInt64Array(sta) => Self::SharedBigInt64Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedBigUint64Array(sta) => Self::SharedBigUint64Array(sta),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Object::SharedFloat16Array(sta) => Self::SharedFloat16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedFloat32Array(sta) => Self::SharedFloat32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedFloat64Array(sta) => Self::SharedFloat64Array(sta),

            Object::AsyncGenerator(data) => Self::AsyncGenerator(data),
            Object::ArrayIterator(data) => Self::ArrayIterator(data),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => Self::SetIterator(data),
            Object::MapIterator(data) => Self::MapIterator(data),
            Object::StringIterator(data) => Self::StringIterator(data),
            #[cfg(feature = "regexp")]
            Object::RegExpStringIterator(data) => Self::RegExpStringIterator(data),
            Object::Generator(data) => Self::Generator(data),
            Object::Module(data) => Self::Module(data),
            Object::EmbedderObject(data) => Self::EmbedderObject(data),
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
            Value::Object(x) => Ok(Self::from(x)),
            Value::Array(x) => Ok(Self::from(x)),
            #[cfg(feature = "date")]
            Value::Date(x) => Ok(Self::Date(x)),
            Value::Error(x) => Ok(Self::from(x)),
            Value::BoundFunction(x) => Ok(Self::from(x)),
            Value::BuiltinFunction(x) => Ok(Self::from(x)),
            Value::ECMAScriptFunction(x) => Ok(Self::from(x)),
            Value::BuiltinConstructorFunction(data) => Ok(Self::BuiltinConstructorFunction(data)),
            Value::BuiltinPromiseResolvingFunction(data) => {
                Ok(Self::BuiltinPromiseResolvingFunction(data))
            }
            Value::BuiltinPromiseFinallyFunction(data) => {
                Ok(Self::BuiltinPromiseFinallyFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Self::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            Value::PrimitiveObject(data) => Ok(Self::PrimitiveObject(data)),
            Value::Arguments(data) => Ok(Self::Arguments(data)),
            Value::FinalizationRegistry(data) => Ok(Self::FinalizationRegistry(data)),
            Value::Map(data) => Ok(Self::Map(data)),
            Value::Promise(data) => Ok(Self::Promise(data)),
            Value::Proxy(data) => Ok(Self::Proxy(data)),
            #[cfg(feature = "regexp")]
            Value::RegExp(idx) => Ok(Self::RegExp(idx)),
            #[cfg(feature = "set")]
            Value::Set(data) => Ok(Self::Set(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => Ok(Self::WeakMap(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => Ok(Self::WeakRef(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => Ok(Self::WeakSet(data)),

            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(ab) => Ok(Self::ArrayBuffer(ab)),
            #[cfg(feature = "array-buffer")]
            Value::DataView(dv) => Ok(Self::DataView(dv)),
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(ta) => Ok(Self::Int8Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(ta) => Ok(Self::Uint8Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(ta) => Ok(Self::Uint8ClampedArray(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(ta) => Ok(Self::Int16Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(ta) => Ok(Self::Uint16Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(ta) => Ok(Self::Int32Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(ta) => Ok(Self::Uint32Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(ta) => Ok(Self::BigInt64Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(ta) => Ok(Self::BigUint64Array(ta)),
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(ta) => Ok(Self::Float16Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(ta) => Ok(Self::Float32Array(ta)),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(ta) => Ok(Self::Float64Array(ta)),

            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(sab) => Ok(Self::SharedArrayBuffer(sab)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedDataView(sdv) => Ok(Self::SharedDataView(sdv)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt8Array(sta) => Ok(Self::SharedInt8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint8Array(sta) => Ok(Self::SharedUint8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint8ClampedArray(sta) => Ok(Self::SharedUint8ClampedArray(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt16Array(sta) => Ok(Self::SharedInt16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint16Array(sta) => Ok(Self::SharedUint16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt32Array(sta) => Ok(Self::SharedInt32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint32Array(sta) => Ok(Self::SharedUint32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedBigInt64Array(sta) => Ok(Self::SharedBigInt64Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedBigUint64Array(sta) => Ok(Self::SharedBigUint64Array(sta)),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Value::SharedFloat16Array(sta) => Ok(Self::SharedFloat16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedFloat32Array(sta) => Ok(Self::SharedFloat32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedFloat64Array(sta) => Ok(Self::SharedFloat64Array(sta)),

            Value::AsyncGenerator(data) => Ok(Self::AsyncGenerator(data)),
            Value::ArrayIterator(data) => Ok(Self::ArrayIterator(data)),
            #[cfg(feature = "set")]
            Value::SetIterator(data) => Ok(Self::SetIterator(data)),
            Value::MapIterator(data) => Ok(Self::MapIterator(data)),
            Value::StringIterator(data) => Ok(Self::StringIterator(data)),
            #[cfg(feature = "regexp")]
            Value::RegExpStringIterator(data) => Ok(Self::RegExpStringIterator(data)),
            Value::Generator(data) => Ok(Self::Generator(data)),
            Value::Module(data) => Ok(Self::Module(data)),
            Value::EmbedderObject(data) => Ok(Self::EmbedderObject(data)),
        }
    }
}

impl<'a> OrdinaryObject<'a> {
    pub fn property_storage(self) -> PropertyStorage<'a> {
        PropertyStorage::new(self)
    }
}

macro_rules! object_delegate {
    ($value: ident, $method: ident, $($arg:expr),*) => {
        match $value {
            Self::Object(data) => data.$method($($arg),+),
            Self::Array(data) => data.$method($($arg),+),
            #[cfg(feature = "date")]
            Self::Date(data) => data.$method($($arg),+),
            Self::Error(data) => data.$method($($arg),+),
            Self::BoundFunction(data) => data.$method($($arg),+),
            Self::BuiltinFunction(data) => data.$method($($arg),+),
            Self::ECMAScriptFunction(data) => data.$method($($arg),+),
            Self::BuiltinConstructorFunction(data) => data.$method($($arg),+),
            Self::BuiltinPromiseResolvingFunction(data) => data.$method($($arg),+),
            Self::BuiltinPromiseFinallyFunction(data) => data.$method($($arg),+),
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::PrimitiveObject(data) => data.$method($($arg),+),
            Self::Arguments(data) => data.$method($($arg),+),
            Self::FinalizationRegistry(data) => data.$method($($arg),+),
            Self::Map(data) => data.$method($($arg),+),
            Self::Promise(data) => data.$method($($arg),+),
            Self::Proxy(data) => data.$method($($arg),+),
            #[cfg(feature = "regexp")]
            Self::RegExp(data) => data.$method($($arg),+),
            #[cfg(feature = "set")]
            Self::Set(data) => data.$method($($arg),+),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.$method($($arg),+),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.$method($($arg),+),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.$method($($arg),+),

            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => ab.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::DataView(dv) => dv.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.$method($($arg),+),

            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sab) => sab.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sdv) => sdv.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => sta.$method($($arg),+),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => sta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => sta.$method($($arg),+),

            Self::AsyncGenerator(data) => data.$method($($arg),+),
            Self::ArrayIterator(data) => data.$method($($arg),+),
            #[cfg(feature = "set")]
            Self::SetIterator(data) => data.$method($($arg),+),
            Self::MapIterator(data) => data.$method($($arg),+),
            Self::StringIterator(data) => data.$method($($arg),+),
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(data) => data.$method($($arg),+),
            Self::Generator(data) => data.$method($($arg),+),
            Self::Module(data) => data.$method($($arg),+),
            Self::EmbedderObject(data) => data.$method($($arg),+),
        }
    };
}

impl<'a> InternalSlots<'a> for Object<'a> {
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        object_delegate!(self, get_backing_object, agent)
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("Object should not try to set its backing object");
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!("Object should not try to create its backing object");
    }

    fn get_or_create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        object_delegate!(self, get_or_create_backing_object, agent)
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        object_delegate!(self, object_shape, agent)
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        object_delegate!(self, internal_extensible, agent)
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        object_delegate!(self, internal_set_extensible, agent, value)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        object_delegate!(self, internal_prototype, agent)
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        object_delegate!(self, internal_set_prototype, agent, prototype)
    }
}

impl<'a> InternalMethods<'a> for Object<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<Object<'gc>>> {
        object_delegate!(self, try_get_prototype_of, agent, gc)
    }

    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Object<'gc>>> {
        object_delegate!(self, internal_get_prototype_of, agent, gc)
    }

    fn try_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        object_delegate!(self, try_set_prototype_of, agent, prototype, gc)
    }

    fn internal_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        object_delegate!(self, internal_set_prototype_of, agent, prototype, gc)
    }

    fn try_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        object_delegate!(self, try_is_extensible, agent, gc)
    }

    fn internal_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        object_delegate!(self, internal_is_extensible, agent, gc)
    }

    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        object_delegate!(self, try_prevent_extensions, agent, gc)
    }

    fn internal_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        object_delegate!(self, internal_prevent_extensions, agent, gc)
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        object_delegate!(self, try_get_own_property, agent, property_key, cache, gc)
    }

    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
        object_delegate!(self, internal_get_own_property, agent, property_key, gc)
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        object_delegate!(
            self,
            try_define_own_property,
            agent,
            property_key,
            property_descriptor,
            cache,
            gc
        )
    }

    fn internal_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        object_delegate!(
            self,
            internal_define_own_property,
            agent,
            property_key,
            property_descriptor,
            gc
        )
    }

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        object_delegate!(self, try_has_property, agent, property_key, cache, gc)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        object_delegate!(self, internal_has_property, agent, property_key, gc)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        object_delegate!(self, try_get, agent, property_key, receiver, cache, gc)
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        object_delegate!(self, internal_get, agent, property_key, receiver, gc)
    }

    fn try_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        object_delegate!(
            self,
            try_set,
            agent,
            property_key,
            value,
            receiver,
            cache,
            gc
        )
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        object_delegate!(self, internal_set, agent, property_key, value, receiver, gc)
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        object_delegate!(self, try_delete, agent, property_key, gc)
    }

    fn internal_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        object_delegate!(self, internal_delete, agent, property_key, gc)
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        object_delegate!(self, try_own_property_keys, agent, gc)
    }

    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
        object_delegate!(self, internal_own_property_keys, agent, gc)
    }

    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        object_delegate!(self, get_own_property_at_offset, agent, offset, gc)
    }

    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        object_delegate!(self, set_at_offset, agent, props, offset, gc)
    }

    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        if let Ok(f) = Function::try_from(self) {
            f.internal_call(agent, this_value, arguments, gc)
        } else {
            unreachable!()
        }
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        if let Ok(f) = Function::try_from(self) {
            f.internal_construct(agent, arguments, new_target, gc)
        } else {
            unreachable!()
        }
    }
}

impl HeapMarkAndSweep for Object<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        object_delegate!(self, mark_values, queues)
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        object_delegate!(self, sweep_values, compactions)
    }
}

impl HeapSweepWeakReference for Object<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        match self {
            Self::Object(data) => data.sweep_weak_reference(compactions).map(Self::Object),
            Self::Array(data) => data.sweep_weak_reference(compactions).map(Self::Array),
            #[cfg(feature = "date")]
            Self::Date(data) => data.sweep_weak_reference(compactions).map(Self::Date),
            Self::Error(data) => data.sweep_weak_reference(compactions).map(Self::Error),
            Self::BoundFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BoundFunction),
            Self::BuiltinFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinFunction),
            Self::ECMAScriptFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::ECMAScriptFunction),
            Self::BuiltinConstructorFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinConstructorFunction),
            Self::BuiltinPromiseResolvingFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinPromiseResolvingFunction),
            Self::BuiltinPromiseFinallyFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinPromiseFinallyFunction),
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::PrimitiveObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::PrimitiveObject),
            Self::Arguments(data) => data.sweep_weak_reference(compactions).map(Self::Arguments),
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
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.sweep_weak_reference(compactions).map(Self::WeakMap),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.sweep_weak_reference(compactions).map(Self::WeakRef),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.sweep_weak_reference(compactions).map(Self::WeakSet),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => ab.sweep_weak_reference(compactions).map(Self::ArrayBuffer),
            #[cfg(feature = "array-buffer")]
            Self::DataView(dv) => dv.sweep_weak_reference(compactions).map(Self::DataView),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Int8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Uint8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta
                .sweep_weak_reference(compactions)
                .map(Self::Uint8ClampedArray),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Int16Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Uint16Array),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Int32Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Uint32Array),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta
                .sweep_weak_reference(compactions)
                .map(Self::BigInt64Array),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta
                .sweep_weak_reference(compactions)
                .map(Self::BigUint64Array),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Float16Array),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Float32Array),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Float64Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sab) => sab
                .sweep_weak_reference(compactions)
                .map(Self::SharedArrayBuffer),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sdv) => sdv
                .sweep_weak_reference(compactions)
                .map(Self::SharedDataView),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedInt8Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint8Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint8ClampedArray),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedInt16Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint16Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedInt32Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint32Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedBigInt64Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedBigUint64Array),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedFloat16Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedFloat32Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedFloat64Array),
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
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::RegExpStringIterator),
            Self::Generator(data) => data.sweep_weak_reference(compactions).map(Self::Generator),
            Self::Module(data) => data.sweep_weak_reference(compactions).map(Self::Module),
            Self::EmbedderObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::EmbedderObject),
        }
    }
}

impl HeapMarkAndSweep for OrdinaryObject<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.objects.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for OrdinaryObject<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.objects.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<ObjectRecord<'a>, OrdinaryObject<'a>> for Heap {
    fn create(&mut self, data: ObjectRecord<'a>) -> OrdinaryObject<'a> {
        self.objects.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<Option<ObjectRecord<'static>>>();
        OrdinaryObject(BaseIndex::last_t(&self.objects))
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
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => Ok(
                Self::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Ok(Self::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function,
                ))
            }
            HeapRootData::BuiltinPromiseFinallyFunction(builtin_promise_finally_function) => Ok(
                Self::BuiltinPromiseFinallyFunction(builtin_promise_finally_function),
            ),
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Ok(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            HeapRootData::PrimitiveObject(primitive_object) => {
                Ok(Self::PrimitiveObject(primitive_object))
            }
            HeapRootData::Arguments(ordinary_object) => Ok(Self::Arguments(ordinary_object)),
            HeapRootData::Array(array) => Ok(Self::Array(array)),
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
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => Ok(Self::WeakMap(weak_map)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => Ok(Self::WeakRef(weak_ref)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => Ok(Self::WeakSet(weak_set)),

            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(ab) => Ok(Self::ArrayBuffer(ab)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(dv) => Ok(Self::DataView(dv)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(ta) => Ok(Self::Int8Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(ta) => Ok(Self::Uint8Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(ta) => Ok(Self::Uint8ClampedArray(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(ta) => Ok(Self::Int16Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(ta) => Ok(Self::Uint16Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(ta) => Ok(Self::Int32Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(ta) => Ok(Self::Uint32Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(ta) => Ok(Self::BigInt64Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(ta) => Ok(Self::BigUint64Array(ta)),
            #[cfg(feature = "proposal-float16array")]
            HeapRootData::Float16Array(ta) => Ok(Self::Float16Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(ta) => Ok(Self::Float32Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(ta) => Ok(Self::Float64Array(ta)),

            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(sab) => Ok(Self::SharedArrayBuffer(sab)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedDataView(sdv) => Ok(Self::SharedDataView(sdv)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedInt8Array(sta) => Ok(Self::SharedInt8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint8Array(sta) => Ok(Self::SharedUint8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint8ClampedArray(sta) => Ok(Self::SharedUint8ClampedArray(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedInt16Array(sta) => Ok(Self::SharedInt16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint16Array(sta) => Ok(Self::SharedUint16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedInt32Array(sta) => Ok(Self::SharedInt32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint32Array(sta) => Ok(Self::SharedUint32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedBigInt64Array(sta) => Ok(Self::SharedBigInt64Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedBigUint64Array(sta) => Ok(Self::SharedBigUint64Array(sta)),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            HeapRootData::SharedFloat16Array(sta) => Ok(Self::SharedFloat16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedFloat32Array(sta) => Ok(Self::SharedFloat32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedFloat64Array(sta) => Ok(Self::SharedFloat64Array(sta)),

            HeapRootData::AsyncGenerator(r#gen) => Ok(Self::AsyncGenerator(r#gen)),
            HeapRootData::ArrayIterator(array_iterator) => Ok(Self::ArrayIterator(array_iterator)),
            #[cfg(feature = "set")]
            HeapRootData::SetIterator(set_iterator) => Ok(Self::SetIterator(set_iterator)),
            HeapRootData::MapIterator(map_iterator) => Ok(Self::MapIterator(map_iterator)),
            HeapRootData::StringIterator(map_iterator) => Ok(Self::StringIterator(map_iterator)),
            #[cfg(feature = "regexp")]
            HeapRootData::RegExpStringIterator(map_iterator) => {
                Ok(Self::RegExpStringIterator(map_iterator))
            }
            HeapRootData::Generator(generator) => Ok(Self::Generator(generator)),
            HeapRootData::Module(module) => Ok(Self::Module(module)),
            HeapRootData::EmbedderObject(embedder_object) => {
                Ok(Self::EmbedderObject(embedder_object))
            }
            HeapRootData::AwaitReaction(_)
            | HeapRootData::PromiseReaction(_)
            | HeapRootData::PromiseAll(_)
            | HeapRootData::PromiseAllSettled(_)
            | HeapRootData::PromiseGroup(_)
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
            | HeapRootData::PrivateEnvironment(_)
            | HeapRootData::PropertyLookupCache(_) => Err(()),
        }
    }
}
