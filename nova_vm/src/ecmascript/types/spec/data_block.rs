// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [6.2.9 Data Blocks](https://tc39.es/ecma262/#sec-data-blocks)

use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::{NonNull, read_unaligned, write_unaligned},
};
use std::{
    alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error, realloc},
    f32, f64,
};

use num_bigint::Sign;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int64_big_int, to_big_uint64_big_int, to_int8_number, to_int16_number,
            to_int32_number, to_uint8_clamp_number, to_uint8_number, to_uint16_number,
            to_uint32_number,
        },
        execution::{Agent, JsResult, agent::ExceptionType},
        types::{BigInt, IntoNumeric, Number, Numeric, Value},
    },
    engine::context::NoGcScope,
};

#[cfg(feature = "array-buffer")]
use crate::ecmascript::execution::ProtoIntrinsics;

/// # [Data Block](https://tc39.es/ecma262/#sec-data-blocks)
///
/// The Data Block specification type is used to describe a distinct and
/// mutable sequence of byte-sized (8 bit) numeric values. A byte value
/// is an integer in the inclusive interval from 0 to 255. A Data Block
/// value is created with a fixed number of bytes that each have the
/// initial value 0.
///
/// The `ptr` points to a continuous buffer
/// of bytes, the length of which is determined by
/// the capacity. The pointer can be None if the
/// capacity of the buffer is zero.
#[derive(Debug, Clone)]
pub(crate) struct DataBlock {
    ptr: Option<NonNull<u8>>,
    byte_length: usize,
}

impl Drop for DataBlock {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            if self.byte_length == 0 {
                // dangling data block; don't dealloc.
                return;
            }
            let layout = Layout::from_size_align(self.byte_length, 8).unwrap();
            unsafe { dealloc(ptr.as_ptr(), layout) }
        }
    }
}

impl Deref for DataBlock {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        if let Some(ptr) = self.ptr {
            // SAFETY: DataBlock has a non-null, pointer. We guarantee it
            // points to a valid allocation of byte_length initialized bytes
            // (note, length can be 0 in which case pointer is dangling).
            unsafe { std::slice::from_raw_parts(ptr.as_ptr(), self.byte_length) }
        } else {
            &[]
        }
    }
}

