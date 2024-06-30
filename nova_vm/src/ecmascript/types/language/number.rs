mod data;

use std::ops::{Index, IndexMut};

use super::{
    value::{FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, NUMBER_DISCRIMINANT},
    IntoNumeric, IntoPrimitive, IntoValue, Numeric, Primitive, String, Value,
};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_int32,
        execution::{Agent, JsResult},
    },
    heap::{
        indexes::NumberIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
    SmallInteger,
};

pub use data::NumberHeapData;

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
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    // 56-bit signed integer.
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
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
            Number::Float(data) => Value::Float(data),
        }
    }
}

impl IntoNumeric for HeapNumber {
    fn into_numeric(self) -> Numeric {
        Numeric::Number(self)
    }
}

impl IntoPrimitive for Number {
    fn into_primitive(self) -> Primitive {
        match self {
            Number::Number(idx) => Primitive::Number(idx),
            Number::Integer(data) => Primitive::Integer(data),
            Number::Float(data) => Primitive::Float(data),
        }
    }
}

impl IntoNumeric for Number {
    fn into_numeric(self) -> Numeric {
        match self {
            Number::Number(idx) => Numeric::Number(idx),
            Number::Integer(data) => Numeric::Integer(data),
            Number::Float(data) => Numeric::Float(data),
        }
    }
}

impl std::fmt::Debug for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Number::Number(idx) => write!(f, "Number({:?})", idx),
            Number::Integer(value) => write!(f, "{}", value.into_i64()),
            Number::Float(value) => write!(f, "{}", value),
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

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Number::Integer(
            SmallInteger::try_from(value.clamp(SmallInteger::MIN_NUMBER, SmallInteger::MAX_NUMBER))
                .unwrap(),
        )
    }
}

impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Number::Float(value)
    }
}

const MAX_NUMBER: f64 = ((1u64 << 53) - 1) as f64;
const MIN_NUMBER: f64 = -MAX_NUMBER;

impl TryFrom<f64> for Number {
    type Error = ();

    fn try_from(value: f64) -> Result<Self, ()> {
        if value.is_finite() && value.trunc() == value && (MIN_NUMBER..=MAX_NUMBER).contains(&value)
        {
            debug_assert_eq!(value as i64 as f64, value);
            Ok(Number::from(value as i64))
        } else if value as f32 as f64 == value {
            Ok(Number::Float(value as f32))
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
            Value::Float(data) => Ok(Number::Float(data)),
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
            Primitive::Float(data) => Ok(Number::Float(data)),
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
            Numeric::Float(data) => Ok(Number::Float(data)),
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

    pub fn is_nan(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => agent[n].is_nan(),
            Number::Integer(_) => false,
            Number::Float(n) => n.is_nan(),
        }
    }

    pub fn is_pos_zero(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => f64::to_bits(0.0) == f64::to_bits(agent[n]),
            Number::Integer(n) => 0i64 == n.into(),
            Number::Float(n) => f32::to_bits(0.0) == f32::to_bits(n),
        }
    }

