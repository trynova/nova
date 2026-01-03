// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use small_string::SmallString;

use crate::{
    SmallInteger,
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_bigint::SmallBigInt,
        small_f64::SmallF64,
    },
};

use super::{
    Symbol, Value,
    bigint::HeapBigInt,
    number::HeapNumber,
    string::HeapString,
    value::{
        BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT,
        NULL_DISCRIMINANT, NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT,
        SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
        UNDEFINED_DISCRIMINANT,
    },
};

/// ### [4.4.5 primitive value](https://tc39.es/ecma262/#sec-primitive-value)
///
/// One of the types Undefined, Null, Boolean, Number, BigInt, Symbol, or
/// String as defined in clause [6](https://tc39.es/ecma262/#sec-ecmascript-data-types-and-values).
///
/// > NOTE: A primitive value is a datum that is represented directly at the
/// > lowest level of the language implementation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Primitive<'a> {
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
    String(HeapString<'a>) = STRING_DISCRIMINANT,
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// 7-byte UTF-8 string on the stack. End of the string is determined by
    /// the first 0xFF byte in the data. UTF-16 indexing is calculated on
    /// demand from the data.
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    /// ### [6.1.5 The Symbol Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type)
    Symbol(Symbol<'a>) = SYMBOL_DISCRIMINANT,
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber<'a>) = NUMBER_DISCRIMINANT,
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
    BigInt(HeapBigInt<'a>) = BIGINT_DISCRIMINANT,
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// 56-bit signed integer on the stack.
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}
bindable_handle!(Primitive);
primitive_value!(bool, Boolean);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PrimitiveRootRepr {
    Undefined = UNDEFINED_DISCRIMINANT,
    Null = NULL_DISCRIMINANT,
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

/// A primitive value that is stored on the heap.
///
/// Note: Symbol is not considered a primitive in this sense, as while its data
/// is stored on the heap, the Symbol's value is the Symbol itself and it is
/// stored on the stack.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum HeapPrimitive<'a> {
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// UTF-8 string on the heap. Accessing the data must be done through the
    /// Agent. ECMAScript specification compliant UTF-16 indexing is
    /// implemented through an index mapping.
    String(HeapString<'a>) = STRING_DISCRIMINANT,
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber<'a>) = NUMBER_DISCRIMINANT,
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// Unlimited size integer data on the heap. Accessing the data must be
    /// done through the Agent.
    BigInt(HeapBigInt<'a>) = BIGINT_DISCRIMINANT,
}

impl<'a> From<HeapPrimitive<'a>> for Primitive<'a> {
    fn from(value: HeapPrimitive<'a>) -> Self {
        match value {
            HeapPrimitive::String(data) => Self::String(data),
            HeapPrimitive::Number(data) => Self::Number(data),
            HeapPrimitive::BigInt(data) => Self::BigInt(data),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for HeapPrimitive<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::String(data) => Ok(Self::String(data)),
            Value::Number(data) => Ok(Self::Number(data)),
            Value::BigInt(data) => Ok(Self::BigInt(data)),
            _ => Err(()),
        }
    }
}

impl Primitive<'_> {
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

impl<'a> From<Primitive<'a>> for Value<'a> {
    #[inline]
    fn from(value: Primitive<'a>) -> Self {
        match value {
            Primitive::Undefined => Self::Undefined,
            Primitive::Null => Self::Null,
            Primitive::Boolean(p) => Self::Boolean(p),
            Primitive::String(p) => Self::String(p),
            Primitive::SmallString(p) => Self::SmallString(p),
            Primitive::Symbol(p) => Self::Symbol(p),
            Primitive::Number(p) => Self::Number(p),
            Primitive::Integer(p) => Self::Integer(p),
            Primitive::SmallF64(p) => Self::SmallF64(p),
            Primitive::BigInt(p) => Self::BigInt(p),
            Primitive::SmallBigInt(p) => Self::SmallBigInt(p),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for Primitive<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Undefined => Ok(Primitive::Undefined),
            Value::Null => Ok(Primitive::Null),
            Value::Boolean(p) => Ok(Primitive::Boolean(p)),
            Value::String(p) => Ok(Primitive::String(p)),
            Value::SmallString(p) => Ok(Primitive::SmallString(p)),
            Value::Symbol(p) => Ok(Primitive::Symbol(p)),
            Value::Number(p) => Ok(Primitive::Number(p)),
            Value::Integer(p) => Ok(Primitive::Integer(p)),
            Value::SmallF64(p) => Ok(Primitive::SmallF64(p)),
            Value::BigInt(p) => Ok(Primitive::BigInt(p)),
            Value::SmallBigInt(p) => Ok(Primitive::SmallBigInt(p)),
            _ => Err(()),
        }
    }
}

impl Rootable for Primitive<'_> {
    type RootRepr = PrimitiveRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Undefined => Ok(Self::RootRepr::Undefined),
            Self::Null => Ok(Self::RootRepr::Null),
            Self::Boolean(p) => Ok(Self::RootRepr::Boolean(p)),
            Self::String(p) => Err(HeapRootData::String(p.unbind())),
            Self::SmallString(p) => Ok(Self::RootRepr::SmallString(p)),
            Self::Symbol(p) => Err(HeapRootData::Symbol(p.unbind())),
            Self::Number(p) => Err(HeapRootData::Number(p.unbind())),
            Self::Integer(p) => Ok(Self::RootRepr::Integer(p)),
            Self::SmallF64(p) => Ok(Self::RootRepr::SmallF64(p)),
            Self::BigInt(p) => Err(HeapRootData::BigInt(p.unbind())),
            Self::SmallBigInt(p) => Ok(Self::RootRepr::SmallBigInt(p)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Undefined => Ok(Self::Undefined),
            Self::RootRepr::Null => Ok(Self::Null),
            Self::RootRepr::Boolean(p) => Ok(Self::Boolean(p)),
            Self::RootRepr::SmallString(p) => Ok(Self::SmallString(p)),
            Self::RootRepr::Integer(p) => Ok(Self::Integer(p)),
            Self::RootRepr::SmallF64(p) => Ok(Self::SmallF64(p)),
            Self::RootRepr::SmallBigInt(p) => Ok(Self::SmallBigInt(p)),
            Self::RootRepr::HeapRef(p) => Err(p),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::String(p) => Some(Self::String(p)),
            HeapRootData::Symbol(p) => Some(Self::Symbol(p)),
            HeapRootData::Number(p) => Some(Self::Number(p)),
            HeapRootData::BigInt(p) => Some(Self::BigInt(p)),
            _ => None,
        }
    }
}

