// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::{Hash, Hasher};
use std::{marker::PhantomData, ops::ControlFlow};

use data::{SharedTypedArrayRecord, TypedArrayArrayLength};

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::canonical_numeric_index_string,
        builtins::indexed_collections::typed_array_objects::abstract_operations::{
            is_typed_array_fixed_length, is_typed_array_out_of_bounds_specialised,
            is_valid_integer_index_specialised,
            make_typed_array_with_buffer_witness_record_specialised,
            typed_array_length_specialised,
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics,
            agent::{TryResult, js_result_into_try, unwrap_try},
        },
        types::{
            BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
            FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT,
            INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, InternalMethods, InternalSlots,
            IntoObject, IntoValue, Number, Numeric, Object, OrdinaryObject, PropertyDescriptor,
            PropertyKey, SetCachedProps, SetResult, String, TryGetResult, TryHasResult, U8Clamped,
            UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
            UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, Value, Viewable,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::types::FLOAT_16_ARRAY_DISCRIMINANT;

use self::data::TypedArrayRecord;

use super::{
    ArrayBuffer,
    array_buffer::{Ordering, ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
    indexed_collections::typed_array_objects::abstract_operations::{
        try_typed_array_set_element, typed_array_get_element, typed_array_set_element,
    },
    ordinary::{
        caches::{PropertyLookupCache, PropertyOffset},
        ordinary_define_own_property, ordinary_delete, ordinary_get, ordinary_get_own_property,
        ordinary_has_property_entry, ordinary_prevent_extensions, ordinary_set, ordinary_try_get,
        ordinary_try_has_property, ordinary_try_set,
        shape::ObjectShape,
    },
};

pub mod data;

/// ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// A generic TypedArray its concrete type encoded in a type parameter.
pub struct GenericTypedArray<'a, T: Viewable>(
    BaseIndex<'a, TypedArrayRecord<'static>>,
    PhantomData<T>,
);

impl<'ta, T: Viewable> GenericTypedArray<'ta, T> {
    /// \[\[ViewedArrayBuffer]]
    #[inline(always)]
    pub fn get_viewed_array_buffer(self, agent: &Agent) -> ArrayBuffer<'ta> {
        self.into_void_array().get(agent).viewed_array_buffer
    }

    /// \[\[ArrayLength]]
    #[inline(always)]
    pub fn array_length<'a>(self, agent: &'a Agent) -> Option<usize> {
        let array_length = self.into_void_array().get(agent).array_length;
        if array_length == TypedArrayArrayLength::heap() {
            Some(
                *agent
                    .heap
                    .typed_array_array_lengths
                    .get(&self.into_void_array().unbind())
                    .unwrap(),
            )
        } else if array_length == TypedArrayArrayLength::auto() {
            None
        } else {
            Some(array_length.0 as usize)
        }
    }

    /// \[\[ByteLength]]
    #[inline(always)]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = self.into_void_array().get(agent).byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(
                *agent
                    .heap
                    .typed_array_byte_lengths
                    .get(&self.into_void_array().unbind())
                    .unwrap(),
            )
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    /// \[\[ByteOffset]]
    #[inline(always)]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = self.into_void_array().get(agent).byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent
                .heap
                .typed_array_byte_offsets
                .get(&self.into_void_array().unbind())
                .unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    /// Constant to be used only for creating a build-time Self.
    pub(crate) const _DEF: Self = Self(BaseIndex::ZERO, PhantomData);

    /// Convert self into a VoidArray, losing type information.
    #[inline(always)]
    const fn into_void_array(self) -> VoidArray<'ta> {
        GenericTypedArray(self.0, PhantomData)
    }

    /// ### [10.4.5.17 TypedArrayGetElement ( O, index )](https://tc39.es/ecma262/#sec-typedarraygetelement)
    ///
    /// The abstract operation TypedArrayGetElement takes arguments O (a
    /// TypedArray) and index (a Number) and returns a Number, a BigInt,
    /// or undefined.
    #[inline(always)]
    pub(crate) fn get_element(
        self,
        agent: &mut Agent,
        index: i64,
        gc: NoGcScope<'ta, '_>,
    ) -> Option<Numeric<'ta>> {
        typed_array_get_element(agent, self, index, gc)
    }

    /// ### [10.4.5.18 TypedArraySetElement ( O, index, value )](https://tc39.es/ecma262/#sec-typedarraysetelement)
    ///
    /// The abstract operation TypedArraySetElement takes arguments O (a
    /// TypedArray), index (a Number), and value (an ECMAScript language value) and
    /// returns either a normal completion containing unused or a throw completion.
    ///
    /// > Note
    /// >
    /// > This operation always appears to succeed, but it has no effect when
    /// > attempting to write past the end of a TypedArray or to a TypedArray which
    /// > is backed by a detached ArrayBuffer.
    #[inline(always)]
    pub(crate) fn set_element<'gc>(
        self,
        agent: &mut Agent,
        index: i64,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        typed_array_set_element(agent, self, index, value, gc)
    }

    /// ### [10.4.5.18 Infallible TypedArraySetElement ( O, index, value )](https://tc39.es/ecma262/#sec-typedarraysetelement)
    ///
    /// The abstract operation TypedArraySetElement takes arguments O (a
    /// TypedArray), index (a Number), and value (an ECMAScript language value) and
    /// returns either a normal completion containing unused or a throw completion.
    ///
    /// > Note
    /// >
    /// > This operation always appears to succeed, but it has no effect when
    /// > attempting to write past the end of a TypedArray or to a TypedArray which
    /// > is backed by a detached ArrayBuffer.
    #[inline(always)]
    pub(crate) fn try_set_element<'gc>(
        self,
        agent: &mut Agent,
        index: i64,
        value: Value,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, ()> {
        try_typed_array_set_element(agent, self, index, value)
    }

    /// ### [10.4.5.16 IsValidIntegerIndex ( O, index )](https://tc39.es/ecma262/#sec-isvalidintegerindex)
    ///
    /// The abstract operation IsValidIntegerIndex takes arguments O (a TypedArray)
    /// and index (a Number) and returns a Boolean.
    pub(crate) fn is_valid_integer_index(self, agent: &Agent, index: i64) -> Option<usize> {
        is_valid_integer_index_specialised(agent, self, index)
    }

    /// Initialise the heap data of a TypedArray.
    ///
    /// # Safety
    ///
    /// The TypedArray must be newly created; re-initialising is not allowed.
    pub(crate) unsafe fn initialise_data(
        self,
        agent: &mut Agent,
        ab: ArrayBuffer,
        byte_length: ViewedArrayBufferByteLength,
        byte_offset: ViewedArrayBufferByteOffset,
        array_length: TypedArrayArrayLength,
    ) {
        let d = self.into_void_array().get_mut(agent);

        d.viewed_array_buffer = ab;
        d.byte_length = byte_length;
        d.byte_offset = byte_offset;
        d.array_length = array_length;
    }

    pub(crate) fn set_overflowing_byte_offset(self, agent: &mut Agent, byte_offset: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(Uint8Array, usize)>();
        agent
            .heap
            .typed_array_byte_offsets
            .insert(self.into_void_array().unbind(), byte_offset);
    }

    pub(crate) fn set_overflowing_byte_length(self, agent: &mut Agent, byte_length: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(Uint8Array, usize)>();
        agent
            .heap
            .typed_array_byte_lengths
            .insert(self.into_void_array().unbind(), byte_length);
    }

    pub(crate) fn set_overflowing_array_length(self, agent: &mut Agent, array_length: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(Uint8Array, usize)>();
        agent
            .heap
            .typed_array_array_lengths
            .insert(self.into_void_array().unbind(), array_length);
    }
}

