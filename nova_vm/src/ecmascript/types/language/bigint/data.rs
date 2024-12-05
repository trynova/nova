// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cmp::Ordering, hint::unreachable_unchecked};

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};
use num_bigint::{BigInt, Sign};

#[derive(Debug, Clone)]
pub struct BigIntHeapData {
    pub(crate) data: BigInt,
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
        let mut base: f64 = match self.data.sign() {
            Sign::Minus => -1.0,
            // We should never have 0 value BigIntHeapData.
            Sign::NoSign => unreachable!(),
            Sign::Plus => 1.0,
        };
        self.data
            .iter_u64_digits()
            .enumerate()
            .for_each(|(index, digit)| {
                base += (digit as f64) * 2f64.powi(index as i32);
            });
        base == *other
    }
}

impl PartialOrd<f64> for BigIntHeapData {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        let mut base: f64 = match self.data.sign() {
            Sign::Minus => -1.0,
            // We should never have 0 value BigIntHeapData.
            Sign::NoSign => unreachable!(),
            Sign::Plus => 1.0,
        };
        self.data
            .iter_u64_digits()
            .enumerate()
            .for_each(|(index, digit)| {
                base += (digit as f64) * 2f64.powi(index as i32);
            });
        base.partial_cmp(other)
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
