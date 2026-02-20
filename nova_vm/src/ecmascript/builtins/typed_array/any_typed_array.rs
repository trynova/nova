// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::{FLOAT_16_ARRAY_DISCRIMINANT, Float16Array};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::{
    SHARED_BIGINT_64_ARRAY_DISCRIMINANT, SHARED_BIGUINT_64_ARRAY_DISCRIMINANT,
    SHARED_FLOAT_32_ARRAY_DISCRIMINANT, SHARED_FLOAT_64_ARRAY_DISCRIMINANT,
    SHARED_INT_8_ARRAY_DISCRIMINANT, SHARED_INT_16_ARRAY_DISCRIMINANT,
    SHARED_INT_32_ARRAY_DISCRIMINANT, SHARED_UINT_8_ARRAY_DISCRIMINANT,
    SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT, SHARED_UINT_16_ARRAY_DISCRIMINANT,
    SHARED_UINT_32_ARRAY_DISCRIMINANT, SharedBigInt64Array, SharedBigUint64Array,
    SharedFloat32Array, SharedFloat64Array, SharedInt8Array, SharedInt16Array, SharedInt32Array,
    SharedUint8Array, SharedUint8ClampedArray, SharedUint16Array, SharedUint32Array,
};
#[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
use crate::ecmascript::{SHARED_FLOAT_16_ARRAY_DISCRIMINANT, SharedFloat16Array};
use crate::{
    ecmascript::{
        Agent, AnyArrayBuffer, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
        BigInt64Array, BigUint64Array, CachedBufferByteLength, DataBlock,
        FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, Float32Array, Float64Array,
        Function, INT_8_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
        Int8Array, Int16Array, Int32Array, InternalMethods, InternalSlots, JsResult, Numeric,
        Object, ObjectShape, OrdinaryObject, PropertyDescriptor, PropertyKey, PropertyLookupCache,
        PropertyOffset, ProtoIntrinsics, SetAtOffsetProps, SetResult, TryGetResult, TryHasResult,
        TryResult, TypedArray, TypedArrayAbstractOperations, UINT_8_ARRAY_DISCRIMINANT,
        UINT_8_CLAMPED_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT,
        Uint8Array, Uint8ClampedArray, Uint16Array, Uint32Array, Value,
    },
    engine::{GcScope, HeapRootData, NoGcScope, Scoped, bindable_handle},
    heap::HeapMarkAndSweep,
};

/// ## [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
///
/// An AnyTypedArray presents an array-like view of an underlying binary data
/// buffer (25.1).
///
/// In Nova engine, TypedArrays viewing a [`SharedArrayBuffer`] are represented
/// by a [`SharedTypedArray`], and TypedArrays viewing an [`ArrayBuffer`] are
/// represented by a [`TypedArray`].
///
/// [`ArrayBuffer`]: crate::ecmascript::builtins::ArrayBuffer
/// [`SharedArrayBuffer`]: crate::ecmascript::builtins::SharedArrayBuffer
/// [`SharedTypedArray`]: crate::ecmascript::builtins::SharedTypedArray
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AnyTypedArray<'a> {
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
}
bindable_handle!(AnyTypedArray);