/// Type-erased TypedArray; used only as a marker type.
pub(crate) type VoidArray<'a> = GenericTypedArray<'a, ()>;

impl<'gc> VoidArray<'gc> {
    #[inline(always)]
    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> &'a TypedArrayRecord<'gc> {
        self.get_direct(&agent.heap.typed_arrays)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut TypedArrayRecord<'gc> {
        self.get_direct_mut(&mut agent.heap.typed_arrays)
    }

    #[inline(always)]
    fn get_direct<'a>(
        self,
        typed_arrays: &'a [TypedArrayRecord<'static>],
    ) -> &'a TypedArrayRecord<'gc> {
        typed_arrays
            .get(self.get_index())
            .expect("Invalid TypedArray reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        typed_arrays: &'a mut [TypedArrayRecord<'static>],
    ) -> &'a mut TypedArrayRecord<'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<&'a mut TypedArrayRecord<'static>, &'a mut TypedArrayRecord<'gc>>(
                typed_arrays
                    .get_mut(self.get_index())
                    .expect("Invalid TypedArray reference"),
            )
        }
    }
}

pub type Uint8Array<'a> = GenericTypedArray<'a, u8>;
pub type Uint8ClampedArray<'a> = GenericTypedArray<'a, U8Clamped>;
pub type Int8Array<'a> = GenericTypedArray<'a, i8>;
pub type Uint16Array<'a> = GenericTypedArray<'a, u16>;
pub type Int16Array<'a> = GenericTypedArray<'a, i16>;
pub type Uint32Array<'a> = GenericTypedArray<'a, u32>;
pub type Int32Array<'a> = GenericTypedArray<'a, i32>;
pub type BigUint64Array<'a> = GenericTypedArray<'a, u64>;
pub type BigInt64Array<'a> = GenericTypedArray<'a, i64>;
#[cfg(feature = "proposal-float16array")]
pub type Float16Array<'a> = GenericTypedArray<'a, f16>;
pub type Float32Array<'a> = GenericTypedArray<'a, f32>;
pub type Float64Array<'a> = GenericTypedArray<'a, f64>;

