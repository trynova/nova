// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use std::ops::{Index, IndexMut};

use super::{
    into_numeric::IntoNumeric,
    numeric::Numeric,
    value::{BIGINT_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT},
    IntoPrimitive, IntoValue, Primitive, String, Value,
};
use crate::{
    ecmascript::execution::{agent::ExceptionType, Agent, JsResult},
    heap::{
        indexes::BigIntIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
    SmallInteger,
};

pub use data::BigIntHeapData;

impl IntoValue for BigInt {
    fn into_value(self) -> Value {
        match self {
            BigInt::BigInt(data) => Value::BigInt(data),
            BigInt::SmallBigInt(data) => Value::SmallBigInt(data),
        }
    }
}

impl IntoPrimitive for BigInt {
    fn into_primitive(self) -> Primitive {
        self.into()
    }
}

impl IntoNumeric for BigInt {
    fn into_numeric(self) -> Numeric {
        self.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HeapBigInt(BigIntIndex);

impl HeapBigInt {
    pub(crate) const fn _def() -> Self {
        Self(BigIntIndex::from_u32_index(0))
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
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

impl std::ops::Not for SmallBigInt {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl std::ops::Neg for SmallBigInt {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl From<HeapBigInt> for BigInt {
    fn from(value: HeapBigInt) -> Self {
        Self::BigInt(value)
    }
}

impl From<SmallBigInt> for BigInt {
    fn from(value: SmallBigInt) -> Self {
        Self::SmallBigInt(value)
    }
}

impl From<HeapBigInt> for Value {
    fn from(value: HeapBigInt) -> Self {
        Self::BigInt(value)
    }
}

impl From<SmallBigInt> for Value {
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

/// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
///
/// The BigInt type represents an integer value. The value may be any size and
/// is not limited to a particular bit-width. Generally, where not otherwise
/// noted, operations are designed to return exact mathematically-based
/// answers. For binary operations, BigInts act as two's complement binary
/// strings, with negative numbers treated as having bits set infinitely to the
/// left.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum BigInt {
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

impl BigInt {
    /// ### [6.1.6.2.1 BigInt::unaryMinus ( x )](https://tc39.es/ecma262/#sec-numeric-types-bigint-unaryMinus)
    ///
    /// The abstract operation BigInt::unaryMinus takes argument x (a BigInt)
    /// and returns a BigInt.
    pub(crate) fn unary_minus(agent: &mut Agent, x: BigInt) -> BigInt {
        // 1. If x is 0ℤ, return 0ℤ.
        // NOTE: This is handled with the negation below.

        // 2. Return -x.
        match x {
            BigInt::SmallBigInt(x) => {
                // We need to check if the negation will overflow.
                if x.into_i64() != SmallInteger::MAX_BIGINT {
                    BigInt::SmallBigInt(-x)
                } else {
                    agent.heap.create(BigIntHeapData {
                        data: -num_bigint::BigInt::from(x.into_i64()),
                    })
                }
            }
            BigInt::BigInt(x_index) => {
                let x_data = &agent[x_index];
                agent.heap.create(BigIntHeapData {
                    data: -&x_data.data,
                })
            }
        }
    }

    /// ### [6.1.6.2.2 BigInt::bitwiseNOT ( x )](https://tc39.es/ecma262/#sec-numeric-types-bigint-bitwiseNOT)
    ///
    /// The abstract operation BigInt::bitwiseNOT takes argument x (a BigInt)
    /// and returns a BigInt. It returns the one's complement of x.
    pub(crate) fn bitwise_not(agent: &mut Agent, x: BigInt) -> BigInt {
        // 1. Return -x - 1ℤ.
        // NOTE: We can use the builtin bitwise not operations instead.
        match x {
            BigInt::SmallBigInt(x) => BigInt::SmallBigInt(!x),
            BigInt::BigInt(x_index) => {
                let x_data = &agent[x_index];
                agent.heap.create(BigIntHeapData {
                    data: !&x_data.data,
                })
            }
        }
    }

    /// ### [6.1.6.2.3 BigInt::exponentiate ( base, exponent )](https://tc39.es/ecma262/#sec-numeric-types-bigint-exponentiate)
    ///
    /// The abstract operation BigInt::exponentiate takes arguments base (a
    /// BigInt) and exponent (a BigInt) and returns either a normal completion
    /// containing a BigInt or a throw completion.
    pub(crate) fn exponentiate(
        agent: &mut Agent,
        _base: BigInt,
        exponent: BigInt,
    ) -> JsResult<BigInt> {
        // 1. If exponent < 0ℤ, throw a RangeError exception.
        if match exponent {
            BigInt::SmallBigInt(x) if x.into_i64() < 0 => true,
            BigInt::BigInt(x) => agent[x].data < 0.into(),
            _ => false,
        } {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "exponent must be positive",
            ));
        }

        // TODO: 2. If base is 0ℤ and exponent is 0ℤ, return 1ℤ.
        // TODO: 3. Return base raised to the power exponent.
        // NOTE: The BigInt implementation does not support native
        // exponentiation.

        Err(agent.throw_exception_with_static_message(
            ExceptionType::EvalError,
            "Unsupported operation.",
        ))
    }

    /// ### [6.1.6.2.4 BigInt::multiply ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-multiply)
    ///
    /// The abstract operation BigInt::multiply takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a BigInt.
    pub(crate) fn multiply(agent: &mut Agent, x: BigInt, y: BigInt) -> JsResult<BigInt> {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                let (x, y) = (x.into_i64() as i128, y.into_i64() as i128);
                let result = x.checked_mul(y).unwrap();

                if let Ok(result) = SmallInteger::try_from(result) {
                    Ok(BigInt::SmallBigInt(SmallBigInt(result)))
                } else {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    }))
                }
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y))
            | (BigInt::BigInt(y), BigInt::SmallBigInt(x)) => {
                let x = x.into_i64();
                let y = &agent[y];
                Ok(agent.heap.create(BigIntHeapData { data: x * &y.data }))
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let (x, y) = (&agent[x], &agent[y]);
                Ok(agent.heap.create(BigIntHeapData {
                    data: &x.data * &y.data,
                }))
            }
        }
    }

    /// ### [BigInt::add ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-add)
    pub(crate) fn add(agent: &mut Agent, x: BigInt, y: BigInt) -> JsResult<BigInt> {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                let (x, y) = (x.into_i64() as i128, y.into_i64() as i128);
                let result = x + y;

                if let Ok(result) = SmallInteger::try_from(result) {
                    Ok(BigInt::SmallBigInt(SmallBigInt(result)))
                } else {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    }))
                }
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y))
            | (BigInt::BigInt(y), BigInt::SmallBigInt(x)) => {
                let x = x.into_i64();
                let y = &agent[y];
                Ok(agent.heap.create(BigIntHeapData { data: x + &y.data }))
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let (x, y) = (&agent[x], &agent[y]);
                Ok(agent.heap.create(BigIntHeapData {
                    data: &x.data + &y.data,
                }))
            }
        }
    }

    /// ### [6.1.6.2.8 BigInt::subtract ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-subtract)
    pub(crate) fn subtract(agent: &mut Agent, x: BigInt, y: BigInt) -> JsResult<BigInt> {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                let (x, y) = (x.into_i64() as i128, y.into_i64() as i128);
                let result = x - y;

                if let Ok(result) = SmallInteger::try_from(result) {
                    Ok(BigInt::SmallBigInt(SmallBigInt(result)))
                } else {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    }))
                }
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y)) => {
                let x = x.into_i64();
                let y = &agent[y];
                Ok(agent.heap.create(BigIntHeapData { data: x - &y.data }))
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y)) => {
                let x = &agent[x];
                let y = y.into_i64();
                Ok(agent.heap.create(BigIntHeapData { data: &x.data - y }))
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let (x, y) = (&agent[x], &agent[y]);
                Ok(agent.heap.create(BigIntHeapData {
                    data: &x.data - &y.data,
                }))
            }
        }
    }

    /// ### [6.1.6.2.5 BigInt::divide ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-divide)
    pub(crate) fn divide(agent: &mut Agent, x: BigInt, y: BigInt) -> JsResult<BigInt> {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                if y == SmallBigInt::zero() {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                    ));
                }
                let (x, y) = (x.into_i64() as i128, y.into_i64() as i128);
                let result = x / y;

                if let Ok(result) = SmallInteger::try_from(result) {
                    Ok(BigInt::SmallBigInt(SmallBigInt(result)))
                } else {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    }))
                }
            }
            (BigInt::SmallBigInt(x), BigInt::BigInt(y)) => {
                let x = x.into_i64();
                let y = &agent[y];
                Ok(agent.heap.create(BigIntHeapData { data: x / &y.data }))
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y)) => {
                if y == SmallBigInt::zero() {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                    ));
                }
                let x = &agent[x];
                let y = y.into_i64();
                Ok(agent.heap.create(BigIntHeapData { data: &x.data / y }))
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let (x, y) = (&agent[x], &agent[y]);
                Ok(agent.heap.create(BigIntHeapData {
                    data: &x.data / &y.data,
                }))
            }
        }
    }
    /// ### [6.1.6.2.12 BigInt::lessThan ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-lessThan)
    ///
    /// The abstract operation BigInt::lessThan takes arguments x (a BigInt)
    /// and y (a BigInt) and returns a Boolean.
    pub(crate) fn less_than(agent: &mut Agent, x: BigInt, y: BigInt) -> bool {
        // 1. If ℝ(x) < ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(_), BigInt::SmallBigInt(_)) => false,
            (BigInt::SmallBigInt(_), BigInt::BigInt(_)) => true,
            (BigInt::BigInt(b1), BigInt::BigInt(b2)) => {
                let (b1, b2) = (&agent[b1], &agent[b2]);
                b1.data < b2.data
            }
            (BigInt::SmallBigInt(b1), BigInt::SmallBigInt(b2)) => b1.into_i64() < b2.into_i64(),
        }
    }

    /// ### [6.1.6.2.6 BigInt::remainder ( n, d )](https://tc39.es/ecma262/#sec-numeric-types-bigint-remainder)
    pub(crate) fn remainder(agent: &mut Agent, n: BigInt, d: BigInt) -> JsResult<BigInt> {
        match (n, d) {
            (BigInt::SmallBigInt(n), BigInt::SmallBigInt(d)) => {
                if d == SmallBigInt::zero() {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                    ));
                }
                let (n, d) = (n.into_i64() as i128, d.into_i64() as i128);
                let result = n % d;

                if let Ok(result) = SmallInteger::try_from(result) {
                    Ok(BigInt::SmallBigInt(SmallBigInt(result)))
                } else {
                    Ok(agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    }))
                }
            }
            (BigInt::SmallBigInt(n), BigInt::BigInt(d)) => {
                let n = n.into_i64();
                let d = &agent[d];
                Ok(agent.heap.create(BigIntHeapData { data: n % &d.data }))
            }
            (BigInt::BigInt(n), BigInt::SmallBigInt(d)) => {
                if d == SmallBigInt::zero() {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
                    ));
                }
                let n = &agent[n];
                let d = d.into_i64();
                Ok(agent.heap.create(BigIntHeapData { data: &n.data % d }))
            }
            (BigInt::BigInt(n), BigInt::BigInt(d)) => {
                let (n, d) = (&agent[n], &agent[d]);
                Ok(agent.heap.create(BigIntHeapData {
                    data: &n.data % &d.data,
                }))
            }
        }
    }

    /// ### [
    /// ### [6.1.6.2.13 BigInt::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-equal)
    ///
    /// The abstract operation BigInt::equal takes arguments x (a BigInt) and y
    /// (a BigInt) and returns a Boolean.
    pub(crate) fn equal(agent: &Agent, x: BigInt, y: BigInt) -> bool {
        // 1. If ℝ(x) = ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let (x, y) = (&agent[x], &agent[y]);
                x.data == y.data
            }
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => x == y,
            _ => false,
        }
    }

    // ### [6.1.6.2.21 BigInt::toString ( x, radix )](https://tc39.es/ecma262/#sec-numeric-types-bigint-tostring)
    pub(crate) fn to_string_radix_10(agent: &mut Agent, x: Self) -> JsResult<String> {
        Ok(String::from_string(
            agent,
            match x {
                BigInt::SmallBigInt(x) => x.into_i64().to_string(),
                BigInt::BigInt(x) => agent[x].data.to_string(),
            },
        ))
    }
}