impl AnyTypedArray<'_> {
    /// Returns true if the TypedArray contains bigints.
    #[inline]
    pub(crate) fn is_bigint(self) -> bool {
        #[cfg(not(feature = "shared-array-buffer"))]
        {
            matches!(self, Self::BigInt64Array(_) | Self::BigUint64Array(_))
        }
        #[cfg(feature = "shared-array-buffer")]
        {
            matches!(
                self,
                Self::BigInt64Array(_)
                    | Self::BigUint64Array(_)
                    | Self::SharedBigInt64Array(_)
                    | Self::SharedBigUint64Array(_)
            )
        }
    }

    /// Returns true if the TypedArray is an Int32Array or BigInt64Array
    /// (shared or not), false otherwise.
    #[inline(always)]
    #[cfg(feature = "atomics")]
    pub(crate) fn is_waitable(self) -> bool {
        #[cfg(not(feature = "shared-array-buffer"))]
        {
            matches!(self, Self::Int32Array(_) | Self::BigInt64Array(_))
        }
        #[cfg(feature = "shared-array-buffer")]
        {
            matches!(
                self,
                Self::Int32Array(_)
                    | Self::BigInt64Array(_)
                    | Self::SharedInt32Array(_)
                    | Self::SharedBigInt64Array(_)
            )
        }
    }

    /// Returns true if the TypedArray contains integers with wrapping overflow
    /// semantics.
    #[cfg(feature = "atomics")]
    pub(crate) fn is_integer(self) -> bool {
        #[cfg(not(feature = "shared-array-buffer"))]
        {
            matches!(
                self,
                Self::Uint8Array(_)
                    | Self::Int8Array(_)
                    | Self::Uint16Array(_)
                    | Self::Int16Array(_)
                    | Self::Uint32Array(_)
                    | Self::Int32Array(_)
                    | Self::BigUint64Array(_)
                    | Self::BigInt64Array(_)
            )
        }
        #[cfg(feature = "shared-array-buffer")]
        {
            matches!(
                self,
                Self::Uint8Array(_)
                    | Self::Int8Array(_)
                    | Self::Uint16Array(_)
                    | Self::Int16Array(_)
                    | Self::Uint32Array(_)
                    | Self::Int32Array(_)
                    | Self::BigUint64Array(_)
                    | Self::BigInt64Array(_)
                    | Self::SharedUint8Array(_)
                    | Self::SharedInt8Array(_)
                    | Self::SharedUint16Array(_)
                    | Self::SharedInt16Array(_)
                    | Self::SharedUint32Array(_)
                    | Self::SharedInt32Array(_)
                    | Self::SharedBigUint64Array(_)
                    | Self::SharedBigInt64Array(_)
            )
        }
    }

    pub(crate) fn intrinsic_default_constructor(self) -> ProtoIntrinsics {
        match self {
            Self::Int8Array(_) => ProtoIntrinsics::Int8Array,
            Self::Uint8Array(_) => ProtoIntrinsics::Uint8Array,
            Self::Uint8ClampedArray(_) => ProtoIntrinsics::Uint8ClampedArray,
            Self::Int16Array(_) => ProtoIntrinsics::Int16Array,
            Self::Uint16Array(_) => ProtoIntrinsics::Uint16Array,
            Self::Int32Array(_) => ProtoIntrinsics::Int32Array,
            Self::Uint32Array(_) => ProtoIntrinsics::Uint32Array,
            Self::BigInt64Array(_) => ProtoIntrinsics::BigInt64Array,
            Self::BigUint64Array(_) => ProtoIntrinsics::BigUint64Array,
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(_) => ProtoIntrinsics::Float16Array,
            Self::Float32Array(_) => ProtoIntrinsics::Float32Array,
            Self::Float64Array(_) => ProtoIntrinsics::Float64Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(_) => ProtoIntrinsics::Int8Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(_) => ProtoIntrinsics::Uint8Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(_) => ProtoIntrinsics::Uint8ClampedArray,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(_) => ProtoIntrinsics::Int16Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(_) => ProtoIntrinsics::Uint16Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(_) => ProtoIntrinsics::Int32Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(_) => ProtoIntrinsics::Uint32Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(_) => ProtoIntrinsics::BigInt64Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(_) => ProtoIntrinsics::BigUint64Array,
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(_) => ProtoIntrinsics::Float16Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(_) => ProtoIntrinsics::Float32Array,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(_) => ProtoIntrinsics::Float64Array,
        }
    }
}

macro_rules! any_typed_array_delegate {
    ($value: ident, $method: ident, $($arg:expr),*) => {
        match $value {
            Self::Int8Array(ta) => ta.$method($($arg),*),
            Self::Uint8Array(ta) => ta.$method($($arg),*),
            Self::Uint8ClampedArray(ta) => ta.$method($($arg),*),
            Self::Int16Array(ta) => ta.$method($($arg),*),
            Self::Uint16Array(ta) => ta.$method($($arg),*),
            Self::Int32Array(ta) => ta.$method($($arg),*),
            Self::Uint32Array(ta) => ta.$method($($arg),*),
            Self::BigInt64Array(ta) => ta.$method($($arg),*),
            Self::BigUint64Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.$method($($arg),*),
            Self::Float32Array(ta) => ta.$method($($arg),*),
            Self::Float64Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(ta) => ta.$method($($arg),*),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(ta) => ta.$method($($arg),*),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(ta) => ta.$method($($arg),*),
        }
    };
}

