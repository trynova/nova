// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::{Hash, Hasher};
use std::hint::{assert_unchecked, unreachable_unchecked};
use std::marker::PhantomData;
use std::ops::ControlFlow;

use ecmascript_atomics::{Ordering, RacySlice};

use crate::ecmascript::abstract_operations::operations_on_objects::{call_function, set};
use crate::ecmascript::abstract_operations::type_conversion::{
    to_big_int, to_big_int_primitive, to_boolean, to_number, to_number_primitive,
};
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::array_buffer::{
    AnyArrayBuffer, ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset,
};
use crate::ecmascript::builtins::indexed_collections::typed_array_objects::abstract_operations::{
    CachedBufferByteLength, TypedArrayAbstractOperations, typed_array_create_from_data_block,
    typed_array_species_create_with_length,
};
use crate::ecmascript::builtins::ordinary::caches::{PropertyLookupCache, PropertyOffset};
use crate::ecmascript::builtins::ordinary::shape::ObjectShape;
use crate::ecmascript::builtins::ordinary::{
    ordinary_define_own_property, ordinary_delete, ordinary_get, ordinary_get_own_property,
    ordinary_has_property_entry, ordinary_prevent_extensions, ordinary_set, ordinary_try_get,
    ordinary_try_has_property, ordinary_try_set,
};
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
use crate::ecmascript::builtins::typed_array::data::{
    SharedTypedArrayRecord, TypedArrayArrayLength,
};
use crate::ecmascript::builtins::typed_array::{
    AnyTypedArray, TypedArray, canonicalize_numeric_index_string, for_normal_typed_array,
};
use crate::ecmascript::execution::agent::{JsError, TryError, js_result_into_try, unwrap_try};
#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::types::SHARED_FLOAT_16_ARRAY_DISCRIMINANT;
use crate::ecmascript::types::{
    BigInt, DataBlock, Function, IntoNumeric, IntoValue, Number, Numeric, Primitive,
    SHARED_BIGINT_64_ARRAY_DISCRIMINANT, SHARED_BIGUINT_64_ARRAY_DISCRIMINANT,
    SHARED_FLOAT_32_ARRAY_DISCRIMINANT, SHARED_FLOAT_64_ARRAY_DISCRIMINANT,
    SHARED_INT_8_ARRAY_DISCRIMINANT, SHARED_INT_16_ARRAY_DISCRIMINANT,
    SHARED_INT_32_ARRAY_DISCRIMINANT, SHARED_UINT_8_ARRAY_DISCRIMINANT,
    SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT, SHARED_UINT_16_ARRAY_DISCRIMINANT,
    SHARED_UINT_32_ARRAY_DISCRIMINANT, SharedDataBlock, create_byte_data_block,
};
use crate::engine::Scoped;
use crate::engine::rootable::{HeapRootRef, Rootable, Scopable};
use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics, agent::TryResult},
        types::{
            InternalMethods, InternalSlots, IntoObject, Object, OrdinaryObject, PropertyDescriptor,
            PropertyKey, SetCachedProps, SetResult, TryGetResult, TryHasResult, U8Clamped, Value,
            Viewable,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

/// ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// A generic TypedArray viewing a SharedArrayBuffer with its concrete type
/// encoded in a type parameter.
pub struct GenericSharedTypedArray<'a, T: Viewable>(
    BaseIndex<'a, SharedTypedArrayRecord<'static>>,
    PhantomData<T>,
);

impl<'ta, T: Viewable> GenericSharedTypedArray<'ta, T> {
    /// Constant to be used only for creating a build-time Self.
    pub(crate) const _DEF: Self = Self(BaseIndex::ZERO, PhantomData);

    /// Convert self into a VoidArray, losing type information.
    #[inline(always)]
    const fn into_void_array(self) -> SharedVoidArray<'ta> {
        GenericSharedTypedArray(self.0, PhantomData)
    }

    fn check_not_void_array() {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<()>() {
            panic!("Invalid GenericSharedTypedArray invocation using void type");
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
    pub(crate) fn as_slice<'a>(self, agent: &'a Agent) -> RacySlice<'a, T::Storage> {
        Self::check_not_void_array();

        let key = self.into_void_array();
        let data = self.into_void_array().get(agent);
        let buffer = data.viewed_array_buffer;
        let byte_offset = data.get_byte_offset(key, &agent.heap.shared_typed_array_byte_offsets);
        let byte_length = data.get_byte_length(key, &agent.heap.shared_typed_array_byte_offsets);
        let mut slice = buffer.as_slice(agent).slice_from(byte_offset);
        if let Some(byte_length) = byte_length {
            slice = slice.slice_to(byte_length);
        }
        let (head, slice, _) = slice.align_to::<T::Storage>();
        assert!(head.is_empty());
        slice
    }

    /// Initialise the heap data of a SharedTypedArray.
    ///
    /// # Safety
    ///
    /// The SharedTypedArray must be newly created; re-initialising is not
    /// allowed.
    pub(crate) unsafe fn initialise_data(
        self,
        agent: &mut Agent,
        ab: SharedArrayBuffer,
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
        agent.heap.alloc_counter += core::mem::size_of::<(SharedVoidArray, usize)>();
        agent
            .heap
            .shared_typed_array_byte_offsets
            .insert(self.into_void_array().unbind(), byte_offset);
    }

    pub(crate) fn set_overflowing_byte_length(self, agent: &mut Agent, byte_length: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(SharedVoidArray, usize)>();
        agent
            .heap
            .shared_typed_array_byte_lengths
            .insert(self.into_void_array().unbind(), byte_length);
    }

    pub(crate) fn set_overflowing_array_length(self, agent: &mut Agent, array_length: usize) {
        agent.heap.alloc_counter += core::mem::size_of::<(SharedVoidArray, usize)>();
        agent
            .heap
            .shared_typed_array_array_lengths
            .insert(self.into_void_array().unbind(), array_length);
    }
}

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

