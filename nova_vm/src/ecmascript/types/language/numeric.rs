// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    SmallInteger,
    ecmascript::{
        execution::Agent,
        types::{
            BIGINT_DISCRIMINANT, FLOAT_DISCRIMINANT, HeapNumber, INTEGER_DISCRIMINANT,
            NUMBER_DISCRIMINANT, Number, Primitive, SMALL_BIGINT_DISCRIMINANT, Value,
            bigint::HeapBigInt,
        },
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_bigint::SmallBigInt,
        small_f64::SmallF64,
    },
};

/// ### [6.1.6 Numeric Types](https://tc39.es/ecma262/#sec-numeric-types)
///
/// ECMAScript has two built-in numeric types: Number and BigInt. This type
/// abstracts over the two.
///
/// Because the numeric types are in general not convertible without loss of
/// precision or truncation, the ECMAScript language provides no implicit
/// conversion among these types. Programmers must explicitly call `Number` and
/// `BigInt` functions to convert among types when calling a function which
/// requires another type.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Numeric<'a> {
    Number(HeapNumber<'a>) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt<'a>) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum NumericRootRepr {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl Numeric<'_> {
    pub fn is_bigint(self) -> bool {
        matches!(self, Self::BigInt(_) | Self::SmallBigInt(_))
    }

    pub fn is_number(self) -> bool {
        matches!(self, Self::Number(_) | Self::SmallF64(_) | Self::Integer(_))
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
}

bindable_handle!(Numeric);

impl Rootable for Numeric<'_> {
    type RootRepr = NumericRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Number(n) => Err(HeapRootData::Number(n.unbind())),
            Self::Integer(n) => Ok(Self::RootRepr::Integer(n)),
            Self::SmallF64(n) => Ok(Self::RootRepr::SmallF64(n)),
            Self::BigInt(n) => Err(HeapRootData::BigInt(n.unbind())),
            Self::SmallBigInt(n) => Ok(Self::RootRepr::SmallBigInt(n)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Integer(n) => Ok(Self::Integer(n)),
            Self::RootRepr::SmallF64(n) => Ok(Self::SmallF64(n)),
            Self::RootRepr::SmallBigInt(n) => Ok(Self::SmallBigInt(n)),
            Self::RootRepr::HeapRef(n) => Err(n),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Number(n) => Some(Self::Number(n)),
            HeapRootData::BigInt(n) => Some(Self::BigInt(n)),
            _ => None,
        }
    }
}

// === OUTOUT OF primitive_handle! MACRO ADAPTED FOR Numeric ===

impl<'a> From<Numeric<'a>> for Value<'a> {
    #[inline(always)]
    fn from(value: Numeric<'a>) -> Self {
        match value {
            Numeric::Number(n) => Self::Number(n),
            Numeric::Integer(n) => Self::Integer(n),
            Numeric::SmallF64(n) => Self::SmallF64(n),
            Numeric::BigInt(n) => Self::BigInt(n),
            Numeric::SmallBigInt(n) => Self::SmallBigInt(n),
        }
    }
}
impl<'a> TryFrom<Value<'a>> for Numeric<'a> {
    type Error = ();
    #[inline]
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Number(data) => Ok(Numeric::Number(data)),
            Value::Integer(data) => Ok(Numeric::Integer(data)),
            Value::SmallF64(data) => Ok(Numeric::SmallF64(data)),
            Value::BigInt(data) => Ok(Numeric::BigInt(data)),
            Value::SmallBigInt(data) => Ok(Numeric::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}
impl<'a> From<Numeric<'a>> for Primitive<'a> {
    #[inline(always)]
    fn from(value: Numeric<'a>) -> Self {
        match value {
            Numeric::Number(n) => Self::Number(n),
            Numeric::Integer(n) => Self::Integer(n),
            Numeric::SmallF64(n) => Self::SmallF64(n),
            Numeric::BigInt(n) => Self::BigInt(n),
            Numeric::SmallBigInt(n) => Self::SmallBigInt(n),
        }
    }
}
impl<'a> TryFrom<Primitive<'a>> for Numeric<'a> {
    type Error = ();
    #[inline]
    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        match value {
            Primitive::Number(data) => Ok(Numeric::Number(data)),
            Primitive::Integer(data) => Ok(Numeric::Integer(data)),
            Primitive::SmallF64(data) => Ok(Numeric::SmallF64(data)),
            Primitive::BigInt(data) => Ok(Numeric::BigInt(data)),
            Primitive::SmallBigInt(data) => Ok(Numeric::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}

// === END ===

macro_rules! numeric_value {
    ($name: tt) => {
        crate::ecmascript::types::numeric_value!($name, $name);
    };
    ($name: ident, $variant: ident) => {
        crate::ecmascript::types::primitive_value!($name, $variant);

        impl From<$name> for crate::ecmascript::types::Numeric<'static> {
            #[inline(always)]
            fn from(value: $name) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<crate::ecmascript::types::Numeric<'_>> for $name {
            type Error = ();

            #[inline]
            fn try_from(value: crate::ecmascript::types::Numeric) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::types::Numeric::$variant(data) => Ok(data),
                    _ => Err(()),
                }
            }
        }
    };
}
pub(crate) use numeric_value;

macro_rules! numeric_handle {
    ($name: tt) => {
        crate::ecmascript::types::numeric_handle!($name, $name);
    };
    ($name: ident, $variant: ident) => {
        crate::ecmascript::types::primitive_handle!($name, $variant);

        impl<'a> From<$name<'a>> for crate::ecmascript::types::Numeric<'a> {
            fn from(value: $name<'a>) -> Self {
                Self::$variant(value)
            }
        }

        impl<'a> TryFrom<crate::ecmascript::types::Numeric<'a>> for $name<'a> {
            type Error = ();

            fn try_from(value: crate::ecmascript::types::Numeric<'a>) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::types::Numeric::$variant(data) => Ok(data),
                    _ => Err(()),
                }
            }
        }
    };
}
pub(crate) use numeric_handle;
