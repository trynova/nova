// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use std::ops::{Index, IndexMut};

use super::{
    value::{FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, NUMBER_DISCRIMINANT},
    IntoNumeric, IntoPrimitive, IntoValue, Numeric, Primitive, String, Value,
};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_int32, to_uint32},
        execution::{Agent, JsResult},
    },
    engine::{
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_f64::SmallF64,
    },
    heap::{
        indexes::NumberIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        PrimitiveHeap, WorkQueues,
    },
    SmallInteger,
};

pub use data::NumberHeapData;
use num_traits::{PrimInt, Zero};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HeapNumber(pub(crate) NumberIndex);

impl HeapNumber {
    pub(crate) const fn _def() -> Self {
        HeapNumber(NumberIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

/// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Number {
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    /// 53-bit signed integer on the stack.
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    /// 56-bit f64 on the stack. The missing byte is a zero least significant
    /// byte.
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum NumberRootRepr {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl IntoValue for HeapNumber {
    fn into_value(self) -> Value {
        Value::Number(self)
    }
}

impl IntoPrimitive for HeapNumber {
    fn into_primitive(self) -> Primitive {
        Primitive::Number(self)
    }
}

impl IntoValue for Number {
    fn into_value(self) -> Value {
        match self {
            Number::Number(idx) => Value::Number(idx),
            Number::Integer(data) => Value::Integer(data),
            Number::SmallF64(data) => Value::SmallF64(data),
        }
    }
}

impl IntoNumeric for HeapNumber {
    fn into_numeric(self) -> Numeric {
        Numeric::Number(self)
    }
}

impl TryFrom<Value> for HeapNumber {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Number(x) = value {
            Ok(x)
        } else {
            Err(())
        }
    }
}

impl IntoPrimitive for Number {
    fn into_primitive(self) -> Primitive {
        match self {
            Number::Number(idx) => Primitive::Number(idx),
            Number::Integer(data) => Primitive::Integer(data),
            Number::SmallF64(data) => Primitive::SmallF64(data),
        }
    }
}

impl IntoNumeric for Number {
    fn into_numeric(self) -> Numeric {
        match self {
            Number::Number(idx) => Numeric::Number(idx),
            Number::Integer(data) => Numeric::Integer(data),
            Number::SmallF64(data) => Numeric::SmallF64(data),
        }
    }
}

impl std::fmt::Debug for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Number::Number(idx) => write!(f, "Number({:?})", idx),
            Number::Integer(value) => write!(f, "{}", value.into_i64()),
            Number::SmallF64(value) => write!(f, "{}", value.into_f64()),
        }
    }
}

impl From<HeapNumber> for Number {
    fn from(value: HeapNumber) -> Self {
        Number::Number(value)
    }
}

impl From<SmallInteger> for Number {
    fn from(value: SmallInteger) -> Self {
        Number::Integer(value)
    }
}

impl From<SmallF64> for Number {
    fn from(value: SmallF64) -> Self {
        Number::SmallF64(value)
    }
}

impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Number::SmallF64(SmallF64::from(value))
    }
}

const MAX_NUMBER: f64 = ((1u64 << 53) - 1) as f64;
const MIN_NUMBER: f64 = -MAX_NUMBER;

impl TryFrom<i64> for Number {
    type Error = ();

    fn try_from(value: i64) -> Result<Self, ()> {
        Ok(Number::Integer(SmallInteger::try_from(value)?))
    }
}

