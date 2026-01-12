// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod radix;

use super::{
    Numeric, Primitive, String, Value,
    value::{FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, NUMBER_DISCRIMINANT},
};
use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::type_conversion::{to_int32_number, to_uint32_number},
        execution::Agent,
        types::language::numeric::numeric_handle,
    },
    engine::{
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_f64::SmallF64,
    },
    heap::{
        ArenaAccess, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, NumberHeapAccess,
        WorkQueues, arena_vec_access, indexes::BaseIndex,
    },
};

pub(crate) use data::*;
use num_traits::{PrimInt, Zero};
use radix::make_float_string_ascii_lowercase;
pub(crate) use radix::with_radix;

/// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Number<'a> {
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber<'a>) = NUMBER_DISCRIMINANT,
    /// 53-bit signed integer on the stack.
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    /// 56-bit f64 on the stack. The missing byte is a zero least significant
    /// byte.
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
}
bindable_handle!(Number);

impl<'a> Number<'a> {
    /// Allocate a 64-bit floating point number onto the Agent heap
    ///
    /// # Safety
    ///
    /// The number being allocated must not be representable
    /// as a SmallInteger or f32. All stack-allocated numbers must be
    /// inequal to any heap-allocated number.
    unsafe fn alloc_number<'gc>(heap: &mut Heap, number: f64) -> HeapNumber<'gc> {
        debug_assert!(
            SmallInteger::try_from(number).is_err() && SmallF64::try_from(number).is_err()
        );
        heap.numbers.push(number.into());
        heap.alloc_counter += core::mem::size_of::<Option<NumberHeapData>>();
        HeapNumber(BaseIndex::last(&heap.numbers))
    }

    pub fn from_f64(agent: &mut Agent, value: f64, gc: NoGcScope<'a, '_>) -> Self {
        if let Ok(value) = Number::try_from(value) {
            value
        } else {
            // SAFETY: Number was not representable as a
            // stack-allocated Number.
            let id = unsafe { Self::alloc_number(&mut agent.heap, value) };
            Number::Number(id.unbind().bind(gc))
        }
    }

    pub fn from_i64(agent: &mut Agent, value: i64, gc: NoGcScope<'a, '_>) -> Self {
        if let Ok(value) = Number::try_from(value) {
            value
        } else {
            let value = value as f64;
            if let Ok(value) = SmallF64::try_from(value) {
                // Number did not fit the safe integer range but could be
                // represented as a SmallF64.
                Number::SmallF64(value)
            } else {
                // SAFETY: Number was not representable as a
                // stack-allocated Number.
                let id = unsafe { Self::alloc_number(&mut agent.heap, value) };
                Number::Number(id.unbind().bind(gc))
            }
        }
    }

    /// Create a Number from a usize.
    pub fn from_usize(agent: &mut Agent, value: usize, gc: NoGcScope<'a, '_>) -> Self {
        if let Ok(value) = Number::try_from(value) {
            value
        } else {
            let value = value as f64;
            if let Ok(value) = SmallF64::try_from(value) {
                // Number did not fit the safe integer range but could be
                // represented as a SmallF64.
                Number::SmallF64(value)
            } else {
                // SAFETY: Number was not representable as a
                // stack-allocated Number.
                let id = unsafe { Self::alloc_number(&mut agent.heap, value) };
                Number::Number(id.unbind().bind(gc))
            }
        }
    }

    pub fn nan() -> Self {
        Self::from(f32::NAN)
    }

