use std::mem::size_of;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_int32, to_number, to_numeric, to_uint32},
        execution::{Agent, JsResult},
    },
    heap::indexes::{
        ArrayBufferIndex, ArrayIndex, BigIntIndex, DateIndex, ErrorIndex, FunctionIndex,
        NumberIndex, ObjectIndex, RegExpIndex, StringIndex, SymbolIndex,
    },
    Heap, SmallInteger, SmallString,
};

use super::Number;

/// 6.1 ECMAScript Language Types
/// https://tc39.es/ecma262/#sec-ecmascript-language-types
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(u8)]
pub enum Value {
    /// 6.1.1 The Undefined Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type
    #[default]
    Undefined = 1,

    /// 6.1.2 The Null Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-null-type
    Null,

    /// 6.1.3 The Boolean Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-boolean-type
    Boolean(bool),

    /// 6.1.4 The String Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type
    String(StringIndex),
    SmallString(SmallString),

    /// 6.1.5 The Symbol Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type
    Symbol(SymbolIndex),

    /// 6.1.6.1 The Number Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type
    Number(NumberIndex),
    Integer(SmallInteger), // 56-bit signed integer.
    Float(f32),

    /// 6.1.6.2 The BigInt Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type
    BigInt(BigIntIndex),
    SmallBigInt(SmallInteger),

    /// 6.1.7 The Object Type
    /// https://tc39.es/ecma262/#sec-object-type
    Object(ObjectIndex),

    // Well-known object types
    // Roughly corresponding to 6.1.7.4 Well-Known Intrinsic Objects
    // https://tc39.es/ecma262/#sec-well-known-intrinsic-objects
    Array(ArrayIndex),
    ArrayBuffer(ArrayBufferIndex),
    Date(DateIndex),
    Error(ErrorIndex),
    Function(FunctionIndex),
    RegExp(RegExpIndex),
    // TODO: Implement primitive value objects, those useless things.
    // BigIntObject(u32),
    // BooleanObject(u32),
    // NumberObject(u32),
    // StringObject(u32),
    // SymbolObject(u32),
}

/// We want to guarantee that all handles to JS values are register sized. This assert must never be removed or broken.
const _VALUE_SIZE_IS_WORD: () = assert!(size_of::<Value>() == size_of::<usize>());
// We may also want to keep Option<Value> register sized so that eg. holes in arrays do not start requiring extra bookkeeping.
const _OPTIONAL_VALUE_SIZE_IS_WORD: () = assert!(size_of::<Option<Value>>() == size_of::<usize>());

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

pub(crate) const UNDEFINED_DISCRIMINANT: u8 = value_discriminant(Value::Undefined);
pub(crate) const NULL_DISCRIMINANT: u8 = value_discriminant(Value::Null);
pub(crate) const BOOLEAN_DISCRIMINANT: u8 = value_discriminant(Value::Boolean(true));
pub(crate) const STRING_DISCRIMINANT: u8 =
    value_discriminant(Value::String(StringIndex::from_u32_index(0)));
pub(crate) const SMALL_STRING_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallString(SmallString::new_empty()));
pub(crate) const SYMBOL_DISCRIMINANT: u8 =
    value_discriminant(Value::Symbol(SymbolIndex::from_u32_index(0)));
pub(crate) const NUMBER_DISCRIMINANT: u8 =
    value_discriminant(Value::Number(NumberIndex::from_u32_index(0)));
pub(crate) const INTEGER_DISCRIMINANT: u8 =
    value_discriminant(Value::Integer(SmallInteger::zero()));
pub(crate) const FLOAT_DISCRIMINANT: u8 = value_discriminant(Value::Float(0f32));
pub(crate) const BIGINT_DISCRIMINANT: u8 =
    value_discriminant(Value::BigInt(BigIntIndex::from_u32_index(0)));
pub(crate) const SMALL_BIGINT_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallBigInt(SmallInteger::zero()));
pub(crate) const OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::Object(ObjectIndex::from_u32_index(0)));
pub(crate) const ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Array(ArrayIndex::from_u32_index(0)));
pub(crate) const ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayBuffer(ArrayBufferIndex::from_u32_index(0)));
pub(crate) const DATE_DISCRIMINANT: u8 =
    value_discriminant(Value::Date(DateIndex::from_u32_index(0)));
