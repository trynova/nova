mod data;

use super::{
    value::{FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, NUMBER_DISCRIMINANT},
    Value,
};
use crate::{
    ecmascript::execution::{Agent, JsResult},
    heap::{indexes::NumberIndex, CreateHeapData, GetHeapData},
    SmallInteger,
};

pub use data::NumberHeapData;

/// 6.1.6.1 The Number Type
/// https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Number {
    Number(NumberIndex) = NUMBER_DISCRIMINANT,
    // 56-bit signed integer.
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
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

impl From<SmallInteger> for Number {
    fn from(value: SmallInteger) -> Self {
        Number::Integer(value)
    }
}

impl From<i32> for Number {
    fn from(value: i32) -> Self {
        Number::Integer(SmallInteger::from_i64_unchecked(value as i64))
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        let n = value
            .min(SmallInteger::MAX_NUMBER)
            .max(SmallInteger::MIN_NUMBER);
        Number::Integer(SmallInteger::from_i64_unchecked(n))
    }
}

impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Number::Float(value)
    }
}

impl TryFrom<Value> for Number {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if matches!(
            value,
            Value::Number(_) | Value::Integer(_) | Value::Float(_)
        ) {
            // SAFETY: Sub-enum.
            Ok(unsafe { std::mem::transmute::<Value, Number>(value) })
        } else {
            Err(())
        }
    }
}

impl Number {
    pub fn new(value: Value) -> Self {
        debug_assert!(matches!(
            value,
            Value::Number(_) | Value::Integer(_) | Value::Float(_)
        ));
        // SAFETY: Sub-enum.
        unsafe { std::mem::transmute::<Value, Number>(value) }
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

    pub fn into_value(self) -> Value {
        // SAFETY: Sub-enum.
        unsafe { std::mem::transmute::<Number, Value>(self) }
    }

    pub fn is_nan(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => agent.heap.get(n).is_nan(),
            Number::Integer(_) => false,
            Number::Float(n) => n.is_nan(),
        }
    }

