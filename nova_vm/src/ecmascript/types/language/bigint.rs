// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod operators;

use super::{
    Primitive, String, Value,
    numeric::Numeric,
    value::{BIGINT_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT},
    with_radix,
};
use crate::{
    SmallInteger,
    ecmascript::{
        execution::{Agent, JsResult, agent::ExceptionType},
        types::primitive_handle,
    },
    engine::{
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_bigint::SmallBigInt,
    },
    heap::{
        ArenaAccess, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
        arena_vec_access, indexes::BaseIndex,
    },
};
use core::ops::Neg;
pub(crate) use data::BigIntHeapData;
use num_bigint::{Sign, ToBigInt, TryFromBigIntError};
use operators::{
    bigint_bitwise_op, left_shift_bigint, left_shift_i64, right_shift_bigint, right_shift_i64,
};
use std::ops::{BitAnd, BitOr, BitXor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HeapBigInt<'a>(BaseIndex<'a, BigIntHeapData>);
primitive_handle!(HeapBigInt, BigInt);
arena_vec_access!(HeapBigInt, BigIntHeapData, bigints, BigIntHeapData);

#[derive(Debug, Clone, Copy)]
pub(crate) enum BigIntMathematicalValue {
    Integer(i64),
    Number(f64),
}

impl<'a> HeapBigInt<'a> {
    pub(crate) fn mathematical_value(
        self,
        agent: &Agent,
        _: NoGcScope<'a, '_>,
    ) -> BigIntMathematicalValue {
        let sign = self.get(agent).sign();
        let mut iter = self.get(agent).iter_u64_digits();
        if iter.len() == 1 {
            let data = iter.next().unwrap();
            return if data < i64::MAX as u64 {
                let data = data as i64;
                if sign == Sign::Minus {
                    BigIntMathematicalValue::Integer(data.neg())
                } else {
                    BigIntMathematicalValue::Integer(data)
                }
            } else {
                let data = data as f64;
                if sign == Sign::Minus {
                    BigIntMathematicalValue::Number(data.neg())
                } else {
                    BigIntMathematicalValue::Number(data)
                }
            };
        }
        let mut base = 0.0f64;
        let sign = if sign == Sign::Minus { -1.0 } else { 1.0 };
        for (repeat, part) in iter.enumerate() {
            let multiplier = sign * 2f64.powi(repeat as i32 * 64);
            base += (part as f64) * multiplier;
        }
        BigIntMathematicalValue::Number(base)
    }
}

impl<'a> From<HeapBigInt<'a>> for BigInt<'a> {
    fn from(value: HeapBigInt<'a>) -> Self {
        Self::BigInt(value)
    }
}

impl<'a> TryFrom<BigInt<'a>> for HeapBigInt<'a> {
    type Error = ();

    fn try_from(value: BigInt<'a>) -> Result<Self, Self::Error> {
        match value {
            BigInt::BigInt(b) => Ok(b),
            _ => Err(()),
        }
    }
}

impl From<SmallBigInt> for BigInt<'static> {
    fn from(value: SmallBigInt) -> Self {
        Self::SmallBigInt(value)
    }
}

impl<'a> TryFrom<BigInt<'a>> for SmallBigInt {
    type Error = ();

    fn try_from(value: BigInt<'a>) -> Result<Self, Self::Error> {
        match value {
            BigInt::SmallBigInt(b) => Ok(b),
            _ => Err(()),
        }
    }
}

impl TryFrom<&num_bigint::BigInt> for SmallBigInt {
    type Error = ();

    fn try_from(value: &num_bigint::BigInt) -> Result<Self, Self::Error> {
        let value = i64::try_from(value).map_err(|_| ())?;
        SmallBigInt::try_from(value)
    }
}

/// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
///
/// The BigInt type represents an integer value. The value may be any size and
/// is not limited to a particular bit-width. Generally, where not otherwise
/// noted, operations are designed to return exact mathematically-based
/// answers. For binary operations, BigInts act as two's complement binary
/// strings, with negative numbers treated as having bits set infinitely to the
/// left.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum BigInt<'a> {
    BigInt(HeapBigInt<'a>) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum BigIntRootRepr {
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl<'a> BigInt<'a> {
    pub fn is_zero(self, agent: &Agent) -> bool {
        match self {
            BigInt::BigInt(b) => {
                // Zero BigInts should never be heap allocated.
                debug_assert!(b.get(agent).bits() != 0);
                false
            }
            BigInt::SmallBigInt(b) => b.is_zero(),
        }
    }

    pub const fn zero() -> Self {
        Self::SmallBigInt(SmallBigInt::zero())
    }

    #[inline]
    pub fn from_i64(agent: &mut Agent, value: i64) -> Self {
        if let Ok(result) = SmallBigInt::try_from(value) {
            Self::SmallBigInt(result)
        } else {
            agent.heap.create(BigIntHeapData { data: value.into() })
        }
    }

    #[inline]
    pub fn from_u64(agent: &mut Agent, value: u64) -> Self {
        if let Ok(result) = SmallBigInt::try_from(value) {
            Self::SmallBigInt(result)
        } else {
            agent.heap.create(BigIntHeapData { data: value.into() })
        }
    }

    #[inline]
    pub fn from_i128(agent: &mut Agent, value: i128, gc: NoGcScope<'a, '_>) -> Self {
        if let Ok(result) = SmallBigInt::try_from(value) {
            Self::SmallBigInt(result)
        } else {
            agent
                .heap
                .create(BigIntHeapData { data: value.into() })
                .bind(gc)
        }
    }

    #[inline]
    pub fn from_u128(agent: &mut Agent, value: u128, gc: NoGcScope<'a, '_>) -> Self {
        if let Ok(result) = SmallBigInt::try_from(value) {
            Self::SmallBigInt(result)
        } else {
            agent
                .heap
                .create(BigIntHeapData { data: value.into() })
                .bind(gc)
        }
    }

    pub fn try_into_i64(self, agent: &Agent) -> Result<i64, TryFromBigIntError<()>> {
        match self {
            BigInt::BigInt(b) => i64::try_from(&b.get(agent).data),
            BigInt::SmallBigInt(b) => Ok(b.into_i64()),
        }
    }

    #[inline]
    pub(crate) fn from_num_bigint(agent: &mut Agent, value: num_bigint::BigInt) -> Self {
        if let Ok(result) = SmallBigInt::try_from(&value) {
            Self::SmallBigInt(result)
        } else {
            agent.heap.create(BigIntHeapData { data: value })
        }
    }

    /// ### [6.1.6.2.1 BigInt::unaryMinus ( x )](https://tc39.es/ecma262/#sec-numeric-types-bigint-unaryMinus)
    ///
    /// The abstract operation BigInt::unaryMinus takes argument x (a BigInt)
    /// and returns a BigInt.
    pub(crate) fn unary_minus(agent: &mut Agent, x: Self) -> Self {
        // 1. If x is 0ℤ, return 0ℤ.
        // NOTE: This is handled with the negation below.

        // 2. Return -x.
        match x {
            // It's possible to overflow SmallBigInt limits with negation.
            BigInt::SmallBigInt(x) => Self::from_i64(agent, -x.into_i64()),
            // But it's likewise possible to "de-overflow"!
            BigInt::BigInt(x) => Self::from_num_bigint(agent, -&x.get(agent).data),
        }
    }

    /// ### [6.1.6.2.2 BigInt::bitwiseNOT ( x )](https://tc39.es/ecma262/#sec-numeric-types-bigint-bitwiseNOT)
    ///
    /// The abstract operation BigInt::bitwiseNOT takes argument x (a BigInt)
    /// and returns a BigInt. It returns the one's complement of x.
    pub(crate) fn bitwise_not(agent: &mut Agent, x: Self) -> Self {
        // 1. Return -x - 1ℤ.
        // NOTE: We can use the builtin bitwise not operations instead.
        match x {
            BigInt::SmallBigInt(x) => BigInt::SmallBigInt(!x),
            BigInt::BigInt(x) => agent.heap.create(BigIntHeapData {
                data: !&x.get(agent).data,
            }),
        }
    }

    /// ### [6.1.6.2.3 BigInt::exponentiate ( base, exponent )](https://tc39.es/ecma262/#sec-numeric-types-bigint-exponentiate)
    ///
    /// The abstract operation BigInt::exponentiate takes arguments base (a
    /// BigInt) and exponent (a BigInt) and returns either a normal completion
    /// containing a BigInt or a throw completion.
    pub(crate) fn exponentiate(
        agent: &mut Agent,
        base: Self,
        exponent: Self,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        // 1. If exponent < 0ℤ, throw a RangeError exception.
        if match exponent {
            BigInt::SmallBigInt(x) if x.into_i64() < 0 => true,
            BigInt::BigInt(x) => x.get(agent).data.sign() == Sign::Minus,
            _ => false,
        } {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "exponent must be positive",
                gc,
            ));
        }

        let BigInt::SmallBigInt(exponent) = exponent else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "exponent over bounds",
                gc,
            ));
        };
        let Ok(exponent) = u32::try_from(exponent.into_i64()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "exponent over bounds",
                gc,
            ));
        };

        if exponent == 1 {
            // Uninteresting pow.
            return Ok(base);
        }

        match base {
            BigInt::SmallBigInt(base) => {
                // 2. If base is 0ℤ and exponent is 0ℤ, return 1ℤ.
                // 3. Return base raised to the power exponent.
                let base = base.into_i64();
                if base == 0 && exponent == 0 || base == 1 {
                    return Ok(BigInt::SmallBigInt(1.into()));
                }
                if let Some(result) = base.checked_pow(exponent) {
                    Ok(Self::from_i64(agent, result))
                } else if let Some(result) = (base as i128).checked_pow(exponent) {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    }))
                } else {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: num_bigint::BigInt::from(base).pow(exponent),
                    }))
                }
            }
            BigInt::BigInt(base) => Ok(agent.heap.create(BigIntHeapData {
                data: base.get(agent).pow(exponent),
            })),
        }
        // NOTE: The BigInt implementation does not support native
        // exponentiation.
    }

    /// ### [6.1.6.2.4 BigInt::multiply ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-multiply)
    ///
    /// The abstract operation BigInt::multiply takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a BigInt.
    pub(crate) fn multiply(agent: &mut Agent, x: Self, y: Self) -> Self {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                let (x, y) = (x.into_i64(), y.into_i64());
                // Note: Perform optimistic multiplication; only a subset of
                // i54 values overflow upon multiplication.
                let (result, overflowed) = x.overflowing_mul(y);

                if overflowed {
                    // If we indeed did overflow, then we must redo the
                    // multiplication with double the bit width.
                    let result = x as i128 * y as i128;
                    agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    })
                } else {
                    Self::from_i64(agent, result)
                }
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y))
            | (BigInt::BigInt(y), BigInt::SmallBigInt(x)) => match x.into_i64() {
                // Optimise out the few special cases.
                0 => BigInt::SmallBigInt(x),
                1 => BigInt::BigInt(y),
                x => agent.heap.create(BigIntHeapData {
                    data: x * &y.get(agent).data,
                }),
            },
            (BigInt::BigInt(x), BigInt::BigInt(y)) => agent.heap.create(BigIntHeapData {
                data: &x.get(agent).data * &y.get(agent).data,
            }),
        }
    }

    /// ### [6.1.6.2.5 BigInt::divide ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-divide)
    pub(crate) fn divide(
        agent: &mut Agent,
        x: Self,
        y: Self,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                let y = y.into_i64();
                match y {
                    0 => Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                        gc,
                    )),
                    1 => Ok(BigInt::SmallBigInt(x)),
                    y => Ok(BigInt::SmallBigInt(
                        SmallBigInt::try_from(x.into_i64() / y).unwrap(),
                    )),
                }
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y)) => {
                if x == SmallBigInt::zero() {
                    return Ok(Self::SmallBigInt(SmallBigInt::zero()));
                }
                Ok(Self::from_num_bigint(
                    agent,
                    x.into_i64() / &y.get(agent).data,
                ))
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y)) => {
                let y = y.into_i64();
                match y {
                    0 => Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                        gc,
                    )),
                    1 => Ok(BigInt::BigInt(x)),
                    y => Ok(Self::from_num_bigint(agent, &x.get(agent).data / y)),
                }
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => Ok(Self::from_num_bigint(
                agent,
                &x.get(agent).data / &y.get(agent).data,
            )),
        }
    }

    /// ### [6.1.6.2.6 BigInt::remainder ( n, d )](https://tc39.es/ecma262/#sec-numeric-types-bigint-remainder)
    ///
    /// The abstract operation BigInt::remainder takes arguments n (a BigInt)
    /// and d (a BigInt) and returns either a normal completion containing a
    /// BigInt or a throw completion.
    ///
    /// > NOTE: The sign of the result is the sign of the dividend.
    pub(crate) fn remainder(
        agent: &mut Agent,
        n: Self,
        d: Self,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        match (n, d) {
            (BigInt::SmallBigInt(n), BigInt::SmallBigInt(d)) => {
                if d == SmallBigInt::zero() {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                        gc,
                    ));
                }
                let (n, d) = (n.into_i64(), d.into_i64());
                let result = n % d;

                Ok(BigInt::SmallBigInt(SmallBigInt::try_from(result).unwrap()))
            }
            (BigInt::SmallBigInt(n), BigInt::BigInt(d)) => Ok(Self::from_num_bigint(
                agent,
                n.into_i64() % &d.get(agent).data,
            )),
            (BigInt::BigInt(n), BigInt::SmallBigInt(d)) => {
                if d == SmallBigInt::zero() {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                        gc,
                    ));
                }
                Ok(Self::SmallBigInt(
                    SmallBigInt::try_from(
                        // Remainder can never be bigger than the divisor.
                        i64::try_from(&n.get(agent).data % d.into_i64()).unwrap(),
                    )
                    .unwrap(),
                ))
            }
            (BigInt::BigInt(n), BigInt::BigInt(d)) => Ok(Self::from_num_bigint(
                agent,
                &n.get(agent).data % &d.get(agent).data,
            )),
        }
    }

    /// ### [6.1.6.2.7 BigInt::add ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-add)
    pub(crate) fn add(agent: &mut Agent, x: Self, y: Self) -> Self {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                // Note: The result can still overflow stack bigint limits.
                // SAFETY: SmallBigInt is i54; add cannot overflow an i64.
                Self::from_i64(agent, unsafe { x.into_i64().unchecked_add(y.into_i64()) })
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y))
            | (BigInt::BigInt(y), BigInt::SmallBigInt(x)) => match x.into_i64() {
                0 => y.into(),
                x => {
                    // Note: Adding a heap bigint and a stack bigint can
                    // produce a stack bigint if the two have opposing signs.
                    Self::from_num_bigint(agent, &y.get(agent).data + x)
                }
            },
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                // Note: Adding two a heap bigints can produce a stack
                // bigint if the two have opposing signs.
                Self::from_num_bigint(agent, &x.get(agent).data + &y.get(agent).data)
            }
        }
    }

    /// ### [6.1.6.2.8 BigInt::subtract ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-subtract)
    pub(crate) fn subtract(agent: &mut Agent, x: Self, y: Self) -> Self {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                // Note: The result can still overflow stack bigint limits.
                // SAFETY: SmallBigInt is i54; subtract cannot overflow an i64.
                Self::from_i64(agent, unsafe { x.into_i64().unchecked_sub(y.into_i64()) })
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y)) => {
                // Note: Subtract can produce a stack bigint.
                Self::from_num_bigint(agent, x.into_i64() - y.get(agent).data.clone())
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y)) => match y.into_i64() {
                0 => BigInt::BigInt(x),
                y => {
                    // Note: Subtract can produce a stack bigint.
                    Self::from_num_bigint(agent, &x.get(agent).data - y)
                }
            },
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                // Note: Subtract can produce a stack bigint.
                Self::from_num_bigint(agent, &x.get(agent).data - &y.get(agent).data)
            }
        }
    }

    /// ### [6.1.6.2.9 BigInt::leftShift ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-leftShift)
    ///
    /// The abstract operation BigInt::leftShift takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a BigInt.
    ///
    /// > NOTE: Semantics here should be equivalent to a bitwise shift, treating
    /// > the BigInt as an infinite length string of binary two's complement digits.
    pub(crate) fn left_shift(
        agent: &mut Agent,
        x: Self,
        y: Self,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        if let Some(r) = match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                left_shift_i64(agent, x.into_i64(), y.into_i64())
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let x = x.get(agent).data.clone();
                let y = y.get(agent).data.clone();
                left_shift_bigint(agent, &x, y)
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y)) => {
                let x = x.get(agent).data.clone();
                left_shift_bigint(agent, &x, y.into_i64())
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y)) => {
                let y = y.get(agent).data.clone();
                left_shift_i64(agent, x.into_i64(), y)
            }
        } {
            Ok(r)
        } else {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "BigInt is too large to allocate",
                gc,
            ))
        }
    }

    /// ### [6.1.6.2.10 BigInt::signedRightShift ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-signedRightShift)
    ///
    /// The abstract operation BigInt::signedRightShift takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a BigInt.
    pub(crate) fn signed_right_shift(
        agent: &mut Agent,
        x: Self,
        y: Self,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        if let Some(r) = match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                right_shift_i64(agent, x.into_i64(), y.into_i64())
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let x = x.get(agent).data.clone();
                let y = y.get(agent).data.clone();
                right_shift_bigint(agent, &x, y)
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y)) => {
                let x = x.get(agent).data.clone();
                right_shift_bigint(agent, &x, y.into_i64())
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y)) => {
                let y = y.get(agent).data.clone();
                right_shift_i64(agent, x.into_i64(), y)
            }
        } {
            Ok(r)
        } else {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "BigInt is too large to allocate",
                gc,
            ))
        }
    }

    /// ### [6.1.6.2.11 BigInt::unsignedRightShift ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-unsignedRightShift)
    ///
    /// The abstract operation BigInt::unsignedRightShift takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a throw completion.
    pub(crate) fn unsigned_right_shift(
        agent: &mut Agent,
        _x: Self,
        _y: Self,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "BigInts have no unsigned right shift, use >> instead",
            gc,
        ))
    }

    /// ### [6.1.6.2.12 BigInt::lessThan ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-lessThan)
    ///
    /// The abstract operation BigInt::lessThan takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a Boolean.
    pub(crate) fn less_than<T>(agent: &'a T, x: Self, y: Self) -> bool
    where
        HeapBigInt<'a>: ArenaAccess<T, Output = BigIntHeapData>,
    {
        // 1. If ℝ(x) < ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(_), BigInt::SmallBigInt(_)) => false,
            (BigInt::SmallBigInt(_), BigInt::BigInt(_)) => true,
            (BigInt::BigInt(b1), BigInt::BigInt(b2)) => b1.get(agent).data < b2.get(agent).data,
            (BigInt::SmallBigInt(b1), BigInt::SmallBigInt(b2)) => b1.into_i64() < b2.into_i64(),
        }
    }

    /// ### [6.1.6.2.13 BigInt::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-equal)
    ///
    /// The abstract operation BigInt::equal takes arguments x (a BigInt) and y
    /// (a BigInt) and returns a Boolean.
    pub(crate) fn equal<T>(agent: &'a T, x: Self, y: Self) -> bool
    where
        HeapBigInt<'a>: ArenaAccess<T, Output = BigIntHeapData>,
    {
        // 1. If ℝ(x) = ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                x == y || x.get(agent).data == y.get(agent).data
            }
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => x == y,
            _ => false,
        }
    }

    /// ### [6.1.6.2.18 BigInt::bitwiseAND ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-bitwiseAND)
    ///
    /// The abstract operation BigInt::bitwiseAND takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a BigInt.
    pub(crate) fn bitwise_and(agent: &mut Agent, x: Self, y: Self) -> Self {
        bigint_bitwise_op!(agent, x, y, BitAnd::bitand)
    }

    /// ### [6.1.6.2.19 BigInt::bitwiseXOR ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-bitwiseXOR)
    ///
    /// The abstract operation BigInt::bitwiseXOR takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a BigInt.
    pub(crate) fn bitwise_xor(agent: &mut Agent, x: Self, y: Self) -> Self {
        bigint_bitwise_op!(agent, x, y, BitXor::bitxor)
    }

    /// ### [6.1.6.2.20 BigInt::bitwiseOR ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-bitwiseOR)
    ///
    /// The abstract operation BigInt::bitwiseOR takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a BigInt.
    pub(crate) fn bitwise_or(agent: &mut Agent, x: Self, y: Self) -> Self {
        bigint_bitwise_op!(agent, x, y, BitOr::bitor)
    }

    // ### [6.1.6.2.21 BigInt::toString ( x, radix )](https://tc39.es/ecma262/#sec-numeric-types-bigint-tostring)
    pub(crate) fn to_string_radix_n<'gc>(
        agent: &mut Agent,
        x: Self,
        radix: u32,
        gc: NoGcScope<'gc, '_>,
    ) -> String<'gc> {
        String::from_string(
            agent,
            match x {
                BigInt::SmallBigInt(x) => with_radix!(
                    radix,
                    lexical::to_string_with_options::<_, RADIX>(
                        x.into_i64(),
                        &lexical::write_integer_options::STANDARD,
                    )
                )
                .to_ascii_lowercase(),
                BigInt::BigInt(x) => x.get(agent).to_str_radix(radix),
            },
            gc,
        )
    }

    // ### [6.1.6.2.21 BigInt::toString ( x, radix )](https://tc39.es/ecma262/#sec-numeric-types-bigint-tostring)
    pub(crate) fn to_string_radix_10<'gc>(
        agent: &mut Agent,
        x: Self,
        gc: NoGcScope<'gc, '_>,
    ) -> String<'gc> {
        String::from_string(
            agent,
            match x {
                BigInt::SmallBigInt(x) => x.into_i64().to_string(),
                BigInt::BigInt(x) => x.get(agent).to_string(),
            },
            gc,
        )
    }
}

