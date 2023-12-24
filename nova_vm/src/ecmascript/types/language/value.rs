use std::mem::size_of;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int, to_int32, to_number, to_numeric, to_uint32,
        },
        execution::{Agent, JsResult},
    },
    heap::{
        indexes::{
            ArrayBufferIndex, ArrayIndex, BigIntIndex, DateIndex, ErrorIndex, FunctionIndex,
            NumberIndex, ObjectIndex, RegExpIndex, StringIndex, SymbolIndex,
        },
        CreateHeapData, GetHeapData,
    },
    Heap, SmallInteger, SmallString,
};

use super::{BigInt, Number};

// TODO: Handle this and enum subtypes via a macro for compile-time guarantees and safety

/// 6.1 ECMAScript Language Types
/// https://tc39.es/ecma262/#sec-ecmascript-language-types
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(u8)]
pub enum Value {
    /// 6.1.1 The Undefined Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type
    #[default]
    Undefined = UNDEFINED_DISCRIMINANT,

    /// 6.1.2 The Null Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-null-type
    Null = NULL_DISCRIMINANT,

    /// 6.1.3 The Boolean Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-boolean-type
    Boolean(bool) = BOOLEAN_DISCRIMINANT,

    /// 6.1.4 The String Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type
    String(StringIndex) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,

    /// 6.1.5 The Symbol Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type
    Symbol(SymbolIndex) = SYMBOL_DISCRIMINANT,

    /// 6.1.6.1 The Number Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type
    Number(NumberIndex) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT, // 56-bit signed integer.
    Float(f32) = FLOAT_DISCRIMINANT,

    /// 6.1.6.2 The BigInt Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type
    BigInt(BigIntIndex) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallInteger) = SMALL_BIGINT_DISCRIMINANT,

    /// 6.1.7 The Object Type
    /// https://tc39.es/ecma262/#sec-object-type
    Object(ObjectIndex) = OBJECT_DISCRIMINANT,

    // Well-known object types
    // Roughly corresponding to 6.1.7.4 Well-Known Intrinsic Objects
    // https://tc39.es/ecma262/#sec-well-known-intrinsic-objects
    Array(ArrayIndex) = ARRAY_DISCRIMINANT,
    ArrayBuffer(ArrayBufferIndex) = ARRAY_BUFFER_DISCRIMINANT,
    Date(DateIndex) = DATE_DISCRIMINANT,
    Error(ErrorIndex) = ERROR_DISCRIMINANT,
    Function(FunctionIndex) = FUNCTION_DISCRIMINANT,
    RegExp(RegExpIndex) = REGEXP_DISCRIMINANT,
    // TODO: Implement primitive value objects, those useless things.
    // BigIntObject(u32),
    // BooleanObject(u32),
    // NumberObject(u32),
    // StringObject(u32),
    // SymbolObject(u32),
}

/// We want to guarantee that all handles to JS values are register sized. These asserts must never be removed or broken.
const _: () = assert!(size_of::<Value>() <= size_of::<usize>(), "Handles to JavaScript values should fit within a 64-bit CPU register.");
const _: () = assert!(size_of::<usize>() >= size_of::<u64>(), "Registers should be at least 64-bits");
// We may also want to keep Option<Value> register sized so that eg. holes in arrays do not start requiring extra bookkeeping.
const _: () = assert!(size_of::<Option<Value>>() == size_of::<usize>(), "OPTIONAL_VALUE_SIZE_IS_WORD");

#[derive(Debug, Clone, Copy)]
pub enum PreferredType {
    String,
    Number,
}

const fn value_discriminant(value: Value) -> u8 {
    // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
    // between `repr(C)` structs, each of which has the `u8` discriminant as its first
    // field, so we can read the discriminant without offsetting the pointer.
    unsafe { *(&value as *const Value).cast::<u8>() }
}

// These correspond to the order that each item appears in the Value enum.
// This will be replaced when we modularise the enum subtyping via a crate or macro or whatever.
pub(crate) const UNDEFINED_DISCRIMINANT: u8 = 1;
pub(crate) const NULL_DISCRIMINANT: u8 = 2;
pub(crate) const BOOLEAN_DISCRIMINANT: u8 = 3;
pub(crate) const STRING_DISCRIMINANT: u8 = 4;
pub(crate) const SMALL_STRING_DISCRIMINANT: u8 = 5;
pub(crate) const SYMBOL_DISCRIMINANT: u8 = 6;
pub(crate) const NUMBER_DISCRIMINANT: u8 = 7;
pub(crate) const INTEGER_DISCRIMINANT: u8 = 8;
pub(crate) const FLOAT_DISCRIMINANT: u8 = 9;
pub(crate) const BIGINT_DISCRIMINANT: u8 = 10;
pub(crate) const SMALL_BIGINT_DISCRIMINANT: u8 = 11;
pub(crate) const OBJECT_DISCRIMINANT: u8 = 12;
pub(crate) const ARRAY_DISCRIMINANT: u8 = 13;
pub(crate) const ARRAY_BUFFER_DISCRIMINANT: u8 = 14;
pub(crate) const DATE_DISCRIMINANT: u8 = 15;
pub(crate) const ERROR_DISCRIMINANT: u8 = 16;
pub(crate) const FUNCTION_DISCRIMINANT: u8 = 17;
pub(crate) const REGEXP_DISCRIMINANT: u8 = 18;