    pub fn is_pos_zero(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => f64::to_bits(0.0) == f64::to_bits(*agent.heap.get(n)),
            Number::Integer(n) => 0i64 == n.into(),
            Number::Float(n) => f32::to_bits(0.0) == f32::to_bits(n),
        }
    }

    pub fn is_neg_zero(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => f64::to_bits(-0.0) == f64::to_bits(*agent.heap.get(n)),
            Number::Integer(_) => false,
            Number::Float(n) => f32::to_bits(-0.0) == f32::to_bits(n),
        }
    }

    pub fn is_pos_infinity(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => *agent.heap.get(n) == f64::INFINITY,
            Number::Integer(_) => false,
            Number::Float(n) => n == f32::INFINITY,
        }
    }

    pub fn is_neg_infinity(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => *agent.heap.get(n) == f64::NEG_INFINITY,
            Number::Integer(_) => false,
            Number::Float(n) => n == f32::NEG_INFINITY,
        }
    }

    pub fn is_finite(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => agent.heap.get(n).is_finite(),
            Number::Integer(_) => true,
            Number::Float(n) => n.is_finite(),
        }
    }

    pub fn is_nonzero(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => 0.0 != *agent.heap.get(n),
            Number::Integer(n) => 0i64 != n.into(),
            Number::Float(n) => 0.0 != n,
        }
    }

    /// https://tc39.es/ecma262/#eqn-truncate
    pub fn truncate(self, agent: &mut Agent) -> Number {
        match self {
            Number::Number(n) => {
                let n = agent.heap.get(n).trunc();
                agent.heap.create(n)
            }
            Number::Integer(_) => self,
            Number::Float(n) => n.trunc().into(),
        }
    }

    pub fn into_f64(self, agent: &Agent) -> f64 {
        match self {
            Number::Number(n) => *agent.heap.get(n),
            Number::Integer(n) => Into::<i64>::into(n) as f64,
            Number::Float(n) => n as f64,
        }
    }

    pub fn into_i64(self, agent: &Agent) -> i64 {
        match self {
            Number::Number(n) => *agent.heap.get(n) as i64,
            Number::Integer(n) => Into::<i64>::into(n),
            Number::Float(n) => n as i64,
        }
    }

    /// Compare two Numbers with each other: This is used when the spec asks if
    /// `x is y` when talking of Numbers. Generally this is asked after various
    /// NaN and non-zero checks, depending on which spec algorithm is being used.
    #[inline(always)]
    fn is(self, agent: &mut Agent, y: Self) -> bool {
        match (self, y) {
            // Optimisation: First compare by-reference; only read from heap if needed.
            (Number::Number(x), Number::Number(y)) => {
                x == y || agent.heap.get(x) == agent.heap.get(y)
            }
            (Number::Integer(x), Number::Integer(y)) => x == y,
            (Number::Float(x), Number::Float(y)) => x == y,
            (Number::Number(x), Number::Integer(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!(*agent.heap.get(x) != y.into_i64() as f64);
                false
            }
            (Number::Number(x), Number::Float(y)) => {
                // Optimisation: f32s should never be allocated into the heap
                debug_assert!(*agent.heap.get(x) != y as f64);
                false
            }
            (Number::Integer(x), Number::Number(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!((x.into_i64() as f64) != *agent.heap.get(y));
                false
            }
            (Number::Integer(x), Number::Float(y)) => {
                debug_assert!((x.into_i64() as f64) != y as f64);
                false
            }
            (Number::Float(x), Number::Number(y)) => {
                // Optimisation: f32s should never be allocated into the heap
                debug_assert!((x as f64) != *agent.heap.get(y));
                false
            }
            (Number::Float(x), Number::Integer(y)) => {
                debug_assert!((x as f64) != y.into_i64() as f64);
                false
            }
        }
    }

    pub fn is_odd_integer(self, agent: &mut Agent) -> bool {
        match self {
            Number::Number(n) => *agent.heap.get(n) % 2.0 == 1.0,
            Number::Integer(n) => Into::<i64>::into(n) % 2 == 1,
            Number::Float(n) => n % 2.0 == 1.0,
        }
    }

    pub fn abs(self, agent: &mut Agent) -> Self {
        match self {
            Number::Number(n) => {
                let n = *agent.heap.get(n);
                if n > 0.0 {
                    self
                } else {
                    agent.heap.create(-n)
                }
            }
            Number::Integer(n) => {
                let n = n.into_i64();
                Number::Integer(SmallInteger::from_i64_unchecked(n.abs()))
            }
            Number::Float(n) => Number::Float(n.abs()),
        }
    }

    pub fn greater_than(self, agent: &mut Agent, y: Self) -> Option<bool> {
        y.less_than(agent, self).map(|x| !x)
    }

    /// 6.1.6.1.1 Number::unaryMinus ( x )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-unaryMinus
    pub fn unary_minus(self, agent: &mut Agent) -> Self {
        // 1. If x is NaN, return NaN.
        // NOTE: Computers do this automatically.

        // 2. Return the result of negating x; that is, compute a Number with the same magnitude but opposite sign.
        match self {
            Number::Number(n) => {
                let value = *agent.heap.get(n);
                agent.heap.create(-value)
            }
            Number::Integer(n) => SmallInteger::from_i64_unchecked(-n.into_i64()).into(),
            Number::Float(n) => (-n).into(),
        }
    }

    /// 6.1.6.1.2 Number::bitwiseNOT ( x )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseNOT
    pub fn bitwise_not(self, agent: &mut Agent) -> JsResult<Self> {
        let x = self.into_value();

        // 1. Let oldValue be ! ToInt32(x).
        let old_value = x.to_int32(agent)?;

        // 2. Return the result of applying bitwise complement to oldValue. The mathematical value of the result is exactly representable as a 32-bit two's complement bit string.
        Ok(Number::from(!old_value))
    }

    /// 6.1.6.1.3 Number::exponentiate ( base, exponent )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-exponentiate
    pub fn exponentiate(self, agent: &mut Agent, exponent: Self) -> Self {
        let base = self;

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
            return if exponent
                .greater_than(agent, Number::from(0))
                .unwrap_or(false)
            {
                Number::pos_inf()
            } else {
                Number::pos_zero()
            };
        }

        // 5. If base is -∞𝔽, then
        if base.is_neg_infinity(agent) {
            // a. If exponent > +0𝔽, then
            return if exponent.greater_than(agent, 0.into()).unwrap_or(false) {
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
            return if exponent
                .greater_than(agent, Number::pos_zero())
                .unwrap_or(false)
            {
                Number::pos_zero()
            } else {
                Number::pos_inf()
            };
        }

        // 7. If base is -0𝔽, then
        if base.is_neg_zero(agent) {
            // a. If exponent > +0𝔽, then
            return if exponent
                .greater_than(agent, Number::pos_zero())
                .unwrap_or(false)
            {
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
            return if base.greater_than(agent, Number::from(1)).unwrap_or(false) {
                Number::pos_inf()
            }
            // b. If abs(ℝ(base)) = 1, return NaN.
            else if base.is(agent, Number::from(1)) {
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
        if base.less_than(agent, Number::neg_zero()).unwrap_or(false)
            && !exponent.is_odd_integer(agent)
        {
            return Number::nan();
        }

        // 13. Return an implementation-approximated Number value representing the result of raising ℝ(base) to the ℝ(exponent) power.
        agent
            .heap
            .create(base.into_f64(agent).powf(exponent.into_f64(agent)))
    }

    // ...

    /// 6.1.6.1.12 Number::lessThan ( x, y )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-lessThan
    pub fn less_than(self, agent: &mut Agent, y: Self) -> Option<bool> {
        let x = self;

        // 1. If x is NaN, return undefined.
        if x.is_nan(agent) {
            return None;
        }

        // 2. If y is NaN, return undefined.
        if y.is_nan(agent) {
            return None;
        }

        // 3. If x is y, return false.
        if x.is(agent, y) {
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

        // 10. Assert: x and y are finite and non-zero.
        assert!(
            x.is_finite(agent) && x.is_nonzero(agent) && y.is_finite(agent) && y.is_nonzero(agent)
        );

        // 11. If ℝ(x) < ℝ(y), return true. Otherwise, return false.
        Some(match (x, y) {
            (Number::Number(x), Number::Number(y)) => agent.heap.get(x) < agent.heap.get(y),
            (Number::Number(x), Number::Integer(y)) => *agent.heap.get(x) < y.into_i64() as f64,
            (Number::Number(x), Number::Float(y)) => *agent.heap.get(x) < y as f64,
            (Number::Integer(x), Number::Number(y)) => (x.into_i64() as f64) < *agent.heap.get(y),
            (Number::Integer(x), Number::Integer(y)) => x.into_i64() < y.into_i64(),
            (Number::Integer(x), Number::Float(y)) => (x.into_i64() as f64) < y as f64,
            (Number::Float(x), Number::Number(y)) => (x as f64) < *agent.heap.get(y),
            (Number::Float(x), Number::Integer(y)) => (x as f64) < y.into_i64() as f64,
            (Number::Float(x), Number::Float(y)) => x < y,
        })
    }

    /// 6.1.6.1.13 Number::equal ( x, y )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-equal
    pub fn equal(self, agent: &mut Agent, y: Self) -> bool {
        let x = self;

        // 1. If x is NaN, return false.
        if x.is_nan(agent) {
            return false;
        }

        // 2. If y is NaN, return false.
        if y.is_nan(agent) {
            return false;
        }

        // 3. If x is y, return true.
        if x.is(agent, y) {
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

    /// 6.1.6.1.14 Number::sameValue ( x, y )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-sameValue
    pub fn same_value(self, agent: &mut Agent, y: Self) -> bool {
        let x = self;

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
        if x.is(agent, y) {
            return true;
        }

        // 5. Return false.
        false
    }

    /// 6.1.6.1.15 Number::sameValueZero ( x, y )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-sameValueZero
    pub fn same_value_zero(self, agent: &mut Agent, y: Self) -> bool {
        let x = self;

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
        if x.is(agent, y) {
            return true;
        }

        // 5. Return false.
        false
    }

    /// 6.1.6.1.16 NumberBitwiseOp ( op, x, y )
    /// https://tc39.es/ecma262/#sec-numberbitwiseop
    pub fn bitwise_op(self, agent: &mut Agent, op: BitwiseOp, y: Self) -> JsResult<Self> {
        let x = self;

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
        Ok(Number::from(result))
    }

    /// 6.1.6.1.17 Number::bitwiseAND ( x, y )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseAND
    pub fn bitwise_and(self, agent: &mut Agent, y: Self) -> JsResult<Self> {
        let x = self;

        // 1. Return NumberBitwiseOp(&, x, y).
        x.bitwise_op(agent, BitwiseOp::And, y)
    }

    /// 6.1.6.1.18 Number::bitwiseXOR ( x, y )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseXOR
    pub fn bitwise_xor(self, agent: &mut Agent, y: Self) -> JsResult<Self> {
        let x = self;

        // 1. Return NumberBitwiseOp(^, x, y).
        x.bitwise_op(agent, BitwiseOp::Xor, y)
    }

    /// 6.1.6.1.19 Number::bitwiseOR ( x, y )
    /// https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseOR
    pub fn bitwise_or(self, agent: &mut Agent, y: Self) -> JsResult<Self> {
        let x = self;

        // 1. Return NumberBitwiseOp(|, x, y).
        x.bitwise_op(agent, BitwiseOp::Or, y)
    }

    // ...
}

#[derive(Debug, Clone, Copy)]
pub enum BitwiseOp {
    And,
    Xor,
    Or,
}
