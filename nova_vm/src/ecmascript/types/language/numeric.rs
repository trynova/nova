// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::NoGcScope;
use crate::engine::Scoped;
use crate::{
    ecmascript::execution::Agent,
    engine::{
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_f64::SmallF64,
    },
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

impl<'a> Numeric<'a> {
    /// Unbind this Numeric from its current lifetime. This is necessary to use
    /// the Numeric as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> Numeric<'static> {
        unsafe { std::mem::transmute::<Self, Numeric<'static>>(self) }
    }

    // Bind this Numeric to the garbage collection lifetime. This enables
    // Rust's borrow checker to verify that your Numerics cannot not be
    // invalidated by garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let numeric = numeric.bind(&gc);
    // ```
    // to make sure that the unbound Numeric cannot be used after binding.
    pub const fn bind(self, _: NoGcScope<'a, '_>) -> Self {
        unsafe { std::mem::transmute::<Numeric<'_>, Self>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Numeric<'static>> {
        Scoped::new(agent, gc, self.unbind())
    }

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

impl IntoValue for Numeric<'_> {
    fn into_value(self) -> Value {
        match self {
            Numeric::Number(data) => Value::Number(data.unbind()),
            Numeric::Integer(data) => Value::Integer(data),
            Numeric::SmallF64(data) => Value::SmallF64(data),
            Numeric::BigInt(data) => Value::BigInt(data.unbind()),
            Numeric::SmallBigInt(data) => Value::SmallBigInt(data),
        }
    }
}

impl<'a> IntoPrimitive<'a> for Numeric<'a> {
    fn into_primitive(self) -> Primitive<'a> {
        match self {
            Numeric::Number(data) => Primitive::Number(data.unbind()),
            Numeric::Integer(data) => Primitive::Integer(data),
            Numeric::SmallF64(data) => Primitive::SmallF64(data),
            Numeric::BigInt(data) => Primitive::BigInt(data.unbind()),
            Numeric::SmallBigInt(data) => Primitive::SmallBigInt(data),
        }
    }
}

impl From<Numeric<'_>> for Value {
    fn from(value: Numeric<'_>) -> Self {
        value.into_value()
    }
}

impl<'a> From<Numeric<'a>> for Primitive<'a> {
    fn from(value: Numeric<'a>) -> Self {
        value.into_primitive()
    }
}

impl TryFrom<Value> for Numeric<'_> {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
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