    pub fn neg_zero() -> Self {
        Self::from(-0.0f32)
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

    #[inline(always)]
    pub fn is_nan(self, agent: &Agent) -> bool {
        self.is_nan_(agent)
    }

    pub(crate) fn is_nan_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => n.get(agent).is_nan(),
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64().is_nan(),
        }
    }

    #[inline(always)]
    pub fn is_pos_zero(self, agent: &Agent) -> bool {
        self.is_pos_zero_(agent)
    }

    pub(crate) fn is_pos_zero_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => f64::to_bits(0.0) == f64::to_bits(*n.get(agent)),
            Number::Integer(n) => 0i64 == n.into_i64(),
            Number::SmallF64(n) => n.into_f64().to_bits() == 0.0f64.to_bits(),
        }
    }

    #[inline(always)]
    pub fn is_neg_zero(self, agent: &Agent) -> bool {
        self.is_neg_zero_(agent)
    }

    pub(crate) fn is_neg_zero_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => f64::to_bits(-0.0) == f64::to_bits(*n.get(agent)),
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64().to_bits() == (-0.0f64).to_bits(),
        }
    }

    #[inline(always)]
    pub fn is_pos_infinity(self, agent: &Agent) -> bool {
        self.is_pos_infinity_(agent)
    }

    pub(crate) fn is_pos_infinity_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => *n.get(agent) == f64::INFINITY,
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64() == f64::INFINITY,
        }
    }

    #[inline(always)]
    pub fn is_neg_infinity(self, agent: &Agent) -> bool {
        self.is_neg_infinity_(agent)
    }

    pub(crate) fn is_neg_infinity_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => *n.get(agent) == f64::NEG_INFINITY,
            Number::Integer(_) => false,
            Number::SmallF64(n) => n.into_f64() == f64::NEG_INFINITY,
        }
    }

    #[inline(always)]
    pub fn is_finite(self, agent: &Agent) -> bool {
        self.is_finite_(agent)
    }

    pub(crate) fn is_finite_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => n.get(agent).is_finite(),
            Number::Integer(_) => true,
            Number::SmallF64(n) => n.into_f64().is_finite(),
        }
    }

    #[inline(always)]
    pub fn is_integer(self, agent: &Agent) -> bool {
        self.is_integer_(agent)
    }

    pub(crate) fn is_integer_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => n.get(agent).fract() == 0.0,
            Number::Integer(_) => true,
            Number::SmallF64(n) => n.into_f64().fract() == 0.0,
        }
    }

    #[inline(always)]
    pub fn is_nonzero(self, agent: &Agent) -> bool {
        self.is_nonzero_(agent)
    }

    pub(crate) fn is_nonzero_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => &0.0 != n.get(agent),
            Number::Integer(n) => 0i64 != n.into_i64(),
            Number::SmallF64(n) => !n.into_f64().is_zero(),
        }
    }

    #[inline(always)]
    pub fn is_pos_one(self, agent: &Agent) -> bool {
        self.is_pos_one_(agent)
    }

    pub(crate) fn is_pos_one_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        // NOTE: Only the integer variant should ever return true, if any other
        // variant returns true, that's a bug as it means that our variants are
        // no longer "sound".
        match self {
            Number::Integer(n) => 1i64 == n.into_i64(),
            Number::Number(n) => {
                debug_assert_ne!(*n.get(agent), 1.0);
                false
            }
            Number::SmallF64(n) => {
                debug_assert_ne!(n.into_f64(), 1.0);
                false
            }
        }
    }

    #[inline(always)]
    pub fn is_neg_one(self, agent: &Agent) -> bool {
        self.is_neg_one_(agent)
    }

    pub(crate) fn is_neg_one_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Integer(n) => -1i64 == n.into_i64(),
            Number::Number(n) => {
                debug_assert_ne!(*n.get(agent), -1.0);
                false
            }
            Number::SmallF64(n) => {
                debug_assert_ne!(n.into_f64(), -1.0);
                false
            }
        }
    }

    #[inline(always)]
    pub fn is_sign_positive(self, agent: &Agent) -> bool {
        self.is_sign_positive_(agent)
    }

    pub(crate) fn is_sign_positive_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => n.get(agent).is_sign_positive(),
            Number::Integer(n) => n.into_i64() >= 0,
            Number::SmallF64(n) => n.into_f64().is_sign_positive(),
        }
    }

    #[inline(always)]
    pub fn is_sign_negative(self, agent: &Agent) -> bool {
        self.is_sign_negative_(agent)
    }

    pub(crate) fn is_sign_negative_<T>(self, agent: &'a T) -> bool
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => n.get(agent).is_sign_negative(),
            Number::Integer(n) => n.into_i64().is_negative(),
            Number::SmallF64(n) => n.into_f64().is_sign_negative(),
        }
    }

    /// # [truncate](https://tc39.es/ecma262/#eqn-truncate)
    pub fn truncate(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Self {
        match self {
            Number::Number(n) => {
                let n = n.get(agent).trunc();
                Number::from_f64(agent, n, gc)
            }
            Number::Integer(_) => self,
            Number::SmallF64(n) => Number::from_f64(agent, n.into_f64().trunc(), gc),
        }
    }

    #[inline(always)]
    pub fn into_f64(self, agent: &Agent) -> f64 {
        self.into_f64_(agent)
    }

    pub(crate) fn into_f64_<T>(self, agent: &'a T) -> f64
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => *n.get(agent),
            Number::Integer(n) => n.into_i64() as f64,
            Number::SmallF64(n) => n.into_f64(),
        }
    }

    #[inline(always)]
    pub fn into_f32(self, agent: &Agent) -> f32 {
        self.into_f32_(agent)
    }

    pub(crate) fn into_f32_<T>(self, agent: &'a T) -> f32
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => *n.get(agent) as f32,
            Number::Integer(n) => Into::<i64>::into(n) as f32,
            Number::SmallF64(n) => n.into_f64() as f32,
        }
    }

    #[cfg(feature = "proposal-float16array")]
    #[inline(always)]
    pub fn into_f16(self, agent: &Agent) -> f16 {
        self.into_f16_(agent)
    }

    #[cfg(feature = "proposal-float16array")]
    pub(crate) fn into_f16_<T>(self, agent: &'a T) -> f16
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => *n.get(agent) as f16,
            Number::Integer(n) => Into::<i64>::into(n) as f16,
            Number::SmallF64(n) => n.into_f64() as f16,
        }
    }

    /// Returns the number cast to an [`i64`].
    ///
    /// If the number isn't representable as an i64:
    /// - NaN becomes 0.
    /// - Numbers are clamped between [`i64::MIN`] and [`i64::MAX`].
    /// - All other numbers round towards zero.
    #[inline(always)]
    pub fn into_i64(self, agent: &Agent) -> i64 {
        self.into_i64_(agent)
    }

    pub(crate) fn into_i64_<T>(self, agent: &'a T) -> i64
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => *n.get(agent) as i64,
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
    #[inline(always)]
    pub fn into_usize(self, agent: &Agent) -> usize {
        self.into_usize_(agent)
    }

    pub(crate) fn into_usize_<T>(self, agent: &'a T) -> usize
    where
        T: NumberHeapAccess,
    {
        match self {
            Number::Number(n) => *n.get(agent) as usize,
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
    fn is<T>(agent: &'a T, x: Self, y: Self) -> bool
    where
        T: NumberHeapAccess,
    {
        match (x, y) {
            // Optimisation: First compare by-reference; only read from heap if needed.
            (Number::Number(x), Number::Number(y)) => x == y || x.get(agent) == y.get(agent),
            (Number::Integer(x), Number::Integer(y)) => x == y,
            (Number::SmallF64(x), Number::SmallF64(y)) => x == y,
            (Number::Number(x), Number::Integer(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!(*x.get(agent) != y.into_i64() as f64);
                false
            }
            (Number::Number(x), Number::SmallF64(y)) => {
                // Optimisation: f32s should never be allocated into the heap
                debug_assert!(*x.get(agent) != y.into_f64());
                false
            }
            (Number::Integer(x), Number::Number(y)) => {
                // Optimisation: Integers should never be allocated into the heap as f64s.
                debug_assert!((x.into_i64() as f64) != *y.get(agent));
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
                debug_assert!(&x.into_f64() != y.get(agent));
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
            Number::Number(n) => n.get(agent).rem_euclid(2.0) == 1.0,
            Number::Integer(n) => i64::from(n).rem_euclid(2) == 1,
            Number::SmallF64(n) => n.into_f64().rem_euclid(2.0) == 1.0,
        }
    }

    pub fn abs(self, agent: &mut Agent) -> Self {
        match self {
            Number::Number(n) => {
                let n = *n.get(agent);
                if n > 0.0 { self } else { agent.heap.create(-n) }
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

    /// `x > y`
    pub fn greater_than(agent: &mut Agent, x: Self, y: Self) -> Option<bool> {
        Number::less_than(agent, x, y).map(|lt| !lt && !Number::is(agent, x, y))
    }

    /// ### [6.1.6.1.1 Number::unaryMinus ( x )](https://tc39.es/ecma262/#sec-numeric-types-number-unaryMinus)
    pub fn unary_minus(agent: &mut Agent, x: Self) -> Self {
        // 1. If x is NaN, return NaN.
        // NOTE: Computers do this automatically.

        // 2. Return the result of negating x; that is, compute a Number with the same magnitude but opposite sign.
        match x {
            Number::Number(n) => {
                let value = n.get(agent);
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
    pub fn bitwise_not(agent: &mut Agent, x: Self) -> Self {
        // 1. Let oldValue be ! ToInt32(x).
        let old_value = to_int32_number(agent, x);

        // 2. Return the result of applying bitwise complement to oldValue. The mathematical value of the result is exactly representable as a 32-bit two's complement bit string.
        Number::from(!old_value)
    }

    /// ### [6.1.6.1.3 Number::exponentiate ( base, exponent )](https://tc39.es/ecma262/#sec-numeric-types-number-exponentiate)
    pub fn exponentiate(agent: &mut Agent, base: Self, exponent: Self) -> Self {
        // 1. If exponent is NaN, return NaN.
        if exponent.is_nan_(agent) {
            return Number::nan();
        }

        // 2. If exponent is either +0𝔽 or -0𝔽, return 1𝔽.
        if exponent.is_pos_zero_(agent) || exponent.is_neg_zero_(agent) {
            return Number::from(1);
        }

        // 3. If base is NaN, return NaN.
        if base.is_nan_(agent) {
            return Number::nan();
        }

        // 4. If base is +∞𝔽, then
        if base.is_pos_infinity_(agent) {
            // a. If exponent > +0𝔽, return +∞𝔽. Otherwise, return +0𝔽.
            return if Number::greater_than(agent, exponent, Number::from(0)).unwrap_or(false) {
                Number::pos_inf()
            } else {
                Number::pos_zero()
            };
        }

        // 5. If base is -∞𝔽, then
        if base.is_neg_infinity_(agent) {
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
        if base.is_pos_zero_(agent) {
            // a. If exponent > +0𝔽, return +0𝔽. Otherwise, return +∞𝔽.
            return if Number::greater_than(agent, exponent, Number::pos_zero()).unwrap_or(false) {
                Number::pos_zero()
            } else {
                Number::pos_inf()
            };
        }

        // 7. If base is -0𝔽, then
        if base.is_neg_zero_(agent) {
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
        debug_assert!(base.is_finite_(agent) && base.is_nonzero_(agent));

        // 9. If exponent is +∞𝔽, then
        if exponent.is_pos_infinity_(agent) {
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
        if exponent.is_neg_infinity_(agent) {
            let base = base.into_f64_(agent).abs();

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
        debug_assert!(exponent.is_finite_(agent) && exponent.is_nonzero_(agent));

        // 12. If base < -0𝔽 and exponent is not an integral Number, return NaN.
        if Number::less_than(agent, base, Number::neg_zero()).unwrap_or(false)
            && !exponent.is_odd_integer(agent)
        {
            return Number::nan();
        }

        // 13. Return an implementation-approximated Number value representing the result of raising ℝ(base) to the ℝ(exponent) power.
        agent
            .heap
            .create(base.into_f64_(agent).powf(exponent.into_f64_(agent)))
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
    pub fn multiply(agent: &mut Agent, x: Self, y: Self, gc: NoGcScope<'a, '_>) -> Self {
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
                return Self::from_f64(agent, result as f64, gc);
            }
            return Self::from_f64(agent, x as f64 * y as f64, gc);
        }
        // 1. If x is NaN or y is NaN, return NaN.
        if x.is_nan_(agent) || y.is_nan_(agent) {
            return Self::nan();
        }
        // 2. If x is either +∞𝔽 or -∞𝔽, then
        if x.is_pos_infinity_(agent) || x.is_neg_infinity_(agent) {
            // a. If y is either +0𝔽 or -0𝔽, return NaN.
            if y.is_pos_zero_(agent) || y.is_neg_zero_(agent) {
                return Self::nan();
            }
            // b. If y > +0𝔽, return x.
            if y.is_sign_positive_(agent) {
                return x;
            }
            // c. Return -x.
            return if x.is_pos_infinity_(agent) {
                Self::neg_inf()
            } else {
                Self::pos_inf()
            };
        }
        // 3. If y is either +∞𝔽 or -∞𝔽, then
        if y.is_pos_infinity_(agent) || y.is_neg_infinity_(agent) {
            // a. If x is either +0𝔽 or -0𝔽, return NaN.
            if x.is_pos_zero_(agent) || x.is_neg_zero_(agent) {
                return Self::nan();
            }
            // b. If x > +0𝔽, return y.
            if x.is_sign_positive_(agent) {
                return y;
            }
            // c. Return -y.
            return if y.is_pos_infinity_(agent) {
                Self::neg_inf()
            } else {
                Self::pos_inf()
            };
        }
        // 4. If x is -0𝔽, then
        if x.is_neg_zero_(agent) {
            // a. If y is -0𝔽 or y < -0𝔽, return +0𝔽.
            if y.is_neg_zero_(agent) || y.is_sign_negative_(agent) {
                return Self::pos_zero();
            }
            // b. Else, return -0𝔽.
            return Self::neg_zero();
        }
        // 5. If y is -0𝔽, then
        if y.is_neg_zero_(agent) {
            // a. If x < -0𝔽, return +0𝔽.
            if x.is_sign_negative_(agent) {
                return Self::pos_zero();
            }
            // b. Else, return -0𝔽.
            return Self::neg_zero();
        }
        // 6. Return 𝔽(ℝ(x) × ℝ(y)).
        Self::from_f64(agent, x.to_real(agent) * y.to_real(agent), gc)
    }

    /// ### [6.1.6.1.5 Number::divide ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-divide)
    ///
    /// The abstract operation Number::divide takes arguments x (a Number) and
    /// y (a Number) and returns a Number. It performs division according to
    /// the rules of IEEE 754-2019 binary double-precision arithmetic,
    /// producing the quotient of x and y where x is the dividend and y is the
    /// divisor.
    pub fn divide(agent: &mut Agent, x: Self, y: Self, gc: NoGcScope<'a, '_>) -> Self {
        // 1. If x is NaN or y is NaN, return NaN.
        if x.is_nan_(agent) || y.is_nan_(agent) {
            return Number::nan();
        }
        // 2. If x is either +∞𝔽 or -∞𝔽, then
        if x.is_pos_infinity_(agent) || x.is_neg_infinity_(agent) {
            // a. If y is either +∞𝔽 or -∞𝔽, return NaN.
            if y.is_pos_infinity_(agent) || y.is_neg_infinity_(agent) {
                return Number::nan();
            }
            // b. If y is +0𝔽 or y > +0𝔽, return x.
            if y.is_pos_zero_(agent) || y.to_real(agent) > 0.0 {
                return x;
            }
            // c. Return -x.
            return if x.is_pos_infinity_(agent) {
                Number::neg_inf()
            } else {
                Number::pos_inf()
            };
        }
        // 3. If y is +∞𝔽, then
        if y.is_pos_infinity_(agent) {
            // a. If x is +0𝔽 or x > +0𝔽, return +0𝔽. Otherwise, return -0𝔽.
            if x.is_pos_zero_(agent) || x.to_real(agent) > 0.0 {
                return Number::pos_zero();
            } else {
                return Number::neg_zero();
            }
        }
        // 4. If y is -∞𝔽, then
        if y.is_neg_infinity_(agent) {
            // a. If x is +0𝔽 or x > +0𝔽, return -0𝔽. Otherwise, return +0𝔽.
            if x.is_pos_zero_(agent) || x.to_real(agent) > 0.0 {
                return Number::neg_zero();
            } else {
                return Number::pos_zero();
            }
        }
        // 5. If x is either +0𝔽 or -0𝔽, then
        if x.is_pos_zero_(agent) || x.is_neg_zero_(agent) {
            // a. If y is either +0𝔽 or -0𝔽, return NaN.
            if y.is_pos_zero_(agent) || y.is_neg_zero_(agent) {
                return Number::nan();
            }
            // b. If y > +0𝔽, return x.
            if y.to_real(agent) > 0.0 {
                return x;
            }
            // c. Return -x.
            return if x.is_pos_zero_(agent) {
                Number::neg_zero()
            } else {
                Number::pos_zero()
            };
        }
        // 6. If y is +0𝔽, then
        if y.is_pos_zero_(agent) {
            // a. If x > +0𝔽, return +∞𝔽. Otherwise, return -∞𝔽.
            return if x.to_real(agent) > 0.0 {
                Number::pos_inf()
            } else {
                Number::neg_inf()
            };
        }
        // 7. If y is -0𝔽, then
        if y.is_neg_zero_(agent) {
            // a. If x > +0𝔽, return -∞𝔽. Otherwise, return +∞𝔽.
            return if x.to_real(agent) > 0.0 {
                Number::neg_inf()
            } else {
                Number::pos_inf()
            };
        }
        // 8. Return 𝔽(ℝ(x) / ℝ(y)).
        let result = x.to_real(agent) / y.to_real(agent);
        Number::from_f64(agent, result, gc)
    }

    /// ### [6.1.6.1.6 Number::remainder ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-remainder)
    ///
    /// The abstract operation Number::remainder takes arguments n (a Number)
    /// and d (a Number) and returns a Number. It yields the remainder from an
    /// implied division of its operands where n is the dividend and d is the
    /// divisor.
    pub fn remainder(agent: &mut Agent, n: Self, d: Self, gc: NoGcScope<'a, '_>) -> Self {
        // 1. If n is NaN or d is NaN, return NaN.
        if n.is_nan_(agent) || d.is_nan_(agent) {
            return Self::nan();
        }

        // 2. If n is either +∞𝔽 or -∞𝔽, return NaN.
        if n.is_pos_infinity_(agent) || n.is_neg_infinity_(agent) {
            return Self::nan();
        }

        // 3. If d is either +∞𝔽 or -∞𝔽, return n.
        if d.is_pos_infinity_(agent) || d.is_neg_infinity_(agent) {
            return n;
        }

        // 4. If d is either +0𝔽 or -0𝔽, return NaN.
        if d.is_pos_zero_(agent) || d.is_neg_zero_(agent) {
            return Self::nan();
        }

        // 5. If n is either +0𝔽 or -0𝔽, return n.
        if n.is_pos_zero_(agent) || n.is_neg_zero_(agent) {
            return n;
        }

        // 6. Assert: n and d are finite and non-zero.
        debug_assert!(n.is_finite_(agent) && n.is_nonzero_(agent));

        let n = n.into_f64_(agent);
        let d = d.into_f64_(agent);

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
        Self::from_f64(agent, r, gc)
    }

    /// ### [6.1.6.1.7 Number::add ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-add)
    ///
    /// The abstract operation Number::add takes arguments x (a Number) and y
    /// (a Number) and returns a Number. It performs addition according to the
    /// rules of IEEE 754-2019 binary double-precision arithmetic, producing
    /// the sum of its arguments.
    pub(crate) fn add(agent: &mut Agent, x: Self, y: Self) -> Self {
        // 1. If x is NaN or y is NaN, return NaN.
        if x.is_nan_(agent) || y.is_nan_(agent) {
            return Number::nan();
        }

        // 2. If x is +∞𝔽 and y is -∞𝔽, return NaN.
        if x.is_pos_infinity_(agent) && y.is_neg_infinity_(agent) {
            return Number::nan();
        }

        // 3. If x is -∞𝔽 and y is +∞𝔽, return NaN.
        if x.is_neg_infinity_(agent) && y.is_pos_infinity_(agent) {
            return Number::nan();
        }

        // 4. If x is either +∞𝔽 or -∞𝔽, return x.
        if x.is_pos_infinity_(agent) || x.is_neg_infinity_(agent) {
            return x;
        }

        // 5. If y is either +∞𝔽 or -∞𝔽, return y.
        if y.is_pos_infinity_(agent) || y.is_neg_infinity_(agent) {
            return y;
        }

        // 6. Assert: x and y are both finite.
        debug_assert!(x.is_finite_(agent) && y.is_finite_(agent));

        // 7. If x is -0𝔽 and y is -0𝔽, return -0𝔽.
        if x.is_neg_zero_(agent) && y.is_neg_zero_(agent) {
            return Number::neg_zero();
        }

        // 8. Return 𝔽(ℝ(x) + ℝ(y)).
        agent.heap.create(x.into_f64_(agent) + y.into_f64_(agent))
    }

    /// ### [6.1.6.1.8 Number::subtract ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-subtract)
    ///
    /// The abstract operation Number::subtract takes arguments x (a Number)
    /// and y (a Number) and returns a Number. It performs subtraction,
    /// producing the difference of its operands; x is the minuend and y is the
    /// subtrahend.
    pub(crate) fn subtract(agent: &mut Agent, x: Self, y: Self) -> Self {
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
        let lnum = to_int32_number(agent, x);
        // 2. Let rnum be ! ToUint32(y).
        let rnum = to_uint32_number(agent, y);
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
        let lnum = to_int32_number(agent, x);
        // 2. Let rnum be ! ToUint32(y).
        let rnum = to_uint32_number(agent, y);
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
        let lnum = to_uint32_number(agent, x);
        // 2. Let rnum be ! ToUint32(y).
        let rnum = to_uint32_number(agent, y);
        // 3. Let shiftCount be ℝ(rnum) modulo 32.
        let shift_count = rnum % 32;
        // 4. Return the result of performing a zero-filling right shift of lnum by shiftCount bits. Vacated bits are filled with zero. The mathematical value of the result is exactly representable as a 32-bit unsigned bit string.
        Number::from(lnum.unsigned_shr(shift_count))
    }

    /// ### [6.1.6.1.12 Number::lessThan ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-lessThan)
    pub fn less_than(agent: &Agent, x: Self, y: Self) -> Option<bool> {
        // 1. If x is NaN, return undefined.
        if x.is_nan_(agent) {
            return None;
        }

        // 2. If y is NaN, return undefined.
        if y.is_nan_(agent) {
            return None;
        }

        // 3. If x is y, return false.
        if Number::is(agent, x, y) {
            return Some(false);
        }

        // 4. If x is +0𝔽 and y is -0𝔽, return false.
        if x.is_pos_zero_(agent) && y.is_neg_zero_(agent) {
            return Some(false);
        }

        // 5. If x is -0𝔽 and y is +0𝔽, return false.
        if x.is_neg_zero_(agent) && y.is_pos_zero_(agent) {
            return Some(false);
        }

        // 6. If x is +∞𝔽, return false.
        if x.is_pos_infinity_(agent) {
            return Some(false);
        }

        // 7. If y is +∞𝔽, return true.
        if y.is_pos_infinity_(agent) {
            return Some(true);
        }

        // 8. If y is -∞𝔽, return false.
        if y.is_neg_infinity_(agent) {
            return Some(false);
        }

        // 9. If x is -∞𝔽, return true.
        if x.is_neg_infinity_(agent) {
            return Some(true);
        }

        // 10. Assert: x and y are finite.
        assert!(x.is_finite_(agent) && y.is_finite_(agent));

        // 11. If ℝ(x) < ℝ(y), return true. Otherwise, return false.
        Some(match (x, y) {
            (Number::Number(x), Number::Number(y)) => x.get(agent) < y.get(agent),
            (Number::Number(x), Number::Integer(y)) => x.get(agent) < &(y.into_i64() as f64),
            (Number::Number(x), Number::SmallF64(y)) => x.get(agent) < &y.into_f64(),
            (Number::Integer(x), Number::Number(y)) => &(x.into_i64() as f64) < y.get(agent),
            (Number::Integer(x), Number::Integer(y)) => x.into_i64() < y.into_i64(),
            (Number::Integer(x), Number::SmallF64(y)) => (x.into_i64() as f64) < y.into_f64(),
            (Number::SmallF64(x), Number::Number(y)) => &x.into_f64() < y.get(agent),
            (Number::SmallF64(x), Number::Integer(y)) => x.into_f64() < y.into_i64() as f64,
            (Number::SmallF64(x), Number::SmallF64(y)) => x.into_f64() < y.into_f64(),
        })
    }

    /// ### [6.1.6.1.13 Number::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-equal)
    #[inline(always)]
    pub fn equal<T>(agent: &'a Agent, x: Self, y: Self) -> bool {
        Self::equal_(agent, x, y)
    }

    pub(crate) fn equal_<T>(agent: &'a T, x: Self, y: Self) -> bool
    where
        T: NumberHeapAccess,
    {
        // 1. If x is NaN, return false.
        if x.is_nan_(agent) {
            return false;
        }

        // 2. If y is NaN, return false.
        if y.is_nan_(agent) {
            return false;
        }

        // 3. If x is y, return true.
        if Number::is(agent, x, y) {
            return true;
        }

        // 4. If x is +0𝔽 and y is -0𝔽, return true.
        if x.is_pos_zero_(agent) && y.is_neg_zero_(agent) {
            return true;
        }

        // 5. If x is -0𝔽 and y is +0𝔽, return true.
        if x.is_neg_zero_(agent) && y.is_pos_zero_(agent) {
            return true;
        }

        // 6. Return false.
        false
    }

    /// ### [6.1.6.1.14 Number::sameValue ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-sameValue)
    #[inline(always)]
    pub fn same_value<T>(agent: &'a Agent, x: Self, y: Self) -> bool {
        Self::same_value_(agent, x, y)
    }

    pub(crate) fn same_value_<T>(agent: &'a T, x: Self, y: Self) -> bool
    where
        T: NumberHeapAccess,
    {
        // 1. If x is NaN and y is NaN, return true.
        if x.is_nan_(agent) && y.is_nan_(agent) {
            return true;
        }

        // 2. If x is +0𝔽 and y is -0𝔽, return false.
        if x.is_pos_zero_(agent) && y.is_neg_zero_(agent) {
            return false;
        }

        // 3. If x is -0𝔽 and y is +0𝔽, return false.
        if x.is_neg_zero_(agent) && y.is_pos_zero_(agent) {
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
    #[inline(always)]
    pub fn same_value_zero<T>(agent: &'a Agent, x: Self, y: Self) -> bool {
        Self::same_value_zero_(agent, x, y)
    }

    pub(crate) fn same_value_zero_<T>(agent: &'a T, x: Self, y: Self) -> bool
    where
        T: NumberHeapAccess,
    {
        // 1. If x is NaN and y is NaN, return true.
        if x.is_nan_(agent) && y.is_nan_(agent) {
            return true;
        }

        // 2. If x is +0𝔽 and y is -0𝔽, return true.
        if x.is_pos_zero_(agent) && y.is_neg_zero_(agent) {
            return true;
        }

        // 3. If x is -0𝔽 and y is +0𝔽, return true.
        if x.is_neg_zero_(agent) && y.is_pos_zero_(agent) {
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
    fn bitwise_op(agent: &mut Agent, op: BitwiseOp, x: Self, y: Self) -> i32 {
        // 1. Let lnum be ! ToInt32(x).
        let lnum = to_int32_number(agent, x);

        // 2. Let rnum be ! ToInt32(y).
        let rnum = to_int32_number(agent, y);

        // 3. Let lbits be the 32-bit two's complement bit string representing ℝ(lnum).
        let lbits = lnum;

        // 4. Let rbits be the 32-bit two's complement bit string representing ℝ(rnum).
        let rbits = rnum;

        // 8. Return the Number value for the integer represented by the 32-bit two's complement bit string result.
        match op {
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
        }
    }

    /// ### [6.1.6.1.17 Number::bitwiseAND ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseAND)
    pub fn bitwise_and(agent: &mut Agent, x: Self, y: Self) -> i32 {
        // 1. Return NumberBitwiseOp(&, x, y).
        Number::bitwise_op(agent, BitwiseOp::And, x, y)
    }

    /// ### [6.1.6.1.18 Number::bitwiseXOR ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseXOR)
    pub fn bitwise_xor(agent: &mut Agent, x: Self, y: Self) -> i32 {
        // 1. Return NumberBitwiseOp(^, x, y).
        Number::bitwise_op(agent, BitwiseOp::Xor, x, y)
    }

    /// ### [6.1.6.1.19 Number::bitwiseOR ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-number-bitwiseOR)
    pub fn bitwise_or(agent: &mut Agent, x: Self, y: Self) -> i32 {
        // 1. Return NumberBitwiseOp(|, x, y).
        Number::bitwise_op(agent, BitwiseOp::Or, x, y)
    }

    // ### [6.1.6.1.20 Number::toString ( x, radix )](https://tc39.es/ecma262/#sec-numeric-types-number-tostring)
    pub(crate) fn to_string_radix_n<'gc>(
        agent: &mut Agent,
        x: Self,
        radix: u32,
        gc: NoGcScope<'gc, '_>,
    ) -> String<'gc> {
        String::from_string(
            agent,
            with_radix!(
                radix,
                match x {
                    Number::Integer(x) => {
                        lexical::to_string_with_options::<_, RADIX>(
                            x.into_i64(),
                            &lexical::write_integer_options::STANDARD,
                        )
                        .to_ascii_lowercase()
                    }
                    Number::Number(x) => {
                        let x = *x.get(agent);
                        let mut string = lexical::to_string_with_options::<_, RADIX>(
                            x,
                            &lexical::write_float_options::JAVASCRIPT_LITERAL,
                        );
                        make_float_string_ascii_lowercase(&mut string);
                        string
                    }
                    Number::SmallF64(x) => {
                        let mut string = lexical::to_string_with_options::<_, RADIX>(
                            x.into_f64(),
                            &lexical::write_float_options::JAVASCRIPT_LITERAL,
                        );
                        make_float_string_ascii_lowercase(&mut string);
                        string
                    }
                }
            ),
            gc,
        )
    }

    // ### [6.1.6.1.20 Number::toString ( x, radix )](https://tc39.es/ecma262/#sec-numeric-types-number-tostring)
    pub(crate) fn to_string_radix_10<'gc>(
        agent: &mut Agent,
        x: Self,
        gc: NoGcScope<'gc, '_>,
    ) -> String<'gc> {
        match x {
            Number::Number(x) => {
                let mut buffer = ryu_js::Buffer::new();
                String::from_string(agent, buffer.format(*x.get(agent)).to_string(), gc)
            }
            Number::Integer(x) => String::from_string(agent, x.into_i64().to_string(), gc),
            Number::SmallF64(x) => {
                let mut buffer = ryu_js::Buffer::new();
                String::from_string(agent, buffer.format(x.into_f64()).to_string(), gc)
            }
        }
    }

    /// # [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub(crate) fn to_real<T>(self, agent: &'a T) -> f64
    where
        T: NumberHeapAccess,
    {
        match self {
            Self::Number(n) => *n.get(agent),
            Self::Integer(i) => i.into_i64() as f64,
            Self::SmallF64(f) => f.into_f64(),
        }
    }
}

impl core::fmt::Debug for Number<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self {
            Number::Number(idx) => write!(f, "Number({idx:?})"),
            Number::Integer(value) => write!(f, "{}", value.into_i64()),
            Number::SmallF64(value) => write!(f, "{}", value.into_f64()),
        }
    }
}

#[cfg(feature = "proposal-float16array")]
impl From<f16> for Number<'_> {
    fn from(value: f16) -> Self {
        if value.is_finite()
            && value.trunc() == value
            && !(value.is_sign_negative() && value == 0.0)
        {
            let int = value as i64;
            if let Ok(int) = SmallInteger::try_from(int) {
                Number::Integer(int)
            } else {
                Number::SmallF64(value.into())
            }
        } else {
            Number::SmallF64(SmallF64::from(value))
        }
    }
}

impl From<f32> for Number<'_> {
    fn from(value: f32) -> Self {
        if value.is_finite()
            && value.trunc() == value
            && !(value.is_sign_negative() && value == 0.0)
        {
            let int = value as i64;
            if let Ok(int) = SmallInteger::try_from(int) {
                Number::Integer(int)
            } else {
                Number::SmallF64(value.into())
            }
        } else {
            Number::SmallF64(SmallF64::from(value))
        }
    }
}

const MAX_NUMBER: f64 = ((1u64 << 53) - 1) as f64;
const MIN_NUMBER: f64 = -MAX_NUMBER;

impl TryFrom<i64> for Number<'static> {
    type Error = ();

    fn try_from(value: i64) -> Result<Self, ()> {
        Ok(Number::Integer(SmallInteger::try_from(value)?))
    }
}

impl TryFrom<u64> for Number<'static> {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, ()> {
        Ok(Number::Integer(SmallInteger::try_from(value)?))
    }
}

impl TryFrom<usize> for Number<'static> {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, ()> {
        if let Ok(i64) = i64::try_from(value) {
            Self::try_from(i64)
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for Number<'static> {
    type Error = ();

    fn try_from(value: f64) -> Result<Self, ()> {
        if value.is_finite()
            && value.trunc() == value
            && (MIN_NUMBER..=MAX_NUMBER).contains(&value)
            && !(value.is_zero() && value.is_sign_negative())
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

impl<'a> From<Number<'a>> for Numeric<'a> {
    fn from(value: Number<'a>) -> Self {
        match value {
            Number::Number(data) => Self::Number(data),
            Number::Integer(data) => Self::Integer(data),
            Number::SmallF64(data) => Self::SmallF64(data),
        }
    }
}

impl<'a> From<Number<'a>> for Primitive<'a> {
    #[inline]
    fn from(value: Number<'a>) -> Self {
        match value {
            Number::Number(d) => Self::Number(d),
            Number::Integer(d) => Self::Integer(d),
            Number::SmallF64(d) => Self::SmallF64(d),
        }
    }
}

impl<'a> From<Number<'a>> for Value<'a> {
    fn from(value: Number<'a>) -> Self {
        match value {
            Number::Number(data) => Self::Number(data),
            Number::Integer(data) => Self::Integer(data),
            Number::SmallF64(data) => Self::SmallF64(data),
        }
    }
}

impl<'a> TryFrom<Numeric<'a>> for Number<'a> {
    type Error = ();
    fn try_from(value: Numeric<'a>) -> Result<Self, Self::Error> {
        match value {
            Numeric::Number(data) => Ok(Number::Number(data)),
            Numeric::Integer(data) => Ok(Number::Integer(data)),
            Numeric::SmallF64(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Primitive<'a>> for Number<'a> {
    type Error = ();
    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        match value {
            Primitive::Number(data) => Ok(Number::Number(data)),
            Primitive::Integer(data) => Ok(Number::Integer(data)),
            Primitive::SmallF64(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for Number<'a> {
    type Error = ();
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Number(data) => Ok(Number::Number(data)),
            Value::Integer(data) => Ok(Number::Integer(data)),
            Value::SmallF64(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for Number<'_> {
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

// impl<'n> HeapAccess<Agent> for Number<'n> {
//     type OutputRef<'a>
//         = &'a f64
//     where
//         Self: 'a,
//         Agent: 'a;

//     type OutputMut<'a>
//         = &'a mut f64
//     where
//         Self: 'a,
//         Agent: 'a;

//     fn get<'a>(self, source: &'a Agent) -> Self::OutputRef<'a> {
//         todo!()
//     }

//     fn get_mut<'a>(self, source: &'a mut Agent) -> Self::OutputMut<'a> {
//         todo!()
//     }
// }

impl<'a> CreateHeapData<f64, Number<'a>> for Heap {
    fn create(&mut self, data: f64) -> Number<'a> {
        // NOTE: This function cannot currently be implemented
        // directly using `Number::from_f64` as it takes an Agent
        // parameter that we do not have access to here.
        if let Ok(value) = Number::try_from(data) {
            value
        } else {
            // SAFETY: Number was not representable as a
            // stack-allocated Number.
            let heap_number = unsafe { Number::alloc_number(self, data) };
            Number::Number(heap_number)
        }
    }
}

impl HeapMarkAndSweep for Number<'static> {
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

impl HeapMarkAndSweep for HeapNumber<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.numbers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.numbers.shift_index(&mut self.0);
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum NumberRootRepr {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl Rootable for Number<'_> {
    type RootRepr = NumberRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Number(d) => Err(HeapRootData::Number(d.unbind())),
            Self::Integer(d) => Ok(Self::RootRepr::Integer(d)),
            Self::SmallF64(d) => Ok(Self::RootRepr::SmallF64(d)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Integer(d) => Ok(Self::Integer(d)),
            Self::RootRepr::SmallF64(d) => Ok(Self::SmallF64(d)),
            Self::RootRepr::HeapRef(d) => Err(d),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Number(d) => Some(Self::Number(d)),
            _ => None,
        }
    }
}

/// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
///
/// Heap-allocated [Number] data. Accessing the data must be done through the
/// [Agent].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HeapNumber<'a>(BaseIndex<'a, NumberHeapData>);
numeric_handle!(HeapNumber, Number);
arena_vec_access!(HeapNumber, NumberHeapData, numbers, f64);

impl<'a> From<HeapNumber<'a>> for Number<'a> {
    fn from(value: HeapNumber<'a>) -> Self {
        Number::Number(value)
    }
}

impl<'a> TryFrom<Number<'a>> for HeapNumber<'a> {
    type Error = ();

    fn try_from(value: Number<'a>) -> Result<Self, Self::Error> {
        match value {
            Number::Number(s) => Ok(s),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BitwiseOp {
    And,
    Xor,
    Or,
}

#[cfg(test)]
mod tests {
    use super::Number;
    use crate::{
        ecmascript::execution::{
            Agent,
            agent::{HostHooks, Job, Options},
        },
        engine::context::GcScope,
    };

    #[derive(Default, Debug)]
    struct TestAgentHooks;

    impl HostHooks for TestAgentHooks {
        fn enqueue_generic_job(&self, _job: Job) {
            // No-op
        }

        fn enqueue_promise_job(&self, _job: Job) {
            // no-op
        }

        fn enqueue_timeout_job(&self, _timeout_job: Job, _milliseconds: u64) {
            // no-op
        }
    }

    #[test]
    fn test_greater_than() {
        let hooks = Box::leak(Box::new(TestAgentHooks));
        let mut agent = Agent::new(Options::default(), hooks);
        let (mut token, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut token, &mut scope);

        let x = Number::from_f64(&mut agent, 1.0, gc.nogc());
        let y = Number::from_f64(&mut agent, 2.0, gc.nogc());

        assert_eq!(Number::greater_than(&mut agent, x, y), Some(false));
        assert_eq!(Number::greater_than(&mut agent, y, x), Some(true));

        assert_eq!(Number::greater_than(&mut agent, x, x), Some(false));
        agent.gc(gc);
    }
}

macro_rules! number_value {
    ($name: tt) => {
        crate::ecmascript::types::number_value!($name, $name);
    };
    ($name: ident, $variant: ident) => {
        crate::ecmascript::types::numeric_value!($name, $variant);

        impl From<$name> for crate::ecmascript::types::Number<'static> {
            #[inline(always)]
            fn from(value: $name) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<crate::ecmascript::types::Number<'_>> for $name {
            type Error = ();

            #[inline]
            fn try_from(value: crate::ecmascript::types::Number) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::types::Number::$variant(data) => Ok(data),
                    _ => Err(()),
                }
            }
        }
    };
}
pub(crate) use number_value;