impl DerefMut for DataBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(mut ptr) = self.ptr {
            // SAFETY: DataBlock has a non-null, pointer. We guarantee it
            // points to a valid allocation of byte_length initialized bytes
            // (note, length can be 0 in which case pointer is dangling).
            unsafe { std::slice::from_raw_parts_mut(ptr.as_mut(), self.byte_length) }
        } else {
            &mut []
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct U8Clamped(pub u8);

impl core::fmt::Debug for U8Clamped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

mod private {
    use super::U8Clamped;

    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for U8Clamped {}
    impl Sealed for i8 {}
    impl Sealed for u16 {}
    impl Sealed for i16 {}
    impl Sealed for u32 {}
    impl Sealed for i32 {}
    impl Sealed for u64 {}
    impl Sealed for i64 {}
    #[cfg(feature = "proposal-float16array")]
    impl Sealed for f16 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

pub trait Viewable: 'static + private::Sealed + Copy + PartialEq {
    /// Functions as the \[\[ContentType\]\] internal slot of the TypedArray and
    /// as a marker for data views. Used to determine that the viewable type is
    /// a BigInt.
    const IS_BIGINT: bool = false;
    const IS_FLOAT: bool = false;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics;

    fn into_be_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a>;
    fn into_le_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a>;
    #[inline(always)]
    fn into_ne_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a> {
        if cfg!(target_endian = "little") {
            self.into_le_value(agent, gc)
        } else {
            self.into_be_value(agent, gc)
        }
    }
    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self;
    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self;
    #[inline(always)]
    fn from_ne_value(agent: &mut Agent, value: Numeric) -> Self {
        if cfg!(target_endian = "little") {
            Self::from_le_value(agent, value)
        } else {
            Self::from_be_value(agent, value)
        }
    }
    /// Try reinterpret a Value to Viewable.
    ///
    /// This method is intended for cases where the ECMAScript specification
    /// requires repeatedly converting existing Viewable values to Value and
    /// comparing them with a search element. In this case, the Viewable to
    /// Value conversion is lossless and the comparison function is the only
    /// place where some value coercion may happen; this is generally the -0.0
    /// value being coerced to 0.
    ///
    /// Thus, this method must not do conversion, rounding, or clamping of
    /// numeric values.
    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self>;
    fn default() -> Self;

    /// Convert a Viewable value into a u64 holding an integer.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn into_bits(self) -> u64;
    /// Convert a u64 holding an integer into a Viewable.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn from_bits(bits: u64) -> Self;

    /// Convert a Viewable value into an f64.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn into_f64(self) -> f64;
    /// Convert an f64 into a Viewable.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn from_f64(value: f64) -> Self;
}

impl Viewable for u8 {
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint8Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint8_number(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint8_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        u8::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self.into()
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for U8Clamped {
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint8ClampedArray;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.0.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.0.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self(to_uint8_clamp_number(agent, value).to_be())
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self(to_uint8_clamp_number(agent, value).to_le())
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(U8Clamped(0));
            }
            return None;
        };
        u8::try_from(value.into_i64()).ok().map(U8Clamped)
    }

    fn default() -> Self {
        U8Clamped(0)
    }

    fn into_bits(self) -> u64 {
        self.0.into()
    }

    fn from_bits(bits: u64) -> Self {
        U8Clamped(bits.clamp(0, 255) as u8)
    }

    fn into_f64(self) -> f64 {
        self.0.into()
    }

    fn from_f64(value: f64) -> Self {
        U8Clamped(value.clamp(0.0, 255.0).round_ties_even() as u8)
    }
}
impl Viewable for i8 {
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int8Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int8_number(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int8_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        i8::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for u16 {
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint16Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint16_number(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint16_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        u16::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for i16 {
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int16Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int16_number(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int16_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        i16::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for u32 {
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint32Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint32_number(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint32_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        u32::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for i32 {
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int32Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int32_number(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int32_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        i32::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for u64 {
    const IS_BIGINT: bool = true;
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::BigUint64Array;

    fn into_be_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_u64(agent, self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_u64(agent, self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_uint64_big_int(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_uint64_big_int(agent, value).to_le()
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        if let Value::SmallBigInt(value) = value {
            let value = value.into_i64();
            return u64::try_from(value).ok();
        };
        if let Value::BigInt(value) = value {
            let data = &agent[value];
            let mut iter = data.data.iter_u64_digits();
            let sign = data.data.sign();
            if sign == Sign::Minus {
                return None;
            }
            if iter.len() > 1 {
                return None;
            }
            let value = iter.next().unwrap();
            return Some(value);
        };
        None
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self
    }

    fn from_bits(bits: u64) -> Self {
        bits
    }

    fn into_f64(self) -> f64 {
        self as f64
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for i64 {
    const IS_BIGINT: bool = true;
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::BigInt64Array;

    fn into_be_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_i64(agent, self.to_be()).into_numeric()
    }

    fn into_le_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_i64(agent, self.to_le()).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_int64_big_int(agent, value).to_be()
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_int64_big_int(agent, value).to_le()
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        if let Value::SmallBigInt(value) = value {
            return Some(value.into_i64());
        };
        if let Value::BigInt(value) = value {
            let data = &agent[value];
            let mut iter = data.data.iter_u64_digits();
            if iter.len() > 1 {
                return None;
            }
            let sign = data.data.sign();
            let value = iter.next().unwrap();
            if sign == Sign::Minus {
                if value <= i64::MIN.unsigned_abs() {
                    return Some(value.wrapping_neg() as i64);
                } else {
                    return None;
                }
            } else if value <= i64::MAX as u64 {
                return Some(value as i64);
            } else {
                return None;
            }
        };
        None
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self as f64
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
#[cfg(feature = "proposal-float16array")]
impl Viewable for f16 {
    const IS_FLOAT: bool = true;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float16Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_be_bytes())).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_le_bytes())).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_be_bytes())
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_le_bytes())
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        let Ok(value) = Number::try_from(value) else {
            return None;
        };
        let value = value.into_f64(agent);
        if value.is_nan() {
            return Some(f16::NAN);
        }
        if value as f16 as f64 == value {
            Some(value as f16)
        } else {
            None
        }
    }
    fn default() -> Self {
        f16::NAN
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for f32 {
    const IS_FLOAT: bool = true;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float32Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_be_bytes())).into_numeric()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_le_bytes())).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_be_bytes())
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_le_bytes())
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        let Ok(value) = Number::try_from(value) else {
            return None;
        };
        let value = value.into_f64(agent);
        if value.is_nan() {
            return Some(f32::NAN);
        }
        if value as f32 as f64 == value {
            Some(value as f32)
        } else {
            None
        }
    }

    fn default() -> Self {
        f32::NAN
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl Viewable for f64 {
    const IS_FLOAT: bool = true;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float64Array;

    fn into_be_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from_f64(agent, Self::from_ne_bytes(self.to_be_bytes()), gc).into_numeric()
    }

    fn into_le_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from_f64(agent, Self::from_ne_bytes(self.to_le_bytes()), gc).into_numeric()
    }

    fn from_be_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_be_bytes())
    }

