// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
use super::{
    IntoPrimitive, IntoValue, Primitive, String, Value,
    into_numeric::IntoNumeric,
    numeric::Numeric,
    value::{BIGINT_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT},
};
use crate::{
    SmallInteger,
    ecmascript::execution::{Agent, JsResult, agent::ExceptionType},
    engine::{
        context::{Bindable, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, PrimitiveHeap, WorkQueues,
        indexes::BigIntIndex,
    },
};
use core::ops::{Index, IndexMut, Neg};
pub use data::BigIntHeapData;
use num_bigint::Sign;

impl<'a> IntoValue<'a> for BigInt<'a> {
    fn into_value(self) -> Value<'a> {
        match self {
            BigInt::BigInt(data) => Value::BigInt(data.unbind()),
            BigInt::SmallBigInt(data) => Value::SmallBigInt(data),
        }
    }
}

impl<'a> IntoPrimitive<'a> for BigInt<'a> {
    fn into_primitive(self) -> Primitive<'a> {
        self.into()
    }
}

impl<'a> IntoNumeric<'a> for BigInt<'a> {
    fn into_numeric(self) -> Numeric<'a> {
        self.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HeapBigInt<'a>(BigIntIndex<'a>);

#[derive(Debug, Clone, Copy)]
pub enum BigIntMathematicalValue {
    Integer(i64),
    Number(f64),
}

impl<'a> HeapBigInt<'a> {
    pub(crate) const fn _def() -> Self {
        Self(BigIntIndex::from_u32_index(0))
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn mathematical_value(
        self,
        agent: &Agent,
        _: NoGcScope<'a, '_>,
    ) -> BigIntMathematicalValue {
        let sign = agent[self].data.sign();
        let mut iter = agent[self].data.iter_u64_digits();
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for HeapBigInt<'_> {
    type Of<'a> = HeapBigInt<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> IntoValue<'a> for HeapBigInt<'a> {
    fn into_value(self) -> Value<'a> {
        Value::BigInt(self.unbind())
    }
}

impl<'a> IntoPrimitive<'a> for HeapBigInt<'a> {
    fn into_primitive(self) -> Primitive<'a> {
        Primitive::BigInt(self.unbind())
    }
}

impl<'a> TryFrom<Value<'a>> for HeapBigInt<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        if let Value::BigInt(x) = value {
            Ok(x)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Primitive<'a>> for HeapBigInt<'a> {
    type Error = ();

    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        if let Primitive::BigInt(x) = value {
            Ok(x)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct SmallBigInt(SmallInteger);

impl SmallBigInt {
    #[inline(always)]
    pub(crate) const fn zero() -> Self {
        Self(SmallInteger::zero())
    }

    #[inline(always)]
    pub(crate) fn into_i64(self) -> i64 {
        self.0.into_i64()
    }

    pub(crate) const fn into_inner(self) -> SmallInteger {
        self.0
    }
}

impl core::ops::Not for SmallBigInt {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl core::ops::Neg for SmallBigInt {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl<'a> From<HeapBigInt<'a>> for BigInt<'a> {
    fn from(value: HeapBigInt<'a>) -> Self {
        Self::BigInt(value)
    }
}

impl From<SmallBigInt> for BigInt<'static> {
    fn from(value: SmallBigInt) -> Self {
        Self::SmallBigInt(value)
    }
}

impl<'a> From<HeapBigInt<'a>> for Value<'a> {
    fn from(value: HeapBigInt<'a>) -> Self {
        Self::BigInt(value)
    }
}

impl From<SmallBigInt> for Value<'static> {
    fn from(value: SmallBigInt) -> Self {
        Self::SmallBigInt(value)
    }
}

impl From<SmallInteger> for SmallBigInt {
    fn from(value: SmallInteger) -> Self {
        SmallBigInt(value)
    }
}

impl TryFrom<i64> for SmallBigInt {
    type Error = ();

    #[inline(always)]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl TryFrom<u64> for SmallBigInt {
    type Error = ();

