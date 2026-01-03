// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    SmallInteger,
    ecmascript::execution::Agent,
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_bigint::SmallBigInt,
        small_f64::SmallF64,
    },
};

use super::{
    Number, Primitive, Value,
    bigint::HeapBigInt,
    number::HeapNumber,
    value::{
        BIGINT_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, NUMBER_DISCRIMINANT,
        SMALL_BIGINT_DISCRIMINANT,
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

impl<'a> From<Numeric<'a>> for Primitive<'a> {
    fn from(value: Numeric<'a>) -> Self {
        value.into()
    }
}

impl<'a> TryFrom<Value<'a>> for Numeric<'a> {
    type Error = ();

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

impl<'a> TryFrom<Primitive<'a>> for Numeric<'a> {
    type Error = ();

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

impl Rootable for Numeric<'_> {
    type RootRepr = NumericRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Number(heap_number) => Err(HeapRootData::Number(heap_number.unbind())),
            Self::Integer(integer) => Ok(Self::RootRepr::Integer(integer)),
            Self::SmallF64(small_f64) => Ok(Self::RootRepr::SmallF64(small_f64)),
            Self::BigInt(heap_big_int) => Err(HeapRootData::BigInt(heap_big_int.unbind())),
            Self::SmallBigInt(small_big_int) => Ok(Self::RootRepr::SmallBigInt(small_big_int)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Integer(small_integer) => Ok(Self::Integer(small_integer)),
            Self::RootRepr::SmallF64(small_f64) => Ok(Self::SmallF64(small_f64)),
            Self::RootRepr::SmallBigInt(small_big_int) => Ok(Self::SmallBigInt(small_big_int)),
            Self::RootRepr::HeapRef(heap_root_ref) => Err(heap_root_ref),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Number(heap_number) => Some(Self::Number(heap_number)),
            HeapRootData::BigInt(heap_big_int) => Some(Self::BigInt(heap_big_int)),
            _ => None,
        }
    }
}