    fn from_le_value(agent: &mut Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_le_bytes())
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        let Ok(value) = Number::try_from(value) else {
            return None;
        };
        Some(value.into_f64(agent))
    }

    fn default() -> Self {
        f64::NAN
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    fn into_f64(self) -> f64 {
        self
    }

    fn from_f64(value: f64) -> Self {
        value
    }
}

impl DataBlock {
    /// A detached DataBlock.
    pub(crate) const DETACHED_DATA_BLOCK: DataBlock = DataBlock {
        ptr: None,
        byte_length: 0,
    };

    /// An empty DataBlock.
    const EMPTY_DATA_BLOCK: DataBlock = DataBlock {
        ptr: Some(NonNull::<usize>::dangling().cast::<u8>()),
        byte_length: 0,
    };

    pub fn is_detached(&self) -> bool {
        self.ptr.is_none()
    }

    fn new(len: usize) -> Self {
        if len == 0 {
            Self::EMPTY_DATA_BLOCK
        } else {
            let layout = Layout::from_size_align(len, 8).unwrap();
            // SAFETY: Size of allocation is non-zero.
            let ptr = unsafe { alloc_zeroed(layout) };
            let Some(ptr) = NonNull::new(ptr) else {
                // TODO: Throw error?
                handle_alloc_error(layout);
            };
            debug_assert_eq!(ptr.align_offset(8), 0);
            Self {
                ptr: Some(ptr),
                byte_length: len,
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.byte_length
    }

    pub fn view_len<T: Viewable>(&self, byte_offset: usize) -> usize {
        let size = core::mem::size_of::<T>();
        (self.byte_length - byte_offset) / size
    }

    fn as_ptr(&self, byte_offset: usize) -> Option<*const u8> {
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { data.as_ptr().add(byte_offset) as *const _ }
            })
        }
    }

    fn as_mut_ptr(&mut self, byte_offset: usize) -> Option<*mut u8> {
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { data.as_ptr().add(byte_offset) }
            })
        }
    }

    pub fn get<T: Viewable>(&self, offset: usize) -> Option<T> {
        let size = core::mem::size_of::<T>();
        let byte_offset = offset * size;
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { read_unaligned(data.as_ptr().add(offset).cast()) }
            })
        }
    }

    pub fn get_offset_by_byte<T: Viewable>(&self, byte_offset: usize) -> Option<T> {
        let size = core::mem::size_of::<T>();
        let end_byte_offset = byte_offset + size;
        if end_byte_offset > self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { read_unaligned(data.as_ptr().byte_add(byte_offset).cast()) }
            })
        }
    }

    pub fn set<T: Viewable>(&mut self, offset: usize, value: T) {
        let size = core::mem::size_of::<T>();
        if let Some(data) = self.ptr {
            // Note: We have to check offset + 1 to ensure that the write does
            // not reach data beyond the end of the DataBlock allocation.
            let end_byte_offset = (offset + 1) * size;
            if end_byte_offset <= self.byte_length {
                // SAFETY: The data is properly initialized, and the T being written is
                // checked to be fully within the length of the data allocation.
                unsafe { write_unaligned(data.as_ptr().add(offset).cast(), value) }
            }
        }
    }

    pub fn set_offset_by_byte<T: Viewable>(&mut self, byte_offset: usize, value: T) {
        let size = core::mem::size_of::<T>();
        if let Some(data) = self.ptr {
            // Note: We have to check offset + 1 to ensure that the write does
            // not reach data beyond the end of the DataBlock allocation.
            let end_byte_offset = byte_offset + size;
            if end_byte_offset <= self.byte_length {
                // SAFETY: The data is properly initialized, and the T being written is
                // checked to be fully within the length of the data allocation.
                unsafe { write_unaligned(data.as_ptr().byte_add(byte_offset).cast(), value) }
            }
        }
    }

    pub fn set_from<T: Viewable>(
        &mut self,
        dst_offset: usize,
        src: &DataBlock,
        src_offset: usize,
        count: usize,
    ) {
        let size = core::mem::size_of::<T>();
        let byte_length = count * size;
        if byte_length == 0 {
            return;
        }
        let dst_byte_offset = dst_offset * size;
        let src_byte_offset = src_offset * size;
        debug_assert!(dst_byte_offset + byte_length <= self.byte_length);
        debug_assert!(src_byte_offset + byte_length <= src.byte_length);
        let src_ptr = src.as_ptr(src_byte_offset);
        let dst_ptr = self.as_mut_ptr(dst_byte_offset);
        if let (Some(src), Some(dst)) = (src_ptr, dst_ptr) {
            // SAFETY: Source buffer length is valid, destination buffer
            // is likewise at least equal in length to source, and both
            // are properly aligned for bytes.
            unsafe { dst.copy_from_nonoverlapping(src, byte_length) }
        }
    }

    pub fn copy_within<T: Viewable>(&mut self, dst_offset: usize, src_offset: usize, count: usize) {
        let size = core::mem::size_of::<T>();
        let byte_length = count * size;
        if byte_length == 0 {
            return;
        }
        let dst_byte_offset = dst_offset * size;
        let src_byte_offset = src_offset * size;
        debug_assert!(dst_byte_offset + byte_length <= self.byte_length);
        debug_assert!(src_byte_offset + byte_length <= self.byte_length);
        if let Some(ptr) = self.as_mut_ptr(0) {
            // SAFETY: Buffer is valid for reads and writes of u8 for the whole length.
            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, self.byte_length) };
            slice.copy_within(
                src_byte_offset..(src_byte_offset + byte_length),
                dst_byte_offset,
            );
        }
    }

    /// ### [6.2.9.1 CreateByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createbytedatablock)
    ///
    /// The abstract operation CreateByteDataBlock takes argument size (a
    /// non-negative integer) and returns either a normal completion containing
    /// a Data Block or a throw completion.
    pub fn create_byte_data_block<'a>(
        agent: &mut Agent,
        size: u64,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        // 1. If size > 2**53 - 1, throw a RangeError exception.
        if size > u64::pow(2, 53) - 1 {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Not a safe integer",
                gc,
            ))
        } else if let Ok(size) = usize::try_from(size) {
            // 2. Let db be a new Data Block value consisting of size bytes.
            // 3. Set all of the bytes of db to 0.
            // 4. Return db.
            Ok(Self::new(size))
        } else {
            // 2. cont: If it is impossible to create such a Data Block, throw a RangeError exception.
            // TODO: throw a RangeError exception
            Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Invalid Data Block length",
                gc,
            ))
        }
    }

    /// ### [6.2.9.2 CreateSharedByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createsharedbytedatablock)
    ///
    /// The abstract operation CreateSharedByteDataBlock takes argument size (a
    /// non-negative integer) and returns either a normal completion containing
    /// a Shared Data Block or a throw completion.
    pub fn create_shared_byte_data_block<'a>(
        agent: &mut Agent,
        size: u64,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        // 1. Let db be a new Shared Data Block value consisting of size bytes. If it is impossible to create such a Shared Data Block, throw a RangeError exception.
        if let Ok(size) = usize::try_from(size) {
            // 2. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
            // 3. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
            // 4. Let zero be « 0 ».
            // 5. For each index i of db, do
            // a. Append WriteSharedMemory { [[Order]]: INIT, [[NoTear]]: true, [[Block]]: db, [[ByteIndex]]: i, [[ElementSize]]: 1, [[Payload]]: zero } to eventsRecord.[[EventList]].
            Ok(Self::new(size))
        } else {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid Shared Data Block length",
                gc,
            ))
        }
        // 6. Return db.
    }

    /// ### [6.2.9.3 CopyDataBlockBytes ( toBlock, toIndex, fromBlock, fromIndex, count )](https://tc39.es/ecma262/#sec-copydatablockbytes)
    ///
    /// The abstract operation CopyDataBlockBytes takes arguments toBlock (a
    /// Data Block or a Shared Data Block), toIndex (a non-negative integer),
    /// fromBlock (a Data Block or a Shared Data Block), fromIndex (a
    /// non-negative integer), and count (a non-negative integer) and returns
    /// UNUSED.
    pub fn copy_data_block_bytes(
        &mut self,
        to_index: usize,
        from_block: &Self,
        from_index: usize,
        count: usize,
    ) {
        let to_block = self;
        // 1. Assert: fromBlock and toBlock are distinct values.
        assert!(
            to_block.ptr.is_none()
                || from_block.ptr.is_none()
                || unsafe {
                    to_block.ptr.unwrap().as_ptr().add(to_block.len())
                        <= from_block.ptr.unwrap().as_ptr()
                        || from_block.ptr.unwrap().as_ptr().add(from_block.len())
                            <= to_block.ptr.unwrap().as_ptr()
                }
        );
        // 2. Let fromSize be the number of bytes in fromBlock.
        let from_size = from_block.byte_length;
        // 3. Assert: fromIndex + count ≤ fromSize.
        assert!(from_index + count <= from_size);
        // 4. Let toSize be the number of bytes in toBlock.
        let to_size = to_block.byte_length;
        // 5. Assert: toIndex + count ≤ toSize.
        assert!(to_index + count <= to_size);
        // 6. Repeat, while count > 0,
        //      a. If fromBlock is a Shared Data Block, then
        //          i. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
        //          ii. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
        //          iii. Let bytes be a List whose sole element is a nondeterministically chosen byte value.
        //          iv. NOTE: In implementations, bytes is the result of a non-atomic read instruction on the underlying hardware. The nondeterminism is a semantic prescription of the memory model to describe observable behaviour of hardware with weak consistency.
        //          v. Let readEvent be ReadSharedMemory { [[Order]]: UNORDERED, [[NoTear]]: true, [[Block]]: fromBlock, [[ByteIndex]]: fromIndex, [[ElementSize]]: 1 }.
        //          vi. Append readEvent to eventsRecord.[[EventList]].
        //          vii. Append Chosen Value Record { [[Event]]: readEvent, [[ChosenValue]]: bytes } to execution.[[ChosenValues]].
        //          viii. If toBlock is a Shared Data Block, then
        //              1. Append WriteSharedMemory { [[Order]]: UNORDERED, [[NoTear]]: true, [[Block]]: toBlock, [[ByteIndex]]: toIndex, [[ElementSize]]: 1, [[Payload]]: bytes } to eventsRecord.[[EventList]].
        //          ix. Else,
        //              1. Set toBlock[toIndex] to bytes[0].
        //      b. Else,
        //           i. Assert: toBlock is not a Shared Data Block.
        //           ii. Set toBlock[toIndex] to fromBlock[fromIndex].
        //           c. Set toIndex to toIndex + 1.
        //           d. Set fromIndex to fromIndex + 1.
        //           e. Set count to count - 1.
        let to_ptr = if let Some(ptr) = to_block.as_mut_ptr(to_index) {
            ptr
        } else {
            return;
        };
        let from_ptr = if let Some(ptr) = from_block.as_ptr(from_index) {
            ptr
        } else {
            return;
        };
        // SAFETY: Pointers have been checked to not overlap.
        unsafe { to_ptr.copy_from_nonoverlapping(from_ptr, count) };
        // 7. Return UNUSED.
    }

    pub fn realloc(&mut self, new_byte_length: usize) {
        // Max byte length should be within safe integer length.
        debug_assert!(new_byte_length < 2usize.pow(53));
        let ptr = if let Some(ptr) = self.ptr {
            if new_byte_length == 0 {
                // When resizing to zero, we can just reassign self to an epty
                // data block; that drops the previous block which deallocs.
                *self = Self::EMPTY_DATA_BLOCK;
                return;
            } else {
                // SAFETY: `ptr` can currently only come from GlobalAllocator, it was
                // allocated with `Layout::from_size_align(self.byte_length, 8)`, new
                // size is non-zero, and cannot overflow isize (on a 64-bit machine).
                if self.byte_length > 0 {
                    let layout = Layout::from_size_align(self.byte_length, 8).unwrap();
                    unsafe { realloc(ptr.as_ptr(), layout, new_byte_length) }
                } else {
                    let layout = Layout::from_size_align(new_byte_length, 8).unwrap();
                    unsafe { alloc_zeroed(layout) }
                }
            }
        } else {
            // Detached.
            return;
        };
        let Some(ptr) = NonNull::new(ptr) else {
            let layout = Layout::from_size_align(new_byte_length, 8).unwrap();
            handle_alloc_error(layout);
        };
        self.ptr = Some(ptr);
        if new_byte_length > self.byte_length {
            // Need to zero out the new data.
            // SAFETY: The new pointer does point to valid data which is
            // big enough.
            let new_data_ptr = unsafe { ptr.add(self.byte_length) };
            // SAFETY: The new pointer does point to valid, big enough
            // allocation which contains uninitialized bytes. No one else
            // can hold a reference to it currently.
            let data_slice = unsafe {
                core::slice::from_raw_parts_mut(
                    new_data_ptr.as_ptr().cast::<MaybeUninit<u8>>(),
                    new_byte_length - self.byte_length,
                )
            };
            data_slice.fill(MaybeUninit::new(0));
        }
        self.byte_length = new_byte_length;
    }
}