macro_rules! for_any_typed_array {
    ($value: tt, $ta: tt, $expr: tt) => {
        for_any_typed_array!($value, $ta, $expr, _TA)
    };
    ($value: ident, $ta: ident, $expr: expr, $TA: ident) => {
        match $value {
            AnyTypedArray::Int8Array($ta) => {
                type $TA = i8;
                $expr
            }
            AnyTypedArray::Uint8Array($ta) => {
                type $TA = u8;
                $expr
            }
            AnyTypedArray::Uint8ClampedArray($ta) => {
                type $TA = crate::ecmascript::types::U8Clamped;
                $expr
            }
            AnyTypedArray::Int16Array($ta) => {
                type $TA = i16;
                $expr
            }
            AnyTypedArray::Uint16Array($ta) => {
                type $TA = u16;
                $expr
            }
            AnyTypedArray::Int32Array($ta) => {
                type $TA = i32;
                $expr
            }
            AnyTypedArray::Uint32Array($ta) => {
                type $TA = u32;
                $expr
            }
            AnyTypedArray::BigInt64Array($ta) => {
                type $TA = i64;
                $expr
            }
            AnyTypedArray::BigUint64Array($ta) => {
                type $TA = u64;
                $expr
            }
            #[cfg(feature = "proposal-float16array")]
            AnyTypedArray::Float16Array($ta) => {
                type $TA = f16;
                $expr
            }
            AnyTypedArray::Float32Array($ta) => {
                type $TA = f32;
                $expr
            }
            AnyTypedArray::Float64Array($ta) => {
                type $TA = f64;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt8Array($ta) => {
                type $TA = i8;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8Array($ta) => {
                type $TA = u8;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8ClampedArray($ta) => {
                type $TA = crate::ecmascript::types::U8Clamped;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt16Array($ta) => {
                type $TA = i16;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint16Array($ta) => {
                type $TA = u16;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt32Array($ta) => {
                type $TA = i32;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint32Array($ta) => {
                type $TA = u32;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigInt64Array($ta) => {
                type $TA = i64;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigUint64Array($ta) => {
                type $TA = u64;
                $expr
            }
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            AnyTypedArray::SharedFloat16Array($ta) => {
                type $TA = f16;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat32Array($ta) => {
                type $TA = f32;
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat64Array($ta) => {
                type $TA = f64;
                $expr
            }
        }
    };
}
pub(crate) use for_any_typed_array;

impl<'a> InternalSlots<'a> for AnyTypedArray<'a> {
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        any_typed_array_delegate!(self, get_backing_object, agent)
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("TypedArray should not try to set its backing object");
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!("TypedArray should not try to create its backing object");
    }

    fn get_or_create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        any_typed_array_delegate!(self, get_or_create_backing_object, agent)
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        any_typed_array_delegate!(self, object_shape, agent)
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        any_typed_array_delegate!(self, internal_extensible, agent)
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        any_typed_array_delegate!(self, internal_set_extensible, agent, value)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        any_typed_array_delegate!(self, internal_prototype, agent)
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        any_typed_array_delegate!(self, internal_set_prototype, agent, prototype)
    }
}

impl<'a> InternalMethods<'a> for AnyTypedArray<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<Object<'gc>>> {
        any_typed_array_delegate!(self, try_get_prototype_of, agent, gc)
    }

    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Object<'gc>>> {
        any_typed_array_delegate!(self, internal_get_prototype_of, agent, gc)
    }

    fn try_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        any_typed_array_delegate!(self, try_set_prototype_of, agent, prototype, gc)
    }

    fn internal_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        any_typed_array_delegate!(self, internal_set_prototype_of, agent, prototype, gc)
    }

    fn try_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        any_typed_array_delegate!(self, try_is_extensible, agent, gc)
    }

    fn internal_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        any_typed_array_delegate!(self, internal_is_extensible, agent, gc)
    }

    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        any_typed_array_delegate!(self, try_prevent_extensions, agent, gc)
    }

    fn internal_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        any_typed_array_delegate!(self, internal_prevent_extensions, agent, gc)
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        any_typed_array_delegate!(self, try_get_own_property, agent, property_key, cache, gc)
    }

    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
        any_typed_array_delegate!(self, internal_get_own_property, agent, property_key, gc)
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        any_typed_array_delegate!(
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
        any_typed_array_delegate!(
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
        any_typed_array_delegate!(self, try_has_property, agent, property_key, cache, gc)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        any_typed_array_delegate!(self, internal_has_property, agent, property_key, gc)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        any_typed_array_delegate!(self, try_get, agent, property_key, receiver, cache, gc)
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        any_typed_array_delegate!(self, internal_get, agent, property_key, receiver, gc)
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
        any_typed_array_delegate!(
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
        any_typed_array_delegate!(self, internal_set, agent, property_key, value, receiver, gc)
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        any_typed_array_delegate!(self, try_delete, agent, property_key, gc)
    }

    fn internal_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        any_typed_array_delegate!(self, internal_delete, agent, property_key, gc)
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        any_typed_array_delegate!(self, try_own_property_keys, agent, gc)
    }

    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
        any_typed_array_delegate!(self, internal_own_property_keys, agent, gc)
    }

    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        any_typed_array_delegate!(self, get_own_property_at_offset, agent, offset, gc)
    }

    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        props: &SetAtOffsetProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        any_typed_array_delegate!(self, set_at_offset, agent, props, offset, gc)
    }
}

