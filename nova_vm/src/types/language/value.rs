use std::mem::size_of;

use crate::{
    heap::indexes::{
        ArrayIndex, BigIntIndex, DateIndex, ErrorIndex, FunctionIndex, NumberIndex, ObjectIndex,
        RegExpIndex, StringIndex, SymbolIndex,
    },
    Heap, SmallInteger, SmallString,
};

/// 6.1 ECMAScript Language Types
/// https://tc39.es/ecma262/#sec-ecmascript-language-types
#[derive(Debug, Clone, Copy)]
pub enum Value {
    /// 6.1.1 The Undefined Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type
    Undefined,

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

    pub const fn nan() -> Self {
        Self::Float(f32::NAN)
    }

    pub const fn infinity() -> Self {
        Self::Float(f32::INFINITY)
    }

    pub const fn neg_infinity() -> Self {
        Self::Float(f32::NEG_INFINITY)
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