macro_rules! primitive_value {
    ($name: tt) => {
        crate::ecmascript::types::primitive_value!($name, $name);
    };
    ($name: ident, $variant: ident) => {
        impl From<$name> for crate::ecmascript::types::Primitive<'static> {
            #[inline(always)]
            fn from(value: $name) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<crate::ecmascript::types::Primitive<'_>> for $name {
            type Error = ();

            #[inline]
            fn try_from(value: crate::ecmascript::types::Primitive) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::types::Primitive::$variant(data) => Ok(data),
                    _ => Err(()),
                }
            }
        }

        impl From<$name> for crate::ecmascript::types::Value<'static> {
            #[inline(always)]
            fn from(value: $name) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<crate::ecmascript::types::Value<'_>> for $name {
            type Error = ();

            #[inline]
            fn try_from(value: crate::ecmascript::types::Value) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::types::Value::$variant(data) => Ok(data),
                    _ => Err(()),
                }
            }
        }
    };
}
pub(crate) use primitive_value;

macro_rules! primitive_handle {
    ($name: tt) => {
        crate::ecmascript::types::primitive_handle!($name, $name);
    };
    ($name: ident, $variant: ident) => {
        crate::ecmascript::types::value_handle!($name, $variant);

        impl<'a> From<$name<'a>> for crate::ecmascript::types::Primitive<'a> {
            #[inline(always)]
            fn from(value: $name<'a>) -> Self {
                Self::$variant(value)
            }
        }

        impl<'a> TryFrom<crate::ecmascript::types::Primitive<'a>> for $name<'a> {
            type Error = ();

            #[inline]
            fn try_from(
                value: crate::ecmascript::types::Primitive<'a>,
            ) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::types::Primitive::$variant(data) => Ok(data),
                    _ => Err(()),
                }
            }
        }
    };
}
pub(crate) use primitive_handle;