impl<'a> TypedArrayAbstractOperations<'a> for AnyTypedArray<'a> {
    type ElementType = ();

    fn is_detached(self, agent: &Agent) -> bool {
        any_typed_array_delegate!(self, is_detached, agent)
    }

    fn is_fixed_length(self, agent: &Agent) -> bool {
        any_typed_array_delegate!(self, is_detached, agent)
    }

    fn is_shared(self) -> bool {
        TypedArray::try_from(self).is_err()
    }

    /// \[\[ByteOffset]]
    fn byte_offset(self, agent: &Agent) -> usize {
        any_typed_array_delegate!(self, byte_offset, agent)
    }

    fn byte_length(self, agent: &Agent) -> Option<usize> {
        any_typed_array_delegate!(self, byte_length, agent)
    }

    fn array_length(self, agent: &Agent) -> Option<usize> {
        any_typed_array_delegate!(self, array_length, agent)
    }

    fn typed_array_element_size(self) -> usize {
        any_typed_array_delegate!(self, typed_array_element_size,)
    }

    fn typed_array_set_element(self, agent: &mut Agent, index: i64, num_value: Numeric) {
        any_typed_array_delegate!(self, typed_array_set_element, agent, index, num_value)
    }

    fn typed_array_get_element<'gc>(
        self,
        agent: &mut Agent,
        index: i64,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<Numeric<'gc>> {
        any_typed_array_delegate!(self, typed_array_get_element, agent, index, gc)
    }

    fn viewed_array_buffer(self, agent: &Agent) -> AnyArrayBuffer<'a> {
        any_typed_array_delegate!(self, viewed_array_buffer, agent)
    }

    fn get_cached_buffer_byte_length(
        self,
        agent: &Agent,
        order: ecmascript_atomics::Ordering,
    ) -> CachedBufferByteLength {
        any_typed_array_delegate!(self, get_cached_buffer_byte_length, agent, order)
    }

    fn copy_within<'gc>(
        self,
        agent: &mut Agent,
        start_index: usize,
        target_index: usize,
        count: usize,
    ) {
        any_typed_array_delegate!(self, copy_within, agent, start_index, target_index, count)
    }

    fn fill(self, agent: &mut Agent, value: Numeric, start_index: usize, count: usize) {
        any_typed_array_delegate!(self, fill, agent, value, start_index, count)
    }

    fn filter<'gc>(
        self,
        agent: &mut Agent,
        callback: Function,
        this_arg: Value,
        len: usize,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        any_typed_array_delegate!(self, filter, agent, callback, this_arg, len, gc)
    }

    fn search<const ASCENDING: bool>(
        self,
        agent: &mut Agent,
        search_element: Value,
        start: usize,
        end: usize,
    ) -> Option<usize> {
        match self {
            Self::Int8Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::Uint8Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::Uint8ClampedArray(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            Self::Int16Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::Uint16Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::Int32Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::Uint32Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::BigInt64Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::BigUint64Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::Float32Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            Self::Float64Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(ta) => ta.search::<ASCENDING>(agent, search_element, start, end),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(ta) => {
                ta.search::<ASCENDING>(agent, search_element, start, end)
            }
        }
    }

    fn map<'gc>(
        self,
        agent: &mut Agent,
        callback: Function,
        this_arg: Value,
        len: usize,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        any_typed_array_delegate!(self, map, agent, callback, this_arg, len, gc)
    }

    fn reverse(self, agent: &mut Agent, len: usize) {
        any_typed_array_delegate!(self, reverse, agent, len)
    }

    fn set_into_data_block<'gc>(
        self,
        agent: &Agent,
        target: &mut DataBlock,
        start_index: usize,
        count: usize,
    ) {
        any_typed_array_delegate!(self, set_into_data_block, agent, target, start_index, count)
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
        any_typed_array_delegate!(
            self,
            set_from_typed_array,
            agent,
            target_offset,
            source,
            source_offset,
            length,
            gc
        )
    }

    fn slice(self, agent: &mut Agent, source: AnyTypedArray, source_offset: usize, length: usize) {
        any_typed_array_delegate!(self, slice, agent, source, source_offset, length)
    }

    fn sort_with_comparator<'gc>(
        self,
        agent: &mut Agent,
        len: usize,
        comparator: Scoped<Function>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        any_typed_array_delegate!(self, sort_with_comparator, agent, len, comparator, gc)
    }

    fn sort<'gc>(self, agent: &mut Agent, len: usize) {
        any_typed_array_delegate!(self, sort, agent, len)
    }

    #[inline(always)]
    fn typed_array_create_same_type_and_copy_data<'gc>(
        self,
        agent: &mut Agent,
        len: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, TypedArray<'gc>> {
        any_typed_array_delegate!(
            self,
            typed_array_create_same_type_and_copy_data,
            agent,
            len,
            gc
        )
    }
}