#[test]
fn new_data_block() {
    let db = DataBlock::new(0);
    assert_eq!(db.len(), 0);
    assert_eq!(db.get::<u8>(0), None);

    let db = DataBlock::new(8);
    assert_eq!(db.len(), 8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i), Some(0));
    }
}

#[test]
fn data_block_set() {
    let mut db = DataBlock::new(8);
    assert_eq!(db.len(), 8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i), Some(0));
    }

    for i in 0..8 {
        db.set::<u8>(i as usize, i + 1);
    }

    for i in 0..8 {
        assert_eq!(db.get::<u8>(i as usize), Some(i + 1));
    }
}

#[test]
fn data_block_set_from() {
    let mut db = DataBlock::new(8);
    let mut db2 = DataBlock::new(8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(0), Some(0));
        db2.set::<u8>(i as usize, i + 1);
    }
    assert_eq!(db2.get::<u8>(0), Some(1));
    assert_eq!(db2.get::<u8>(1), Some(2));
    assert_eq!(db2.get::<u8>(2), Some(3));
    assert_eq!(db2.get::<u8>(3), Some(4));
    assert_eq!(db2.get::<u8>(4), Some(5));
    assert_eq!(db2.get::<u8>(5), Some(6));
    assert_eq!(db2.get::<u8>(6), Some(7));
    assert_eq!(db2.get::<u8>(7), Some(8));
    db.set_from::<u8>(0, &db2, 4, 4);
    assert_eq!(db.get::<u8>(0), Some(5));
    assert_eq!(db.get::<u8>(1), Some(6));
    assert_eq!(db.get::<u8>(2), Some(7));
    assert_eq!(db.get::<u8>(3), Some(8));
    assert_eq!(db.get::<u8>(4), Some(0));
    assert_eq!(db.get::<u8>(5), Some(0));
    assert_eq!(db.get::<u8>(6), Some(0));
    assert_eq!(db.get::<u8>(7), Some(0));

    // Reset
    for i in 0..8 {
        db.set::<u8>(i as usize, i + 1);
    }
    db.copy_within::<u8>(2, 4, 4);
    assert_eq!(db.get::<u8>(0), Some(1));
    assert_eq!(db.get::<u8>(1), Some(2));
    assert_eq!(db.get::<u8>(2), Some(5));
    assert_eq!(db.get::<u8>(3), Some(6));
    assert_eq!(db.get::<u8>(4), Some(7));
    assert_eq!(db.get::<u8>(5), Some(8));
    assert_eq!(db.get::<u8>(6), Some(7));
    assert_eq!(db.get::<u8>(7), Some(8));
}