impl<T: Viewable> Rootable for GenericTypedArray<'_, T> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            Err(HeapRootData::Uint8Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Uint8Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U8Clamped>() {
            Err(HeapRootData::Uint8ClampedArray(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Uint8ClampedArray>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i8>() {
            Err(HeapRootData::Int8Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Int8Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
            Err(HeapRootData::Uint16Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Uint16Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i16>() {
            Err(HeapRootData::Int16Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Int16Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u32>() {
            Err(HeapRootData::Uint32Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Uint32Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i32>() {
            Err(HeapRootData::Int32Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Int32Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u64>() {
            Err(HeapRootData::BigUint64Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, BigUint64Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i64>() {
            Err(HeapRootData::BigInt64Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, BigInt64Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>() {
            Err(HeapRootData::Float32Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Float32Array>(value.unbind())
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>() {
            Err(HeapRootData::Float64Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'_, T>, Float64Array>(value.unbind())
            }))
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f16>() {
                return Err(HeapRootData::Float16Array(unsafe {
                    core::mem::transmute::<GenericTypedArray<'_, T>, Float16Array>(value.unbind())
                }));
            }
            unreachable!()
        }
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            match heap_data {
                HeapRootData::Uint8Array(ta) => Some(unsafe {
                    core::mem::transmute::<Uint8Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U8Clamped>() {
            match heap_data {
                HeapRootData::Uint8ClampedArray(ta) => Some(unsafe {
                    core::mem::transmute::<Uint8ClampedArray, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i8>() {
            match heap_data {
                HeapRootData::Int8Array(ta) => {
                    Some(unsafe { core::mem::transmute::<Int8Array, GenericTypedArray<'_, T>>(ta) })
                }
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
            match heap_data {
                HeapRootData::Uint16Array(ta) => Some(unsafe {
                    core::mem::transmute::<Uint16Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i16>() {
            match heap_data {
                HeapRootData::Int16Array(ta) => Some(unsafe {
                    core::mem::transmute::<Int16Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u32>() {
            match heap_data {
                HeapRootData::Uint32Array(ta) => Some(unsafe {
                    core::mem::transmute::<Uint32Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i32>() {
            match heap_data {
                HeapRootData::Int32Array(ta) => Some(unsafe {
                    core::mem::transmute::<Int32Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u64>() {
            match heap_data {
                HeapRootData::BigUint64Array(ta) => Some(unsafe {
                    core::mem::transmute::<BigUint64Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i64>() {
            match heap_data {
                HeapRootData::BigInt64Array(ta) => Some(unsafe {
                    core::mem::transmute::<BigInt64Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>() {
            match heap_data {
                HeapRootData::Float32Array(ta) => Some(unsafe {
                    core::mem::transmute::<Float32Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>() {
            match heap_data {
                HeapRootData::Float64Array(ta) => Some(unsafe {
                    core::mem::transmute::<Float64Array, GenericTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f16>() {
                return match heap_data {
                    HeapRootData::Float16Array(ta) => Some(unsafe {
                        core::mem::transmute::<Float16Array, GenericTypedArray<'_, T>>(ta)
                    }),
                    _ => None,
                };
            }
            unreachable!()
        }
    }
}

/// ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// A generic TypedArray viewing a SharedArrayBuffer with its concrete type
/// encoded in a type parameter.
pub struct GenericSharedTypedArray<'a, T: Viewable>(
    BaseIndex<'a, TypedArrayRecord<'static>>,
    PhantomData<T>,
);

/// Type-erased TypedArray viewing a SharedArrayBuffer; used only as a marker
/// type.
pub(crate) type SharedVoidArray<'a> = GenericSharedTypedArray<'a, ()>;

impl<'gc> SharedVoidArray<'gc> {
    #[inline(always)]
    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> &'a SharedTypedArrayRecord<'gc> {
        self.get_direct(&agent.heap.shared_typed_arrays)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut SharedTypedArrayRecord<'gc> {
        self.get_direct_mut(&mut agent.heap.shared_typed_arrays)
    }

    #[inline(always)]
    fn get_direct<'a>(
        self,
        shared_typed_arrays: &'a [SharedTypedArrayRecord<'static>],
    ) -> &'a SharedTypedArrayRecord<'gc> {
        shared_typed_arrays
            .get(self.get_index())
            .expect("Invalid TypedArray reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        shared_typed_arrays: &'a mut [SharedTypedArrayRecord<'static>],
    ) -> &'a mut SharedTypedArrayRecord<'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<
                &'a mut SharedTypedArrayRecord<'static>,
                &'a mut SharedTypedArrayRecord<'gc>,
            >(
                shared_typed_arrays
                    .get_mut(self.get_index())
                    .expect("Invalid TypedArray reference"),
            )
        }
    }
}

pub type SharedUint8Array<'a> = GenericSharedTypedArray<'a, u8>;
pub type SharedUint8ClampedArray<'a> = GenericSharedTypedArray<'a, U8Clamped>;
pub type SharedInt8Array<'a> = GenericSharedTypedArray<'a, i8>;
pub type SharedUint16Array<'a> = GenericSharedTypedArray<'a, u16>;
pub type SharedInt16Array<'a> = GenericSharedTypedArray<'a, i16>;
pub type SharedUint32Array<'a> = GenericSharedTypedArray<'a, u32>;
pub type SharedInt32Array<'a> = GenericSharedTypedArray<'a, i32>;
pub type SharedBigUint64Array<'a> = GenericSharedTypedArray<'a, u64>;
pub type SharedBigInt64Array<'a> = GenericSharedTypedArray<'a, i64>;
#[cfg(feature = "proposal-float16array")]
pub type SharedFloat16Array<'a> = GenericSharedTypedArray<'a, f16>;
pub type SharedFloat32Array<'a> = GenericSharedTypedArray<'a, f32>;
pub type SharedFloat64Array<'a> = GenericSharedTypedArray<'a, f64>;

impl<'a, T: Viewable> GenericSharedTypedArray<'a, T> {
    /// Constant to be used only for creating a build-time Self.
    pub(crate) const _DEF: Self = Self(BaseIndex::ZERO, PhantomData);

    /// Convert self into a VoidArray, losing type information.
    #[inline(always)]
    const fn into_void_array(self) -> SharedVoidArray<'a> {
        GenericSharedTypedArray(self.0, PhantomData)
    }
}

/// ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// A TypedArray presents an array-like view of an underlying binary data
/// buffer (25.1).
///
/// In Nova engine, TypedArrays view an ArrayBuffer. TypedArrays viewing a
/// SharedArrayBuffer are represented by a different type.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TypedArray<'a> {
    Int8Array(Int8Array<'a>) = INT_8_ARRAY_DISCRIMINANT,
    Uint8Array(Uint8Array<'a>) = UINT_8_ARRAY_DISCRIMINANT,
    Uint8ClampedArray(Uint8ClampedArray<'a>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    Int16Array(Int16Array<'a>) = INT_16_ARRAY_DISCRIMINANT,
    Uint16Array(Uint16Array<'a>) = UINT_16_ARRAY_DISCRIMINANT,
    Int32Array(Int32Array<'a>) = INT_32_ARRAY_DISCRIMINANT,
    Uint32Array(Uint32Array<'a>) = UINT_32_ARRAY_DISCRIMINANT,
    BigInt64Array(BigInt64Array<'a>) = BIGINT_64_ARRAY_DISCRIMINANT,
    BigUint64Array(BigUint64Array<'a>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(Float16Array<'a>) = FLOAT_16_ARRAY_DISCRIMINANT,
    Float32Array(Float32Array<'a>) = FLOAT_32_ARRAY_DISCRIMINANT,
    Float64Array(Float64Array<'a>) = FLOAT_64_ARRAY_DISCRIMINANT,
}
bindable_handle!(TypedArray);

impl<'a> TypedArray<'a> {
    #[inline(always)]
    const fn into_void_array(self) -> VoidArray<'a> {
        match self {
            TypedArray::Int8Array(ta) => ta.into_void_array(),
            TypedArray::Uint8Array(ta) => ta.into_void_array(),
            TypedArray::Uint8ClampedArray(ta) => ta.into_void_array(),
            TypedArray::Int16Array(ta) => ta.into_void_array(),
            TypedArray::Uint16Array(ta) => ta.into_void_array(),
            TypedArray::Int32Array(ta) => ta.into_void_array(),
            TypedArray::Uint32Array(ta) => ta.into_void_array(),
            TypedArray::BigInt64Array(ta) => ta.into_void_array(),
            TypedArray::BigUint64Array(ta) => ta.into_void_array(),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(ta) => ta.into_void_array(),
            TypedArray::Float32Array(ta) => ta.into_void_array(),
            TypedArray::Float64Array(ta) => ta.into_void_array(),
        }
    }

    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = self.into_void_array().get(agent).byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(
                *agent
                    .heap
                    .typed_array_byte_lengths
                    .get(&self.into_void_array().unbind())
                    .unwrap(),
            )
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    #[inline]
    pub fn array_length(self, agent: &Agent) -> Option<usize> {
        let array_length = self.into_void_array().get(agent).array_length;
        if array_length == TypedArrayArrayLength::heap() {
            Some(
                *agent
                    .heap
                    .typed_array_array_lengths
                    .get(&self.into_void_array().unbind())
                    .unwrap(),
            )
        } else if array_length == TypedArrayArrayLength::auto() {
            None
        } else {
            Some(array_length.0 as usize)
        }
    }

    #[inline]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = self.into_void_array().get(agent).byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent
                .heap
                .typed_array_byte_offsets
                .get(&self.into_void_array().unbind())
                .unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    #[inline]
    pub fn get_viewed_array_buffer(self, agent: &Agent) -> ArrayBuffer<'a> {
        self.into_void_array().get(agent).viewed_array_buffer
    }
}

impl<'a> From<TypedArray<'a>> for Value<'a> {
    fn from(value: TypedArray<'a>) -> Self {
        match value {
            TypedArray::Int8Array(ta) => Self::Int8Array(ta),
            TypedArray::Uint8Array(ta) => Self::Uint8Array(ta),
            TypedArray::Uint8ClampedArray(ta) => Self::Uint8ClampedArray(ta),
            TypedArray::Int16Array(ta) => Self::Int16Array(ta),
            TypedArray::Uint16Array(ta) => Self::Uint16Array(ta),
            TypedArray::Int32Array(ta) => Self::Int32Array(ta),
            TypedArray::Uint32Array(ta) => Self::Uint32Array(ta),
            TypedArray::BigInt64Array(ta) => Self::BigInt64Array(ta),
            TypedArray::BigUint64Array(ta) => Self::BigUint64Array(ta),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(ta) => Self::Float16Array(ta),
            TypedArray::Float32Array(ta) => Self::Float32Array(ta),
            TypedArray::Float64Array(ta) => Self::Float64Array(ta),
        }
    }
}

impl<'a> From<TypedArray<'a>> for Object<'a> {
    fn from(value: TypedArray<'a>) -> Self {
        match value {
            TypedArray::Int8Array(ta) => Self::Int8Array(ta),
            TypedArray::Uint8Array(ta) => Self::Uint8Array(ta),
            TypedArray::Uint8ClampedArray(ta) => Self::Uint8ClampedArray(ta),
            TypedArray::Int16Array(ta) => Self::Int16Array(ta),
            TypedArray::Uint16Array(ta) => Self::Uint16Array(ta),
            TypedArray::Int32Array(ta) => Self::Int32Array(ta),
            TypedArray::Uint32Array(ta) => Self::Uint32Array(ta),
            TypedArray::BigInt64Array(ta) => Self::BigInt64Array(ta),
            TypedArray::BigUint64Array(ta) => Self::BigUint64Array(ta),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(ta) => Self::Float16Array(ta),
            TypedArray::Float32Array(ta) => Self::Float32Array(ta),
            TypedArray::Float64Array(ta) => Self::Float64Array(ta),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for TypedArray<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Int8Array(base_index) => Ok(Self::Int8Array(base_index)),
            Value::Uint8Array(base_index) => Ok(Self::Uint8Array(base_index)),
            Value::Uint8ClampedArray(base_index) => Ok(Self::Uint8ClampedArray(base_index)),
            Value::Int16Array(base_index) => Ok(Self::Int16Array(base_index)),
            Value::Uint16Array(base_index) => Ok(Self::Uint16Array(base_index)),
            Value::Int32Array(base_index) => Ok(Self::Int32Array(base_index)),
            Value::Uint32Array(base_index) => Ok(Self::Uint32Array(base_index)),
            Value::BigInt64Array(base_index) => Ok(Self::BigInt64Array(base_index)),
            Value::BigUint64Array(base_index) => Ok(Self::BigUint64Array(base_index)),
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(base_index) => Ok(Self::Float16Array(base_index)),
            Value::Float32Array(base_index) => Ok(Self::Float32Array(base_index)),
            Value::Float64Array(base_index) => Ok(Self::Float64Array(base_index)),
            _ => Err(()),
        }
    }
}

impl<'a, T: Viewable> From<GenericTypedArray<'a, T>> for Object<'a> {
    fn from(value: GenericTypedArray<'a, T>) -> Self {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            // SAFETY: type checked.
            Self::Uint8Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, u8>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U8Clamped>() {
            // SAFETY: type checked.
            Self::Uint8ClampedArray(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, U8Clamped>>(
                    value,
                )
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i8>() {
            // SAFETY: type checked.
            Self::Int8Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, i8>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
            // SAFETY: type checked.
            Self::Uint16Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, u16>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i16>() {
            // SAFETY: type checked.
            Self::Int16Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, i16>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u32>() {
            // SAFETY: type checked.
            Self::Uint32Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, u32>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i32>() {
            // SAFETY: type checked.
            Self::Int32Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, i32>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u64>() {
            // SAFETY: type checked.
            Self::BigUint64Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, u64>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i64>() {
            // SAFETY: type checked.
            Self::BigInt64Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, i64>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>() {
            // SAFETY: type checked.
            Self::Float32Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, f32>>(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>() {
            // SAFETY: type checked.
            Self::Float64Array(unsafe {
                core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, f64>>(value)
            })
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f16>() {
                // SAFETY: type checked.
                return Self::Float16Array(unsafe {
                    core::mem::transmute::<GenericTypedArray<'a, T>, GenericTypedArray<'a, f16>>(
                        value,
                    )
                });
            }
            unreachable!()
        }
    }
}

impl<'a, T: Viewable> From<GenericTypedArray<'a, T>> for Value<'a> {
    fn from(value: GenericTypedArray<'a, T>) -> Self {
        let value: Object = value.into();
        value.into()
    }
}

impl<'a, T: Viewable> InternalSlots<'a> for GenericTypedArray<'a, T> {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.into_void_array().unbind().get(agent).object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.into_void_array()
                .unbind()
                .get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        if let Some(bo) = self.get_backing_object(agent) {
            bo.object_shape(agent)
        } else {
            let prototype = self.internal_prototype(agent);
            ObjectShape::get_shape_for_prototype(agent, prototype)
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        if let Some(object_index) = self.into_void_array().get(agent).object_index {
            object_index.internal_prototype(agent)
        } else {
            let intrinsics = agent.current_realm_record().intrinsics();
            let default_proto = match T::PROTO {
                ProtoIntrinsics::BigInt64Array => intrinsics.big_int64_array_prototype(),
                ProtoIntrinsics::BigUint64Array => intrinsics.big_uint64_array_prototype(),
                ProtoIntrinsics::Float32Array => intrinsics.float32_array_prototype(),
                ProtoIntrinsics::Float64Array => intrinsics.float64_array_prototype(),
                ProtoIntrinsics::Int16Array => intrinsics.int16_array_prototype(),
                ProtoIntrinsics::Int32Array => intrinsics.int32_array_prototype(),
                ProtoIntrinsics::Int8Array => intrinsics.int8_array_prototype(),
                ProtoIntrinsics::Uint16Array => intrinsics.uint16_array_prototype(),
                ProtoIntrinsics::Uint32Array => intrinsics.uint32_array_prototype(),
                ProtoIntrinsics::Uint8Array => intrinsics.uint8_array_prototype(),
                ProtoIntrinsics::Uint8ClampedArray => intrinsics.uint8_clamped_array_prototype(),
                _ => unreachable!(),
            };
            Some(default_proto.into_object())
        }
    }
}

impl<'a, T: Viewable> InternalMethods<'a> for GenericTypedArray<'a, T> {
    /// ### [10.4.5.2 Infallible \[\[GetOwnProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-getownproperty)
    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        // 1. NOTE: The extensibility-related invariants specified in 6.1.7.3
        //    do not allow this method to return true when O can gain (or lose
        //    and then regain) properties, which might occur for properties
        //    with integer index names when its underlying buffer is resized.
        if !is_typed_array_fixed_length(agent, self) {
            // 2. If IsTypedArrayFixedLength(O) is false, return false.
            TryResult::Continue(false)
        } else {
            // 3. Return OrdinaryPreventExtensions(O).
            TryResult::Continue(match self.get_backing_object(agent) {
                Some(backing_object) => ordinary_prevent_extensions(agent, backing_object),
                None => {
                    self.internal_set_extensible(agent, false);
                    true
                }
            })
        }
    }

    /// ### [10.4.5.2 Infallible \[\[GetOwnProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-getownproperty)
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        let o = self.bind(gc);
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. Let value be TypedArrayGetElement(O, numericIndex).
            let value = self.get_element(agent, numeric_index.into_i64(), gc);
            if let Some(value) = value {
                // iii. Return the PropertyDescriptor {
                //          [[Value]]: value,
                //          [[Writable]]: true,
                //          [[Enumerable]]: true,
                //          [[Configurable]]: true
                //      }.
                TryResult::Continue(Some(PropertyDescriptor {
                    value: Some(value.into_value().unbind()),
                    writable: Some(true),
                    enumerable: Some(true),
                    configurable: Some(true),
                    ..Default::default()
                }))
            } else {
                // ii. If value is undefined, return undefined.
                TryResult::Continue(None)
            }
        } else {
            // 2. Return OrdinaryGetOwnProperty(O, P).
            TryResult::Continue(o.get_backing_object(agent).and_then(|backing_o| {
                ordinary_get_own_property(
                    agent,
                    o.into_object(),
                    backing_o,
                    property_key,
                    cache,
                    gc,
                )
            }))
        }
    }

    /// ### [10.4.5.3 Infallible \[\[HasProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-hasproperty)
    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, return IsValidIntegerIndex(O, numericIndex).
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            let result = is_valid_integer_index_specialised(agent, self, numeric_index);
            if let Some(result) = result {
                TryHasResult::Custom(
                    result.min(u32::MAX as usize) as u32,
                    self.into_object().bind(gc),
                )
                .into()
            } else {
                TryHasResult::Unset.into()
            }
        } else {
            // 2. Return ? OrdinaryHasProperty(O, P).
            ordinary_try_has_property(
                agent,
                self.into_object(),
                self.get_backing_object(agent),
                property_key,
                cache,
                gc,
            )
        }
    }

    /// ### [10.4.5.3 \[\[HasProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-hasproperty)
    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        if let PropertyKey::Integer(_) = property_key {
            Ok(!matches!(
                self.try_has_property(agent, property_key, None, gc.into_nogc()),
                ControlFlow::Continue(TryHasResult::Unset)
            ))
        } else {
            // 2. Return ? OrdinaryHasProperty(O, P).
            ordinary_has_property_entry(agent, self, property_key, gc)
        }
    }

    /// ### [10.4.5.4 Infallible \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-typedarray-defineownproperty)
    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return false.
            let numeric_index = numeric_index.into_i64();
            let numeric_index = is_valid_integer_index_specialised(agent, self, numeric_index);
            let Some(numeric_index) = numeric_index else {
                return TryResult::Continue(false);
            };
            // ii. If Desc has a [[Configurable]] field and
            //     Desc.[[Configurable]] is false, return false.
            if property_descriptor.configurable == Some(false) {
                return TryResult::Continue(false);
            }
            // iii. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]]
            //      is false, return false.
            if property_descriptor.enumerable == Some(false) {
                return TryResult::Continue(false);
            }
            // iv. If IsAccessorDescriptor(Desc) is true, return false.
            if property_descriptor.is_accessor_descriptor() {
                return TryResult::Continue(false);
            }
            // v. If Desc has a [[Writable]] field and Desc.[[Writable]] is
            //    false, return false.
            if property_descriptor.writable == Some(false) {
                return TryResult::Continue(false);
            }
            // vi. If Desc has a [[Value]] field, perform ?
            //     TypedArraySetElement(O, numericIndex, Desc.[[Value]]).
            if let Some(value) = property_descriptor.value {
                let numeric_index = numeric_index as i64;
                try_typed_array_set_element(agent, self, numeric_index, value)?;
            }
            // vii. Return true.
            TryResult::Continue(true)
        } else {
            // 2. Return ! OrdinaryDefineOwnProperty(O, P, Desc).
            let backing_object = self
                .get_backing_object(agent)
                .unwrap_or_else(|| self.create_backing_object(agent));
            js_result_into_try(ordinary_define_own_property(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                property_descriptor,
                cache,
                gc,
            ))
        }
    }

    /// ### [10.4.5.4 \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-typedarray-defineownproperty)
    fn internal_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let o = self.bind(gc.nogc());
        let property_descriptor = property_descriptor.bind(gc.nogc());
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc.nogc());
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return false.
            let numeric_index = is_valid_integer_index_specialised(agent, o, numeric_index);
            let Some(numeric_index) = numeric_index else {
                return Ok(false);
            };
            // ii. If Desc has a [[Configurable]] field and
            //     Desc.[[Configurable]] is false, return false.
            if property_descriptor.configurable == Some(false) {
                return Ok(false);
            }
            // iii. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]]
            //      is false, return false.
            if property_descriptor.enumerable == Some(false) {
                return Ok(false);
            }
            // iv. If IsAccessorDescriptor(Desc) is true, return false.
            if property_descriptor.is_accessor_descriptor() {
                return Ok(false);
            }
            // v. If Desc has a [[Writable]] field and Desc.[[Writable]] is
            //    false, return false.
            if property_descriptor.writable == Some(false) {
                return Ok(false);
            }
            // vi. If Desc has a [[Value]] field, perform ?
            //     TypedArraySetElement(O, numericIndex, Desc.[[Value]]).
            if let Some(value) = property_descriptor.value {
                let numeric_index = numeric_index as i64;
                o.unbind()
                    .set_element(agent, numeric_index, value.unbind(), gc)?;
            }
            // vii. Return true.
            Ok(true)
        } else {
            // 2. Return ! OrdinaryDefineOwnProperty(O, P, Desc).
            let backing_object = o
                .get_backing_object(agent)
                .unwrap_or_else(|| o.create_backing_object(agent));
            ordinary_define_own_property(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                property_descriptor.unbind(),
                None,
                gc.into_nogc(),
            )
        }
    }

    /// ### [10.4.5.5 Infallible \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-typedarray-get)
    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        let o = self.bind(gc);
        // 1. 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. Return TypedArrayGetElement(O, numericIndex).
            let numeric_index = numeric_index.into_i64();
            let result = o.get_element(agent, numeric_index, gc);
            result
                .map_or(TryGetResult::Unset, |v| TryGetResult::Value(v.into_value()))
                .into()
        } else {
            // 2. Return ? OrdinaryGet(O, P, Receiver).
            ordinary_try_get(
                agent,
                o.into_object(),
                o.get_backing_object(agent),
                property_key,
                receiver,
                cache,
                gc,
            )
        }
    }

    /// ### [10.4.5.5 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-typedarray-get)
    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let o = self.bind(gc.nogc());
        let mut property_key = property_key.bind(gc.nogc());
        let receiver = receiver.bind(gc.nogc());

        // 1. 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc.nogc());
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            Ok(o.unbind()
                .get_element(agent, numeric_index.into_i64(), gc.into_nogc())
                .map_or(Value::Undefined, Numeric::into_value))
        } else {
            // 2. Return ? OrdinaryGet(O, P, Receiver).
            match self.get_backing_object(agent) {
                Some(backing_object) => ordinary_get(
                    agent,
                    backing_object,
                    property_key.unbind(),
                    receiver.unbind(),
                    gc,
                ),
                None => {
                    // a. Let parent be ? O.[[GetPrototypeOf]]().
                    // Note: [[GetPrototypeOf]] of TypedArray cannot call into
                    // JavaScript.
                    let Some(parent) = unwrap_try(o.try_get_prototype_of(agent, gc.nogc())) else {
                        // b. If parent is null, return undefined.
                        return Ok(Value::Undefined);
                    };

                    // c. Return ? parent.[[Get]](P, Receiver).
                    parent.unbind().internal_get(
                        agent,
                        property_key.unbind(),
                        receiver.unbind(),
                        gc,
                    )
                }
            }
        }
    }

    /// ### [10.4.5.6 Infallible \[\[Set\]\] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-typedarray-set)
    fn try_set<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        value: Value,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        let o = self.bind(gc);
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If SameValue(O, Receiver) is true, then
            if self.into_value() == receiver {
                // 1. Perform ? TypedArraySetElement(O, numericIndex, V).
                o.try_set_element(agent, numeric_index, value, gc)?;
                // 2. Return true.
                return SetResult::Done.into();
            } else {
                // ii. If IsValidIntegerIndex(O, numericIndex) is false, return true.
                let result = o.is_valid_integer_index(agent, numeric_index);
                if result.is_none() {
                    return SetResult::Done.into();
                }
            }
        }
        // 2. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(
            agent,
            self.into_object(),
            property_key,
            value,
            receiver,
            cache,
            gc,
        )
    }

    /// ### [10.4.5.6 \[\[Set\]\] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-typedarray-set)
    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let o = self.bind(gc.nogc());
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc.nogc());
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If SameValue(O, Receiver) is true, then
            if self.into_value() == receiver {
                // 1. Perform ? TypedArraySetElement(O, numericIndex, V).
                o.unbind().set_element(agent, numeric_index, value, gc)?;
                // 2. Return true.
                return Ok(true);
            } else {
                // ii. If IsValidIntegerIndex(O, numericIndex) is false, return true.
                let result = o.is_valid_integer_index(agent, numeric_index);
                if result.is_none() {
                    return Ok(true);
                }
            }
        }
        // 2. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    /// ### [10.4.5.7 Infallible \[\[Delete\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-delete)
    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        let o = self.bind(gc);
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        ta_canonical_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return true; else return false.
            let result = o.is_valid_integer_index(agent, numeric_index);
            TryResult::Continue(result.is_none())
        } else {
            // 2. Return ! OrdinaryDelete(O, P).
            TryResult::Continue(self.get_backing_object(agent).is_none_or(|object| {
                ordinary_delete(agent, self.into_object(), object, property_key, gc)
            }))
        }
    }

    /// ### [10.4.5.8 \[\[OwnPropertyKeys\]\] ( )](https://tc39.es/ecma262/#sec-typedarray-ownpropertykeys)
    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        let o = self.bind(gc);
        // 1. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let (o, cached_byte_length) =
            make_typed_array_with_buffer_witness_record_specialised(agent, o, Ordering::SeqCst);
        // 3. If IsTypedArrayOutOfBounds(taRecord) is false, then
        let length = if !is_typed_array_out_of_bounds_specialised(agent, o, cached_byte_length) {
            // a. Let length be TypedArrayLength(taRecord).
            typed_array_length_specialised(agent, o, cached_byte_length)
        } else {
            0
        };
        // 2. Let keys be a new empty List.
        let mut keys = Vec::with_capacity(length);
        // b. For each integer i such that 0 ≤ i < length, in ascending order, do
        // i. Append ! ToString(𝔽(i)) to keys.
        for i in 0..length {
            keys.push(i.try_into().unwrap());
        }
        if let Some(backing_object) = self.get_backing_object(agent) {
            // 4. For each own property key P of O such that P is a String and P is
            //    not an integer index, in ascending chronological order of
            //    property creation, do
            // a. Append P to keys.
            // 5. For each own property key P of O such that P is a Symbol, in
            //    ascending chronological order of property creation, do
            // a. Append P to keys.
            keys.append(&mut unwrap_try(
                backing_object.try_own_property_keys(agent, gc),
            ));
        }
        // 6. Return keys.
        TryResult::Continue(keys)
    }
}

macro_rules! typed_array_delegate {
    ($value: ident, $method: ident, $($arg:expr),*) => {
        match $value {
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

            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedInt8Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedUint8Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedUint8ClampedArray(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedInt16Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedUint16Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedInt32Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedUint32Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedBigInt64Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedBigUint64Array(sta) => sta.$method($($arg),+),
            // #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            // Self::SharedFloat16Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedFloat32Array(sta) => sta.$method($($arg),+),
            // #[cfg(feature = "shared-array-buffer")]
            // Self::SharedFloat64Array(sta) => sta.$method($($arg),+),
        }
    };
}

impl<'a> InternalSlots<'a> for TypedArray<'a> {
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        typed_array_delegate!(self, get_backing_object, agent)
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("TypedArray should not try to set its backing object");
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!("TypedArray should not try to create its backing object");
    }

    fn get_or_create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        typed_array_delegate!(self, get_or_create_backing_object, agent)
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        typed_array_delegate!(self, object_shape, agent)
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        typed_array_delegate!(self, internal_extensible, agent)
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        typed_array_delegate!(self, internal_set_extensible, agent, value)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        typed_array_delegate!(self, internal_prototype, agent)
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        typed_array_delegate!(self, internal_set_prototype, agent, prototype)
    }
}

impl<'a> InternalMethods<'a> for TypedArray<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<Object<'gc>>> {
        typed_array_delegate!(self, try_get_prototype_of, agent, gc)
    }

    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Object<'gc>>> {
        typed_array_delegate!(self, internal_get_prototype_of, agent, gc)
    }

    fn try_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        typed_array_delegate!(self, try_set_prototype_of, agent, prototype, gc)
    }

    fn internal_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        typed_array_delegate!(self, internal_set_prototype_of, agent, prototype, gc)
    }

    fn try_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        typed_array_delegate!(self, try_is_extensible, agent, gc)
    }

    fn internal_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        typed_array_delegate!(self, internal_is_extensible, agent, gc)
    }

    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        typed_array_delegate!(self, try_prevent_extensions, agent, gc)
    }

    fn internal_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        typed_array_delegate!(self, internal_prevent_extensions, agent, gc)
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        typed_array_delegate!(self, try_get_own_property, agent, property_key, cache, gc)
    }

    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
        typed_array_delegate!(self, internal_get_own_property, agent, property_key, gc)
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        typed_array_delegate!(
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
        typed_array_delegate!(
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
        typed_array_delegate!(self, try_has_property, agent, property_key, cache, gc)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        typed_array_delegate!(self, internal_has_property, agent, property_key, gc)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        typed_array_delegate!(self, try_get, agent, property_key, receiver, cache, gc)
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        typed_array_delegate!(self, internal_get, agent, property_key, receiver, gc)
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
        typed_array_delegate!(
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
        typed_array_delegate!(self, internal_set, agent, property_key, value, receiver, gc)
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        typed_array_delegate!(self, try_delete, agent, property_key, gc)
    }

    fn internal_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        typed_array_delegate!(self, internal_delete, agent, property_key, gc)
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        typed_array_delegate!(self, try_own_property_keys, agent, gc)
    }

    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
        typed_array_delegate!(self, internal_own_property_keys, agent, gc)
    }

    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        typed_array_delegate!(self, get_own_property_at_offset, agent, offset, gc)
    }

    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        typed_array_delegate!(self, set_at_offset, agent, props, offset, gc)
    }
}

unsafe impl<T: Viewable> Bindable for GenericTypedArray<'_, T> {
    type Of<'a> = GenericTypedArray<'a, T>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<T: ?Sized + Viewable> Clone for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Viewable> Copy for GenericTypedArray<'_, T> {}

impl<T: ?Sized + Viewable> PartialEq for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: ?Sized + Viewable> Eq for GenericTypedArray<'_, T> {}

impl<T: ?Sized + Viewable> PartialOrd for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized + Viewable> Ord for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized + Viewable> Hash for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: Viewable> core::fmt::Debug for GenericTypedArray<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({})", T::NAME, self.0.into_u32_index())
    }
}

// SHARED

/// ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// A SharedTypedArray presents an array-like view of an underlying binary data
/// buffer (25.1) that can be shared between Agents.
///
/// In Nova engine, SharedTypedArrays view a [SharedArrayBuffer]. TypedArrays
/// viewing an [ArrayBuffer] are represented by a [TypedArray].
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SharedTypedArray<'a> {
    #[expect(dead_code)]
    Int8Array(Int8Array<'a>) = INT_8_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Uint8Array(Uint8Array<'a>) = UINT_8_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Uint8ClampedArray(Uint8ClampedArray<'a>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Int16Array(Int16Array<'a>) = INT_16_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Uint16Array(Uint16Array<'a>) = UINT_16_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Int32Array(Int32Array<'a>) = INT_32_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Uint32Array(Uint32Array<'a>) = UINT_32_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    BigInt64Array(BigInt64Array<'a>) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    BigUint64Array(BigUint64Array<'a>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(Float16Array<'a>) = FLOAT_16_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Float32Array(Float32Array<'a>) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[expect(dead_code)]
    Float64Array(Float64Array<'a>) = FLOAT_64_ARRAY_DISCRIMINANT,
}
bindable_handle!(SharedTypedArray);

impl<'a, T: Viewable> From<GenericSharedTypedArray<'a, T>> for Object<'a> {
    fn from(_value: GenericSharedTypedArray<'a, T>) -> Self {
        todo!()
    }
}

impl<'a, T: Viewable> From<GenericSharedTypedArray<'a, T>> for Value<'a> {
    fn from(_value: GenericSharedTypedArray<'a, T>) -> Self {
        todo!()
    }
}

impl<'a, T: Viewable> InternalSlots<'a> for GenericSharedTypedArray<'a, T> {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.into_void_array().unbind().get(agent).object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.into_void_array()
                .unbind()
                .get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        if let Some(bo) = self.get_backing_object(agent) {
            bo.object_shape(agent)
        } else {
            let prototype = self.internal_prototype(agent);
            ObjectShape::get_shape_for_prototype(agent, prototype)
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        if let Some(object_index) = self.into_void_array().get(agent).object_index {
            object_index.internal_prototype(agent)
        } else {
            let intrinsics = agent.current_realm_record().intrinsics();
            let default_proto = match T::PROTO {
                ProtoIntrinsics::BigInt64Array => intrinsics.big_int64_array_prototype(),
                ProtoIntrinsics::BigUint64Array => intrinsics.big_uint64_array_prototype(),
                ProtoIntrinsics::Float32Array => intrinsics.float32_array_prototype(),
                ProtoIntrinsics::Float64Array => intrinsics.float64_array_prototype(),
                ProtoIntrinsics::Int16Array => intrinsics.int16_array_prototype(),
                ProtoIntrinsics::Int32Array => intrinsics.int32_array_prototype(),
                ProtoIntrinsics::Int8Array => intrinsics.int8_array_prototype(),
                ProtoIntrinsics::Uint16Array => intrinsics.uint16_array_prototype(),
                ProtoIntrinsics::Uint32Array => intrinsics.uint32_array_prototype(),
                ProtoIntrinsics::Uint8Array => intrinsics.uint8_array_prototype(),
                ProtoIntrinsics::Uint8ClampedArray => intrinsics.uint8_clamped_array_prototype(),
                _ => unreachable!(),
            };
            Some(default_proto.into_object())
        }
    }
}

impl<'a, T: Viewable> InternalMethods<'a> for GenericSharedTypedArray<'a, T> {}

unsafe impl<T: Viewable> Bindable for GenericSharedTypedArray<'_, T> {
    type Of<'a> = GenericSharedTypedArray<'a, T>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<T: ?Sized + Viewable> Clone for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Viewable> Copy for GenericSharedTypedArray<'_, T> {}

impl<T: ?Sized + Viewable> PartialEq for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: ?Sized + Viewable> Eq for GenericSharedTypedArray<'_, T> {}

impl<T: ?Sized + Viewable> PartialOrd for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized + Viewable> Ord for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized + Viewable> Hash for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: Viewable> core::fmt::Debug for GenericSharedTypedArray<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Shared{}({})", T::NAME, self.0.into_u32_index())
    }
}

impl TryFrom<HeapRootData> for TypedArray<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::Int8Array(ta) => Ok(Self::Int8Array(ta)),
            HeapRootData::Uint8Array(ta) => Ok(Self::Uint8Array(ta)),
            HeapRootData::Uint8ClampedArray(ta) => Ok(Self::Uint8ClampedArray(ta)),
            HeapRootData::Int16Array(ta) => Ok(Self::Int16Array(ta)),
            HeapRootData::Uint16Array(ta) => Ok(Self::Uint16Array(ta)),
            HeapRootData::Int32Array(ta) => Ok(Self::Int32Array(ta)),
            HeapRootData::Uint32Array(ta) => Ok(Self::Uint32Array(ta)),
            HeapRootData::BigInt64Array(ta) => Ok(Self::BigInt64Array(ta)),
            HeapRootData::BigUint64Array(ta) => Ok(Self::BigUint64Array(ta)),
            // HeapRootData::Float16Array(ta) => Ok(Self::Float16Array(ta)),
            HeapRootData::Float32Array(ta) => Ok(Self::Float32Array(ta)),
            HeapRootData::Float64Array(ta) => Ok(Self::Float64Array(ta)),
            _ => Err(()),
        }
    }
}

impl<'a, T: Viewable> CreateHeapData<TypedArrayRecord<'a>, GenericTypedArray<'a, T>> for Heap {
    fn create(&mut self, data: TypedArrayRecord<'a>) -> GenericTypedArray<'a, T> {
        self.typed_arrays.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<TypedArrayRecord<'static>>();
        // TODO: The type should be checked based on data or something equally stupid
        GenericTypedArray(BaseIndex::last_t(&self.typed_arrays), PhantomData)
    }
}

impl<'a> TryFrom<Object<'a>> for TypedArray<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::Uint8Array(t) => Ok(Self::Uint8Array(t)),
            Object::Int8Array(t) => Ok(Self::Int8Array(t)),
            Object::Uint8ClampedArray(t) => Ok(Self::Uint8ClampedArray(t)),
            Object::Int16Array(t) => Ok(Self::Int16Array(t)),
            Object::Uint16Array(t) => Ok(Self::Uint16Array(t)),
            Object::Int32Array(t) => Ok(Self::Int32Array(t)),
            Object::Uint32Array(t) => Ok(Self::Uint32Array(t)),
            Object::BigInt64Array(t) => Ok(Self::BigInt64Array(t)),
            Object::BigUint64Array(t) => Ok(Self::BigUint64Array(t)),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(t) => Ok(Self::Float16Array(t)),
            Object::Float32Array(t) => Ok(Self::Float32Array(t)),
            Object::Float64Array(t) => Ok(Self::Float64Array(t)),
            _ => Err(()),
        }
    }
}

impl<T: Viewable> HeapMarkAndSweep for GenericTypedArray<'static, T> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.typed_arrays.push(self.into_void_array());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.typed_arrays.shift_index(&mut self.0);
    }
}

impl<T: Viewable> HeapSweepWeakReference for GenericTypedArray<'static, T> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .typed_arrays
            .shift_weak_index(self.0)
            .map(|i| GenericTypedArray(i, PhantomData))
    }
}

impl<T: Viewable> HeapMarkAndSweep for GenericSharedTypedArray<'static, T> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.shared_typed_arrays.push(self.into_void_array());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.shared_typed_arrays.shift_index(&mut self.0);
    }
}

impl<T: Viewable> HeapSweepWeakReference for GenericSharedTypedArray<'static, T> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .shared_typed_arrays
            .shift_weak_index(self.0)
            .map(|i| GenericSharedTypedArray(i, PhantomData))
    }
}

impl HeapMarkAndSweep for TypedArray<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            TypedArray::Int8Array(ta) => ta.mark_values(queues),
            TypedArray::Uint8Array(ta) => ta.mark_values(queues),
            TypedArray::Uint8ClampedArray(ta) => ta.mark_values(queues),
            TypedArray::Int16Array(ta) => ta.mark_values(queues),
            TypedArray::Uint16Array(ta) => ta.mark_values(queues),
            TypedArray::Int32Array(ta) => ta.mark_values(queues),
            TypedArray::Uint32Array(ta) => ta.mark_values(queues),
            TypedArray::BigInt64Array(ta) => ta.mark_values(queues),
            TypedArray::BigUint64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(ta) => ta.mark_values(queues),
            TypedArray::Float32Array(ta) => ta.mark_values(queues),
            TypedArray::Float64Array(ta) => ta.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            TypedArray::Int8Array(ta) => ta.sweep_values(compactions),
            TypedArray::Uint8Array(ta) => ta.sweep_values(compactions),
            TypedArray::Uint8ClampedArray(ta) => ta.sweep_values(compactions),
            TypedArray::Int16Array(ta) => ta.sweep_values(compactions),
            TypedArray::Uint16Array(ta) => ta.sweep_values(compactions),
            TypedArray::Int32Array(ta) => ta.sweep_values(compactions),
            TypedArray::Uint32Array(ta) => ta.sweep_values(compactions),
            TypedArray::BigInt64Array(ta) => ta.sweep_values(compactions),
            TypedArray::BigUint64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(ta) => ta.sweep_values(compactions),
            TypedArray::Float32Array(ta) => ta.sweep_values(compactions),
            TypedArray::Float64Array(ta) => ta.sweep_values(compactions),
        }
    }
}

/// Canonicalize the given property key if it is a numeric string key.
fn ta_canonical_numeric_index_string(agent: &mut Agent, p: &mut PropertyKey, gc: NoGcScope) {
    let Ok(numeric_index) = String::try_from(unsafe { p.into_value_unchecked() }) else {
        return;
    };
    let numeric_index = canonical_numeric_index_string(agent, numeric_index, gc);
    let Some(numeric_index) = numeric_index else {
        return;
    };
    if let Number::Integer(numeric_index) = numeric_index {
        // Got proper integer index.
        *p = PropertyKey::Integer(numeric_index);
    } else {
        // Non-integer index: this should pass into the "!IsValidIntegerIndex"
        // code path. Negative indexes are always invalid so we use that.
        *p = PropertyKey::Integer((-1i32).into())
    };
}