// Note: SmallInteger can be a number or BigInt.
// Hence there are no further impls here.
impl From<SmallInteger> for BigInt {
    fn from(value: SmallInteger) -> Self {
        BigInt::SmallBigInt(value.into())
    }
}

impl TryFrom<Value> for BigInt {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::BigInt(x) => Ok(BigInt::BigInt(x)),
            Value::SmallBigInt(x) => Ok(BigInt::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Primitive> for BigInt {
    type Error = ();
    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        match value {
            Primitive::BigInt(x) => Ok(BigInt::BigInt(x)),
            Primitive::SmallBigInt(x) => Ok(BigInt::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Numeric> for BigInt {
    type Error = ();
    fn try_from(value: Numeric) -> Result<Self, Self::Error> {
        match value {
            Numeric::BigInt(x) => Ok(BigInt::BigInt(x)),
            Numeric::SmallBigInt(x) => Ok(BigInt::SmallBigInt(x)),
            _ => Err(()),
        }
    }
}

impl From<BigInt> for Value {
    fn from(value: BigInt) -> Value {
        match value {
            BigInt::BigInt(x) => Value::BigInt(x),
            BigInt::SmallBigInt(x) => Value::SmallBigInt(x),
        }
    }
}

impl From<BigInt> for Primitive {
    fn from(value: BigInt) -> Primitive {
        match value {
            BigInt::BigInt(x) => Primitive::BigInt(x),
            BigInt::SmallBigInt(x) => Primitive::SmallBigInt(x),
        }
    }
}

impl From<BigInt> for Numeric {
    fn from(value: BigInt) -> Numeric {
        match value {
            BigInt::BigInt(x) => Numeric::BigInt(x),
            BigInt::SmallBigInt(x) => Numeric::SmallBigInt(x),
        }
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for BigInt {
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

impl Index<HeapBigInt> for Agent {
    type Output = BigIntHeapData;

    fn index(&self, index: HeapBigInt) -> &Self::Output {
        &self.heap.bigints[index]
    }
}

impl IndexMut<HeapBigInt> for Agent {
    fn index_mut(&mut self, index: HeapBigInt) -> &mut Self::Output {
        &mut self.heap.bigints[index]
    }
}

impl Index<HeapBigInt> for Vec<Option<BigIntHeapData>> {
    type Output = BigIntHeapData;

    fn index(&self, index: HeapBigInt) -> &Self::Output {
        self.get(index.get_index())
            .expect("BigInt out of bounds")
            .as_ref()
            .expect("BigInt slot empty")
    }
}

impl IndexMut<HeapBigInt> for Vec<Option<BigIntHeapData>> {
    fn index_mut(&mut self, index: HeapBigInt) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BigInt out of bounds")
            .as_mut()
            .expect("BigInt slot empty")
    }
}

impl CreateHeapData<BigIntHeapData, BigInt> for Heap {
    fn create(&mut self, data: BigIntHeapData) -> BigInt {
        self.bigints.push(Some(data));
        BigInt::BigInt(HeapBigInt(BigIntIndex::last(&self.bigints)))
    }
}

impl HeapMarkAndSweep for HeapBigInt {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.bigints.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.bigints.shift_index(&mut self.0);
    }
}