impl<T: Viewable> Rootable for GenericSharedTypedArray<'_, T> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            Err(HeapRootData::SharedUint8Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedUint8Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U8Clamped>() {
            Err(HeapRootData::SharedUint8ClampedArray(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedUint8ClampedArray>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i8>() {
            Err(HeapRootData::SharedInt8Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedInt8Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
            Err(HeapRootData::SharedUint16Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedUint16Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i16>() {
            Err(HeapRootData::SharedInt16Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedInt16Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u32>() {
            Err(HeapRootData::SharedUint32Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedUint32Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i32>() {
            Err(HeapRootData::SharedInt32Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedInt32Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u64>() {
            Err(HeapRootData::SharedBigUint64Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedBigUint64Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i64>() {
            Err(HeapRootData::SharedBigInt64Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedBigInt64Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>() {
            Err(HeapRootData::SharedFloat32Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedFloat32Array>(
                    value.unbind(),
                )
            }))
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>() {
            Err(HeapRootData::SharedFloat64Array(unsafe {
                core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedFloat64Array>(
                    value.unbind(),
                )
            }))
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f16>() {
                return Err(HeapRootData::SharedFloat16Array(unsafe {
                    core::mem::transmute::<GenericSharedTypedArray<'_, T>, SharedFloat16Array>(
                        value.unbind(),
                    )
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
                HeapRootData::SharedUint8Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedUint8Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U8Clamped>() {
            match heap_data {
                HeapRootData::SharedUint8ClampedArray(ta) => Some(unsafe {
                    core::mem::transmute::<SharedUint8ClampedArray, GenericSharedTypedArray<'_, T>>(
                        ta,
                    )
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i8>() {
            match heap_data {
                HeapRootData::SharedInt8Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedInt8Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
            match heap_data {
                HeapRootData::SharedUint16Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedUint16Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i16>() {
            match heap_data {
                HeapRootData::SharedInt16Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedInt16Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u32>() {
            match heap_data {
                HeapRootData::SharedUint32Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedUint32Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i32>() {
            match heap_data {
                HeapRootData::SharedInt32Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedInt32Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u64>() {
            match heap_data {
                HeapRootData::SharedBigUint64Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedBigUint64Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i64>() {
            match heap_data {
                HeapRootData::SharedBigInt64Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedBigInt64Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>() {
            match heap_data {
                HeapRootData::SharedFloat32Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedFloat32Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>() {
            match heap_data {
                HeapRootData::SharedFloat64Array(ta) => Some(unsafe {
                    core::mem::transmute::<SharedFloat64Array, GenericSharedTypedArray<'_, T>>(ta)
                }),
                _ => None,
            }
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f16>() {
                return match heap_data {
                    HeapRootData::SharedFloat16Array(ta) => Some(unsafe {
                        core::mem::transmute::<SharedFloat16Array, GenericSharedTypedArray<'_, T>>(
                            ta,
                        )
                    }),
                    _ => None,
                };
            }
            unreachable!()
        }
    }
}

macro_rules! shared_typed_array_delegate {
    ($value: ident, $method: ident, $($arg:expr),*) => {
        match $value {
            Self::SharedInt8Array(ta) => ta.$method($($arg),+),
            Self::SharedUint8Array(ta) => ta.$method($($arg),+),
            Self::SharedUint8ClampedArray(ta) => ta.$method($($arg),+),
            Self::SharedInt16Array(ta) => ta.$method($($arg),+),
            Self::SharedUint16Array(ta) => ta.$method($($arg),+),
            Self::SharedInt32Array(ta) => ta.$method($($arg),+),
            Self::SharedUint32Array(ta) => ta.$method($($arg),+),
            Self::SharedBigInt64Array(ta) => ta.$method($($arg),+),
            Self::SharedBigUint64Array(ta) => ta.$method($($arg),+),
            #[cfg(feature = "proposal-float16array")]
            Self::SharedFloat16Array(ta) => ta.$method($($arg),+),
            Self::SharedFloat32Array(ta) => ta.$method($($arg),+),
            Self::SharedFloat64Array(ta) => ta.$method($($arg),+),
        }
    };
}

impl<'a> InternalSlots<'a> for SharedTypedArray<'a> {
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        shared_typed_array_delegate!(self, get_backing_object, agent)
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("TypedArray should not try to set its backing object");
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!("TypedArray should not try to create its backing object");
    }

    fn get_or_create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        shared_typed_array_delegate!(self, get_or_create_backing_object, agent)
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        shared_typed_array_delegate!(self, object_shape, agent)
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        shared_typed_array_delegate!(self, internal_extensible, agent)
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        shared_typed_array_delegate!(self, internal_set_extensible, agent, value)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        shared_typed_array_delegate!(self, internal_prototype, agent)
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        shared_typed_array_delegate!(self, internal_set_prototype, agent, prototype)
    }
}

impl<'a> InternalMethods<'a> for SharedTypedArray<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<Object<'gc>>> {
        shared_typed_array_delegate!(self, try_get_prototype_of, agent, gc)
    }

    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Object<'gc>>> {
        shared_typed_array_delegate!(self, internal_get_prototype_of, agent, gc)
    }

    fn try_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        shared_typed_array_delegate!(self, try_set_prototype_of, agent, prototype, gc)
    }

    fn internal_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        shared_typed_array_delegate!(self, internal_set_prototype_of, agent, prototype, gc)
    }

    fn try_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        shared_typed_array_delegate!(self, try_is_extensible, agent, gc)
    }

    fn internal_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        shared_typed_array_delegate!(self, internal_is_extensible, agent, gc)
    }

    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        shared_typed_array_delegate!(self, try_prevent_extensions, agent, gc)
    }

    fn internal_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        shared_typed_array_delegate!(self, internal_prevent_extensions, agent, gc)
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        shared_typed_array_delegate!(self, try_get_own_property, agent, property_key, cache, gc)
    }

    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
        shared_typed_array_delegate!(self, internal_get_own_property, agent, property_key, gc)
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        shared_typed_array_delegate!(
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
        shared_typed_array_delegate!(
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
        shared_typed_array_delegate!(self, try_has_property, agent, property_key, cache, gc)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        shared_typed_array_delegate!(self, internal_has_property, agent, property_key, gc)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        shared_typed_array_delegate!(self, try_get, agent, property_key, receiver, cache, gc)
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        shared_typed_array_delegate!(self, internal_get, agent, property_key, receiver, gc)
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
        shared_typed_array_delegate!(
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
        shared_typed_array_delegate!(self, internal_set, agent, property_key, value, receiver, gc)
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        shared_typed_array_delegate!(self, try_delete, agent, property_key, gc)
    }

    fn internal_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        shared_typed_array_delegate!(self, internal_delete, agent, property_key, gc)
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        shared_typed_array_delegate!(self, try_own_property_keys, agent, gc)
    }

    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
        shared_typed_array_delegate!(self, internal_own_property_keys, agent, gc)
    }

    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        shared_typed_array_delegate!(self, get_own_property_at_offset, agent, offset, gc)
    }

    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        shared_typed_array_delegate!(self, set_at_offset, agent, props, offset, gc)
    }
}

/// ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// A SharedTypedArray presents an array-like view of an underlying binary data
/// buffer (25.1) that can be shared between Agents.
///
/// In Nova engine, SharedTypedArrays view a [`SharedArrayBuffer`]. TypedArrays
/// viewing an [`ArrayBuffer`] are represented by a [`TypedArray`].
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SharedTypedArray<'a> {
    SharedInt8Array(SharedInt8Array<'a>) = SHARED_INT_8_ARRAY_DISCRIMINANT,
    SharedUint8Array(SharedUint8Array<'a>) = SHARED_UINT_8_ARRAY_DISCRIMINANT,
    SharedUint8ClampedArray(SharedUint8ClampedArray<'a>) = SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    SharedInt16Array(SharedInt16Array<'a>) = SHARED_INT_16_ARRAY_DISCRIMINANT,
    SharedUint16Array(SharedUint16Array<'a>) = SHARED_UINT_16_ARRAY_DISCRIMINANT,
    SharedInt32Array(SharedInt32Array<'a>) = SHARED_INT_32_ARRAY_DISCRIMINANT,
    SharedUint32Array(SharedUint32Array<'a>) = SHARED_UINT_32_ARRAY_DISCRIMINANT,
    SharedBigInt64Array(SharedBigInt64Array<'a>) = SHARED_BIGINT_64_ARRAY_DISCRIMINANT,
    SharedBigUint64Array(SharedBigUint64Array<'a>) = SHARED_BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    SharedFloat16Array(SharedFloat16Array<'a>) = SHARED_FLOAT_16_ARRAY_DISCRIMINANT,
    SharedFloat32Array(SharedFloat32Array<'a>) = SHARED_FLOAT_32_ARRAY_DISCRIMINANT,
    SharedFloat64Array(SharedFloat64Array<'a>) = SHARED_FLOAT_64_ARRAY_DISCRIMINANT,
}
bindable_handle!(SharedTypedArray);

macro_rules! for_shared_typed_array {
    ($value: ident, $ta: ident, $expr: expr) => {
        for_shared_typed_array($value, $ta, $expr, TA)
    };
    ($value: ident, $ta: ident, $expr: expr, $TA: ident) => {
        match $value {
            SharedTypedArray::SharedInt8Array($ta) => {
                type $TA = i8;
                $expr
            }
            SharedTypedArray::SharedUint8Array($ta) => {
                type $TA = u8;
                $expr
            }
            SharedTypedArray::SharedUint8ClampedArray($ta) => {
                type $TA = U8Clamped;
                $expr
            }
            SharedTypedArray::SharedInt16Array($ta) => {
                type $TA = i16;
                $expr
            }
            SharedTypedArray::SharedUint16Array($ta) => {
                type $TA = u16;
                $expr
            }
            SharedTypedArray::SharedInt32Array($ta) => {
                type $TA = i32;
                $expr
            }
            SharedTypedArray::SharedUint32Array($ta) => {
                type $TA = u32;
                $expr
            }
            SharedTypedArray::SharedBigInt64Array($ta) => {
                type $TA = i64;
                $expr
            }
            SharedTypedArray::SharedBigUint64Array($ta) => {
                type $TA = u64;
                $expr
            }
            #[cfg(feature = "proposal-float16array")]
            SharedTypedArray::SharedFloat16Array($ta) => {
                type $TA = f16;
                $expr
            }
            SharedTypedArray::SharedFloat32Array($ta) => {
                type $TA = f32;
                $expr
            }
            SharedTypedArray::SharedFloat64Array($ta) => {
                type $TA = f64;
                $expr
            }
        }
    };
}
pub(crate) use for_shared_typed_array;

impl<'a> From<SharedTypedArray<'a>> for Value<'a> {
    #[inline(always)]
    fn from(value: SharedTypedArray<'a>) -> Self {
        value.into_object().into_value()
    }
}

impl<'a> From<SharedTypedArray<'a>> for Object<'a> {
    #[inline(always)]
    fn from(value: SharedTypedArray<'a>) -> Self {
        let value: AnyTypedArray = value.into();
        value.into_object()
    }
}

impl<'a> From<SharedTypedArray<'a>> for AnyTypedArray<'a> {
    #[inline(always)]
    fn from(value: SharedTypedArray<'a>) -> Self {
        match value {
            SharedTypedArray::SharedInt8Array(ta) => Self::SharedInt8Array(ta),
            SharedTypedArray::SharedUint8Array(ta) => Self::SharedUint8Array(ta),
            SharedTypedArray::SharedUint8ClampedArray(ta) => Self::SharedUint8ClampedArray(ta),
            SharedTypedArray::SharedInt16Array(ta) => Self::SharedInt16Array(ta),
            SharedTypedArray::SharedUint16Array(ta) => Self::SharedUint16Array(ta),
            SharedTypedArray::SharedInt32Array(ta) => Self::SharedInt32Array(ta),
            SharedTypedArray::SharedUint32Array(ta) => Self::SharedUint32Array(ta),
            SharedTypedArray::SharedBigInt64Array(ta) => Self::SharedBigInt64Array(ta),
            SharedTypedArray::SharedBigUint64Array(ta) => Self::SharedBigUint64Array(ta),
            #[cfg(feature = "proposal-float16array")]
            SharedTypedArray::SharedFloat16Array(ta) => Self::SharedFloat16Array(ta),
            SharedTypedArray::SharedFloat32Array(ta) => Self::SharedFloat32Array(ta),
            SharedTypedArray::SharedFloat64Array(ta) => Self::SharedFloat64Array(ta),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for SharedTypedArray<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::SharedInt8Array(base_index) => Ok(Self::SharedInt8Array(base_index)),
            Value::SharedUint8Array(base_index) => Ok(Self::SharedUint8Array(base_index)),
            Value::SharedUint8ClampedArray(base_index) => {
                Ok(Self::SharedUint8ClampedArray(base_index))
            }
            Value::SharedInt16Array(base_index) => Ok(Self::SharedInt16Array(base_index)),
            Value::SharedUint16Array(base_index) => Ok(Self::SharedUint16Array(base_index)),
            Value::SharedInt32Array(base_index) => Ok(Self::SharedInt32Array(base_index)),
            Value::SharedUint32Array(base_index) => Ok(Self::SharedUint32Array(base_index)),
            Value::SharedBigInt64Array(base_index) => Ok(Self::SharedBigInt64Array(base_index)),
            Value::SharedBigUint64Array(base_index) => Ok(Self::SharedBigUint64Array(base_index)),
            #[cfg(feature = "proposal-float16array")]
            Value::SharedFloat16Array(base_index) => Ok(Self::SharedFloat16Array(base_index)),
            Value::SharedFloat32Array(base_index) => Ok(Self::SharedFloat32Array(base_index)),
            Value::SharedFloat64Array(base_index) => Ok(Self::SharedFloat64Array(base_index)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for SharedTypedArray<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::SharedUint8Array(t) => Ok(Self::SharedUint8Array(t)),
            Object::SharedInt8Array(t) => Ok(Self::SharedInt8Array(t)),
            Object::SharedUint8ClampedArray(t) => Ok(Self::SharedUint8ClampedArray(t)),
            Object::SharedInt16Array(t) => Ok(Self::SharedInt16Array(t)),
            Object::SharedUint16Array(t) => Ok(Self::SharedUint16Array(t)),
            Object::SharedInt32Array(t) => Ok(Self::SharedInt32Array(t)),
            Object::SharedUint32Array(t) => Ok(Self::SharedUint32Array(t)),
            Object::SharedBigInt64Array(t) => Ok(Self::SharedBigInt64Array(t)),
            Object::SharedBigUint64Array(t) => Ok(Self::SharedBigUint64Array(t)),
            #[cfg(feature = "proposal-float16array")]
            Object::SharedFloat16Array(t) => Ok(Self::SharedFloat16Array(t)),
            Object::SharedFloat32Array(t) => Ok(Self::SharedFloat32Array(t)),
            Object::SharedFloat64Array(t) => Ok(Self::SharedFloat64Array(t)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<AnyTypedArray<'a>> for SharedTypedArray<'a> {
    type Error = ();

    fn try_from(value: AnyTypedArray<'a>) -> Result<Self, Self::Error> {
        match value {
            AnyTypedArray::SharedUint8Array(t) => Ok(Self::SharedUint8Array(t)),
            AnyTypedArray::SharedInt8Array(t) => Ok(Self::SharedInt8Array(t)),
            AnyTypedArray::SharedUint8ClampedArray(t) => Ok(Self::SharedUint8ClampedArray(t)),
            AnyTypedArray::SharedInt16Array(t) => Ok(Self::SharedInt16Array(t)),
            AnyTypedArray::SharedUint16Array(t) => Ok(Self::SharedUint16Array(t)),
            AnyTypedArray::SharedInt32Array(t) => Ok(Self::SharedInt32Array(t)),
            AnyTypedArray::SharedUint32Array(t) => Ok(Self::SharedUint32Array(t)),
            AnyTypedArray::SharedBigInt64Array(t) => Ok(Self::SharedBigInt64Array(t)),
            AnyTypedArray::SharedBigUint64Array(t) => Ok(Self::SharedBigUint64Array(t)),
            #[cfg(feature = "proposal-float16array")]
            AnyTypedArray::SharedFloat16Array(t) => Ok(Self::SharedFloat16Array(t)),
            AnyTypedArray::SharedFloat32Array(t) => Ok(Self::SharedFloat32Array(t)),
            AnyTypedArray::SharedFloat64Array(t) => Ok(Self::SharedFloat64Array(t)),
            _ => Err(()),
        }
    }
}

impl<'a, T: Viewable> From<GenericSharedTypedArray<'a, T>> for AnyTypedArray<'a> {
    fn from(value: GenericSharedTypedArray<'a, T>) -> Self {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            // SAFETY: type checked.
            Self::SharedUint8Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, u8>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<U8Clamped>() {
            // SAFETY: type checked.
            Self::SharedUint8ClampedArray(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, U8Clamped>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i8>() {
            // SAFETY: type checked.
            Self::SharedInt8Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, i8>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
            // SAFETY: type checked.
            Self::SharedUint16Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, u16>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i16>() {
            // SAFETY: type checked.
            Self::SharedInt16Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, i16>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u32>() {
            // SAFETY: type checked.
            Self::SharedUint32Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, u32>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i32>() {
            // SAFETY: type checked.
            Self::SharedInt32Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, i32>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u64>() {
            // SAFETY: type checked.
            Self::SharedBigUint64Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, u64>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<i64>() {
            // SAFETY: type checked.
            Self::SharedBigInt64Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, i64>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>() {
            // SAFETY: type checked.
            Self::SharedFloat32Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, f32>,
                >(value)
            })
        } else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>() {
            // SAFETY: type checked.
            Self::SharedFloat64Array(unsafe {
                core::mem::transmute::<
                    GenericSharedTypedArray<'a, T>,
                    GenericSharedTypedArray<'a, f64>,
                >(value)
            })
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f16>() {
                // SAFETY: type checked.
                return Self::SharedFloat16Array(unsafe {
                    core::mem::transmute::<
                        GenericSharedTypedArray<'a, T>,
                        GenericSharedTypedArray<'a, f16>,
                    >(value)
                });
            }
            unreachable!()
        }
    }
}

impl<'a, T: Viewable> From<GenericSharedTypedArray<'a, T>> for Object<'a> {
    fn from(value: GenericSharedTypedArray<'a, T>) -> Self {
        let value: AnyTypedArray = value.into();
        value.into()
    }
}

impl<'a, T: Viewable> From<GenericSharedTypedArray<'a, T>> for Value<'a> {
    fn from(value: GenericSharedTypedArray<'a, T>) -> Self {
        let value: AnyTypedArray = value.into();
        value.into()
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

impl<'a, T: Viewable> InternalMethods<'a> for GenericSharedTypedArray<'a, T> {
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

macro_rules! delegate_shared_data_block {
    ($T: ty, $U: ident, $slice: ident, $expr: expr) => {
        if core::any::TypeId::of::<$T>() == core::any::TypeId::of::<u8>()
            || core::any::TypeId::of::<$T>() == core::any::TypeId::of::<i8>()
        {
            type $U = u8;
            let $slice = $slice;
            $expr
        } else if core::any::TypeId::of::<$T>() == core::any::TypeId::of::<u16>()
            || core::any::TypeId::of::<$T>() == core::any::TypeId::of::<i16>()
        {
            type $U = u16;
            let (head, $slice, _) = $slice.align_to::<u16>();
            assert!(head.is_empty(), "TypedArray is not properly aligned");
            $expr
        } else if core::any::TypeId::of::<$T>() == core::any::TypeId::of::<u32>()
            || core::any::TypeId::of::<$T>() == core::any::TypeId::of::<i32>()
            || core::any::TypeId::of::<$T>() == core::any::TypeId::of::<f32>()
        {
            type $U = u32;
            let (head, $slice, _) = $slice.align_to::<u32>();
            assert!(head.is_empty(), "TypedArray is not properly aligned");
            $expr
        } else if core::any::TypeId::of::<$T>() == core::any::TypeId::of::<u64>()
            || core::any::TypeId::of::<$T>() == core::any::TypeId::of::<i64>()
            || core::any::TypeId::of::<$T>() == core::any::TypeId::of::<f64>()
        {
            type $U = u64;
            let (head, $slice, _) = $slice.align_to::<u64>();
            assert!(head.is_empty(), "TypedArray is not properly aligned");
            $expr
        } else {
            #[cfg(feature = "proposal-float16array")]
            if core::any::TypeId::of::<$T>() == core::any::TypeId::of::<f16>() {
                type $U = u16;
                let (head, $slice, _) = $slice.align_to::<u16>();
                assert!(head.is_empty(), "TypedArray is not properly aligned");
                $expr
            }
            unreachable!("Unexpected read type")
        }
    };
}

impl<'a, T: Viewable> TypedArrayAbstractOperations<'a> for GenericSharedTypedArray<'a, T> {
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
            .is_growable(agent)
    }

    #[inline(always)]
    fn is_shared(self) -> bool {
        true
    }

    #[inline(always)]
    fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = self.into_void_array().get(agent).byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent
                .heap
                .shared_typed_array_byte_offsets
                .get(&self.into_void_array().unbind())
                .unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = self.into_void_array().get(agent).byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(
                *agent
                    .heap
                    .shared_typed_array_byte_lengths
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
                    .shared_typed_array_array_lengths
                    .get(&self.into_void_array().unbind())
                    .unwrap(),
            )
        } else if array_length == TypedArrayArrayLength::auto() {
            None
        } else {
            Some(array_length.0 as usize)
        }
    }

    fn typed_array_element_size(self) -> usize {
        size_of::<T>()
    }

    fn viewed_array_buffer(self, agent: &Agent) -> AnyArrayBuffer<'a> {
        self.into_void_array().get(agent).viewed_array_buffer.into()
    }

    fn get_cached_buffer_byte_length(
        self,
        agent: &Agent,
        order: Ordering,
    ) -> CachedBufferByteLength {
        // 1. Let buffer be obj.[[ViewedArrayBuffer]].
        let buffer = self.into_void_array().get(agent).viewed_array_buffer;

        // 2. If IsDetachedBuffer(buffer) is true, then
        if buffer.is_detached(agent) {
            // a. Let byteLength be detached.
            CachedBufferByteLength::detached()
        } else {
            // 3. Else,
            // a. Let byteLength be ArrayBufferByteLength(buffer, order).
            CachedBufferByteLength::value(buffer.byte_length(agent, order))
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
        let slice = self.as_slice(agent);
        delegate_shared_data_block!(T, _T, slice, {
            let src_slice = slice.slice(start_index, start_index + count);
            let dst_slice = slice.slice(target_index, target_index + count);
            dst_slice.copy_from_racy_slice(&src_slice);
        });
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
        let value = T::into_storage(T::from_ne_value(agent, value));
        let slice = self.as_slice(agent);
        let slice = slice.slice_from(start_index).slice_to(count);
        for i in 0..slice.len() {
            slice.store(i, value, Ordering::Unordered).unwrap()
        }
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

        // 5. Let kept be a new empty List.
        let mut kept = create_byte_data_block(agent, len as u64, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // SAFETY: All viewable types are trivially transmutable.
        let (head, kept_slice, _) = unsafe { kept.align_to_mut::<T>() };
        // Should be properly aligned for all T.
        assert!(head.is_empty());

        let sdb = o
            .into_void_array()
            .get(agent)
            .viewed_array_buffer
            .get_data_block(agent)
            .clone();
        let slice = sdb_as_viewable_slice::<T>(&sdb, byte_offset, byte_length);
        debug_assert!(slice.len() >= len);

        // 6. Let captured be 0.
        let mut captured = 0;
        // 7. Let k be 0.
        // 8. Repeat, while k < len,
        for k in 0..len {
            // b. Let kValue be ! Get(O, Pk).
            let value = T::from_storage(slice.load(k, Ordering::Unordered).unwrap());
            let k_value = value.into_le_value(agent, gc.nogc()).into_value();
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
                kept_slice[captured] = value;
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

        let expected_byte_length = captured * size_of::<T>();
        if captured != len {
            kept.realloc(expected_byte_length)
        }

        let byte_offset = a.byte_offset(agent);
        let buffer = a.viewed_array_buffer(agent);
        match buffer {
            AnyArrayBuffer::ArrayBuffer(ab) => {
                let is_resizable = ab.is_resizable(agent);
                let byte_length = ab.byte_length(agent);

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
        let search_element = T::into_storage(T::try_from_value(agent, search_element)?);
        let slice = self.as_slice(agent);
        // Length of the TypedArray may have changed between when we measured it
        // and here: We'll never try to access past the boundary of the slice if
        // the backing ArrayBuffer shrank.
        let end = end.min(slice.len());
        if start >= end {
            return None;
        }

        if ASCENDING {
            for i in start..end {
                if slice.load(i, Ordering::Unordered).unwrap() == search_element {
                    return Some(i + start);
                }
            }
        } else {
            for i in (0..=start).rev() {
                if slice.load(i, Ordering::Unordered).unwrap() == search_element {
                    return Some(i);
                }
            }
        }
        None
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

        let sdb = o
            .into_void_array()
            .get(agent)
            .viewed_array_buffer
            .get_data_block(agent)
            .clone();
        let slice = sdb_as_viewable_slice::<T>(&sdb, byte_offset, byte_length);
        debug_assert!(slice.len() >= len);

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
            let value = T::from_storage(slice.load(k, Ordering::Unordered).unwrap());
            let k_value = value.into_le_value(agent, gc.nogc()).into_value();
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
        let slice = self.as_slice(agent).slice_to(len);
        // 4. Let middle be floor(len / 2).
        let middle = len / 2;
        // 5. Let lower be 0.
        let mut lower = 0;
        // 6. Repeat, while lower ≠ middle,
        while lower != middle {
            // a. Let upper be len - lower - 1.
            let upper = len - lower - 1;
            // b. Let upperP be ! ToString(𝔽(upper)).
            // c. Let lowerP be ! ToString(𝔽(lower)).
            // d. Let lowerValue be ! Get(O, lowerP).
            let lower_value = slice.load(lower, Ordering::Unordered).unwrap();
            // e. Let upperValue be ! Get(O, upperP).
            let upper_value = slice.load(upper, Ordering::Unordered).unwrap();
            // f. Perform ! Set(O, lowerP, upperValue, true).
            slice.store(lower, upper_value, Ordering::Unordered);
            // g. Perform ! Set(O, upperP, lowerValue, true).
            slice.store(upper, lower_value, Ordering::Unordered);
            // h. Set lower to lower + 1.
            lower += 1;
        }
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
                    && target.as_ptr_range().end.cast::<usize>().is_aligned(),
            )
        };
        let source = self.as_slice(agent).slice(start_index, start_index + count);
        // SAFETY: Viewables are safe to transmute from u8.
        let (head, target, _) = unsafe { target.align_to_mut::<T::Storage>() };
        // SAFETY: precondition.
        unsafe { assert_unchecked(target.len() >= count && head.is_empty()) };
        source.copy_into_slice(&mut target[..count]);
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
                let src_byte_end_offset = src_byte_offset + src_element_size * length;
                let src_slice = &src_buffer.as_slice(agent)[src_byte_offset..src_byte_end_offset];

                let target_slice = sdb_as_viewable_slice::<T>(
                    target_buffer.get_data_block(agent),
                    target_byte_offset,
                    None,
                )
                .slice_from(target_offset)
                .slice_to(length);

                for_normal_typed_array!(
                    source,
                    _ta,
                    copy_into_shared_typed_array::<Source, T>(src_slice, target_slice),
                    Source
                );
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(source_buffer) => {
                use crate::ecmascript::builtins::typed_array::{
                    SharedTypedArray, for_shared_typed_array,
                };

                let Ok(source) = SharedTypedArray::try_from(source) else {
                    // SAFETY: Cannot get SharedArrayBuffer from TypedArray.
                    unsafe { unreachable_unchecked() }
                };
                let target_sdb = target_buffer.get_data_block(agent);
                let source_sdb = source_buffer.get_data_block(agent);

                let src_byte_end_offset = src_byte_offset + src_element_size * length;

                if target_sdb == source_sdb {
                    let source_sdb = source_sdb as *const SharedDataBlock;
                    let target_sdb = target_sdb as *const SharedDataBlock;

                    let mut copy_block = create_byte_data_block(
                        agent,
                        (src_byte_end_offset - src_byte_offset) as u64,
                        gc,
                    )?;

                    // SAFETY: creating a new DataBlock can only mutate Agent
                    // for throwing an error.
                    let source_sdb = unsafe { &*source_sdb };
                    let target_sdb = unsafe { &*target_sdb };

                    let source_slice = source_sdb
                        .as_racy_slice()
                        .slice(src_byte_offset, src_byte_end_offset);
                    source_slice.copy_into_slice(&mut copy_block);

                    let target_slice =
                        sdb_as_viewable_slice::<T>(target_sdb, target_byte_offset, None)
                            .slice_from(target_offset)
                            .slice_to(length);

                    for_shared_typed_array!(
                        source,
                        _ta,
                        copy_into_shared_typed_array::<Source, T>(&copy_block, target_slice),
                        Source
                    );
                } else {
                    let source_slice = source_sdb
                        .as_racy_slice()
                        .slice(src_byte_offset, src_byte_end_offset);
                    let (head, target_slice, _) = target_sdb
                        .as_racy_slice()
                        .slice_from(target_byte_offset)
                        .align_to::<T::Storage>();
                    assert!(head.is_empty());
                    let target_slice = target_slice.slice_from(target_offset).slice_to(length);

                    for_shared_typed_array!(
                        source,
                        _ta,
                        copy_between_shared_typed_arrays::<Source, T>(source_slice, target_slice),
                        Source
                    );
                }
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
        let target_element_size = size_of::<T>();
        // 11. Let targetByteOffset be target.[[ByteOffset]].
        let target_byte_offset = target.byte_offset(agent);
        // 12. Let srcType be TypedArrayElementType(source).
        // 13. Let srcElementSize be TypedArrayElementSize(source).
        let src_element_size = source.typed_array_element_size();
        // 14. Let srcByteOffset be source.[[ByteOffset]].
        let src_byte_offset = source_offset * src_element_size + source.byte_offset(agent);
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
                let src_byte_end_offset = src_byte_offset + src_element_size * length;
                let src_slice = &src_buffer.as_slice(agent)[src_byte_offset..src_byte_end_offset];

                let target_slice = sdb_as_viewable_slice::<T>(
                    target_buffer.get_data_block(agent),
                    target_byte_offset,
                    None,
                )
                .slice_to(length);

                for_normal_typed_array!(
                    source,
                    _ta,
                    copy_into_shared_typed_array::<Source, T>(src_slice, target_slice),
                    Source
                );
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(source_buffer) => {
                use crate::ecmascript::builtins::typed_array::{
                    SharedTypedArray, for_shared_typed_array,
                };

                let Ok(source) = SharedTypedArray::try_from(source) else {
                    // SAFETY: Cannot get SharedArrayBuffer from TypedArray.
                    unsafe { unreachable_unchecked() }
                };
                let target_sdb = target_buffer.get_data_block(agent);
                let source_sdb = source_buffer.get_data_block(agent);

                let src_byte_end_offset = src_byte_offset + src_element_size * length;

                let source_slice = source_sdb
                    .as_racy_slice()
                    .slice(src_byte_offset, src_byte_end_offset);
                let (head, target_slice, _) = target_sdb
                    .as_racy_slice()
                    .slice(
                        target_byte_offset,
                        target_byte_offset + length * target_element_size,
                    )
                    .align_to::<T::Storage>();
                assert!(head.is_empty());

                for_shared_typed_array!(
                    source,
                    _ta,
                    copy_between_shared_typed_arrays::<Source, T>(source_slice, target_slice),
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
        let slice = ta.as_slice(agent).slice_to(len);
        let mut items: Vec<T> = Vec::with_capacity(slice.len());
        for i in 0..slice.len() {
            items.push(T::from_storage(slice.load(i, Ordering::Unordered).unwrap()));
        }
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
        let slice = ta.as_slice(agent);
        let len = len.min(slice.len());
        let items = &items[..len];
        // SAFETY: T::Storage is a different representation of T.
        let items = unsafe { core::mem::transmute::<&[T], &[T::Storage]>(items) };
        slice.copy_from_slice(&items[..len]);
        Ok(())
    }

    fn sort<'gc>(self, agent: &mut Agent, len: usize) {
        let mut items = vec![T::default(); len];
        // SAFETY: Transmute to storage is always safe.
        let items_storage =
            unsafe { core::mem::transmute::<&mut [T], &mut [T::Storage]>(&mut items) };
        let slice = self.as_slice(agent).slice_to(len);
        slice.copy_into_slice(items_storage);
        items.sort_by(|a, b| a.ecmascript_cmp(b));
        // SAFETY: Transmute to storage is always safe.
        let items_storage =
            unsafe { core::mem::transmute::<&mut [T], &mut [T::Storage]>(&mut items) };
        slice.copy_from_slice(items_storage);
    }

    fn typed_array_create_same_type_and_copy_data<'gc>(
        self,
        agent: &mut Agent,
        len: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, TypedArray<'gc>> {
        let byte_length = (len as u64).saturating_mul(self.typed_array_element_size() as u64);
        let mut data_block = create_byte_data_block(agent, byte_length, gc)?;
        let source = self.as_slice(agent).slice_to(len);
        // SAFETY: Viewables can be safely transmuted from bytes.
        let (head, target, tail) = unsafe { data_block.align_to_mut::<T::Storage>() };
        // SAFETY: cannot have any head or tail since we created length by
        // multiplying with `size_of::<T>()`, and allocation is done 8-byte
        // aligned.
        unsafe { assert_unchecked(head.is_empty() && tail.is_empty() && target.len() == len) };
        source.copy_into_slice(target);
        let result = typed_array_create_from_data_block(agent, self, data_block).bind(gc);
        // SAFETY: we know the type matches.
        Ok(unsafe { result.cast::<T>().into() })
    }
}

fn copy_between_shared_typed_arrays<Source: Viewable, Target: Viewable>(
    source: RacySlice<u8>,
    target: RacySlice<Target::Storage>,
) {
    if core::any::TypeId::of::<Target>() == core::any::TypeId::of::<Source>() {
        let (head, source, _) = source.align_to::<Target::Storage>();
        assert!(head.is_empty());
        assert_eq!(target.len(), source.len());
        target.copy_from_racy_slice(&source);
    } else {
        assert_eq!(Source::IS_BIGINT, Target::IS_BIGINT);
        let (head, source, _) = source.align_to::<Source::Storage>();
        assert!(head.is_empty());
        assert_eq!(target.len(), source.len());
        if Target::IS_FLOAT {
            for i in 0..source.len() {
                let src = Source::from_storage(source.load(i, Ordering::Unordered).unwrap());
                let value = Target::from_f64(src.into_f64());
                target.store(i, Target::into_storage(value), Ordering::Unordered);
            }
        } else {
            for i in 0..source.len() {
                let src = Source::from_storage(source.load(i, Ordering::Unordered).unwrap());
                let value = Target::from_bits(src.into_bits());
                target.store(i, Target::into_storage(value), Ordering::Unordered);
            }
        }
    };
}

pub(crate) fn copy_from_shared_typed_array<Source: Viewable, Target: Viewable>(
    source: RacySlice<'_, u8>,
    target: &mut [Target],
) {
    if core::any::TypeId::of::<Target>() == core::any::TypeId::of::<Source>() {
        let (head, source, _) = source.align_to::<Target::Storage>();
        assert!(head.is_empty());
        assert_eq!(target.len(), source.len());
        // SAFETY: transmuting between T and T::Storage is safe.
        let target =
            unsafe { core::mem::transmute::<&mut [Target], &mut [Target::Storage]>(target) };
        source.copy_into_slice(target);
    } else {
        assert_eq!(Source::IS_BIGINT, Target::IS_BIGINT);
        let (head, source, _) = source.align_to::<Source::Storage>();
        assert!(head.is_empty());
        assert_eq!(target.len(), source.len());
        if Target::IS_FLOAT {
            for (i, target) in target.iter_mut().enumerate() {
                let src = Source::from_storage(source.load(i, Ordering::Unordered).unwrap());
                *target = Target::from_f64(src.into_f64());
            }
        } else {
            for (i, target) in target.iter_mut().enumerate() {
                let src = Source::from_storage(source.load(i, Ordering::Unordered).unwrap());
                *target = Target::from_bits(src.into_bits());
            }
        }
    };
}

fn copy_into_shared_typed_array<Source: Viewable, Target: Viewable>(
    source: &[u8],
    target: RacySlice<'_, Target::Storage>,
) {
    if core::any::TypeId::of::<Target>() == core::any::TypeId::of::<Source>() {
        // SAFETY: all viewables are safe to transmute from u8.
        let (head, source, _) = unsafe { source.align_to::<Target::Storage>() };
        assert!(head.is_empty());
        target.copy_from_slice(source);
    } else {
        assert_eq!(Source::IS_BIGINT, Target::IS_BIGINT);
        // SAFETY: all viewables are safe to transmute from u8.
        let (head, source, _) = unsafe { source.align_to::<Source>() };
        assert!(head.is_empty());
        assert_eq!(target.len(), source.len());
        if Target::IS_FLOAT {
            for (i, source) in source.iter().enumerate() {
                let value = Target::into_storage(Target::from_f64(source.into_f64()));
                target.store(i, value, Ordering::Unordered);
            }
        } else {
            for (i, source) in source.iter().enumerate() {
                let value = Target::into_storage(Target::from_bits(source.into_bits()));
                target.store(i, value, Ordering::Unordered);
            }
        }
    };
}

#[inline]
fn sdb_as_viewable_slice<T: Viewable>(
    sdb: &SharedDataBlock,
    byte_offset: usize,
    byte_length: Option<usize>,
) -> RacySlice<'_, T::Storage> {
    let mut slice = sdb.as_racy_slice().slice_from(byte_offset);
    if let Some(byte_length) = byte_length {
        slice = slice.slice_to(byte_length);
    }

    let (head, slice, _) = slice.align_to::<T::Storage>();
    assert!(head.is_empty());
    slice
}

unsafe impl<T: Viewable> Bindable for GenericSharedTypedArray<'_, T> {
    type Of<'a> = GenericSharedTypedArray<'a, T>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<T: Viewable> Clone for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Viewable> Copy for GenericSharedTypedArray<'_, T> {}

impl<T: Viewable> PartialEq for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: Viewable> Eq for GenericSharedTypedArray<'_, T> {}

impl<T: Viewable> PartialOrd for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Viewable> Ord for GenericSharedTypedArray<'_, T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Viewable> Hash for GenericSharedTypedArray<'_, T> {
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

impl TryFrom<HeapRootData> for SharedTypedArray<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::SharedInt8Array(ta) => Ok(Self::SharedInt8Array(ta)),
            HeapRootData::SharedUint8Array(ta) => Ok(Self::SharedUint8Array(ta)),
            HeapRootData::SharedUint8ClampedArray(ta) => Ok(Self::SharedUint8ClampedArray(ta)),
            HeapRootData::SharedInt16Array(ta) => Ok(Self::SharedInt16Array(ta)),
            HeapRootData::SharedUint16Array(ta) => Ok(Self::SharedUint16Array(ta)),
            HeapRootData::SharedInt32Array(ta) => Ok(Self::SharedInt32Array(ta)),
            HeapRootData::SharedUint32Array(ta) => Ok(Self::SharedUint32Array(ta)),
            HeapRootData::SharedBigInt64Array(ta) => Ok(Self::SharedBigInt64Array(ta)),
            HeapRootData::SharedBigUint64Array(ta) => Ok(Self::SharedBigUint64Array(ta)),
            #[cfg(feature = "proposal-float16array")]
            HeapRootData::SharedFloat16Array(ta) => Ok(Self::SharedFloat16Array(ta)),
            HeapRootData::SharedFloat32Array(ta) => Ok(Self::SharedFloat32Array(ta)),
            HeapRootData::SharedFloat64Array(ta) => Ok(Self::SharedFloat64Array(ta)),
            _ => Err(()),
        }
    }
}

impl<'a, T: Viewable> CreateHeapData<SharedTypedArrayRecord<'a>, GenericSharedTypedArray<'a, T>>
    for Heap
{
    fn create(&mut self, data: SharedTypedArrayRecord<'a>) -> GenericSharedTypedArray<'a, T> {
        self.shared_typed_arrays.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<SharedTypedArrayRecord<'static>>();
        // TODO: The type should be checked based on data or something equally stupid
        GenericSharedTypedArray(BaseIndex::last_t(&self.shared_typed_arrays), PhantomData)
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

impl HeapMarkAndSweep for SharedTypedArray<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Self::SharedInt8Array(ta) => ta.mark_values(queues),
            Self::SharedUint8Array(ta) => ta.mark_values(queues),
            Self::SharedUint8ClampedArray(ta) => ta.mark_values(queues),
            Self::SharedInt16Array(ta) => ta.mark_values(queues),
            Self::SharedUint16Array(ta) => ta.mark_values(queues),
            Self::SharedInt32Array(ta) => ta.mark_values(queues),
            Self::SharedUint32Array(ta) => ta.mark_values(queues),
            Self::SharedBigInt64Array(ta) => ta.mark_values(queues),
            Self::SharedBigUint64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            Self::SharedFloat16Array(ta) => ta.mark_values(queues),
            Self::SharedFloat32Array(ta) => ta.mark_values(queues),
            Self::SharedFloat64Array(ta) => ta.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::SharedInt8Array(ta) => ta.sweep_values(compactions),
            Self::SharedUint8Array(ta) => ta.sweep_values(compactions),
            Self::SharedUint8ClampedArray(ta) => ta.sweep_values(compactions),
            Self::SharedInt16Array(ta) => ta.sweep_values(compactions),
            Self::SharedUint16Array(ta) => ta.sweep_values(compactions),
            Self::SharedInt32Array(ta) => ta.sweep_values(compactions),
            Self::SharedUint32Array(ta) => ta.sweep_values(compactions),
            Self::SharedBigInt64Array(ta) => ta.sweep_values(compactions),
            Self::SharedBigUint64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            Self::SharedFloat16Array(ta) => ta.sweep_values(compactions),
            Self::SharedFloat32Array(ta) => ta.sweep_values(compactions),
            Self::SharedFloat64Array(ta) => ta.sweep_values(compactions),
        }
    }
}