bindable_handle!(BigInt);

// Note: SmallInteger can be a number or BigInt.
// Hence there are no further impls here.
impl From<SmallInteger> for BigInt<'static> {
    fn from(value: SmallInteger) -> Self {
        BigInt::SmallBigInt(value.into())
    }
}

impl<'a> TryFrom<Value<'a>> for BigInt<'a> {
    type Error = ();
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::BigInt(x) => Ok(Self::BigInt(x)),
            Value::SmallBigInt(x) => Ok(Self::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Primitive<'a>> for BigInt<'a> {
    type Error = ();
    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        match value {
            Primitive::BigInt(x) => Ok(Self::BigInt(x)),
            Primitive::SmallBigInt(x) => Ok(Self::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Numeric<'a>> for BigInt<'a> {
    type Error = ();
    fn try_from(value: Numeric<'a>) -> Result<Self, Self::Error> {
        match value {
            Numeric::BigInt(x) => Ok(Self::BigInt(x)),
            Numeric::SmallBigInt(x) => Ok(Self::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl<'a> From<BigInt<'a>> for Primitive<'a> {
    fn from(value: BigInt<'a>) -> Self {
        match value {
            BigInt::BigInt(x) => Self::BigInt(x.unbind()),
            BigInt::SmallBigInt(x) => Self::SmallBigInt(x),
        }
    }
}

impl<'a> From<BigInt<'a>> for Numeric<'a> {
    fn from(value: BigInt<'a>) -> Self {
        match value {
            BigInt::BigInt(x) => Self::BigInt(x),
            BigInt::SmallBigInt(x) => Self::SmallBigInt(x),
        }
    }
}

impl<'a> From<BigInt<'a>> for Value<'a> {
    fn from(value: BigInt<'a>) -> Self {
        match value {
            BigInt::BigInt(x) => Value::BigInt(x),
            BigInt::SmallBigInt(x) => Value::SmallBigInt(x),
        }
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for BigInt<'static> {
            fn from(value: $size) -> Self {
                BigInt::SmallBigInt(SmallBigInt::from(value))
            }
        }
    };
}

impl_value_from_n!(u8);
impl_value_from_n!(i8);
impl_value_from_n!(u16);
impl_value_from_n!(i16);
impl_value_from_n!(u32);
impl_value_from_n!(i32);

impl<'a> CreateHeapData<BigIntHeapData, BigInt<'a>> for Heap {
    fn create(&mut self, data: BigIntHeapData) -> BigInt<'a> {
        self.bigints.push(data);
        self.alloc_counter += core::mem::size_of::<BigIntHeapData>();
        BigInt::BigInt(HeapBigInt(BaseIndex::last(&self.bigints)))
    }
}

impl HeapMarkAndSweep for HeapBigInt<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.bigints.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.bigints.shift_index(&mut self.0);
    }
}

impl Rootable for BigInt<'_> {
    type RootRepr = BigIntRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::BigInt(heap_big_int) => Err(HeapRootData::BigInt(heap_big_int.unbind())),
            Self::SmallBigInt(small_big_int) => Ok(Self::RootRepr::SmallBigInt(small_big_int)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::SmallBigInt(small_big_int) => Ok(Self::SmallBigInt(small_big_int)),
            Self::RootRepr::HeapRef(heap_root_ref) => Err(heap_root_ref),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BigInt(heap_big_int) => Some(Self::BigInt(heap_big_int)),
            _ => None,
        }
    }
}
