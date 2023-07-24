use crate::{
    heap::{
        ArrayHeapData, BigIntHeapData, Handle, NumberHeapData, ObjectHeapData, StringHeapData,
        SymbolHeapData,
    },
    SmallInteger, SmallString,
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
    String(Handle<StringHeapData>),
    SmallString(SmallString),

    /// 6.1.5 The Symbol Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type
    Symbol(Handle<SymbolHeapData>),

    /// 6.1.6.1 The Number Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type
    Number(Handle<NumberHeapData>),
    IntegerNumber(SmallInteger), // 56-bit signed integer.
    FloatNumber(f32),

    /// 6.1.6.2 The BigInt Type
    /// https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type
    BigInt(Handle<BigIntHeapData>),

    /// 6.1.7 The Object Type
    /// https://tc39.es/ecma262/#sec-object-type
    Object(Handle<ObjectHeapData>),
    ArrayObject(Handle<ArrayHeapData>),
}

impl Value {
    pub const fn nan() -> Self {
        Self::FloatNumber(f32::NAN)
    }

    pub const fn infinity() -> Self {
        Self::FloatNumber(f32::INFINITY)
    }

    pub const fn neg_infinity() -> Self {
        Self::FloatNumber(f32::NEG_INFINITY)
    }
}

impl From<Option<Value>> for Value {
    fn from(value: Option<Value>) -> Self {
        value.unwrap_or(Value::Undefined)
    }
}

impl TryFrom<&str> for Value {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Ok(data) = value.try_into() {
            Ok(Value::SmallString(data))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for Value {
    type Error = ();
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        // TODO: verify logic
        if value as f32 as f64 == value {
            Ok(Value::FloatNumber(value as f32))
        } else {
            Err(())
        }
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::FloatNumber(value)
    }
}

impl TryFrom<i64> for Value {
    type Error = ();
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Ok(Value::IntegerNumber(SmallInteger::try_from(value)?))
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for Value {
            fn from(value: $size) -> Self {
                let n: i64 = value.into();
                Value::IntegerNumber(SmallInteger::from_i64_unchecked(n))
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