#[test]
fn data_block_copy_within() {
    let mut db = DataBlock::new(8);
    for i in 0..8 {
        db.set::<u8>(i as usize, i + 1);
    }
    assert_eq!(db.get::<u8>(0), Some(1));
    assert_eq!(db.get::<u8>(1), Some(2));
    assert_eq!(db.get::<u8>(2), Some(3));
    assert_eq!(db.get::<u8>(3), Some(4));
    assert_eq!(db.get::<u8>(4), Some(5));
    assert_eq!(db.get::<u8>(5), Some(6));
    assert_eq!(db.get::<u8>(6), Some(7));
    assert_eq!(db.get::<u8>(7), Some(8));
    db.copy_within::<u8>(0, 4, 4);
    assert_eq!(db.get::<u8>(0), Some(5));
    assert_eq!(db.get::<u8>(1), Some(6));
    assert_eq!(db.get::<u8>(2), Some(7));
    assert_eq!(db.get::<u8>(3), Some(8));
    assert_eq!(db.get::<u8>(4), Some(5));
    assert_eq!(db.get::<u8>(5), Some(6));
    assert_eq!(db.get::<u8>(6), Some(7));
    assert_eq!(db.get::<u8>(7), Some(8));

    // Reset
    for i in 0..8 {
        db.set::<u8>(i as usize, i + 1);
    }
    db.copy_within::<u8>(2, 4, 4);
    assert_eq!(db.get::<u8>(0), Some(1));
    assert_eq!(db.get::<u8>(1), Some(2));
    assert_eq!(db.get::<u8>(2), Some(5));
    assert_eq!(db.get::<u8>(3), Some(6));
    assert_eq!(db.get::<u8>(4), Some(7));
    assert_eq!(db.get::<u8>(5), Some(8));
    assert_eq!(db.get::<u8>(6), Some(7));
    assert_eq!(db.get::<u8>(7), Some(8));
}
