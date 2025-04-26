// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[macro_export]
macro_rules! bigint_bitwise_op {
    ($agent:ident, $x:ident, $y:ident, $op:expr) => {
        match ($x, $y) {
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                BigInt::from_num_bigint($agent, $op(&$agent[x].data, &$agent[y].data))
            }
            (BigInt::BigInt(x), BigInt::SmallBigInt(y))
            | (BigInt::SmallBigInt(y), BigInt::BigInt(x)) => {
                let x = &$agent[x].data;
                let sign = x.sign();
                // Take the least significant digit
                let x = x.iter_u64_digits().next().unwrap_or(0) as i128;
                // Possibly flip the sign
                let x = if sign == Sign::Minus { -x } else { x };
                // Try to not have to allocate a bigint
                let result = $op(x, y.into_i64() as i128);
                if let Ok(result) = i64::try_from(result) {
                    BigInt::from_i64($agent, result)
                } else {
                    BigInt::from_num_bigint($agent, result.to_bigint().unwrap())
                }
            }
            (BigInt::SmallBigInt(x), BigInt::SmallBigInt(y)) => {
                BigInt::from_i64($agent, $op(x.into_i64(), y.into_i64()))
            }
        }
    };
}
