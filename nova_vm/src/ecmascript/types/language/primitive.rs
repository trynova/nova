// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use small_string::SmallString;

use crate::{engine::small_f64::SmallF64, SmallInteger};

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
    /// ### [6.1.1 The Undefined Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type)
    Undefined = UNDEFINED_DISCRIMINANT,
    /// ### [6.1.2 The Null Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-null-type)
    Null = NULL_DISCRIMINANT,
    /// ### [6.1.3 The Boolean Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-boolean-type)
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// UTF-8 string on the heap. Accessing the data must be done through the
    /// Agent. ECMAScript specification compliant UTF-16 indexing is
    /// implemented through an index mapping.
    String(HeapString) = STRING_DISCRIMINANT,
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// 7-byte UTF-8 string on the stack. End of the string is determined by
    /// the first 0xFF byte in the data. UTF-16 indexing is calculated on
    /// demand from the data.
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    /// ### [6.1.5 The Symbol Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type)
    Symbol(Symbol) = SYMBOL_DISCRIMINANT,
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 53-bit signed integer on the stack.
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 56-bit f64 on the stack. The missing byte is a zero least significant
    /// byte.
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// Unlimited size integer data on the heap. Accessing the data must be
    /// done through the Agent.
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// 56-bit signed integer on the stack.
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

/// A primitive value that is stored on the heap.
///
/// Note: Symbol is not considered a primitive in this sense, as while its data
/// is stored on the heap, the Symbol's value is the Symbol itself and it is
/// stored on the stack.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum HeapPrimitive {
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// UTF-8 string on the heap. Accessing the data must be done through the
    /// Agent. ECMAScript specification compliant UTF-16 indexing is
    /// implemented through an index mapping.
    String(HeapString) = STRING_DISCRIMINANT,
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// Unlimited size integer data on the heap. Accessing the data must be
    /// done through the Agent.
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
}

impl IntoValue for HeapPrimitive {
    fn into_value(self) -> Value {
        match self {
            HeapPrimitive::String(data) => Value::String(data),
            HeapPrimitive::Number(data) => Value::Number(data),
            HeapPrimitive::BigInt(data) => Value::BigInt(data),
        }
    }
}

impl TryFrom<Value> for HeapPrimitive {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(data) => Ok(Self::String(data)),
            Value::Number(data) => Ok(Self::Number(data)),
            Value::BigInt(data) => Ok(Self::BigInt(data)),
            _ => Err(()),
        }
    }
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
            Primitive::SmallF64(data) => Value::SmallF64(data),
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
        matches!(self, Self::Number(_) | Self::SmallF64(_) | Self::Integer(_))
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
            Value::SmallF64(data) => Ok(Primitive::SmallF64(data)),
            Value::BigInt(data) => Ok(Primitive::BigInt(data)),
            Value::SmallBigInt(data) => Ok(Primitive::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}
