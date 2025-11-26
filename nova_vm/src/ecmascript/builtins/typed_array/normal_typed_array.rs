// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{
    hash::{Hash, Hasher},
    hint::{assert_unchecked, unreachable_unchecked},
    marker::PhantomData,
};

use ecmascript_atomics::Ordering;

#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::types::FLOAT_16_ARRAY_DISCRIMINANT;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::types::SharedDataBlock;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, set},
            type_conversion::{
                to_big_int, to_big_int_primitive, to_boolean, to_number, to_number_primitive,
            },
        },
        builtins::{
            ArgumentsList, ArrayBuffer,
            array_buffer::{
                AnyArrayBuffer, ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset,
            },
            indexed_collections::typed_array_objects::abstract_operations::{
                CachedBufferByteLength, TypedArrayAbstractOperations,
                typed_array_create_from_data_block, typed_array_species_create_with_length,
            },
            ordinary::{
                caches::{PropertyLookupCache, PropertyOffset},
                ordinary_define_own_property, ordinary_delete, ordinary_get,
                ordinary_get_own_property, ordinary_has_property_entry,
                ordinary_prevent_extensions, ordinary_set, ordinary_try_get,
                ordinary_try_has_property, ordinary_try_set,
                shape::ObjectShape,
            },
            typed_array::{
                AnyTypedArray, canonicalize_numeric_index_string,
                data::{TypedArrayArrayLength, TypedArrayRecord},
            },
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics,
            agent::{JsError, TryError, TryResult, js_result_into_try, unwrap_try},
        },
        types::{
            BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT, BigInt, DataBlock,
            FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, Function,
            INT_8_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
            InternalMethods, InternalSlots, IntoNumeric, IntoObject, IntoValue, Number, Numeric,
            Object, OrdinaryObject, Primitive, PropertyDescriptor, PropertyKey, SetCachedProps,
            SetResult, TryGetResult, TryHasResult, U8Clamped, UINT_8_ARRAY_DISCRIMINANT,
            UINT_8_CLAMPED_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
            UINT_32_ARRAY_DISCRIMINANT, Value, Viewable, create_byte_data_block,
        },
    },
    engine::{
        Scoped,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable, Scopable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

/// ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// A generic TypedArray its concrete type encoded in a type parameter.
pub struct GenericTypedArray<'a, T: Viewable>(
    BaseIndex<'a, TypedArrayRecord<'static>>,
    PhantomData<T>,
);

impl<'ta, T: Viewable> GenericTypedArray<'ta, T> {
    /// Constant to be used only for creating a build-time Self.
    pub(crate) const _DEF: Self = Self(BaseIndex::ZERO, PhantomData);

