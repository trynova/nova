// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{cmp::Ordering, hint::unreachable_unchecked};
use std::ops::{Deref, DerefMut};

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};
use num_bigint::{BigInt, Sign};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct BigIntHeapData {
    pub(crate) data: BigInt,
}

impl Deref for BigIntHeapData {
    type Target = BigInt;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for BigIntHeapData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// Convert f64 to exact BigInt for precise comparison.
/// f64 (IEEE 754 double): 1 sign bit, 11 exponent bits, 52 mantissa bits.
fn f64_to_exact_bigint(value: f64) -> BigInt {
    if value == 0.0 {
        return BigInt::from(0);
    }

    let bits = value.to_bits();
    let sign_bit = bits >> 63;
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let mantissa_bits = bits & 0xfffffffffffff;

    // Actual exponent (subtract bias of 1023)
    let exponent = exponent_bits - 1023;
    // Mantissa with implicit leading 1 bit
    let mantissa = mantissa_bits | 0x10000000000000u64;

    // The value is mantissa * 2^(exponent - 52)
    let shift = exponent - 52;

    let result = if shift >= 0 {
        BigInt::from(mantissa) << (shift as usize)
    } else {
        BigInt::from(mantissa) >> ((-shift) as usize)
    };

    if sign_bit == 1 { -result } else { result }
}

impl HeapMarkAndSweep for BigIntHeapData {
    #[inline(always)]
    fn mark_values(&self, _queues: &mut WorkQueues) {
        let Self { data: _ } = self;
    }

    #[inline(always)]
    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        let Self { data: _ } = self;
    }
}

impl PartialEq<f64> for BigIntHeapData {
    fn eq(&self, other: &f64) -> bool {
        if other.trunc() != *other {
            // Cannot be equal to non-integer.
            return false;
        }
        self.data == f64_to_exact_bigint(*other)
    }
}

impl PartialOrd<f64> for BigIntHeapData {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        if other.is_nan() {
            return None;
        }
        if *other == f64::INFINITY {
            return Some(Ordering::Less);
        }
        if *other == f64::NEG_INFINITY {
            return Some(Ordering::Greater);
        }
        // For non-integer f64, compare with truncated value and adjust
        if other.trunc() != *other {
            let truncated = f64_to_exact_bigint(other.trunc());
            return match self.data.cmp(&truncated) {
                Ordering::Equal => {
                    // BigInt equals truncated value, so compare with fractional part
                    if *other > 0.0 {
                        Some(Ordering::Less) // e.g., 5n < 5.5
                    } else {
                        Some(Ordering::Greater) // e.g., -5n > -5.5
                    }
                }
                ord => Some(ord),
            };
        }
        Some(self.data.cmp(&f64_to_exact_bigint(*other)))
    }
}

impl PartialEq<i64> for BigIntHeapData {
    fn eq(&self, other: &i64) -> bool {
        if *other == 0 {
            // We should never have 0 value BigIntHeapData.
            return false;
        }
        let sign = self.data.sign();
        let other_is_negative = other.is_negative();
        match (sign, other_is_negative) {
            (Sign::Minus, false) | (Sign::Plus, true) => {
                // Signs disagree.
                return false;
            }
            (Sign::Minus, true) | (Sign::Plus, false) => {
                // Signs agree.
            }
            // We should never have 0 value BigIntHeapData.
            (Sign::NoSign, _) => unreachable!(),
        }
        // We've checked our sign already, we don't need it anymore.
        let digits_iter = self.data.iter_u64_digits();
        if digits_iter.len() > 1 {
            // i64 cannot show data this large.
            return false;
        }
        let other = other.unsigned_abs();
        let data = self.data.iter_u64_digits().next().unwrap();
        data == other
    }
}

impl PartialOrd<i64> for BigIntHeapData {
    fn partial_cmp(&self, other: &i64) -> Option<Ordering> {
        let sign = match (self.data.sign(), other.is_negative()) {
            (Sign::Minus, false) => {
                // self < 0 && other >= 0
                return Some(Ordering::Less);
            }
            (Sign::Plus, true) => {
                // self > 0 && other < 0
                return Some(Ordering::Greater);
            }
            (Sign::Minus, true) | (Sign::Plus, false) => {
                // self < 0 && other < 0 || self > 0 && other
                self.data.sign()
            }
            // We should never have 0 value BigIntHeapData.
            (Sign::NoSign, true) | (Sign::NoSign, false) => unreachable!(),
        };
        // We've checked our sign already, we don't need it anymore.
        let other = other.abs();
        let digits_iter = self.data.iter_u64_digits();
        if digits_iter.len() > 1 {
            // i64 cannot show data this large.
            return match sign {
                Sign::Minus => Some(Ordering::Less),
                Sign::Plus => Some(Ordering::Greater),
                // SAFETY: We would have already hit the unreachable above.
                Sign::NoSign => unsafe { unreachable_unchecked() },
            };
        }
        let other = other.unsigned_abs();
        let data = self.data.iter_u64_digits().next().unwrap();
        data.partial_cmp(&other)
    }
}