    #[inline(always)]
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl TryFrom<&num_bigint::BigInt> for SmallBigInt {
    type Error = ();

    fn try_from(value: &num_bigint::BigInt) -> Result<Self, Self::Error> {
        Ok(Self(i64::try_from(value).map_err(|_| ())?.try_into()?))
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
            BigInt::BigInt(x) => Self::from_num_bigint(agent, -&agent[x].data),
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
                data: !&agent[x].data,
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
    ) -> JsResult<Self> {
        // 1. If exponent < 0ℤ, throw a RangeError exception.
        if match exponent {
            BigInt::SmallBigInt(x) if x.into_i64() < 0 => true,
            BigInt::BigInt(x) => agent[x].data < 0.into(),
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
                    return Ok(BigInt::SmallBigInt(SmallBigInt(1.into())));
                }
                if let Some(result) = base.checked_pow(exponent) {
                    Ok(Self::from_i64(agent, result))
                } else {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: (base as i128).pow(exponent).into(),
                    }))
                }
            }
            BigInt::BigInt(base) => Ok(agent.heap.create(BigIntHeapData {
                data: agent[base].data.pow(exponent),
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
                    data: x * &agent[y].data,
                }),
            },
            (BigInt::BigInt(x), BigInt::BigInt(y)) => agent.heap.create(BigIntHeapData {
                data: &agent[x].data * &agent[y].data,
            }),
        }
    }

    /// ### [BigInt::add ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-add)
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
                    Self::from_num_bigint(agent, &agent[y].data + x)
                }
            },
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                // Note: Adding two a heap bigints can produce a stack
                // bigint if the two have opposing signs.
                Self::from_num_bigint(agent, &agent[x].data + &agent[y].data)
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
                Self::from_num_bigint(agent, x.into_i64() - &agent[y].data)
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y)) => match y.into_i64() {
                0 => BigInt::BigInt(x),
                y => {
                    // Note: Subtract can produce a stack bigint.
                    Self::from_num_bigint(agent, &agent[x].data - y)
                }
            },
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                // Note: Subtract can produce a stack bigint.
                Self::from_num_bigint(agent, &agent[x].data - &agent[y].data)
            }
        }
    }

    /// ### [6.1.6.2.5 BigInt::divide ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-divide)
    pub(crate) fn divide(agent: &mut Agent, x: Self, y: Self, gc: NoGcScope) -> JsResult<Self> {
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
                Ok(Self::from_num_bigint(agent, x.into_i64() / &agent[y].data))
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
                    y => Ok(Self::from_num_bigint(agent, &agent[x].data / y)),
                }
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => Ok(Self::from_num_bigint(
                agent,
                &agent[x].data / &agent[y].data,
            )),
        }
    }
    /// ### [6.1.6.2.12 BigInt::lessThan ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-lessThan)
    ///
    /// The abstract operation BigInt::lessThan takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a Boolean.
    pub(crate) fn less_than(
        agent: &impl Index<HeapBigInt<'a>, Output = BigIntHeapData>,
        x: Self,
        y: Self,
    ) -> bool {
        // 1. If ℝ(x) < ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(_), BigInt::SmallBigInt(_)) => false,
            (BigInt::SmallBigInt(_), BigInt::BigInt(_)) => true,
            (BigInt::BigInt(b1), BigInt::BigInt(b2)) => agent[b1].data < agent[b2].data,
            (BigInt::SmallBigInt(b1), BigInt::SmallBigInt(b2)) => b1.into_i64() < b2.into_i64(),
        }
    }

    /// ### [6.1.6.2.6 BigInt::remainder ( n, d )](https://tc39.es/ecma262/#sec-numeric-types-bigint-remainder)
    pub(crate) fn remainder(agent: &mut Agent, n: Self, d: Self, gc: NoGcScope) -> JsResult<Self> {
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
            (BigInt::SmallBigInt(n), BigInt::BigInt(d)) => {
                Ok(Self::from_num_bigint(agent, n.into_i64() % &agent[d].data))
            }
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
                        i64::try_from(&agent[n].data % d.into_i64()).unwrap(),
                    )
                    .unwrap(),
                ))
            }
            (BigInt::BigInt(n), BigInt::BigInt(d)) => Ok(Self::from_num_bigint(
                agent,
                &agent[n].data % &agent[d].data,
            )),
        }
    }

    /// ### [6.1.6.2.13 BigInt::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-equal)
    ///
    /// The abstract operation BigInt::equal takes arguments x (a BigInt) and y
    /// (a BigInt) and returns a Boolean.
    pub(crate) fn equal(
        agent: &impl Index<HeapBigInt<'a>, Output = BigIntHeapData>,
        x: Self,
        y: Self,
    ) -> bool {
        // 1. If ℝ(x) = ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(x), BigInt::BigInt(y)) => x == y || agent[x].data == agent[y].data,
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => x == y,
            _ => false,
        }
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
                BigInt::BigInt(x) => agent[x].data.to_string(),
            },
            gc,
        )
    }

    pub(crate) fn to_real(self, agent: &mut Agent) -> f64 {
        match self {
            BigInt::BigInt(heap_big_int) => {
                let mut value = 0f64;
                for (i, digits) in agent[heap_big_int].data.iter_u64_digits().enumerate() {
                    if i == 0 {
                        value += digits as f64;
                    } else {
                        value += ((digits as u128) << (i * 64)) as f64;
                    }
                }
                value
            }
            BigInt::SmallBigInt(small_big_int) => small_big_int.into_i64() as f64,
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for BigInt<'_> {
    type Of<'a> = BigInt<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

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
            Value::BigInt(x) => Ok(BigInt::BigInt(x)),
            Value::SmallBigInt(x) => Ok(BigInt::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Primitive<'a>> for BigInt<'a> {
    type Error = ();
    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        match value {
            Primitive::BigInt(x) => Ok(BigInt::BigInt(x)),
            Primitive::SmallBigInt(x) => Ok(BigInt::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Numeric<'a>> for BigInt<'a> {
    type Error = ();
    fn try_from(value: Numeric<'a>) -> Result<Self, Self::Error> {
        match value {
            Numeric::BigInt(x) => Ok(BigInt::BigInt(x)),
            Numeric::SmallBigInt(x) => Ok(BigInt::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl<'a> From<BigInt<'a>> for Value<'a> {
    fn from(value: BigInt<'a>) -> Self {
        match value {
            BigInt::BigInt(x) => Value::BigInt(x.unbind()),
            BigInt::SmallBigInt(x) => Value::SmallBigInt(x),
        }
    }
}

impl<'a> From<BigInt<'a>> for Primitive<'a> {
    fn from(value: BigInt<'a>) -> Primitive<'a> {
        match value {
            BigInt::BigInt(x) => Primitive::BigInt(x.unbind()),
            BigInt::SmallBigInt(x) => Primitive::SmallBigInt(x),
        }
    }
}

impl<'a> From<BigInt<'a>> for Numeric<'a> {
    fn from(value: BigInt<'a>) -> Numeric<'a> {
        match value {
            BigInt::BigInt(x) => Numeric::BigInt(x),
            BigInt::SmallBigInt(x) => Numeric::SmallBigInt(x),
        }
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for BigInt<'static> {
            fn from(value: $size) -> Self {
                BigInt::SmallBigInt(SmallBigInt(SmallInteger::from(value)))
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

impl Index<HeapBigInt<'_>> for PrimitiveHeap<'_> {
    type Output = BigIntHeapData;

    fn index(&self, index: HeapBigInt<'_>) -> &Self::Output {
        &self.bigints[index]
    }
}

impl Index<HeapBigInt<'_>> for Agent {
    type Output = BigIntHeapData;

    fn index(&self, index: HeapBigInt<'_>) -> &Self::Output {
        &self.heap.bigints[index]
    }
}

impl IndexMut<HeapBigInt<'_>> for Agent {
    fn index_mut(&mut self, index: HeapBigInt<'_>) -> &mut Self::Output {
        &mut self.heap.bigints[index]
    }
}

impl Index<HeapBigInt<'_>> for Vec<Option<BigIntHeapData>> {
    type Output = BigIntHeapData;

    fn index(&self, index: HeapBigInt<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("BigInt out of bounds")
            .as_ref()
            .expect("BigInt slot empty")
    }
}

impl IndexMut<HeapBigInt<'_>> for Vec<Option<BigIntHeapData>> {
    fn index_mut(&mut self, index: HeapBigInt<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BigInt out of bounds")
            .as_mut()
            .expect("BigInt slot empty")
    }
}

impl CreateHeapData<BigIntHeapData, BigInt<'static>> for Heap {
    fn create(&mut self, data: BigIntHeapData) -> BigInt<'static> {
        self.bigints.push(Some(data));
        BigInt::BigInt(HeapBigInt(BigIntIndex::last(&self.bigints)))
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