    /// Convert self into a VoidArray, losing type information.
    #[inline(always)]
    const fn into_void_array(self) -> VoidArray<'ta> {
        GenericTypedArray(self.0, PhantomData)
    }

    #[inline(always)]
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    fn check_not_void_array() {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<()>() {
            panic!("Invalid GenericTypedArray invocation using void type");
        }
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
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        Self::check_not_void_array();

        let mut o = self.bind(gc.nogc());
        let value = value.bind(gc.nogc());
        let num_value = if T::IS_BIGINT {
            // 1. If O.[[ContentType]] is bigint, let numValue be ? ToBigInt(value).
            if let Ok(bigint) = BigInt::try_from(value) {
                bigint.into_numeric()
            } else {
                let scoped_o = o.scope(agent, gc.nogc());
                let bigint = to_big_int(agent, value.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // SAFETY: not shared.
                o = unsafe { scoped_o.take(agent) }.bind(gc.nogc());
                bigint.into_numeric()
            }
        } else {
            // 2. Otherwise, let numValue be ? ToNumber(value).
            if let Ok(number) = Number::try_from(value) {
                number.into_numeric()
            } else {
                let scoped_o = o.scope(agent, gc.nogc());
                let number = to_number(agent, value.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // SAFETY: not shared.
                o = unsafe { scoped_o.take(agent) }.bind(gc.nogc());
                number.into_numeric()
            }
        };
        o.typed_array_set_element(agent, index, num_value);
        Ok(())
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
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, ()> {
        Self::check_not_void_array();

        let o = self.bind(gc);
        let value = value.bind(gc);
        let Ok(value) = Primitive::try_from(value) else {
            return TryError::GcError.into();
        };
        let num_value = if T::IS_BIGINT {
            // 1. If O.[[ContentType]] is bigint, let numValue be ? ToBigInt(value).
            js_result_into_try(to_big_int_primitive(agent, value, gc))?.into_numeric()
        } else {
            // 2. Otherwise, let numValue be ? ToNumber(value).
            js_result_into_try(to_number_primitive(agent, value, gc))?.into_numeric()
        };
        o.typed_array_set_element(agent, index, num_value);
        TryResult::Continue(())
    }

    #[inline(always)]
    pub(crate) fn as_slice(self, agent: &Agent) -> &[T] {
        Self::check_not_void_array();

        let key = self.into_void_array();
        let data = self.into_void_array().get(agent);
        let buffer = data.viewed_array_buffer;
        let byte_slice = buffer.as_slice(agent);
        let byte_offset = data.get_byte_offset(key, &agent.heap.typed_array_byte_offsets);
        let byte_length = data.get_byte_length(key, &agent.heap.typed_array_byte_lengths);
        let byte_limit = byte_length.map(|byte_length| byte_offset.saturating_add(byte_length));
        if byte_limit.unwrap_or(byte_offset) > byte_slice.len() {
            return &[];
        }
        let byte_slice = if let Some(byte_limit) = byte_limit {
            &byte_slice[byte_offset..byte_limit]
        } else {
            &byte_slice[byte_offset..]
        };
        // SAFETY: All bytes in byte_slice are initialized, and all bitwise
        // combinations of T are valid values. Alignment of T's is
        // guaranteed by align_to_mut itself.
        let (head, slice, _) = unsafe { byte_slice.align_to::<T>() };
        if !head.is_empty() {
            panic!("TypedArray is not properly aligned");
        }
        slice
    }

    #[inline(always)]
    pub(crate) fn as_mut_slice(self, agent: &mut Agent) -> &mut [T] {
        Self::check_not_void_array();

        let key = self.into_void_array();
        let data = self.into_void_array().get(agent);
        let buffer = data.viewed_array_buffer;
        let byte_offset = data.get_byte_offset(key, &agent.heap.typed_array_byte_offsets);
        let byte_length = data.get_byte_length(key, &agent.heap.typed_array_byte_lengths);
        let byte_limit = byte_length.map(|byte_length| byte_offset.saturating_add(byte_length));
        let byte_slice = buffer.as_mut_slice(agent);
        if byte_limit.unwrap_or(byte_offset) > byte_slice.len() {
            return &mut [];
        }
        let byte_slice = if let Some(byte_limit) = byte_limit {
            &mut byte_slice[byte_offset..byte_limit]
        } else {
            &mut byte_slice[byte_offset..]
        };
        // SAFETY: All bytes in byte_slice are initialized, and all bitwise
        // combinations of T are valid values. Alignment of T's is
        // guaranteed by align_to_mut itself.
        let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
        if !head.is_empty() {
            panic!("TypedArray is not properly aligned");
        }
        slice
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
        byte_offset: usize,
        byte_and_array_length: Option<(usize, usize)>,
    ) {
        let heap_byte_offset = byte_offset.into();
        let d = self.into_void_array().get_mut(agent);
        d.viewed_array_buffer = ab;
        d.byte_offset = heap_byte_offset;

        if let Some((byte_length, array_length)) = byte_and_array_length {
            let heap_byte_length = byte_length.into();
            let heap_array_length = array_length.into();

            d.byte_length = heap_byte_length;
            d.array_length = heap_array_length;

            if heap_byte_length.is_overflowing() {
                self.set_overflowing_byte_length(agent, byte_length);
                // Note: if byte length doesn't overflow then array length cannot
                // overflow either.
                if heap_array_length.is_overflowing() {
                    self.set_overflowing_array_length(agent, array_length);
                }
            }
        } else {
            d.byte_length = ViewedArrayBufferByteLength::auto();
            d.array_length = TypedArrayArrayLength::auto();
        }

        if heap_byte_offset.is_overflowing() {
            self.set_overflowing_byte_offset(agent, byte_offset);
        }
    }

    pub(crate) fn set_overflowing_byte_offset(self, agent: &mut Agent, byte_offset: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(VoidArray, usize)>();
        agent
            .heap
            .typed_array_byte_offsets
            .insert(self.into_void_array().unbind(), byte_offset);
    }

    pub(crate) fn set_overflowing_byte_length(self, agent: &mut Agent, byte_length: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(VoidArray, usize)>();
        agent
            .heap
            .typed_array_byte_lengths
            .insert(self.into_void_array().unbind(), byte_length);
    }

    pub(crate) fn set_overflowing_array_length(self, agent: &mut Agent, array_length: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(VoidArray, usize)>();
        agent
            .heap
            .typed_array_array_lengths
            .insert(self.into_void_array().unbind(), array_length);
    }
}

/// Type-erased TypedArray; used only as a marker type.
pub(crate) type VoidArray<'a> = GenericTypedArray<'a, ()>;

impl<'gc> VoidArray<'gc> {
    /// Cast a VoidArray into a concrete TypedArray.
    ///
    /// # Safety
    ///
    /// The concrete type has to match the [[ArrayLength]] / [[ByteLength]]
    /// relation in the heap data, and any previous representations of the
    /// TypedArray.
    pub(crate) unsafe fn cast<T: Viewable>(self) -> GenericTypedArray<'gc, T> {
        GenericTypedArray(self.0, PhantomData)
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

macro_rules! for_normal_typed_array {
    ($value: ident, $ta: ident, $expr: expr) => {
        for_normal_typed_array($value, $ta, $expr, TA)
    };
    ($value: ident, $ta: ident, $expr: expr, $TA: ident) => {
        match $value {
            TypedArray::Int8Array($ta) => {
                type $TA = i8;
                $expr
            }
            TypedArray::Uint8Array($ta) => {
                type $TA = u8;
                $expr
            }
            TypedArray::Uint8ClampedArray($ta) => {
                type $TA = U8Clamped;
                $expr
            }
            TypedArray::Int16Array($ta) => {
                type $TA = i16;
                $expr
            }
            TypedArray::Uint16Array($ta) => {
                type $TA = u16;
                $expr
            }
            TypedArray::Int32Array($ta) => {
                type $TA = i32;
                $expr
            }
            TypedArray::Uint32Array($ta) => {
                type $TA = u32;
                $expr
            }
            TypedArray::BigInt64Array($ta) => {
                type $TA = i64;
                $expr
            }
            TypedArray::BigUint64Array($ta) => {
                type $TA = u64;
                $expr
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array($ta) => {
                type $TA = f16;
                $expr
            }
            TypedArray::Float32Array($ta) => {
                type $TA = f32;
                $expr
            }
            TypedArray::Float64Array($ta) => {
                type $TA = f64;
                $expr
            }
        }
    };
}
pub(crate) use for_normal_typed_array;

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

    /// \[\[ByteOffset]]
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
    #[inline(always)]
    fn from(value: TypedArray<'a>) -> Self {
        value.into_object().into_value()
    }
}

impl<'a> From<TypedArray<'a>> for Object<'a> {
    #[inline(always)]
    fn from(value: TypedArray<'a>) -> Self {
        let value: AnyTypedArray = value.into();
        value.into_object()
    }
}

impl<'a> From<TypedArray<'a>> for AnyTypedArray<'a> {
    #[inline(always)]
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

impl<'a> TryFrom<AnyTypedArray<'a>> for TypedArray<'a> {
    type Error = ();

    fn try_from(value: AnyTypedArray<'a>) -> Result<Self, Self::Error> {
        match value {
            AnyTypedArray::Int8Array(base_index) => Ok(Self::Int8Array(base_index)),
            AnyTypedArray::Uint8Array(base_index) => Ok(Self::Uint8Array(base_index)),
            AnyTypedArray::Uint8ClampedArray(base_index) => Ok(Self::Uint8ClampedArray(base_index)),
            AnyTypedArray::Int16Array(base_index) => Ok(Self::Int16Array(base_index)),
            AnyTypedArray::Uint16Array(base_index) => Ok(Self::Uint16Array(base_index)),
            AnyTypedArray::Int32Array(base_index) => Ok(Self::Int32Array(base_index)),
            AnyTypedArray::Uint32Array(base_index) => Ok(Self::Uint32Array(base_index)),
            AnyTypedArray::BigInt64Array(base_index) => Ok(Self::BigInt64Array(base_index)),
            AnyTypedArray::BigUint64Array(base_index) => Ok(Self::BigUint64Array(base_index)),
            #[cfg(feature = "proposal-float16array")]
            AnyTypedArray::Float16Array(base_index) => Ok(Self::Float16Array(base_index)),
            AnyTypedArray::Float32Array(base_index) => Ok(Self::Float32Array(base_index)),
            AnyTypedArray::Float64Array(base_index) => Ok(Self::Float64Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            _ => Err(()),
        }
    }
}

impl<'a, T: Viewable> From<GenericTypedArray<'a, T>> for TypedArray<'a> {
    #[inline(always)]
    fn from(value: GenericTypedArray<'a, T>) -> Self {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            // SAFETY: type checked.
            Self::Uint8Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U8Clamped>() {
            // SAFETY: type checked.
            Self::Uint8ClampedArray(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i8>() {
            // SAFETY: type checked.
            Self::Int8Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
            // SAFETY: type checked.
            Self::Uint16Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i16>() {
            // SAFETY: type checked.
            Self::Int16Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u32>() {
            // SAFETY: type checked.
            Self::Uint32Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i32>() {
            // SAFETY: type checked.
            Self::Int32Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u64>() {
            // SAFETY: type checked.
            Self::BigUint64Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i64>() {
            // SAFETY: type checked.
            Self::BigInt64Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>() {
            // SAFETY: type checked.
            Self::Float32Array(unsafe { value.into_void_array().cast() })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>() {
            // SAFETY: type checked.
            Self::Float64Array(unsafe { value.into_void_array().cast() })
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f16>() {
                // SAFETY: type checked.
                return Self::Float16Array(unsafe { value.into_void_array().cast() });
            }
            unreachable!()
        }
    }
}

impl<'a, T: Viewable> From<GenericTypedArray<'a, T>> for AnyTypedArray<'a> {
    #[inline(always)]
    fn from(value: GenericTypedArray<'a, T>) -> Self {
        let value: TypedArray = value.into();
        value.into()
    }
}

impl<'a, T: Viewable> From<GenericTypedArray<'a, T>> for Object<'a> {
    #[inline(always)]
    fn from(value: GenericTypedArray<'a, T>) -> Self {
        let value: AnyTypedArray = value.into();
        value.into()
    }
}

impl<'a, T: Viewable> From<GenericTypedArray<'a, T>> for Value<'a> {
    #[inline(always)]
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
        if !self.is_typed_array_fixed_length(agent) {
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. Let value be TypedArrayGetElement(O, numericIndex).
            let value = self.typed_array_get_element(agent, numeric_index.into_i64(), gc);
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, return IsValidIntegerIndex(O, numericIndex).
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            let result = self.is_valid_integer_index(agent, numeric_index);
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
        mut property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        canonicalize_numeric_index_string(agent, &mut property_key, gc.nogc());
        // b. If numericIndex is not undefined, return IsValidIntegerIndex(O, numericIndex).
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            Ok(self.is_valid_integer_index(agent, numeric_index).is_some())
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return false.
            let numeric_index = numeric_index.into_i64();
            let numeric_index = self.is_valid_integer_index(agent, numeric_index);
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
                self.try_set_element(agent, numeric_index, value, gc)?;
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc.nogc());
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return false.
            let numeric_index = o.is_valid_integer_index(agent, numeric_index);
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc);
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. Return TypedArrayGetElement(O, numericIndex).
            let numeric_index = numeric_index.into_i64();
            let result = o.typed_array_get_element(agent, numeric_index, gc);
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc.nogc());
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            Ok(o.unbind()
                .typed_array_get_element(agent, numeric_index.into_i64(), gc.into_nogc())
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc);
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc.nogc());
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
        canonicalize_numeric_index_string(agent, &mut property_key, gc);
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
        let cached_byte_length = o.get_cached_buffer_byte_length(agent, Ordering::SeqCst);
        // 3. If IsTypedArrayOutOfBounds(taRecord) is false, then
        let length = if !o.is_typed_array_out_of_bounds(agent, cached_byte_length) {
            // a. Let length be TypedArrayLength(taRecord).
            o.typed_array_length(agent, cached_byte_length)
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
            Self::Int8Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.$method($($arg),*),
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

impl<'a, T: Viewable> TypedArrayAbstractOperations<'a> for GenericTypedArray<'a, T> {
    type ElementType = T;

    #[inline(always)]
    fn is_detached(self, agent: &Agent) -> bool {
        self.into_void_array()
            .get(agent)
            .viewed_array_buffer
            .is_detached(agent)
    }

    #[inline(always)]
    fn is_fixed_length(self, agent: &Agent) -> bool {
        !self
            .into_void_array()
            .get(agent)
            .viewed_array_buffer
            .is_resizable(agent)
    }

    #[inline(always)]
    fn is_shared(self) -> bool {
        false
    }

    /// \[\[ByteOffset]]
    #[inline(always)]
    fn byte_offset(self, agent: &Agent) -> usize {
        let ta = self.into_void_array();
        ta.get(agent)
            .get_byte_offset(ta, &agent.heap.typed_array_byte_offsets)
    }

    fn byte_length(self, agent: &Agent) -> Option<usize> {
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

    fn array_length(self, agent: &Agent) -> Option<usize> {
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

    #[inline(always)]
    fn typed_array_element_size(self) -> usize {
        size_of::<T>()
    }

    fn viewed_array_buffer(self, agent: &Agent) -> AnyArrayBuffer<'a> {
        self.into_void_array().get(agent).viewed_array_buffer.into()
    }

    fn get_cached_buffer_byte_length(self, agent: &Agent, _: Ordering) -> CachedBufferByteLength {
        // 1. Let buffer be obj.[[ViewedArrayBuffer]].
        let buffer = self.into_void_array().get(agent).viewed_array_buffer;

        // 2. If IsDetachedBuffer(buffer) is true, then
        if buffer.is_detached(agent) {
            // a. Let byteLength be detached.
            CachedBufferByteLength::detached()
        } else {
            // 3. Else,
            // a. Let byteLength be ArrayBufferByteLength(buffer, order).
            CachedBufferByteLength::value(buffer.byte_length(agent))
        }
        // 4. Return [[CachedBufferByteLength]]: byteLength.
    }

    fn copy_within<'gc>(
        self,
        agent: &mut Agent,
        start_index: usize,
        target_index: usize,
        count: usize,
    ) {
        let slice = self.as_mut_slice(agent);
        slice.copy_within(start_index..start_index + count, target_index);
        // let before_len = len as usize;
        // if before_len != slice.len() {
        //     let end_bound = (len - target_index).max(0).min(before_len - target_index);
        //     slice.copy_within(start_bound..end_bound, target_index);
        //     return Ok(ta);
        // }
        // if end_bound > 0 {
        //     slice.copy_within(start_bound..start_bound + end_bound, target_index);
        // }
        // Ok(ta)
    }

    fn fill(self, agent: &mut Agent, value: Numeric, start_index: usize, count: usize) {
        let value = T::from_ne_value(agent, value);
        let slice = self.as_mut_slice(agent);
        slice[start_index..start_index + count].fill(value);
    }

    fn filter<'gc>(
        self,
        agent: &mut Agent,
        callback: Function,
        this_arg: Value,
        len: usize,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let o = self.bind(gc.nogc());
        let callback = callback.scope(agent, gc.nogc());
        let this_arg = this_arg.scope(agent, gc.nogc());
        let scoped_o = o.scope(agent, gc.nogc());

        let byte_offset = o.byte_offset(agent);
        let byte_length = o.byte_length(agent);
        let buffer = o.into_void_array().get(agent).viewed_array_buffer;
        let scoped_buffer = buffer.scope(agent, gc.nogc());

        // 5. Let kept be a new empty List.
        let mut kept = create_byte_data_block(
            agent,
            (len as u64).saturating_mul(size_of::<T>() as u64),
            gc.nogc(),
        )
        .unbind()?
        .bind(gc.nogc());
        // SAFETY: All viewable types are trivially transmutable.
        let (head, kept_slice, _) = unsafe { kept.align_to_mut::<T>() };
        // Should be properly aligned for all T.
        assert!(head.is_empty());

        // 6. Let captured be 0.
        let mut captured = 0;
        // 7. Let k be 0.
        // 8. Repeat, while k < len,
        for k in 0..len {
            let slice =
                scoped_buffer
                    .get(agent)
                    .as_viewable_slice::<T>(agent, byte_offset, byte_length);
            let value = slice.get(k).copied();
            // b. Let kValue be ! Get(O, Pk).
            let k_value = value.map_or(Value::Undefined, |v| {
                v.into_le_value(agent, gc.nogc()).into_value()
            });
            let result = call_function(
                agent,
                callback.get(agent),
                this_arg.get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    k_value.unbind(),
                    Number::try_from(k).unwrap().into_value(),
                    scoped_o.get(agent).into_value(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let selected = to_boolean(agent, result);
            if selected {
                kept_slice[captured] = value.unwrap_or(T::default());
                captured += 1;
            }
        }
        // 9. Let A be ? TypedArraySpeciesCreate(O, « 𝔽(captured) »).
        let a = typed_array_species_create_with_length(
            agent,
            unsafe { scoped_o.take(agent) }.unbind().into(),
            captured,
            gc.reborrow(),
        )
        .unbind()?;
        let gc = gc.into_nogc();
        let a = a.bind(gc);

        if captured != len {
            kept.realloc(captured * size_of::<T>())
        }

        let byte_offset = a.byte_offset(agent);
        let expected_byte_length = captured * size_of::<T>();
        let buffer = a.viewed_array_buffer(agent);
        match buffer {
            AnyArrayBuffer::ArrayBuffer(ab) => {
                let is_resizable = buffer.is_resizable(agent);
                let byte_length = buffer.byte_length(agent, Ordering::Unordered);

                if byte_offset == 0 && !is_resizable && byte_length == expected_byte_length {
                    // User cannot detect the switcharoo!
                    let db = agent[ab].get_data_block_mut();
                    core::mem::swap(db, &mut kept);
                } else {
                    // SAFETY: All viewable types are trivially transmutable.
                    let (head, dst, _) = unsafe {
                        agent[ab].get_data_block_mut()[byte_offset..].align_to_mut::<T>()
                    };
                    assert!(head.is_empty());
                    // SAFETY: All viewable types are trivially transmutable.
                    let (head, kept_slice, _) = unsafe { kept.align_to::<T>() };
                    assert!(head.is_empty());
                    dst[..captured].copy_from_slice(&kept_slice[..captured]);
                }
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(sab) => {
                let slice = sab
                    .as_slice(agent)
                    .slice_from(byte_offset)
                    .slice_to(expected_byte_length);
                slice.copy_from_slice(&kept[..expected_byte_length]);
            }
        }

        // 12. Return A.
        Ok(a.into_value())
    }

    fn search<const ASCENDING: bool>(
        self,
        agent: &mut Agent,
        search_element: Value,
        start: usize,
        end: usize,
    ) -> Option<usize> {
        let search_element = T::try_from_value(agent, search_element)?;
        let slice = self.as_slice(agent);

        if ASCENDING {
            // Length of the TypedArray may have changed between when we measured it
            // and here: We'll never try to access past the boundary of the slice if
            // the backing ArrayBuffer shrank.
            let end = end.min(slice.len());
            if start >= end {
                return None;
            }
            slice[start..end]
                .iter()
                .position(|&r| r == search_element)
                .map(|pos| pos + start)
        } else {
            let end = start.saturating_add(1).min(slice.len());
            slice[..end].iter().rposition(|&r| r == search_element)
        }
    }

    fn map<'gc>(
        self,
        agent: &mut Agent,
        callback_fn: Function,
        this_arg: Value,
        len: usize,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let o = self.bind(nogc);
        let scoped_o = self.scope(agent, nogc);
        let callback_fn = callback_fn.scope(agent, nogc);
        let this_arg = this_arg.scope(agent, nogc);

        let byte_offset = o.byte_offset(agent);
        let byte_length = o.byte_length(agent);
        let buffer = o.into_void_array().get(agent).viewed_array_buffer;
        let scoped_buffer = buffer.scope(agent, gc.nogc());

        // 5. Let A be ? TypedArraySpeciesCreate(O, « 𝔽(len) »).
        let a =
            typed_array_species_create_with_length(agent, o.unbind().into(), len, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
        // 6. Let k be 0.
        // 7. Repeat, while k < len,
        let a = a.scope(agent, gc.nogc());
        for k in 0..len {
            // 𝔽(k)
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();
            // b. Let kValue be ! Get(O, Pk).
            let slice =
                scoped_buffer
                    .get(agent)
                    .as_viewable_slice::<T>(agent, byte_offset, byte_length);
            let value = slice.get(k).copied();
            let k_value = value.map_or(Value::Undefined, |v| {
                v.into_ne_value(agent, gc.nogc()).into_value()
            });
            // c. Let mappedValue be ? Call(callback, thisArg, « kValue, 𝔽(k), O »).
            let mapped_value = call_function(
                agent,
                callback_fn.get(agent),
                this_arg.get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    k_value.unbind(),
                    // SAFETY: we want the numeric value, not string.
                    unsafe { pk.into_value_unchecked() },
                    scoped_o.get(agent).into_value(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // d. Perform ? Set(A, Pk, mappedValue, true).
            set(
                agent,
                a.get(agent).into_object(),
                pk,
                mapped_value.unbind(),
                true,
                gc.reborrow(),
            )
            .unbind()?
            // e. Set k to k + 1.
        }
        // 8. Return A.
        Ok(a.get(agent).into_value().unbind())
    }

    fn reverse(self, agent: &mut Agent, len: usize) {
        // 4. Let middle be floor(len / 2).
        // 5. Let lower be 0.
        // 6. Repeat, while lower ≠ middle,
        //    a. Let upper be len - lower - 1.
        //    b. Let upperP be ! ToString(𝔽(upper)).
        //    c. Let lowerP be ! ToString(𝔽(lower)).
        //    d. Let lowerValue be ! Get(O, lowerP).
        //    e. Let upperValue be ! Get(O, upperP).
        //    f. Perform ! Set(O, lowerP, upperValue, true).
        //    g. Perform ! Set(O, upperP, lowerValue, true).
        //    h. Set lower to lower + 1.
        self.as_mut_slice(agent)[..len].reverse();
    }

    fn set_into_data_block<'gc>(
        self,
        agent: &Agent,
        target: &mut DataBlock,
        start_index: usize,
        count: usize,
    ) {
        // SAFETY: precondition.
        unsafe {
            assert_unchecked(
                target.len() >= count * self.typed_array_element_size()
                    && target.as_ptr_range().start.cast::<usize>().is_aligned()
                    && target.as_ptr_range().end.cast::<T>().is_aligned(),
            )
        };
        let source = &self.as_slice(agent)[start_index..start_index + count];
        // SAFETY: Viewables are safe to transmute from u8.
        let (head, target, _) = unsafe { target.align_to_mut::<T>() };
        // SAFETY: precondition.
        unsafe { assert_unchecked(target.len() >= count && head.is_empty()) };
        target[..count].copy_from_slice(source);
    }

    fn set_from_typed_array<'gc>(
        self,
        agent: &mut Agent,
        target_offset: usize,
        source: AnyTypedArray,
        source_offset: usize,
        length: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        let target = self;
        // 1. Let targetBuffer be target.[[ViewedArrayBuffer]].
        let target_buffer = target.into_void_array().get(agent).viewed_array_buffer;
        // 5. Let srcBuffer be source.[[ViewedArrayBuffer]].
        let src_buffer = source.viewed_array_buffer(agent);
        // 9. Let targetType be TypedArrayElementType(target).
        // 10. Let targetElementSize be TypedArrayElementSize(target).
        // 11. Let targetByteOffset be target.[[ByteOffset]].
        let target_byte_offset = target.byte_offset(agent);
        // 12. Let srcType be TypedArrayElementType(source).
        // 13. Let srcElementSize be TypedArrayElementSize(source).
        let src_element_size = source.typed_array_element_size();
        // 14. Let srcByteOffset be source.[[ByteOffset]].
        let src_byte_offset = source_offset * src_element_size + source.byte_offset(agent);
        // a. Let srcByteLength be TypedArrayByteLength(srcRecord).
        let src_byte_length = length * src_element_size;
        let src_byte_end_offset = src_byte_offset + src_byte_length;
        // 18. If IsSharedArrayBuffer(srcBuffer) is true,
        //     IsSharedArrayBuffer(targetBuffer) is true, and
        //     srcBuffer.[[ArrayBufferData]] is targetBuffer.[[ArrayBufferData]],
        //     let sameSharedArrayBuffer be true; otherwise, let
        //     sameSharedArrayBuffer be false.
        // 19. If SameValue(srcBuffer, targetBuffer) is true or
        //     sameSharedArrayBuffer is true, then
        match src_buffer {
            AnyArrayBuffer::ArrayBuffer(src_buffer) => {
                let Ok(source) = TypedArray::try_from(source) else {
                    // SAFETY: Cannot get ArrayBuffer from SharedTypedArray.
                    unsafe { unreachable_unchecked() }
                };
                if src_buffer == target_buffer {
                    // b. Set srcBuffer to
                    //    ? CloneArrayBuffer(srcBuffer, srcByteOffset, srcByteLength).
                    let mut block = create_byte_data_block(agent, src_byte_length as u64, gc)?;
                    block.copy_from_slice(
                        &src_buffer.as_slice(agent)[src_byte_offset..src_byte_end_offset],
                    );
                    // c. Let srcByteIndex be 0.

                    let target_slice = &mut target_buffer.as_mut_viewable_slice::<T>(
                        agent,
                        target_byte_offset,
                        None,
                    )[target_offset..target_offset + length];

                    for_normal_typed_array!(
                        source,
                        _ta,
                        copy_between_typed_arrays::<Source, T>(&block, target_slice),
                        Source
                    );
                } else {
                    let src_slice =
                        &src_buffer.as_slice(agent)[src_byte_offset..src_byte_end_offset];
                    let src_slice = src_slice as *const [u8];

                    let target_slice = &mut target_buffer.as_mut_viewable_slice::<T>(
                        agent,
                        target_byte_offset,
                        None,
                    )[target_offset..target_offset + length];

                    // SAFETY: taking mut slice of target_buffer doesn't invalidate
                    // src_slice pointer and we've checked that they're not the same
                    // buffer.
                    let src_slice = unsafe { &*src_slice };

                    for_normal_typed_array!(
                        source,
                        _ta,
                        copy_between_typed_arrays::<Source, T>(src_slice, target_slice),
                        Source
                    );
                }
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(sab) => {
                use crate::ecmascript::builtins::typed_array::{
                    SharedTypedArray, copy_from_shared_typed_array, for_shared_typed_array,
                };

                let Ok(source) = SharedTypedArray::try_from(source) else {
                    // SAFETY: Cannot get ArrayBuffer from SharedTypedArray.
                    unsafe { unreachable_unchecked() }
                };
                let sdb = sab.get_data_block(agent) as *const SharedDataBlock;

                let target_slice =
                    &mut target_buffer.as_mut_viewable_slice::<T>(agent, target_byte_offset, None)
                        [target_offset..target_offset + length];

                // SAFETY: accessing target_slice as mut cannot invalidate sdb
                // pointer or alias the memory.
                let sdb = unsafe { &*sdb };

                for_shared_typed_array!(
                    source,
                    _ta,
                    copy_from_shared_typed_array::<Source, T>(
                        sdb.as_racy_slice()
                            .slice(src_byte_offset, src_byte_end_offset),
                        target_slice
                    ),
                    Source
                );
            }
        }
        // 25. Return unused.
        Ok(())
    }

    fn slice<'gc>(
        self,
        agent: &mut Agent,
        source: AnyTypedArray,
        source_offset: usize,
        length: usize,
    ) {
        let target = self;
        // 1. Let targetBuffer be target.[[ViewedArrayBuffer]].
        let target_buffer = target.into_void_array().get(agent).viewed_array_buffer;
        // 5. Let srcBuffer be source.[[ViewedArrayBuffer]].
        let src_buffer = source.viewed_array_buffer(agent);
        // 9. Let targetType be TypedArrayElementType(target).
        // 10. Let targetElementSize be TypedArrayElementSize(target).
        // 11. Let targetByteOffset be target.[[ByteOffset]].
        let target_byte_offset = target.byte_offset(agent);
        // 12. Let srcType be TypedArrayElementType(source).
        // 13. Let srcElementSize be TypedArrayElementSize(source).
        let src_element_size = source.typed_array_element_size();
        // 14. Let srcByteOffset be source.[[ByteOffset]].
        let src_byte_offset = source_offset * src_element_size + source.byte_offset(agent);
        // a. Let srcByteLength be TypedArrayByteLength(srcRecord).
        let src_byte_length = length * src_element_size;
        let src_byte_end_offset = src_byte_offset + src_byte_length;
        // 18. If IsSharedArrayBuffer(srcBuffer) is true,
        //     IsSharedArrayBuffer(targetBuffer) is true, and
        //     srcBuffer.[[ArrayBufferData]] is targetBuffer.[[ArrayBufferData]],
        //     let sameSharedArrayBuffer be true; otherwise, let
        //     sameSharedArrayBuffer be false.
        // 19. If SameValue(srcBuffer, targetBuffer) is true or
        //     sameSharedArrayBuffer is true, then
        match src_buffer {
            AnyArrayBuffer::ArrayBuffer(src_buffer) => {
                let Ok(source) = TypedArray::try_from(source) else {
                    // SAFETY: Cannot get ArrayBuffer from SharedTypedArray.
                    unsafe { unreachable_unchecked() }
                };
                if src_buffer == target_buffer {
                    let slice = src_buffer.as_mut_slice(agent);

                    for_normal_typed_array!(
                        source,
                        _ta,
                        copy_within_buffer::<Source, T>(
                            slice,
                            src_byte_offset,
                            target_byte_offset,
                            length
                        ),
                        Source
                    );
                } else {
                    let src_slice =
                        &src_buffer.as_slice(agent)[src_byte_offset..src_byte_end_offset];
                    let src_slice = src_slice as *const [u8];

                    let target_slice = &mut target_buffer.as_mut_viewable_slice::<T>(
                        agent,
                        target_byte_offset,
                        None,
                    )[..length];

                    // SAFETY: taking mut slice of target_buffer doesn't invalidate
                    // src_slice pointer and we've checked that they're not the same
                    // buffer.
                    let src_slice = unsafe { &*src_slice };

                    for_normal_typed_array!(
                        source,
                        _ta,
                        copy_between_typed_arrays::<Source, T>(src_slice, target_slice),
                        Source
                    );
                }
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(sab) => {
                use crate::ecmascript::builtins::typed_array::{
                    SharedTypedArray, copy_from_shared_typed_array, for_shared_typed_array,
                };

                let Ok(source) = SharedTypedArray::try_from(source) else {
                    // SAFETY: Cannot get ArrayBuffer from SharedTypedArray.
                    unsafe { unreachable_unchecked() }
                };
                let sdb = sab.get_data_block(agent) as *const SharedDataBlock;

                let target_slice =
                    &mut target_buffer.as_mut_viewable_slice::<T>(agent, target_byte_offset, None)
                        [..length];

                // SAFETY: accessing target_slice as mut cannot invalidate sdb
                // pointer or alias the memory.
                let sdb = unsafe { &*sdb };

                for_shared_typed_array!(
                    source,
                    _ta,
                    copy_from_shared_typed_array::<Source, T>(
                        sdb.as_racy_slice()
                            .slice(src_byte_offset, src_byte_end_offset),
                        target_slice
                    ),
                    Source
                );
            }
        }
        // 25. Return unused.
    }

    fn sort_with_comparator<'gc>(
        self,
        agent: &mut Agent,
        len: usize,
        comparator: Scoped<Function>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        let ta = self.bind(gc.nogc());
        let slice = &ta.as_slice(agent)[..len];
        let mut items: Vec<T> = slice.to_vec();
        let mut error: Option<JsError> = None;
        let ta = ta.scope(agent, gc.nogc());
        items.sort_by(|a, b| {
            if error.is_some() {
                return std::cmp::Ordering::Equal;
            }
            let a_val = a.into_ne_value(agent, gc.nogc()).into_value();
            let b_val = b.into_ne_value(agent, gc.nogc()).into_value();
            let result = call_function(
                agent,
                comparator.get(agent),
                Value::Undefined,
                Some(ArgumentsList::from_mut_slice(&mut [
                    a_val.unbind(),
                    b_val.unbind(),
                ])),
                gc.reborrow(),
            )
            .unbind()
            .and_then(|v| v.to_number(agent, gc.reborrow()));
            let num = match result {
                Ok(n) => n,
                Err(e) => {
                    error = Some(e.unbind());
                    return std::cmp::Ordering::Equal;
                }
            };
            if num.is_nan(agent) {
                std::cmp::Ordering::Equal
            } else if num.is_sign_positive(agent) {
                std::cmp::Ordering::Greater
            } else if num.is_sign_negative(agent) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });
        if let Some(error) = error {
            return Err(error);
        }
        // SAFETY: not shared.
        let ta = unsafe { ta.take(agent) }.bind(gc.into_nogc());
        let slice = ta.as_mut_slice(agent);
        let len = len.min(slice.len());
        let slice = &mut slice[..len];
        slice.copy_from_slice(&items[..len]);
        Ok(())
    }

    fn sort<'gc>(self, agent: &mut Agent, len: usize) {
        let slice = &mut self.as_mut_slice(agent)[..len];
        slice.sort_by(|a, b| a.ecmascript_cmp(b));
    }

    fn typed_array_create_same_type_and_copy_data<'gc>(
        self,
        agent: &mut Agent,
        len: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, TypedArray<'gc>> {
        let byte_length = (len as u64).saturating_mul(self.typed_array_element_size() as u64);
        let mut data_block = create_byte_data_block(agent, byte_length, gc)?;
        let source = &self.as_slice(agent)[..len];
        // SAFETY: Viewables can be safely transmuted from bytes.
        let (head, target, tail) = unsafe { data_block.align_to_mut::<T>() };
        // SAFETY: cannot have any head or tail since we created length by
        // multiplying with `size_of::<T>()`, and allocation is done 8-byte
        // aligned.
        unsafe { assert_unchecked(head.is_empty() && tail.is_empty() && target.len() == len) };
        target.copy_from_slice(source);
        let result = typed_array_create_from_data_block(agent, self, data_block).bind(gc);
        // SAFETY: we know the type matches.
        Ok(unsafe { result.cast::<T>().into() })
    }
}

fn copy_within_buffer<Source: Viewable, Target: Viewable>(
    buffer: &mut [u8],
    source_byte_offset: usize,
    target_byte_offset: usize,
    len: usize,
) {
    let source_offset = source_byte_offset / size_of::<Target>();
    let target_offset = target_byte_offset / size_of::<Target>();
    if core::any::TypeId::of::<Target>() == core::any::TypeId::of::<Source>() {
        // SAFETY: all viewables are safe to transmute from u8.
        let (head, buffer, _) = unsafe { buffer.align_to_mut::<Target>() };
        assert!(head.is_empty());
        for i in 0..len {
            buffer[target_offset + i] = buffer[source_offset + i];
        }
    } else {
        unsafe { assert_unchecked(Source::IS_BIGINT == Target::IS_BIGINT) };
        // SAFETY: all viewables are safe to transmute from u8.
        let (head, source, _) = unsafe { buffer.align_to::<Source>() };
        assert!(head.is_empty());
        let source = &raw const source[source_offset..source_offset + len];
        let (head, target, _) = unsafe { buffer.align_to_mut::<Target>() };
        assert!(head.is_empty());
        let target = &raw mut target[target_offset..target_offset + len];
        if Target::IS_FLOAT {
            for i in 0..len {
                // SAFETY: copying data within buffer from T -> U
                unsafe {
                    let src = &raw const (&*source)[i];
                    let dst = &raw mut (&mut *target)[i];
                    let value = src.read().into_f64();
                    dst.write(Target::from_f64(value));
                }
            }
        } else {
            for i in 0..len {
                // SAFETY: copying data within buffer from T -> U
                unsafe {
                    let src = &raw const (&*source)[i];
                    let dst = &raw mut (&mut *target)[i];
                    let value = src.read().into_bits();
                    dst.write(Target::from_bits(value));
                }
            }
        }
    };
}

fn copy_between_typed_arrays<Source: Viewable, Target: Viewable>(
    source: &[u8],
    target: &mut [Target],
) {
    if core::any::TypeId::of::<Target>() == core::any::TypeId::of::<Source>() {
        // SAFETY: all viewables are safe to transmute from u8.
        let (head, source, _) = unsafe { source.align_to::<Target>() };
        assert!(head.is_empty());
        target.copy_from_slice(source);
    } else {
        unsafe { assert_unchecked(Source::IS_BIGINT == Target::IS_BIGINT) };
        // SAFETY: all viewables are safe to transmute from u8.
        let (head, source, _) = unsafe { source.align_to::<Source>() };
        assert!(head.is_empty());
        assert_eq!(target.len(), source.len());
        if Target::IS_FLOAT || Source::IS_FLOAT {
            for (dst, src) in target.iter_mut().zip(source.iter()) {
                *dst = Target::from_f64(src.into_f64());
            }
        } else {
            for (dst, src) in target.iter_mut().zip(source.iter()) {
                *dst = Target::from_bits(src.into_bits());
            }
        }
    };
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

impl<T: Viewable> Clone for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Viewable> Copy for GenericTypedArray<'_, T> {}

impl<T: Viewable> PartialEq for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: Viewable> Eq for GenericTypedArray<'_, T> {}

impl<T: Viewable> PartialOrd for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Viewable> Ord for GenericTypedArray<'_, T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Viewable> Hash for GenericTypedArray<'_, T> {
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
        GenericTypedArray(BaseIndex::last(&self.typed_arrays), PhantomData)
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
