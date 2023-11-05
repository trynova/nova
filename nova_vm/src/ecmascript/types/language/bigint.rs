mod data;

use super::{
    value::{BIGINT_DISCRIMINANT, BIGINT_I56_DISCRIMINANT},
    Value,
};
use crate::{
    ecmascript::execution::{agent::ExceptionType, Agent, JsResult},
    heap::{indexes::BigIntIndex, CreateHeapData, GetHeapData},
    I56,
};

pub use data::BigIntHeapData;

/// [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
///
/// The BigInt type represents an integer value. The value may be any size and
/// is not limited to a particular bit-width. Generally, where not otherwise
/// noted, operations are designed to return exact mathematically-based answers.
/// For binary operations, BigInts act as two's complement binary strings, with
/// negative numbers treated as having bits set infinitely to the left.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum BigInt {
    BigInt(BigIntIndex) = BIGINT_DISCRIMINANT,
    BigIntI56(I56) = BIGINT_I56_DISCRIMINANT,
}

impl BigInt {
    /// ### [6.1.6.2.1 BigInt::unaryMinus ( x )](https://tc39.es/ecma262/#sec-numeric-types-bigint-unaryMinus)
    ///
    /// The abstract operation BigInt::unaryMinus takes argument x (a BigInt)
    /// and returns a BigInt.
    pub(crate) fn unary_minus(agent: &mut Agent, x: BigInt) -> BigInt {
        // 1. If x is 0ℤ, return 0ℤ.
        // NOTE: This is handled with the negation below.

        // 2. Return -x.
        match x {
            BigInt::BigIntI56(x) => {
                // We need to check if the negation will overflow.
                if x.into_i64() != I56::MAX_BIGINT {
                    BigInt::BigIntI56(-x)
                } else {
                    agent.heap.create(BigIntHeapData {
                        data: -num_bigint_dig::BigInt::from(x.into_i64()),
                    })
                }
            }
            BigInt::BigInt(x_index) => {
                let x_data = agent.heap.get(x_index);
                agent.heap.create(BigIntHeapData {
                    data: -&x_data.data,
                })
            }
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
            BigInt::BigIntI56(x) => BigInt::BigIntI56(!x),
            BigInt::BigInt(x_index) => {
                let x_data = agent.heap.get(x_index);
                agent.heap.create(BigIntHeapData {
                    data: !&x_data.data,
                })
            }
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
            BigInt::BigIntI56(x) if x.into_i64() < 0 => true,
            BigInt::BigInt(x) => agent.heap.get(x).data < 0.into(),
            _ => false,
        } {
            return Err(
                agent.throw_exception(ExceptionType::RangeError, "exponent must be positive")
            );
        }

        // TODO: 2. If base is 0ℤ and exponent is 0ℤ, return 1ℤ.
        // TODO: 3. Return base raised to the power exponent.
        // NOTE: The BigInt implementation does not support native
        // exponentiation.

        Err(agent.throw_exception(ExceptionType::EvalError, "Unsupported operation."))
    }

    /// ### [6.1.6.2.4 BigInt::multiply ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-multiply)
    ///
    /// The abstract operation BigInt::multiply takes arguments x (a BigInt) and
    /// y (a BigInt) and returns a BigInt.
    pub(crate) fn multiply(agent: &mut Agent, x: BigInt, y: BigInt) -> BigInt {
        match (x, y) {
            (BigInt::BigIntI56(x), BigInt::BigIntI56(y)) => {
                let (x, y) = (x.into_i64() as i128, y.into_i64() as i128);
                let result = x * y;

                if let Ok(result) = I56::try_from(result) {
                    BigInt::BigIntI56(result)
                } else {
                    agent.heap.create(BigIntHeapData {
                        data: result.into(),
                    })
                }
            }
            (BigInt::BigIntI56(x), BigInt::BigInt(y))
            | (BigInt::BigInt(y), BigInt::BigIntI56(x)) => {
                let x = x.into_i64();
                let y = agent.heap.get(y);
                agent.heap.create(BigIntHeapData { data: x * &y.data })
            }
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let (x, y) = (agent.heap.get(x), agent.heap.get(y));
                agent.heap.create(BigIntHeapData {
                    data: &x.data * &y.data,
                })
            }
        }
    }

    /// ### [6.1.6.2.12 BigInt::lessThan ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-lessThan)
    ///
    /// The abstract operation BigInt::lessThan takes arguments x (a BigInt) and
    /// y (a BigInt) and returns a Boolean.
    pub(crate) fn less_than(agent: &mut Agent, x: BigInt, y: BigInt) -> bool {
        // 1. If ℝ(x) < ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(_), BigInt::BigIntI56(_)) => false,
            (BigInt::BigIntI56(_), BigInt::BigInt(_)) => true,
            (BigInt::BigInt(b1), BigInt::BigInt(b2)) => {
                let (b1, b2) = (agent.heap.get(b1), agent.heap.get(b2));
                b1.data < b2.data
            }
            (BigInt::BigIntI56(b1), BigInt::BigIntI56(b2)) => b1.into_i64() < b2.into_i64(),
        }
    }

    /// ### [6.1.6.2.13 BigInt::equal ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-equal)
    ///
    /// The abstract operation BigInt::equal takes arguments x (a BigInt) and y (a
    /// BigInt) and returns a Boolean.
    pub(crate) fn equal(agent: &mut Agent, x: BigInt, y: BigInt) -> bool {
        // 1. If ℝ(x) = ℝ(y), return true; otherwise return false.
        match (x, y) {
            (BigInt::BigInt(x), BigInt::BigInt(y)) => {
                let (x, y) = (agent.heap.get(x), agent.heap.get(y));
                x.data == y.data
            }
            (BigInt::BigIntI56(x), BigInt::BigIntI56(y)) => x == y,
            _ => false,
        }
    }
}

impl TryFrom<Value> for BigInt {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::BigInt(x) => Ok(BigInt::BigInt(x)),
            Value::BigIntI56(x) => Ok(BigInt::BigIntI56(x)),
            _ => Err(()),
        }
    }
}

impl From<BigInt> for Value {
    fn from(value: BigInt) -> Value {
        match value {
            BigInt::BigInt(x) => Value::BigInt(x),
            BigInt::BigIntI56(x) => Value::BigIntI56(x),
        }
    }
}