impl HeapMarkAndSweep for AnyTypedArray<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        match self {
            AnyTypedArray::Int8Array(ta) => ta.mark_values(queues),
            AnyTypedArray::Uint8Array(ta) => ta.mark_values(queues),
            AnyTypedArray::Uint8ClampedArray(ta) => ta.mark_values(queues),
            AnyTypedArray::Int16Array(ta) => ta.mark_values(queues),
            AnyTypedArray::Uint16Array(ta) => ta.mark_values(queues),
            AnyTypedArray::Int32Array(ta) => ta.mark_values(queues),
            AnyTypedArray::Uint32Array(ta) => ta.mark_values(queues),
            AnyTypedArray::BigInt64Array(ta) => ta.mark_values(queues),
            AnyTypedArray::BigUint64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            AnyTypedArray::Float16Array(sta) => sta.mark_values(queues),
            AnyTypedArray::Float32Array(ta) => ta.mark_values(queues),
            AnyTypedArray::Float64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt8Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8ClampedArray(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigInt64Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigUint64Array(sta) => sta.mark_values(queues),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            AnyTypedArray::SharedFloat16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat64Array(sta) => sta.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        match self {
            AnyTypedArray::Int8Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::Uint8Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::Uint8ClampedArray(ta) => ta.sweep_values(compactions),
            AnyTypedArray::Int16Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::Uint16Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::Int32Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::Uint32Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::BigInt64Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::BigUint64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            AnyTypedArray::Float16Array(sta) => sta.sweep_values(compactions),
            AnyTypedArray::Float32Array(ta) => ta.sweep_values(compactions),
            AnyTypedArray::Float64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt8Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8ClampedArray(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigInt64Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigUint64Array(sta) => sta.sweep_values(compactions),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            AnyTypedArray::SharedFloat16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat64Array(sta) => sta.sweep_values(compactions),
        }
    }
}

// === OUTPUT FROM object_handle! ADAPTED TO AnyTypedArray ===
impl<'a> From<AnyTypedArray<'a>> for Object<'a> {
    #[inline(always)]
    fn from(value: AnyTypedArray<'a>) -> Self {
        match value {
            AnyTypedArray::Int8Array(ta) => Self::Int8Array(ta),
            AnyTypedArray::Uint8Array(ta) => Self::Uint8Array(ta),
            AnyTypedArray::Uint8ClampedArray(ta) => Self::Uint8ClampedArray(ta),
            AnyTypedArray::Int16Array(ta) => Self::Int16Array(ta),
            AnyTypedArray::Uint16Array(ta) => Self::Uint16Array(ta),
            AnyTypedArray::Int32Array(ta) => Self::Int32Array(ta),
            AnyTypedArray::Uint32Array(ta) => Self::Uint32Array(ta),
            AnyTypedArray::BigInt64Array(ta) => Self::BigInt64Array(ta),
            AnyTypedArray::BigUint64Array(ta) => Self::BigUint64Array(ta),
            #[cfg(feature = "proposal-float16array")]
            AnyTypedArray::Float16Array(sta) => Self::Float16Array(sta),
            AnyTypedArray::Float32Array(ta) => Self::Float32Array(ta),
            AnyTypedArray::Float64Array(ta) => Self::Float64Array(ta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt8Array(sta) => Self::SharedInt8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8Array(sta) => Self::SharedUint8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint8ClampedArray(sta) => Self::SharedUint8ClampedArray(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt16Array(sta) => Self::SharedInt16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint16Array(sta) => Self::SharedUint16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedInt32Array(sta) => Self::SharedInt32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedUint32Array(sta) => Self::SharedUint32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigInt64Array(sta) => Self::SharedBigInt64Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedBigUint64Array(sta) => Self::SharedBigUint64Array(sta),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            AnyTypedArray::SharedFloat16Array(sta) => Self::SharedFloat16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat32Array(sta) => Self::SharedFloat32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            AnyTypedArray::SharedFloat64Array(sta) => Self::SharedFloat64Array(sta),
        }
    }
}
impl<'a> From<AnyTypedArray<'a>> for Value<'a> {
    #[inline(always)]
    fn from(value: AnyTypedArray<'a>) -> Self {
        let value: Object = value.into();
        value.into()
    }
}
impl<'a> From<AnyTypedArray<'a>> for HeapRootData {
    #[inline(always)]
    fn from(value: AnyTypedArray<'a>) -> Self {
        let value: Object = value.into();
        value.into()
    }
}
impl<'a> TryFrom<Object<'a>> for AnyTypedArray<'a> {
    type Error = ();
    #[inline]
    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        let value: Value = value.into();
        Self::try_from(value)
    }
}
impl<'a> TryFrom<Value<'a>> for AnyTypedArray<'a> {
    type Error = ();
    #[inline]
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Int8Array(ta) => Ok(Self::Int8Array(ta)),
            Value::Uint8Array(ta) => Ok(Self::Uint8Array(ta)),
            Value::Uint8ClampedArray(ta) => Ok(Self::Uint8ClampedArray(ta)),
            Value::Int16Array(ta) => Ok(Self::Int16Array(ta)),
            Value::Uint16Array(ta) => Ok(Self::Uint16Array(ta)),
            Value::Int32Array(ta) => Ok(Self::Int32Array(ta)),
            Value::Uint32Array(ta) => Ok(Self::Uint32Array(ta)),
            Value::BigInt64Array(ta) => Ok(Self::BigInt64Array(ta)),
            Value::BigUint64Array(ta) => Ok(Self::BigUint64Array(ta)),
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(sta) => Ok(Self::Float16Array(sta)),
            Value::Float32Array(ta) => Ok(Self::Float32Array(ta)),
            Value::Float64Array(ta) => Ok(Self::Float64Array(ta)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt8Array(base_index) => Ok(Self::SharedInt8Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint8Array(base_index) => Ok(Self::SharedUint8Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint8ClampedArray(base_index) => {
                Ok(Self::SharedUint8ClampedArray(base_index))
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt16Array(base_index) => Ok(Self::SharedInt16Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint16Array(base_index) => Ok(Self::SharedUint16Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt32Array(base_index) => Ok(Self::SharedInt32Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint32Array(base_index) => Ok(Self::SharedUint32Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedBigInt64Array(base_index) => Ok(Self::SharedBigInt64Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedBigUint64Array(base_index) => Ok(Self::SharedBigUint64Array(base_index)),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Value::SharedFloat16Array(base_index) => Ok(Self::SharedFloat16Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedFloat32Array(base_index) => Ok(Self::SharedFloat32Array(base_index)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedFloat64Array(base_index) => Ok(Self::SharedFloat64Array(base_index)),
            _ => Err(()),
        }
    }
}
impl TryFrom<HeapRootData> for AnyTypedArray<'_> {
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
            #[cfg(feature = "proposal-float16array")]
            HeapRootData::Float16Array(ta) => Ok(Self::Float16Array(ta)),
            HeapRootData::Float32Array(ta) => Ok(Self::Float32Array(ta)),
            HeapRootData::Float64Array(ta) => Ok(Self::Float64Array(ta)),
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
            _ => Err(()),
        }
    }
}
// === END ===