    pub fn is_neg_zero(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => f64::to_bits(-0.0) == f64::to_bits(agent[n]),
            Number::Integer(_) => false,
            Number::Float(n) => f32::to_bits(-0.0) == f32::to_bits(n),
        }
    }

    pub fn is_pos_infinity(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => agent[n] == f64::INFINITY,
            Number::Integer(_) => false,
            Number::Float(n) => n == f32::INFINITY,
        }
    }

    pub fn is_neg_infinity(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => agent[n] == f64::NEG_INFINITY,
            Number::Integer(_) => false,
            Number::Float(n) => n == f32::NEG_INFINITY,
        }
    }

    pub fn is_finite(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => agent[n].is_finite(),
            Number::Integer(_) => true,
            Number::Float(n) => n.is_finite(),
        }
    }

    pub fn is_nonzero(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => 0.0 != agent[n],
            Number::Integer(n) => 0i64 != n.into(),
            Number::Float(n) => 0.0 != n,
        }
    }

    pub fn is_sign_positive(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => agent[n].is_sign_positive(),
            Number::Integer(n) => n.into_i64().is_positive(),
            Number::Float(n) => n.is_sign_positive(),
        }
    }

    pub fn is_sign_negative(self, agent: &Agent) -> bool {
        match self {
            Number::Number(n) => agent[n].is_sign_negative(),
            Number::Integer(n) => n.into_i64().is_negative(),
            Number::Float(n) => n.is_sign_negative(),
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
            Number::Float(n) => n.trunc().into(),
        }
    }

    pub fn into_f64(self, agent: &Agent) -> f64 {
        match self {
            Number::Number(n) => agent[n],
            Number::Integer(n) => Into::<i64>::into(n) as f64,
            Number::Float(n) => n as f64,
        }
    }

    pub fn into_i64(self, agent: &Agent) -> i64 {
        match self {
            Number::Number(n) => agent[n] as i64,
            Number::Integer(n) => Into::<i64>::into(n),
            Number::Float(n) => n as i64,
        }
    }

    /// Compare two Numbers with each other: This is used when the spec asks if
    /// `x is y` when talking of Numbers. Generally this is asked after various
    /// NaN and non-zero checks, depending on which spec algorithm is being
    /// used.
    #[inline(always)]
    fn is(agent: &Agent, x: Self, y: Self) -> bool {
        match (x, y) {
            // Optimisation: First compare by-reference; only read from heap if needed.
            (Number::Number(x), Number::Number(y)) => x == y || agent[x] == agent[y],
            (Number::Integer(x), Number::Integer(y)) => x == y,
            (Number::Float(x), Number::Float(y)) => x == y,
            (Number::Number(x), Number::Integer(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!(agent[x] != y.into_i64() as f64);
                false
            }
            (Number::Number(x), Number::Float(y)) => {
                // Optimisation: f32s should never be allocated into the heap
                debug_assert!(agent[x] != y as f64);
                false
            }
            (Number::Integer(x), Number::Number(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!((x.into_i64() as f64) != agent[y]);
                false
            }
            (Number::Integer(x), Number::Float(y)) => {
                debug_assert!(
                    y.to_bits() == (-0f32).to_bits() || (x.into_i64() as f64) != y as f64
                );
                false
            }
            (Number::Float(x), Number::Number(y)) => {
                // Optimisation: f32s should never be allocated into the heap
                debug_assert!((x as f64) != agent[y]);
                false
            }
            (Number::Float(x), Number::Integer(y)) => {
                debug_assert!(
                    x.to_bits() == (-0f32).to_bits() || (x as f64) != y.into_i64() as f64
                );
                false
            }
        }
    }

    pub fn is_odd_integer(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => agent[n] % 2.0 == 1.0,
            Number::Integer(n) => Into::<i64>::into(n) % 2 == 1,
            Number::Float(n) => n % 2.0 == 1.0,
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
            Number::Float(n) => {
                if n == 0.0 {
                    // Negative zero needs to be turned into a Number::Integer
                    debug_assert!(n.is_sign_negative());
                    Number::Integer(SmallInteger::zero())
                } else {
                    Number::Float(n.abs())
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
                    Number::Float(-0.0)
                } else {
                    SmallInteger::try_from(-n).unwrap().into()
                }
            }
            Number::Float(n) => {
                if n == 0.0 {
                    debug_assert!(n.is_sign_negative());
                    SmallInteger::zero().into()
                } else {
                    (-n).into()
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

    // ...

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
            (Number::Number(x), Number::Float(y)) => agent[x] < y as f64,
            (Number::Integer(x), Number::Number(y)) => (x.into_i64() as f64) < agent[y],
            (Number::Integer(x), Number::Integer(y)) => x.into_i64() < y.into_i64(),
            (Number::Integer(x), Number::Float(y)) => (x.into_i64() as f64) < y as f64,
            (Number::Float(x), Number::Number(y)) => (x as f64) < agent[y],
            (Number::Float(x), Number::Integer(y)) => (x as f64) < y.into_i64() as f64,
            (Number::Float(x), Number::Float(y)) => x < y,
        })
    }

    /// ### [6.1.6.1.13 Number::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-equal)
    pub fn equal(agent: &Agent, x: Self, y: Self) -> bool {
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
    pub fn same_value(agent: &mut Agent, x: Self, y: Self) -> bool {
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
    pub fn same_value_zero(agent: &Agent, x: Self, y: Self) -> bool {
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
    pub(crate) fn to_string_radix_10(agent: &mut Agent, x: Self) -> JsResult<String> {
        match x {
            Number::Number(_) => {
                let mut buffer = ryu_js::Buffer::new();
                Ok(String::from_string(
                    agent,
                    buffer.format(x.into_f64(agent)).to_string(),
                ))
            }
            Number::Integer(x) => {
                let x = x.into_i64();
                Ok(String::from_string(agent, format!("{x}")))
            }
            Number::Float(x) => {
                let mut buffer = ryu_js::Buffer::new();
                Ok(String::from_string(agent, buffer.format(x).to_string()))
            }
        }
    }

    /// ### [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub(crate) fn to_real(self, agent: &Agent) -> f64 {
        match self {
            Self::Number(n) => agent[n],
            Self::Integer(i) => i.into_i64() as f64,
            Self::Float(f) => f as f64,
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
        let self_index = self.0.into_u32();
        self.0 =
            NumberIndex::from_u32(self_index - compactions.numbers.get_shift_for_index(self_index));
    }
}
