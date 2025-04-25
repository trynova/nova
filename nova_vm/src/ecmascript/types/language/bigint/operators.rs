// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use num_bigint::ToBigInt;

use crate::ecmascript::execution::Agent;

use super::BigInt;

pub(crate) fn left_shift_bigint_u32<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: u32,
) -> BigInt<'gc> {
    BigInt::from_num_bigint(agent, x << y)
}

pub(crate) fn right_shift_bigint_u32<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: u32,
) -> BigInt<'gc> {
    BigInt::from_num_bigint(agent, x >> y)
}

pub(crate) fn left_shift_i64_u32<'gc>(agent: &mut Agent, x: i64, y: u32) -> BigInt<'gc> {
    if let Some(r) = x.checked_shl(y) {
        BigInt::from_i64(agent, r)
    } else {
        left_shift_bigint_u32(agent, &x.to_bigint().unwrap(), y)
    }
}

pub(crate) fn right_shift_i64_u32<'gc>(agent: &mut Agent, x: i64, y: u32) -> BigInt<'gc> {
    if let Some(r) = x.checked_shr(y) {
        BigInt::from_i64(agent, r)
    } else {
        right_shift_bigint_u32(agent, &x.to_bigint().unwrap(), y)
    }
}

pub(crate) fn left_shift_i64_i64<'gc>(agent: &mut Agent, x: i64, y: i64) -> Option<BigInt<'gc>> {
    if let Ok(y) = u32::try_from(y) {
        return Some(left_shift_i64_u32(agent, x, y));
    } else if y.is_negative() {
        if let Ok(y) = u32::try_from(-y) {
            return Some(right_shift_i64_u32(agent, x, y));
        }
    }
    None
}

pub(crate) fn right_shift_i64_i64<'gc>(agent: &mut Agent, x: i64, y: i64) -> Option<BigInt<'gc>> {
    if let Ok(y) = u32::try_from(y) {
        return Some(right_shift_i64_u32(agent, x, y));
    } else if y.is_negative() {
        if let Ok(y) = u32::try_from(-y) {
            return Some(left_shift_i64_u32(agent, x, y));
        }
    }
    None
}

pub(crate) fn left_shift_bigint_i64<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: i64,
) -> Option<BigInt<'gc>> {
    if let Ok(y) = u32::try_from(y) {
        return Some(left_shift_bigint_u32(agent, x, y));
    } else if y.is_negative() {
        if let Ok(y) = u32::try_from(-y) {
            return Some(right_shift_bigint_u32(agent, x, y));
        }
    }
    None
}

pub(crate) fn right_shift_bigint_i64<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: i64,
) -> Option<BigInt<'gc>> {
    if let Ok(y) = u32::try_from(y) {
        return Some(right_shift_bigint_u32(agent, x, y));
    } else if y.is_negative() {
        if let Ok(y) = u32::try_from(-y) {
            return Some(left_shift_bigint_u32(agent, x, y));
        }
    }
    None
}

pub(crate) fn left_shift_i64_bigint<'gc>(
    agent: &mut Agent,
    x: i64,
    y: &num_bigint::BigInt,
) -> Option<BigInt<'gc>> {
    if let Ok(y) = i64::try_from(y) {
        left_shift_i64_i64(agent, x, y)
    } else {
        None
    }
}

pub(crate) fn right_shift_i64_bigint<'gc>(
    agent: &mut Agent,
    x: i64,
    y: &num_bigint::BigInt,
) -> Option<BigInt<'gc>> {
    if let Ok(y) = i64::try_from(y) {
        right_shift_i64_i64(agent, x, y)
    } else {
        None
    }
}

pub(crate) fn left_shift_bigint_bigint<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: &num_bigint::BigInt,
) -> Option<BigInt<'gc>> {
    if let Ok(y) = i64::try_from(y) {
        left_shift_bigint_i64(agent, x, y)
    } else {
        None
    }
}

pub(crate) fn right_shift_bigint_bigint<'gc>(
    agent: &mut Agent,
    x: &num_bigint::BigInt,
    y: &num_bigint::BigInt,
) -> Option<BigInt<'gc>> {
    if let Ok(y) = i64::try_from(y) {
        right_shift_bigint_i64(agent, x, y)
    } else {
        None
    }
}
