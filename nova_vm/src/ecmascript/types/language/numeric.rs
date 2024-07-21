// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_number,
        execution::{Agent, JsResult},
    },
    engine::small_f64::SmallF64,
    SmallInteger,
};

use super::{
    bigint::{HeapBigInt, SmallBigInt},
    number::HeapNumber,
    value::{
        BIGINT_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, NUMBER_DISCRIMINANT,
        SMALL_BIGINT_DISCRIMINANT,
    },
    IntoPrimitive, IntoValue, Number, Primitive, Value,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Numeric {
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(SmallF64) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

impl Numeric {
    pub fn is_bigint(self) -> bool {
        matches!(self, Self::BigInt(_) | Self::SmallBigInt(_))
    }

    pub fn is_number(self) -> bool {
        matches!(self, Self::Number(_) | Self::Float(_) | Self::Integer(_))
    }

    pub fn is_pos_zero(self, agent: &mut Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_pos_zero(agent))
            .unwrap_or(false)
    }

    pub fn is_neg_zero(self, agent: &mut Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_neg_zero(agent))
            .unwrap_or(false)
    }

    pub fn is_pos_infinity(self, agent: &mut Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_pos_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_neg_infinity(self, agent: &mut Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_neg_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_nan(self, agent: &mut Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_nan(agent))
            .unwrap_or(false)
    }

    /// ### [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub fn to_real(self, agent: &mut Agent) -> JsResult<f64> {
        Ok(match self {
            Self::Number(n) => agent[n],
            Self::Integer(i) => i.into_i64() as f64,
            Self::Float(f) => f.into_f64(),
            // NOTE: Converting to a number should give us a nice error message.
            _ => to_number(agent, self)?.into_f64(agent),
        })
    }
}

impl IntoValue for Numeric {
    fn into_value(self) -> Value {
        match self {
            Numeric::Number(data) => Value::Number(data),
            Numeric::Integer(data) => Value::Integer(data),
            Numeric::Float(data) => Value::SmallF64(data),
            Numeric::BigInt(data) => Value::BigInt(data),
            Numeric::SmallBigInt(data) => Value::SmallBigInt(data),
        }
    }
}

impl IntoPrimitive for Numeric {
    fn into_primitive(self) -> Primitive {
        match self {
            Numeric::Number(data) => Primitive::Number(data),
            Numeric::Integer(data) => Primitive::Integer(data),
            Numeric::Float(data) => Primitive::SmallF64(data),
            Numeric::BigInt(data) => Primitive::BigInt(data),
            Numeric::SmallBigInt(data) => Primitive::SmallBigInt(data),
        }
    }
}

impl From<Numeric> for Value {
    fn from(value: Numeric) -> Self {
        value.into_value()
    }
}

impl From<Numeric> for Primitive {
    fn from(value: Numeric) -> Self {
        value.into_primitive()
    }
}

impl TryFrom<Value> for Numeric {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(data) => Ok(Numeric::Number(data)),
            Value::Integer(data) => Ok(Numeric::Integer(data)),
            Value::SmallF64(data) => Ok(Numeric::Float(data)),
            Value::BigInt(data) => Ok(Numeric::BigInt(data)),
            Value::SmallBigInt(data) => Ok(Numeric::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Primitive> for Numeric {
    type Error = ();

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        match value {
            Primitive::Number(data) => Ok(Numeric::Number(data)),
            Primitive::Integer(data) => Ok(Numeric::Integer(data)),
            Primitive::SmallF64(data) => Ok(Numeric::Float(data)),
            Primitive::BigInt(data) => Ok(Numeric::BigInt(data)),
            Primitive::SmallBigInt(data) => Ok(Numeric::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}