pub(crate) const ERROR_DISCRIMINANT: u8 =
    value_discriminant(Value::Error(ErrorIndex::from_u32_index(0)));
pub(crate) const FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::Function(FunctionIndex::from_u32_index(0)));
pub(crate) const REGEXP_DISCRIMINANT: u8 =
    value_discriminant(Value::RegExp(RegExpIndex::from_u32_index(0)));

impl Value {
    pub fn from_str(heap: &mut Heap, message: &str) -> Value {
        if let Ok(ascii_string) = SmallString::try_from(message) {
            Value::SmallString(ascii_string)
        } else {
            Value::String(heap.alloc_string(message))
        }
    }

    pub fn from_f64(heap: &mut Heap, value: f64) -> Value {
        let is_int = value.fract() == 0.0;
        if is_int {
            if let Ok(data) = Value::try_from(value as i64) {
                return data;
            }
        }
        if value as f32 as f64 == value {
            // TODO: Verify logic
            Value::Float(value as f32)
        } else {
            Value::Number(heap.alloc_number(value))
        }
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

    pub fn is_nan(self, agent: &mut Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_nan(agent))
            .unwrap_or(false)
    }

    pub fn is_bigint(self) -> bool {
        // TODO: Check for BigInt object instance.
        matches!(self, Value::BigInt(_))
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

    pub fn to_numeric(self, agent: &mut Agent) -> JsResult<Value> {
        to_numeric(agent, self)
    }

    pub fn to_int32(self, agent: &mut Agent) -> JsResult<i32> {
        to_int32(agent, self)
    }

    pub fn to_uint32(self, agent: &mut Agent) -> JsResult<u32> {
        to_uint32(agent, self)
    }

    fn is_same_type(self, y: Self) -> bool {
        let x = self;
        (x.is_undefined() && y.is_undefined())
            || (x.is_null() && y.is_null())
            || (x.is_boolean() && y.is_boolean())
            || (x.is_string() && y.is_string())
            || (x.is_symbol() && y.is_symbol())
            || (x.is_number() && y.is_number())
            || (x.is_object() && y.is_object())
    }

    /// 7.2.10 SameValue ( x, y )
    /// https://tc39.es/ecma262/#sec-samevalue
    pub fn same_value(self, agent: &mut Agent, y: Self) -> bool {
        let x = self;

        // 1. If Type(x) is not Type(y), return false.
        if !x.is_same_type(y) {
            return false;
        }

        // 2. If x is a Number, then
        if let (Ok(x), Ok(y)) = (Number::try_from(x), Number::try_from(y)) {
            // a. Return Number::sameValue(x, y).
            return x.same_value(agent, y);
        }

        // 3. Return SameValueNonNumber(x, y).
        x.same_value_non_number(agent, y)
    }

    /// 7.2.12 SameValueNonNumber ( x, y )
    /// https://tc39.es/ecma262/#sec-samevaluenonnumber
    pub fn same_value_non_number(self, _agent: &mut Agent, y: Self) -> bool {
        let x = self;

        // 1. Assert: Type(x) is Type(y).
        debug_assert!(x.is_same_type(y));

        // 2. If x is either null or undefined, return true.
        if x.is_null() || x.is_undefined() {
            return true;
        }

        // 3. If x is a BigInt, then
        if x.is_bigint() {
            // a. Return BigInt::equal(x, y).
            todo!();
        }

        // 4. If x is a String, then
        if x.is_string() {
            // a. If x and y have the same length and the same code units in the same positions, return true; otherwise, return false.
            todo!();
        }

        // 5. If x is a Boolean, then
        if x.is_boolean() {
            // a. If x and y are both true or both false, return true; otherwise, return false.
            return x.is_true() == y.is_true();
        }

        // 6. NOTE: All other ECMAScript language values are compared by identity.
        // 7. If x is y, return true; otherwise, return false.
        todo!()
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<Option<Value>> for Value {
    fn from(value: Option<Value>) -> Self {
        value.unwrap_or(Value::Undefined)
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
        // TODO: verify logic
        if value as f32 as f64 == value {
            Ok(Value::Float(value as f32))
        } else {
            Err(())
        }
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
                let n: i64 = value.into();
                Value::Integer(SmallInteger::from_i64_unchecked(n))
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
