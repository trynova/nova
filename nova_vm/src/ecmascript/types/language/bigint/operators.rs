// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::BigInt;
use crate::ecmascript::execution::Agent;
use num_bigint::ToBigInt;

fn left_shift_bigint_u32<'gc>(agent: &mut Agent, x: &num_bigint::BigInt, y: u32) -> BigInt<'gc> {
    BigInt::from_num_bigint(agent, x << y)
}

fn right_shift_bigint_u32<'gc>(agent: &mut Agent, x: &num_bigint::BigInt, y: u32) -> BigInt<'gc> {
    BigInt::from_num_bigint(agent, x >> y)
}

fn left_shift_i64_u32<'gc>(agent: &mut Agent, x: i64, y: u32) -> BigInt<'gc> {
    if let Some(r) = x.checked_shl(y) {
        BigInt::from_i64(agent, r)
    } else {
        left_shift_bigint_u32(agent, &x.to_bigint().unwrap(), y)
    }
}

fn right_shift_i64_u32<'gc>(agent: &mut Agent, x: i64, y: u32) -> BigInt<'gc> {
    if let Some(r) = x.checked_shr(y) {
        BigInt::from_i64(agent, r)
    } else {
        right_shift_bigint_u32(agent, &x.to_bigint().unwrap(), y)
    }
}

/// Attempts to convert `rhs` into a valid shifting value
/// within the range ±u32::MAX else returning none.
fn to_shift_rhs_operand(rhs: impl TryInto<i64>) -> Option<i64> {
    if let Ok(rhs) = rhs.try_into() {
        // Makes sure the rhs operand is within ±u32::MAX, anything outside of
        // that is just unrealistic to deal with as a bigint and would require
        // ~4gb of memory so this is where we draw the line.
        if (-(u32::MAX as i64)..=(u32::MAX as i64)).contains(&rhs) {
            Some(rhs)
        } else {
            None
        }
    } else {
        // Any bigint which can't be represented as an i64 is going to be too
        // large to use as the rhs operand of a right shift.
        None
    }
}

pub(crate) fn left_shift_i64<'gc>(
    agent: &mut Agent,
    x: i64,
    y: impl TryInto<i64>,
) -> Option<BigInt<'gc>> {
    to_shift_rhs_operand(y).map(|y| {
        // SAFETY: We know that y is within the range ±u32::MAX
        match (y.unsigned_abs() as u32, y.is_negative()) {
            (y, false) => left_shift_i64_u32(agent, x, y),
            // A negative rhs operand when doing a left shift means is the same as
            // right shifting by the negation of that amount.
            (y, true) => right_shift_i64_u32(agent, x, y),
        }
    })
}

pub(crate) fn right_shift_i64<'gc>(
    agent: &mut Agent,
    x: i64,
    y: impl TryInto<i64>,
) -> Option<BigInt<'gc>> {
    to_shift_rhs_operand(y).map(|y| {
        // SAFETY: We know that y is within the range ±u32::MAX
        match (y.unsigned_abs() as u32, y.is_negative()) {
            (y, false) => right_shift_i64_u32(agent, x, y),
            // A negative rhs operand when doing a right shift means is the same as
            // left shifting by the negation of that amount.
            (y, true) => left_shift_i64_u32(agent, x, y),
        }
    })
}

pub(crate) fn left_shift_bigint<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: impl TryInto<i64>,
) -> Option<BigInt<'gc>> {
    to_shift_rhs_operand(y).map(|y| {
        // SAFETY: We know that y is within the range ±u32::MAX
        match (y.unsigned_abs() as u32, y.is_negative()) {
            (y, false) => left_shift_bigint_u32(agent, x, y),
            // A negative rhs operand when doing a left shift means is the same as
            // right shifting by the negation of that amount.
            (y, true) => right_shift_bigint_u32(agent, x, y),
        }
    })
}

pub(crate) fn right_shift_bigint<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: impl TryInto<i64>,
) -> Option<BigInt<'gc>> {
    to_shift_rhs_operand(y).map(|y| {
        // SAFETY: We know that y is within the range ±u32::MAX
        match (y.unsigned_abs() as u32, y.is_negative()) {
            (y, false) => right_shift_bigint_u32(agent, x, y),
            // A negative rhs operand when doing a right shift means is the same as
            // left shifting by the negation of that amount.
            (y, true) => left_shift_bigint_u32(agent, x, y),
        }
    })
}
