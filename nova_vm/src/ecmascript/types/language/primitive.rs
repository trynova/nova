use small_string::SmallString;

use crate::SmallInteger;

use super::{
    bigint::{HeapBigInt, SmallBigInt},
    number::HeapNumber,
    string::HeapString,
    value::{
        BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT,
        NULL_DISCRIMINANT, NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT,
        SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
        UNDEFINED_DISCRIMINANT,
    },
    IntoValue, Symbol, Value,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Primitive {
    Undefined = UNDEFINED_DISCRIMINANT,
    Null = NULL_DISCRIMINANT,
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(HeapString) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Symbol(Symbol) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

impl IntoValue for Primitive {
    fn into_value(self) -> super::Value {
        match self {
            Primitive::Undefined => Value::Undefined,
            Primitive::Null => Value::Null,
            Primitive::Boolean(data) => Value::Boolean(data),
            Primitive::String(data) => Value::String(data),
            Primitive::SmallString(data) => Value::SmallString(data),
            Primitive::Symbol(data) => Value::Symbol(data),
            Primitive::Number(data) => Value::Number(data),
            Primitive::Integer(data) => Value::Integer(data),
            Primitive::Float(data) => Value::Float(data),
            Primitive::BigInt(data) => Value::BigInt(data),
            Primitive::SmallBigInt(data) => Value::SmallBigInt(data),
        }
    }
}

impl Primitive {
    pub fn is_boolean(self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    pub fn is_bigint(self) -> bool {
        matches!(self, Self::BigInt(_) | Self::SmallBigInt(_))
    }

    pub fn is_null(self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_number(self) -> bool {
        matches!(self, Self::Number(_) | Self::Float(_) | Self::Integer(_))
    }

    pub fn is_string(self) -> bool {
        matches!(self, Self::String(_) | Self::SmallString(_))
    }
    pub fn is_symbol(self) -> bool {
        matches!(self, Self::Symbol(_))
    }

    pub fn is_undefined(self) -> bool {
        matches!(self, Self::Undefined)
    }
}

impl From<Primitive> for Value {
    fn from(value: Primitive) -> Self {
        value.into_value()
    }
}

impl TryFrom<Value> for Primitive {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Undefined => Ok(Primitive::Undefined),
            Value::Null => Ok(Primitive::Null),
            Value::Boolean(data) => Ok(Primitive::Boolean(data)),
            Value::String(data) => Ok(Primitive::String(data)),
            Value::SmallString(data) => Ok(Primitive::SmallString(data)),
            Value::Symbol(data) => Ok(Primitive::Symbol(data)),
            Value::Number(data) => Ok(Primitive::Number(data)),
            Value::Integer(data) => Ok(Primitive::Integer(data)),
            Value::Float(data) => Ok(Primitive::Float(data)),
            Value::BigInt(data) => Ok(Primitive::BigInt(data)),
            Value::SmallBigInt(data) => Ok(Primitive::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}
