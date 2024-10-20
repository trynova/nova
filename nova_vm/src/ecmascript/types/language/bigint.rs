// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
use super::{
    into_numeric::IntoNumeric,
    numeric::Numeric,
    value::{BIGINT_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT},
    IntoPrimitive, IntoValue, Primitive, String, Value,
};
use crate::{
    ecmascript::execution::{agent::ExceptionType, Agent, JsResult},
    engine::rootable::{HeapRootData, HeapRootRef, Rootable},
    heap::{
        indexes::BigIntIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        PrimitiveHeap, WorkQueues,
    },
    SmallInteger,
};
pub use data::BigIntHeapData;
use std::ops::{Index, IndexMut};

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

impl IntoValue for HeapBigInt {
    fn into_value(self) -> Value {
        Value::BigInt(self)
    }
}

impl IntoPrimitive for HeapBigInt {
    fn into_primitive(self) -> Primitive {
        Primitive::BigInt(self)
    }
}

impl TryFrom<Value> for HeapBigInt {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::BigInt(x) = value {
            Ok(x)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Primitive> for HeapBigInt {
    type Error = ();

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
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
pub enum BigInt {
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum BigIntRootRepr {
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl BigInt {
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
    pub(crate) fn unary_minus(agent: &mut Agent, x: BigInt) -> BigInt {
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
    pub(crate) fn bitwise_not(agent: &mut Agent, x: BigInt) -> BigInt {
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
        base: BigInt,
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

        let BigInt::SmallBigInt(exponent) = exponent else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "exponent over bounds",
            ));
        };
        let Ok(exponent) = u32::try_from(exponent.into_i64()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "exponent over bounds",
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
    pub(crate) fn multiply(agent: &mut Agent, x: BigInt, y: BigInt) -> BigInt {
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
    pub(crate) fn add(agent: &mut Agent, x: BigInt, y: BigInt) -> BigInt {
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
    pub(crate) fn subtract(agent: &mut Agent, x: BigInt, y: BigInt) -> BigInt {
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
    pub(crate) fn divide(agent: &mut Agent, x: BigInt, y: BigInt) -> JsResult<BigInt> {
        match (x, y) {
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                let y = y.into_i64();
                match y {
                    0 => Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
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
        agent: &impl Index<HeapBigInt, Output = BigIntHeapData>,
        x: BigInt,
        y: BigInt,
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
    pub(crate) fn remainder(agent: &mut Agent, n: BigInt, d: BigInt) -> JsResult<BigInt> {
        match (n, d) {
            (BigInt::SmallBigInt(n), BigInt::SmallBigInt(d)) => {
                if d == SmallBigInt::zero() {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "Division by zero",
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

    /// ### [
    /// ### [6.1.6.2.13 BigInt::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-equal)
    ///
    /// The abstract operation BigInt::equal takes arguments x (a BigInt) and y
    /// (a BigInt) and returns a Boolean.
    pub(crate) fn equal(
        agent: &impl Index<HeapBigInt, Output = BigIntHeapData>,
        x: BigInt,
        y: BigInt,
    ) -> bool {
        // 1. If ℝ(x) = ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(x), BigInt::BigInt(y)) => x == y || agent[x].data == agent[y].data,
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

impl Index<HeapBigInt> for PrimitiveHeap<'_> {
    type Output = BigIntHeapData;

    fn index(&self, index: HeapBigInt) -> &Self::Output {
        &self.bigints[index]
    }
}

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

impl Rootable for BigInt {
    type RootRepr = BigIntRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::BigInt(heap_big_int) => Err(HeapRootData::BigInt(heap_big_int)),
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