impl TryFrom<usize> for Number {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, ()> {
        if let Ok(i64) = i64::try_from(value) {
            Self::try_from(i64)
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for Number {
    type Error = ();

    fn try_from(value: f64) -> Result<Self, ()> {
        if value.is_finite() && value.trunc() == value && (MIN_NUMBER..=MAX_NUMBER).contains(&value)
        {
            debug_assert_eq!(value as i64 as f64, value);
            Ok(Number::try_from(value as i64).unwrap())
        } else if let Ok(value) = SmallF64::try_from(value) {
            Ok(Number::SmallF64(value))
        } else {
            Err(())
        }
    }
}

impl TryFrom<Value> for Number {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(data) => Ok(Number::Number(data)),
            Value::Integer(data) => Ok(Number::Integer(data)),
            Value::SmallF64(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Primitive> for Number {
    type Error = ();
    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        match value {
            Primitive::Number(data) => Ok(Number::Number(data)),
            Primitive::Integer(data) => Ok(Number::Integer(data)),
            Primitive::SmallF64(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Numeric> for Number {
    type Error = ();
    fn try_from(value: Numeric) -> Result<Self, Self::Error> {
        match value {
            Numeric::Number(data) => Ok(Number::Number(data)),
            Numeric::Integer(data) => Ok(Number::Integer(data)),
            Numeric::SmallF64(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

impl Number {
    pub fn from_f64(agent: &mut Agent, value: f64) -> Self {
        if let Ok(value) = Number::try_from(value) {
            value
        } else {
            // SAFETY: Number was not representable as a
            // stack-allocated Number.
            let id = unsafe { agent.heap.alloc_number(value) };
            Number::Number(id)
        }
    }

    pub fn nan() -> Self {
        Self::from(f32::NAN)
    }

    pub fn neg_zero() -> Self {
        Self::from(-0.0)
    }

    pub fn pos_zero() -> Self {
        Self::from(0)
    }

    pub fn pos_inf() -> Self {
        Self::from(f32::INFINITY)
    }

    pub fn neg_inf() -> Self {
        Self::from(f32::NEG_INFINITY)
    }

    pub fn is_nan(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => agent[n].is_nan(),
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64().is_nan(),
        }
    }

    pub fn is_pos_zero(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => f64::to_bits(0.0) == f64::to_bits(agent[n]),
            Number::Integer(n) => 0i64 == n.into_i64(),
            Number::SmallF64(n) => n.into_f64().to_bits() == 0.0f64.to_bits(),
        }
    }

    pub fn is_neg_zero(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => f64::to_bits(-0.0) == f64::to_bits(agent[n]),
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64().to_bits() == (-0.0f64).to_bits(),
        }
    }

    pub fn is_pos_infinity(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => agent[n] == f64::INFINITY,
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64() == f64::INFINITY,
        }
    }

    pub fn is_neg_infinity(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => agent[n] == f64::NEG_INFINITY,
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64() == f64::NEG_INFINITY,
        }
    }

    pub fn is_finite(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => agent[n].is_finite(),
            Number::Integer(_) => true,
            Number::SmallF64(n) => n.into_f64().is_finite(),
        }
    }

    pub fn is_nonzero(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => 0.0 != agent[n],
            Number::Integer(n) => 0i64 != n.into_i64(),
            Number::SmallF64(n) => !n.into_f64().is_zero(),
        }
    }

    pub fn is_pos_one(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        // NOTE: Only the integer variant should ever return true, if any other
        // variant returns true, that's a bug as it means that our variants are
        // no longer "sound".
        match self {
            Number::Integer(n) => 1i64 == n.into_i64(),
            Number::Number(n) => {
                debug_assert_ne!(agent[n], 1.0);
                false
            }
            Number::SmallF64(n) => {
                debug_assert_ne!(n.into_f64(), 1.0);
                false
            }
        }
    }

    pub fn is_neg_one(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Integer(n) => -1i64 == n.into_i64(),
            Number::Number(n) => {
                debug_assert_ne!(agent[n], -1.0);
                false
            }
            Number::SmallF64(n) => {
                debug_assert_ne!(n.into_f64(), -1.0);
                false
            }
        }
    }

    pub fn is_sign_positive(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => agent[n].is_sign_positive(),
            Number::Integer(n) => n.into_i64().is_positive(),
            Number::SmallF64(n) => n.into_f64().is_sign_positive(),
        }
    }

    pub fn is_sign_negative(self, agent: &impl Index<HeapNumber, Output = f64>) -> bool {
        match self {
            Number::Number(n) => agent[n].is_sign_negative(),
            Number::Integer(n) => n.into_i64().is_negative(),
            Number::SmallF64(n) => n.into_f64().is_sign_negative(),
        }
    }

    /// https://tc39.es/ecma262/#eqn-truncate
    pub fn truncate(self, agent: &mut Agent) -> Number {
        match self {
            Number::Number(n) => {
                let n = agent[n].trunc();
                agent.heap.create(n)
            }
            Number::Integer(_) => self,
            Number::SmallF64(n) => Number::from_f64(agent, n.into_f64().trunc()),
        }
    }

    pub fn into_f64(self, agent: &impl Index<HeapNumber, Output = f64>) -> f64 {
        match self {
            Number::Number(n) => agent[n],
            Number::Integer(n) => Into::<i64>::into(n) as f64,
            Number::SmallF64(n) => n.into_f64(),
        }
    }

    pub fn into_f32(self, agent: &impl Index<HeapNumber, Output = f64>) -> f32 {
        match self {
            Number::Number(n) => agent[n] as f32,
            Number::Integer(n) => Into::<i64>::into(n) as f32,
            Number::SmallF64(n) => n.into_f64() as f32,
        }
    }

    /// Returns the number cast to an [`i64`].
    ///
    /// If the number isn't representable as an i64:
    /// - NaN becomes 0.
    /// - Numbers are clamped between [`i64::MIN`] and [`i64::MAX`].
    /// - All other numbers round towards zero.
    pub fn into_i64(self, agent: &impl Index<HeapNumber, Output = f64>) -> i64 {
        match self {
            Number::Number(n) => agent[n] as i64,
            Number::Integer(n) => Into::<i64>::into(n),
            Number::SmallF64(n) => n.into_f64() as i64,
        }
    }

    /// Returns the number cast to a [`usize`].
    ///
    /// If the number isn't representable as a usize:
    /// - NaN becomes 0.
    /// - Numbers are clamped between 0 and [`usize::MAX`].
    /// - All other numbers round towards zero.
    pub fn into_usize(self, agent: &impl Index<HeapNumber, Output = f64>) -> usize {
        match self {
            Number::Number(n) => agent[n] as usize,
            Number::Integer(n) => {
                let i64 = Into::<i64>::into(n);
                if i64 < 0 {
                    0
                } else {
                    usize::try_from(i64).unwrap_or(usize::MAX)
                }
            }
            Number::SmallF64(n) => n.into_f64() as usize,
        }
    }

    /// Compare two Numbers with each other: This is used when the spec asks if
    /// `x is y` when talking of Numbers. Generally this is asked after various
    /// NaN and non-zero checks, depending on which spec algorithm is being
    /// used.
    #[inline(always)]
    fn is(agent: &impl Index<HeapNumber, Output = f64>, x: Self, y: Self) -> bool {
        match (x, y) {
            // Optimisation: First compare by-reference; only read from heap if needed.
            (Number::Number(x), Number::Number(y)) => x == y || agent[x] == agent[y],
            (Number::Integer(x), Number::Integer(y)) => x == y,
            (Number::SmallF64(x), Number::SmallF64(y)) => x == y,
            (Number::Number(x), Number::Integer(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!(agent[x] != y.into_i64() as f64);
                false
            }
            (Number::Number(x), Number::SmallF64(y)) => {
                // Optimisation: f32s should never be allocated into the heap
                debug_assert!(agent[x] != y.into_f64());
                false
            }
            (Number::Integer(x), Number::Number(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!((x.into_i64() as f64) != agent[y]);
                false
            }
            (Number::Integer(x), Number::SmallF64(y)) => {
                debug_assert!(
                    y.into_f64().to_bits() == (-0.0f64).to_bits()
                        || (x.into_i64() as f64) != y.into_f64()
                );
                false
            }
            (Number::SmallF64(x), Number::Number(y)) => {
                // Optimisation: f32s should never be allocated into the heap
                debug_assert!(x.into_f64() != agent[y]);
                false
            }
            (Number::SmallF64(x), Number::Integer(y)) => {
                debug_assert!(
                    x.into_f64().to_bits() == (-0.0f64).to_bits()
                        || x.into_f64() != y.into_i64() as f64
                );
                false
            }
        }
    }

    pub fn is_odd_integer(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => agent[n] % 2.0 == 1.0,
            Number::Integer(n) => Into::<i64>::into(n) % 2 == 1,
            Number::SmallF64(n) => n.into_f64() % 2.0 == 1.0,
        }
    }

    pub fn abs(self, agent: &mut Agent) -> Self {
        match self {
            Number::Number(n) => {
                let n = agent[n];
                if n > 0.0 {
                    self
                } else {
                    agent.heap.create(-n)
                }
            }
            Number::Integer(n) => {
                let n = n.into_i64();
                Number::Integer(SmallInteger::try_from(n.abs()).unwrap())
            }
            Number::SmallF64(n) => {
                let n = n.into_f64();
                if n == 0.0 {
                    // Negative zero needs to be turned into a Number::Integer
                    debug_assert!(n.is_sign_negative());
                    Number::Integer(SmallInteger::zero())
                } else {
                    Number::SmallF64(SmallF64::try_from(n.abs()).unwrap())
                }
            }
        }
    }

    pub fn greater_than(agent: &mut Agent, x: Self, y: Self) -> Option<bool> {
        Number::less_than(agent, y, x).map(|x| !x)
    }

    /// ### [6.1.6.1.1 Number::unaryMinus ( x )](https://tc39.es/ecma262/#sec-numeric-types-number-unaryMinus)
    pub fn unary_minus(agent: &mut Agent, x: Self) -> Self {
        // 1. If x is NaN, return NaN.
        // NOTE: Computers do this automatically.

        // 2. Return the result of negating x; that is, compute a Number with the same magnitude but opposite sign.
        match x {
            Number::Number(n) => {
                let value = agent[n];
                agent.heap.create(-value)
            }
            Number::Integer(n) => {
                let n = n.into_i64();
                if n == 0 {
                    Number::SmallF64(SmallF64::try_from(-0.0).unwrap())
                } else {
                    SmallInteger::try_from(-n).unwrap().into()
                }
            }
            Number::SmallF64(n) => {
                let n = n.into_f64();
                if n == 0.0 {
                    debug_assert!(n.is_sign_negative());
                    SmallInteger::zero().into()
                } else {
                    (-n).try_into().unwrap()
                }
            }
        }
    }

    /// ### [6.1.6.1.2 Number::bitwiseNOT ( x )](https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseNOT)
    pub fn bitwise_not(agent: &mut Agent, x: Self) -> JsResult<Self> {
        // 1. Let oldValue be ! ToInt32(x).
        let old_value = to_int32(agent, x.into_value())?;

        // 2. Return the result of applying bitwise complement to oldValue. The mathematical value of the result is exactly representable as a 32-bit two's complement bit string.
        Ok(Number::from(!old_value))
    }

    /// ### [6.1.6.1.3 Number::exponentiate ( base, exponent )](https://tc39.es/ecma262/#sec-numeric-types-number-exponentiate)
    pub fn exponentiate(agent: &mut Agent, base: Self, exponent: Self) -> Self {
        // 1. If exponent is NaN, return NaN.
        if exponent.is_nan(agent) {
            return Number::nan();
        }

        // 2. If exponent is either +0𝔽 or -0𝔽, return 1𝔽.
        if exponent.is_pos_zero(agent) || exponent.is_neg_zero(agent) {
            return Number::from(1);
        }

        // 3. If base is NaN, return NaN.
        if base.is_nan(agent) {
            return Number::nan();
        }

        // 4. If base is +∞𝔽, then
        if base.is_pos_infinity(agent) {
            // a. If exponent > +0𝔽, return +∞𝔽. Otherwise, return +0𝔽.
            return if Number::greater_than(agent, exponent, Number::from(0)).unwrap_or(false) {
                Number::pos_inf()
            } else {
                Number::pos_zero()
            };
        }

        // 5. If base is -∞𝔽, then
        if base.is_neg_infinity(agent) {
            // a. If exponent > +0𝔽, then
            return if Number::greater_than(agent, exponent, 0.into()).unwrap_or(false) {
                // i. If exponent is an odd integral Number, return -∞𝔽. Otherwise, return +∞𝔽.
                if exponent.is_odd_integer(agent) {
                    Number::neg_inf()
                } else {
                    Number::pos_inf()
                }
            }
            // b. Else,
            else {
                // i. If exponent is an odd integral Number, return -0𝔽. Otherwise, return +0𝔽.
                if exponent.is_odd_integer(agent) {
                    Number::neg_zero()
                } else {
                    Number::pos_zero()
                }
            };
        }

        // 6. If base is +0𝔽, then
        if base.is_pos_zero(agent) {
            // a. If exponent > +0𝔽, return +0𝔽. Otherwise, return +∞𝔽.
            return if Number::greater_than(agent, exponent, Number::pos_zero()).unwrap_or(false) {
                Number::pos_zero()
            } else {
                Number::pos_inf()
            };
        }

        // 7. If base is -0𝔽, then
        if base.is_neg_zero(agent) {
            // a. If exponent > +0𝔽, then
            return if Number::greater_than(agent, exponent, Number::pos_zero()).unwrap_or(false) {
                // i. If exponent is an odd integral Number, return -0𝔽. Otherwise, return +0𝔽.
                if exponent.is_odd_integer(agent) {
                    Number::neg_zero()
                } else {
                    Number::pos_zero()
                }
            }
            // b. Else,
            else {
                // i. If exponent is an odd integral Number, return -∞𝔽. Otherwise, return +∞𝔽.
                if exponent.is_odd_integer(agent) {
                    Number::neg_inf()
                } else {
                    Number::pos_inf()
                }
            };
        }

        // 8. Assert: base is finite and is neither +0𝔽 nor -0𝔽.
        debug_assert!(base.is_finite(agent) && base.is_nonzero(agent));

        // 9. If exponent is +∞𝔽, then
        if exponent.is_pos_infinity(agent) {
            let base = base.abs(agent);

            // a. If abs(ℝ(base)) > 1, return +∞𝔽.
            return if Number::greater_than(agent, base, Number::from(1)).unwrap_or(false) {
                Number::pos_inf()
            }
            // b. If abs(ℝ(base)) = 1, return NaN.
            else if Number::is(agent, base, Number::from(1)) {
                Number::nan()
            }
            // c. If abs(ℝ(base)) < 1, return +0𝔽.
            else {
                Number::pos_zero()
            };
        }

        // 10. If exponent is -∞𝔽, then
        if exponent.is_neg_infinity(agent) {
            let base = base.into_f64(agent).abs();

            // a. If abs(ℝ(base)) > 1, return +0𝔽.
            return if base > 1.0 {
                Number::pos_inf()
            }
            // b. If abs(ℝ(base)) = 1, return NaN.
            else if base == 1.0 {
                Number::nan()
            }
            // c. If abs(ℝ(base)) < 1, return +∞𝔽.
            else {
                Number::pos_inf()
            };
        }

        // 11. Assert: exponent is finite and is neither +0𝔽 nor -0𝔽.
        debug_assert!(exponent.is_finite(agent) && exponent.is_nonzero(agent));

        // 12. If base < -0𝔽 and exponent is not an integral Number, return NaN.
        if Number::less_than(agent, base, Number::neg_zero()).unwrap_or(false)
            && !exponent.is_odd_integer(agent)
        {
            return Number::nan();
        }

        // 13. Return an implementation-approximated Number value representing the result of raising ℝ(base) to the ℝ(exponent) power.
        agent
            .heap
            .create(base.into_f64(agent).powf(exponent.into_f64(agent)))
    }

    /// ### [6.1.6.1.4 Number::multiply ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-multiply)
    ///
    /// The abstract operation Number::multiply takes arguments x (a Number)
    /// and y (a Number) and returns a Number. It performs multiplication
    /// according to the rules of IEEE 754-2019 binary double-precision
    /// arithmetic, producing the product of x and y.
    ///
    /// > NOTE: Finite-precision multiplication is commutative, but not always
    /// > associative.
    pub fn multiply(agent: &mut Agent, x: Self, y: Self) -> Self {
        // Nonstandard fast path: If both numbers are integers, use integer
        // multiplication and try to return a safe integer as integer.
        if let (Self::Integer(x), Self::Integer(y)) = (x, y) {
            let x = x.into_i64();
            let y = y.into_i64();
            let result = x.checked_mul(y);
            if let Some(result) = result {
                if let Ok(result) = SmallInteger::try_from(result) {
                    return result.into();
                }
                return Self::from_f64(agent, result as f64);
            }
            return Self::from_f64(agent, x as f64 * y as f64);
        }
        // 1. If x is NaN or y is NaN, return NaN.
        if x.is_nan(agent) || y.is_nan(agent) {
            return Self::nan();
        }
        // 2. If x is either +∞𝔽 or -∞𝔽, then
        if x.is_pos_infinity(agent) || x.is_neg_infinity(agent) {
            // a. If y is either +0𝔽 or -0𝔽, return NaN.
            if y.is_pos_zero(agent) || y.is_neg_zero(agent) {
                return Self::nan();
            }
            // b. If y > +0𝔽, return x.
            if y.is_sign_positive(agent) {
                return x;
            }
            // c. Return -x.
            return if x.is_pos_infinity(agent) {
                Self::neg_inf()
            } else {
                Self::pos_inf()
            };
        }
        // 3. If y is either +∞𝔽 or -∞𝔽, then
        if y.is_pos_infinity(agent) || y.is_neg_infinity(agent) {
            // a. If x is either +0𝔽 or -0𝔽, return NaN.
            if x.is_pos_zero(agent) || x.is_neg_zero(agent) {
                return Self::nan();
            }
            // b. If x > +0𝔽, return y.
            if x.is_sign_positive(agent) {
                return y;
            }
            // c. Return -y.
            return if y.is_pos_infinity(agent) {
                Self::neg_inf()
            } else {
                Self::pos_inf()
            };
        }
        // 4. If x is -0𝔽, then
        if x.is_neg_zero(agent) {
            // a. If y is -0𝔽 or y < -0𝔽, return +0𝔽.
            if y.is_neg_zero(agent) || y.is_sign_negative(agent) {
                return Self::pos_zero();
            }
            // b. Else, return -0𝔽.
            return Self::neg_zero();
        }
        // 5. If y is -0𝔽, then
        if y.is_neg_zero(agent) {
            // a. If x < -0𝔽, return +0𝔽.
            if x.is_sign_negative(agent) {
                return Self::pos_zero();
            }
            // b. Else, return -0𝔽.
            return Self::neg_zero();
        }
        // 6. Return 𝔽(ℝ(x) × ℝ(y)).
        Self::from_f64(agent, x.to_real(agent) * y.to_real(agent))
    }

    /// ### [6.1.6.1.5 Number::divide ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-divide)
    ///
    /// The abstract operation Number::divide takes arguments x (a Number) and
    /// y (a Number) and returns a Number. It performs division according to
    /// the rules of IEEE 754-2019 binary double-precision arithmetic,
    /// producing the quotient of x and y where x is the dividend and y is the
    /// divisor.
    pub fn divide(agent: &mut Agent, x: Self, y: Self) -> Self {
        // 1. If x is NaN or y is NaN, return NaN.
        if x.is_nan(agent) || y.is_nan(agent) {
            return Number::nan();
        }
        // 2. If x is either +∞𝔽 or -∞𝔽, then
        if x.is_pos_infinity(agent) || x.is_neg_infinity(agent) {
            // a. If y is either +∞𝔽 or -∞𝔽, return NaN.
            if y.is_pos_infinity(agent) || y.is_neg_infinity(agent) {
                return Number::nan();
            }
            // b. If y is +0𝔽 or y > +0𝔽, return x.
            if y.is_pos_zero(agent) || y.to_real(agent) > 0.0 {
                return x;
            }
            // c. Return -x.
            return if x.is_pos_infinity(agent) {
                Number::neg_inf()
            } else {
                Number::pos_inf()
            };
        }
        // 3. If y is +∞𝔽, then
        if y.is_pos_infinity(agent) {
            // a. If x is +0𝔽 or x > +0𝔽, return +0𝔽. Otherwise, return -0𝔽.
            if x.is_pos_zero(agent) || x.to_real(agent) > 0.0 {
                return Number::pos_zero();
            } else {
                return Number::neg_zero();
            }
        }
        // 4. If y is -∞𝔽, then
        if y.is_neg_infinity(agent) {
            // a. If x is +0𝔽 or x > +0𝔽, return -0𝔽. Otherwise, return +0𝔽.
            if x.is_pos_zero(agent) || x.to_real(agent) > 0.0 {
                return Number::neg_zero();
            } else {
                return Number::pos_zero();
            }
        }
        // 5. If x is either +0𝔽 or -0𝔽, then
        if x.is_pos_zero(agent) || x.is_neg_zero(agent) {
            // a. If y is either +0𝔽 or -0𝔽, return NaN.
            if y.is_pos_zero(agent) || y.is_neg_zero(agent) {
                return Number::nan();
            }
            // b. If y > +0𝔽, return x.
            if y.to_real(agent) > 0.0 {
                return x;
            }
            // c. Return -x.
            return if x.is_pos_zero(agent) {
                Number::neg_zero()
            } else {
                Number::pos_zero()
            };
        }
        // 6. If y is +0𝔽, then
        if y.is_pos_zero(agent) {
            // a. If x > +0𝔽, return +∞𝔽. Otherwise, return -∞𝔽.
            return if x.to_real(agent) > 0.0 {
                Number::pos_inf()
            } else {
                Number::neg_inf()
            };
        }
        // 7. If y is -0𝔽, then
        if y.is_neg_zero(agent) {
            // a. If x > +0𝔽, return -∞𝔽. Otherwise, return +∞𝔽.
            return if x.to_real(agent) > 0.0 {
                Number::neg_inf()
            } else {
                Number::pos_inf()
            };
        }
        // 8. Return 𝔽(ℝ(x) / ℝ(y)).
        let result = x.to_real(agent) / y.to_real(agent);
        Number::from_f64(agent, result)
    }

    /// ### [6.1.6.1.6 Number::remainder ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-remainder)
    ///
    /// The abstract operation Number::remainder takes arguments n (a Number)
    /// and d (a Number) and returns a Number. It yields the remainder from an
    /// implied division of its operands where n is the dividend and d is the
    /// divisor.
    pub fn remainder(agent: &mut Agent, n: Self, d: Self) -> Self {
        // 1. If n is NaN or d is NaN, return NaN.
        if n.is_nan(agent) || d.is_nan(agent) {
            return Self::nan();
        }

        // 2. If n is either +∞𝔽 or -∞𝔽, return NaN.
        if n.is_pos_infinity(agent) || n.is_neg_infinity(agent) {
            return Self::nan();
        }

        // 3. If d is either +∞𝔽 or -∞𝔽, return n.
        if d.is_pos_infinity(agent) || d.is_neg_infinity(agent) {
            return n;
        }

        // 4. If d is either +0𝔽 or -0𝔽, return NaN.
        if d.is_pos_zero(agent) || d.is_neg_zero(agent) {
            return Self::nan();
        }

        // 5. If n is either +0𝔽 or -0𝔽, return n.
        if n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return n;
        }

        // 6. Assert: n and d are finite and non-zero.
        debug_assert!(n.is_finite(agent) && n.is_nonzero(agent));

        let n = n.into_f64(agent);
        let d = d.into_f64(agent);

        // 7. Let quotient be ℝ(n) / ℝ(d).
        let quotient = n / d;

        // 8. Let q be truncate(quotient).
        let q = quotient.trunc();

        // 9. Let r be ℝ(n) - (ℝ(d) × q).
        let r = n - (d * q);

        // 10. If r = 0 and n < -0𝔽, return -0𝔽.
        if r == 0.0 && n.is_sign_negative() {
            return Self::neg_zero();
        }

        // 11. Return 𝔽(r).
        Self::from_f64(agent, r)
    }

    /// ### [6.1.6.1.7 Number::add ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-add)
    ///
    /// The abstract operation Number::add takes arguments x (a Number) and y
    /// (a Number) and returns a Number. It performs addition according to the
    /// rules of IEEE 754-2019 binary double-precision arithmetic, producing
    /// the sum of its arguments.
    pub(crate) fn add(agent: &mut Agent, x: Number, y: Number) -> Number {
        // 1. If x is NaN or y is NaN, return NaN.
        if x.is_nan(agent) || y.is_nan(agent) {
            return Number::nan();
        }

        // 2. If x is +∞𝔽 and y is -∞𝔽, return NaN.
        if x.is_pos_infinity(agent) && y.is_neg_infinity(agent) {
            return Number::nan();
        }

        // 3. If x is -∞𝔽 and y is +∞𝔽, return NaN.
        if x.is_neg_infinity(agent) && y.is_pos_infinity(agent) {
            return Number::nan();
        }

        // 4. If x is either +∞𝔽 or -∞𝔽, return x.
        if x.is_pos_infinity(agent) || x.is_neg_infinity(agent) {
            return x;
        }

        // 5. If y is either +∞𝔽 or -∞𝔽, return y.
        if y.is_pos_infinity(agent) || y.is_neg_infinity(agent) {
            return y;
        }

        // 6. Assert: x and y are both finite.
        debug_assert!(x.is_finite(agent) && y.is_finite(agent));

        // 7. If x is -0𝔽 and y is -0𝔽, return -0𝔽.
        if x.is_neg_zero(agent) && y.is_neg_zero(agent) {
            return Number::neg_zero();
        }

        // 8. Return 𝔽(ℝ(x) + ℝ(y)).
        agent.heap.create(x.into_f64(agent) + y.into_f64(agent))
    }

    /// ### [6.1.6.1.8 Number::subtract ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-subtract)
    ///
    /// The abstract operation Number::subtract takes arguments x (a Number)
    /// and y (a Number) and returns a Number. It performs subtraction,
    /// producing the difference of its operands; x is the minuend and y is the
    /// subtrahend.
    pub(crate) fn subtract(agent: &mut Agent, x: Number, y: Number) -> Number {
        // 1. Return Number::add(x, Number::unaryMinus(y)).
        let negated_y = Number::unary_minus(agent, y);
        Number::add(agent, x, negated_y)
    }

    /// ### [6.1.6.1.9 Number::leftShift ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-leftShift)
    ///
    /// The abstract operation Number::signedRightShift takes arguments x
    /// (a Number) and y (a Number) and returns an integral Number.
    pub fn left_shift(agent: &mut Agent, x: Self, y: Self) -> Self {
        // 1. Let lnum be ! ToInt32(x).
        let lnum = to_int32(agent, x.into_value()).unwrap();
        // 2. Let rnum be ! ToUint32(y).
        let rnum = to_uint32(agent, y.into_value()).unwrap();
        // 3. Let shiftCount be ℝ(rnum) modulo 32.
        let shift_count = rnum % 32;
        // 4. Return the result of left shifting lnum by shiftCount bits. The mathematical value of the result is exactly representable as a 32-bit two's complement bit string.
        Number::from(lnum.signed_shl(shift_count))
    }

    /// ### [6.1.6.1.10 Number::signedRightShift ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-signedRightShift)
    ///
    /// The abstract operation Number::unsignedRightShift takes arguments x
    /// (a Number) and y (a Number) and returns an integral Number.
    pub fn signed_right_shift(agent: &mut Agent, x: Self, y: Self) -> Self {
        // 1. Let lnum be ! ToInt32(x).
        let lnum = to_int32(agent, x.into_value()).unwrap();
        // 2. Let rnum be ! ToUint32(y).
        let rnum = to_uint32(agent, y.into_value()).unwrap();
        // 3. Let shiftCount be ℝ(rnum) modulo 32.
        let shift_count = rnum % 32;
        // 4. Return the result of performing a sign-extending right shift of lnum by shiftCount bits. The most significant bit is propagated. The mathematical value of the result is exactly representable as a 32-bit two's complement bit string.
        Number::from(lnum.signed_shr(shift_count))
    }

    /// ### [6.1.6.1.11 Number::unsignedRightShift ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-unsignedRightShift)
    ///
    /// The abstract operation Number::lessThan takes arguments x (a Number)
    /// and y (a Number) and returns a Boolean or undefined.
    pub fn unsigned_right_shift(agent: &mut Agent, x: Self, y: Self) -> Self {
        // 1. Let lnum be ! ToUint32(x).
        let lnum = to_uint32(agent, x.into_value()).unwrap();
        // 2. Let rnum be ! ToUint32(y).
        let rnum = to_uint32(agent, y.into_value()).unwrap();
        // 3. Let shiftCount be ℝ(rnum) modulo 32.
        let shift_count = rnum % 32;
        // 4. Return the result of performing a zero-filling right shift of lnum by shiftCount bits. Vacated bits are filled with zero. The mathematical value of the result is exactly representable as a 32-bit unsigned bit string.
        Number::from(lnum.unsigned_shr(shift_count))
    }

    /// ### [6.1.6.1.12 Number::lessThan ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-lessThan)
    pub fn less_than(agent: &mut Agent, x: Self, y: Self) -> Option<bool> {
        // 1. If x is NaN, return undefined.
        if x.is_nan(agent) {
            return None;
        }

        // 2. If y is NaN, return undefined.
        if y.is_nan(agent) {
            return None;
        }

        // 3. If x is y, return false.
        if Number::is(agent, x, y) {
            return Some(false);
        }

        // 4. If x is +0𝔽 and y is -0𝔽, return false.
        if x.is_pos_zero(agent) && y.is_neg_zero(agent) {
            return Some(false);
        }

        // 5. If x is -0𝔽 and y is +0𝔽, return false.
        if x.is_neg_zero(agent) && y.is_pos_zero(agent) {
            return Some(false);
        }

        // 6. If x is +∞𝔽, return false.
        if x.is_pos_infinity(agent) {
            return Some(false);
        }

        // 7. If y is +∞𝔽, return true.
        if y.is_pos_infinity(agent) {
            return Some(true);
        }

        // 8. If y is -∞𝔽, return false.
        if y.is_neg_infinity(agent) {
            return Some(false);
        }

        // 9. If x is -∞𝔽, return true.
        if x.is_neg_infinity(agent) {
            return Some(true);
        }

        // 10. Assert: x and y are finite.
        assert!(x.is_finite(agent) && y.is_finite(agent));

        // 11. If ℝ(x) < ℝ(y), return true. Otherwise, return false.
        Some(match (x, y) {
            (Number::Number(x), Number::Number(y)) => agent[x] < agent[y],
            (Number::Number(x), Number::Integer(y)) => agent[x] < y.into_i64() as f64,
            (Number::Number(x), Number::SmallF64(y)) => agent[x] < y.into_f64(),
            (Number::Integer(x), Number::Number(y)) => (x.into_i64() as f64) < agent[y],
            (Number::Integer(x), Number::Integer(y)) => x.into_i64() < y.into_i64(),
            (Number::Integer(x), Number::SmallF64(y)) => (x.into_i64() as f64) < y.into_f64(),
            (Number::SmallF64(x), Number::Number(y)) => x.into_f64() < agent[y],
            (Number::SmallF64(x), Number::Integer(y)) => x.into_f64() < y.into_i64() as f64,
            (Number::SmallF64(x), Number::SmallF64(y)) => x.into_f64() < y.into_f64(),
        })
    }

    /// ### [6.1.6.1.13 Number::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-equal)
    pub fn equal(agent: &impl Index<HeapNumber, Output = f64>, x: Self, y: Self) -> bool {
        // 1. If x is NaN, return false.
        if x.is_nan(agent) {
            return false;
        }

        // 2. If y is NaN, return false.
        if y.is_nan(agent) {
            return false;
        }

        // 3. If x is y, return true.
        if Number::is(agent, x, y) {
            return true;
        }

        // 4. If x is +0𝔽 and y is -0𝔽, return true.
        if x.is_pos_zero(agent) && y.is_neg_zero(agent) {
            return true;
        }

        // 5. If x is -0𝔽 and y is +0𝔽, return true.
        if x.is_neg_zero(agent) && y.is_pos_zero(agent) {
            return true;
        }

        // 6. Return false.
        false
    }

    /// ### [6.1.6.1.14 Number::sameValue ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-sameValue)
    pub fn same_value(agent: &impl Index<HeapNumber, Output = f64>, x: Self, y: Self) -> bool {
        // 1. If x is NaN and y is NaN, return true.
        if x.is_nan(agent) && y.is_nan(agent) {
            return true;
        }

        // 2. If x is +0𝔽 and y is -0𝔽, return false.
        if x.is_pos_zero(agent) && y.is_neg_zero(agent) {
            return false;
        }

        // 3. If x is -0𝔽 and y is +0𝔽, return false.
        if x.is_neg_zero(agent) && y.is_pos_zero(agent) {
            return false;
        }

        // 4. If x is y, return true.
        if Number::is(agent, x, y) {
            return true;
        }

        // 5. Return false.
        false
    }

    /// ### [6.1.6.1.15 Number::sameValueZero ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-sameValueZero)
    pub fn same_value_zero(agent: &impl Index<HeapNumber, Output = f64>, x: Self, y: Self) -> bool {
        // 1. If x is NaN and y is NaN, return true.
        if x.is_nan(agent) && y.is_nan(agent) {
            return true;
        }

        // 2. If x is +0𝔽 and y is -0𝔽, return true.
        if x.is_pos_zero(agent) && y.is_neg_zero(agent) {
            return true;
        }

        // 3. If x is -0𝔽 and y is +0𝔽, return true.
        if x.is_neg_zero(agent) && y.is_pos_zero(agent) {
            return true;
        }

        // 4. If x is y, return true.
        if Number::is(agent, x, y) {
            return true;
        }

        // 5. Return false.
        false
    }

    /// ### [6.1.6.1.16 NumberBitwiseOp ( op, x, y )](https://tc39.es/ecma262/#sec-numberbitwiseop)
    #[inline(always)]
    fn bitwise_op(agent: &mut Agent, op: BitwiseOp, x: Self, y: Self) -> JsResult<i32> {
        // 1. Let lnum be ! ToInt32(x).
        let lnum = x.into_value().to_int32(agent)?;

        // 2. Let rnum be ! ToInt32(y).
        let rnum = y.into_value().to_int32(agent)?;

        // 3. Let lbits be the 32-bit two's complement bit string representing ℝ(lnum).
        let lbits = lnum;

        // 4. Let rbits be the 32-bit two's complement bit string representing ℝ(rnum).
        let rbits = rnum;

        let result = match op {
            // 5. If op is &, then
            BitwiseOp::And => {
                // a. Let result be the result of applying the bitwise AND operation to lbits and rbits.
                lbits & rbits
            }
            // 6. Else if op is ^, then
            BitwiseOp::Xor => {
                // a. Let result be the result of applying the bitwise exclusive OR (XOR) operation to lbits and rbits.
                lbits ^ rbits
            }
            // 7. Else,
            // a. Assert: op is |.
            BitwiseOp::Or => {
                // b. Let result be the result of applying the bitwise inclusive OR operation to lbits and rbits.
                lbits | rbits
            }
        };

        // 8. Return the Number value for the integer represented by the 32-bit two's complement bit string result.
        Ok(result)
    }

    /// ### [6.1.6.1.17 Number::bitwiseAND ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseAND)
    pub fn bitwise_and(agent: &mut Agent, x: Self, y: Self) -> JsResult<i32> {
        // 1. Return NumberBitwiseOp(&, x, y).
        Number::bitwise_op(agent, BitwiseOp::And, x, y)
    }

    /// ### [6.1.6.1.18 Number::bitwiseXOR ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseXOR)
    pub fn bitwise_xor(agent: &mut Agent, x: Self, y: Self) -> JsResult<i32> {
        // 1. Return NumberBitwiseOp(^, x, y).
        Number::bitwise_op(agent, BitwiseOp::Xor, x, y)
    }

    /// ### [6.1.6.1.19 Number::bitwiseOR ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseOR)
    pub fn bitwise_or(agent: &mut Agent, x: Self, y: Self) -> JsResult<i32> {
        // 1. Return NumberBitwiseOp(|, x, y).
        Number::bitwise_op(agent, BitwiseOp::Or, x, y)
    }

    // ### [6.1.6.1.20 Number::toString ( x, radix )](https://tc39.es/ecma262/#sec-numeric-types-number-tostring)
    pub(crate) fn to_string_radix_10(agent: &mut Agent, x: Self) -> String {
        match x {
            Number::Number(_) => {
                let mut buffer = ryu_js::Buffer::new();
                String::from_string(agent, buffer.format(x.into_f64(agent)).to_string())
            }
            Number::Integer(x) => {
                let x = x.into_i64();
                String::from_string(agent, format!("{x}"))
            }
            Number::SmallF64(x) => {
                let mut buffer = ryu_js::Buffer::new();
                String::from_string(agent, buffer.format(x.into_f64()).to_string())
            }
        }
    }

    /// ### [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub(crate) fn to_real(self, agent: &impl Index<HeapNumber, Output = f64>) -> f64 {
        match self {
            Self::Number(n) => agent[n],
            Self::Integer(i) => i.into_i64() as f64,
            Self::SmallF64(f) => f.into_f64(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BitwiseOp {
    And,
    Xor,
    Or,
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for Number {
            fn from(value: $size) -> Self {
                Number::Integer(SmallInteger::from(value))
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

impl Index<HeapNumber> for PrimitiveHeap<'_> {
    type Output = f64;

    fn index(&self, index: HeapNumber) -> &Self::Output {
        &self.numbers[index]
    }
}

impl Index<HeapNumber> for Agent {
    type Output = f64;

    fn index(&self, index: HeapNumber) -> &Self::Output {
        &self.heap.numbers[index]
    }
}

impl IndexMut<HeapNumber> for Agent {
    fn index_mut(&mut self, index: HeapNumber) -> &mut Self::Output {
        &mut self.heap.numbers[index]
    }
}

impl Index<HeapNumber> for Vec<Option<NumberHeapData>> {
    type Output = f64;

    fn index(&self, index: HeapNumber) -> &Self::Output {
        &self
            .get(index.get_index())
            .expect("HeapNumber out of bounds")
            .as_ref()
            .expect("HeapNumber slot empty")
            .data
    }
}

impl IndexMut<HeapNumber> for Vec<Option<NumberHeapData>> {
    fn index_mut(&mut self, index: HeapNumber) -> &mut Self::Output {
        &mut self
            .get_mut(index.get_index())
            .expect("HeapNumber out of bounds")
            .as_mut()
            .expect("HeapNumber slot empty")
            .data
    }
}

impl CreateHeapData<f64, Number> for Heap {
    fn create(&mut self, data: f64) -> Number {
        // NOTE: This function cannot currently be implemented
        // directly using `Number::from_f64` as it takes an Agent
        // parameter that we do not have access to here.
        if let Ok(value) = Number::try_from(data) {
            value
        } else {
            // SAFETY: Number was not representable as a
            // stack-allocated Number.
            let heap_number = unsafe { self.alloc_number(data) };
            Number::Number(heap_number)
        }
    }
}

impl HeapMarkAndSweep for Number {
    fn mark_values(&self, queues: &mut WorkQueues) {
        if let Self::Number(idx) = self {
            idx.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        if let Self::Number(idx) = self {
            idx.sweep_values(compactions);
        }
    }
}

impl HeapMarkAndSweep for HeapNumber {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.numbers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.numbers.shift_index(&mut self.0);
    }
}

impl Rootable for Number {
    type RootRepr = NumberRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Number(heap_number) => Err(HeapRootData::Number(heap_number)),
            Self::Integer(integer) => Ok(Self::RootRepr::Integer(integer)),
            Self::SmallF64(small_f64) => Ok(Self::RootRepr::SmallF64(small_f64)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Integer(small_integer) => Ok(Self::Integer(small_integer)),
            Self::RootRepr::SmallF64(small_f64) => Ok(Self::SmallF64(small_f64)),
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
            HeapRootData::Number(heap_number) => Some(Self::Number(heap_number)),
            _ => None,
        }
    }
}