impl Value {
    pub fn from_str(heap: &mut Heap, message: &str) -> Value {
        heap.create(message).into()
    }

    pub fn from_f64(agent: &mut Agent, value: f64) -> Value {
        Number::from_f64(agent, value).into()
    }

    pub fn nan() -> Self {
        Number::nan().into_value()
    }

    pub fn infinity() -> Self {
        Number::pos_inf().into_value()
    }

    pub fn neg_infinity() -> Self {
        Number::neg_inf().into_value()
    }

    pub fn is_true(self) -> bool {
        matches!(self, Value::Boolean(true))
    }

    pub fn is_false(self) -> bool {
        matches!(self, Value::Boolean(false))
    }

    pub fn is_object(self) -> bool {
        matches!(
            self,
            Value::Object(_)
                | Value::Array(_)
                | Value::ArrayBuffer(_)
                | Value::Date(_)
                | Value::Function(_)
                | Value::Error(_)
                | Value::RegExp(_)
        )
    }

    pub fn is_string(self) -> bool {
        matches!(self, Value::String(_) | Value::SmallString(_))
    }

    pub fn is_boolean(self) -> bool {
        // TODO: Check for Boolean object instance.
        matches!(self, Value::Boolean(_))
    }

    pub fn is_null(self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_undefined(self) -> bool {
        matches!(self, Value::Undefined)
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

    pub fn is_bigint(self) -> bool {
        // TODO: Check for BigInt object instance.
        matches!(self, Value::BigInt(_) | Value::SmallBigInt(_))
    }

    pub fn is_symbol(self) -> bool {
        matches!(self, Value::Symbol(_))
    }

    pub fn is_number(self) -> bool {
        matches!(self, Value::Number(_) | Value::Float(_) | Value::Integer(_))
    }

    pub fn is_empty_string(self) -> bool {
        if let Value::SmallString(s) = self {
            s.is_empty()
        } else {
            false
        }
    }

    pub fn to_number(self, agent: &mut Agent) -> JsResult<Number> {
        to_number(agent, self)
    }

    pub fn to_bigint(self, agent: &mut Agent) -> JsResult<BigInt> {
        to_big_int(agent, self)
    }

    pub fn to_numeric(self, agent: &mut Agent) -> JsResult<Value> {
        to_numeric(agent, self)
    }

    pub fn to_int32(self, agent: &mut Agent) -> JsResult<i32> {
        to_int32(agent, self)
    }

    pub fn to_uint32(self, agent: &mut Agent) -> JsResult<u32> {
        to_uint32(agent, self)
    }

    /// ### [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub fn to_real(self, agent: &mut Agent) -> JsResult<f64> {
        Ok(match self {
            Value::Number(n) => *agent.heap.get(n),
            Value::Integer(i) => i.into_i64() as f64,
            Value::Float(f) => f as f64,
            // NOTE: Converting to a number should give us a nice error message.
            _ => to_number(agent, self)?.into_f64(agent),
        })
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Value::Undefined, |v| v.into())
    }
}

impl TryFrom<&str> for Value {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, ()> {
        if let Ok(data) = value.try_into() {
            Ok(Value::SmallString(data))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for Value {
    type Error = ();
    fn try_from(value: f64) -> Result<Self, ()> {
        Number::try_from(value).map(|v| v.into())
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        value.into_value()
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Float(value)
    }
}

impl TryFrom<i64> for Value {
    type Error = ();
    fn try_from(value: i64) -> Result<Self, ()> {
        Ok(Value::Integer(SmallInteger::try_from(value)?))
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for Value {
            fn from(value: $size) -> Self {
                Value::Integer(SmallInteger::from(value))
            }
        }
    };
}

impl_value_from_n!(u8);
impl_value_from_n!(i8);
impl_value_from_n!(u16);
impl_value_from_n!(i16);
impl_value_from_n!(u32);
impl_value_from_n!(i32);
