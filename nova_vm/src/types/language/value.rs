use crate::{
    execution::{Agent, JsResult},
    heap::{
        ArrayHeapData, BigIntHeapData, Handle, NumberHeapData, ObjectHeapData, StringHeapData,
        SymbolHeapData,
    },
    SmallInteger, SmallString,
};

use super::{BigInt, Number};

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

#[derive(Debug, Clone, Copy)]
pub enum PreferredType {
    String,
    Number,
}

impl Value {
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
        matches!(self, Value::Object(_) | Value::ArrayObject(_))
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

    pub fn is_empty_string(self) -> bool {
        if let Value::SmallString(s) = self {
            s.len() == 0
        } else {
            false
        }
    }

    /// 7.1.1 ToPrimitive ( input [ , preferredType ] )
    /// https://tc39.es/ecma262/#sec-toprimitive
    pub fn to_primitive(
        self,
        agent: &mut Agent,
        preferred_type: Option<PreferredType>,
    ) -> JsResult<Value> {
        let input = self;

        // 1. If input is an Object, then
        if input.is_object() {
            // a. Let exoticToPrim be ? GetMethod(input, @@toPrimitive).
            // b. If exoticToPrim is not undefined, then
            // i. If preferredType is not present, then
            // 1. Let hint be "default".
            // ii. Else if preferredType is string, then
            // 1. Let hint be "string".
            // iii. Else,
            // 1. Assert: preferredType is number.
            // 2. Let hint be "number".
            // iv. Let result be ? Call(exoticToPrim, input, « hint »).
            // v. If result is not an Object, return result.
            // vi. Throw a TypeError exception.
            // c. If preferredType is not present, let preferredType be number.
            // d. Return ? OrdinaryToPrimitive(input, preferredType).
            todo!();
        }

        // 2. Return input.
        Ok(input)
    }

    /// 7.1.1.1 OrdinaryToPrimitive ( O, hint )
    /// https://tc39.es/ecma262/#sec-ordinarytoprimitive
    pub fn ordinary_to_primitive(self, agent: &mut Agent, hint: PreferredType) -> JsResult<Value> {
        // TODO: This takes in an object...so probably put it in Object.
        let o = self;

        // 1. If hint is string, then
        let method_names = if matches!(hint, PreferredType::String) {
            // a. Let methodNames be « "toString", "valueOf" ».
            &["toString", "valueOf"]
        }
        // 2. Else,
        else {
            // a. Let methodNames be « "valueOf", "toString" ».
            &["valueOf", "toString"]
        };

        // TODO: 3. For each element name of methodNames, do
        for name in method_names.iter() {
            // a. Let method be ? Get(O, name).
            // b. If IsCallable(method) is true, then
            // i. Let result be ? Call(method, O).
            // ii. If result is not an Object, return result.
            // 4. Throw a TypeError exception.
        }

        todo!()
    }

    /// 7.1.2 ToBoolean ( argument )
    /// https://tc39.es/ecma262/#sec-toboolean
    pub fn to_boolean(self, agent: &mut Agent) -> JsResult<Value> {
        let argument = self;

        // 1. If argument is a Boolean, return argument.
        if argument.is_boolean() {
            return Ok(argument);
        }

        // 2. If argument is one of undefined, null, +0𝔽, -0𝔽, NaN, 0ℤ, or the empty String, return false.
        // TODO: checks for 0ℤ
        if argument.is_undefined()
            || argument.is_null()
            || argument.is_pos_zero(agent)
            || argument.is_neg_zero(agent)
            || argument.is_nan(agent)
            || argument.is_empty_string()
        {
            return Ok(false.into());
        }

        // 3. NOTE: This step is replaced in section B.3.6.1.

        // 4. Return true.
        return Ok(true.into());
    }

    /// 7.1.3 ToNumeric ( value )
    /// https://tc39.es/ecma262/#sec-tonumeric
    pub fn to_numeric(self, agent: &mut Agent) -> JsResult<Value> {
        let value = self;

        // 1. Let primValue be ? ToPrimitive(value, number).
        let prim_value = value.to_primitive(agent, Some(PreferredType::Number))?;

        // 2. If primValue is a BigInt, return primValue.
        if prim_value.is_bigint() {
            return Ok(prim_value);
        }

        // 3. Return ? ToNumber(primValue).
        prim_value.to_number(agent).map(|n| n.into_value())
    }

    /// 7.1.4 ToNumber ( argument )
    /// https://tc39.es/ecma262/#sec-tonumber
    pub fn to_number(self, agent: &mut Agent) -> JsResult<Number> {
        let argument = self;

        // 1. If argument is a Number, return argument.
        if let Ok(argument) = Number::try_from(argument) {
            return Ok(argument);
        }

        // 2. If argument is either a Symbol or a BigInt, throw a TypeError exception.
        if argument.is_symbol() || argument.is_bigint() {
            todo!();
        }

        // 3. If argument is undefined, return NaN.
        if argument.is_undefined() {
            return Ok(Number::nan());
        }

        // 4. If argument is either null or false, return +0𝔽.
        if argument.is_null() || argument.is_false() {
            return Ok(Number::from(0));
        }

        // 5. If argument is true, return 1𝔽.
        if argument.is_true() {
            return Ok(Number::from(1));
        }

        // 6. If argument is a String, return StringToNumber(argument).
        if argument.is_string() {
            todo!();
        }

        // 7. Assert: argument is an Object.
        debug_assert!(argument.is_object());

        // 8. Let primValue be ? ToPrimitive(argument, number).
        let prim_value = argument.to_primitive(agent, Some(PreferredType::Number))?;

        // 9. Assert: primValue is not an Object.
        debug_assert!(!prim_value.is_object());

        // 10. Return ? ToNumber(primValue).
        prim_value.to_number(agent)
    }

    /// 7.1.5 ToIntegerOrInfinity ( argument )
    /// https://tc39.es/ecma262/#sec-tointegerorinfinity
    // TODO: Should we add another [`Value`] newtype for IntegerOrInfinity?
    pub fn to_integer_or_infinty(self, agent: &mut Agent) -> JsResult<Number> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is one of NaN, +0𝔽, or -0𝔽, return 0.
        if number.is_nan(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
            return Ok(Number::pos_zero());
        }

        // 3. If number is +∞𝔽, return +∞.
        if number.is_pos_infinity(agent) {
            return Ok(Number::pos_inf());
        }

        // 4. If number is -∞𝔽, return -∞.
        if number.is_neg_infinity(agent) {
            return Ok(Number::neg_inf());
        }

        // 5. Return truncate(ℝ(number)).
        Ok(Number::from(number.truncate(agent)))
    }

    /// 7.1.6 ToInt32 ( argument )
    /// https://tc39.es/ecma262/#sec-toint32
    pub fn to_int32(self, agent: &mut Agent) -> JsResult<i32> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
        if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
            return Ok(0);
        }

        // 3. Let int be truncate(ℝ(number)).
        let int = number.truncate(agent);

        // 4. Let int32bit be int modulo 2^32.
        let int32bit = int % 2i64.pow(32);

        // 5. If int32bit ≥ 2^31, return 𝔽(int32bit - 2^32); otherwise return 𝔽(int32bit).
        Ok(if int32bit >= 2i64.pow(32) {
            int32bit - 2i64.pow(32)
        } else {
            int32bit
        } as i32)
    }

    /// 7.1.7 ToUint32 ( argument )
    /// https://tc39.es/ecma262/#sec-touint32
    pub fn to_uint32(self, agent: &mut Agent) -> JsResult<u32> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
        if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
            return Ok(0);
        }

        // 3. Let int be truncate(ℝ(number)).
        let int = number.truncate(agent);

        // 4. Let int32bit be int modulo 2^32.
        let int32bit = int % 2i64.pow(32);

        // 5. Return 𝔽(int32bit).
        Ok(int32bit as u32)
    }

    /// 7.1.8 ToInt16 ( argument )
    /// https://tc39.es/ecma262/#sec-toint16
    pub fn to_int16(self, agent: &mut Agent) -> JsResult<i16> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
        if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
            return Ok(0);
        }

        // 3. Let int be truncate(ℝ(number)).
        let int = number.truncate(agent);

        // 4. Let int16bit be int modulo 2^16.
        let int16bit = int % 2i64.pow(16);

        // 5. If int16bit ≥ 2^15, return 𝔽(int16bit - 2^16); otherwise return 𝔽(int16bit).
        Ok(if int16bit >= 2i64.pow(15) {
            int16bit - 2i64.pow(16)
        } else {
            int16bit
        } as i16)
    }

    /// 7.1.9 ToUint16 ( argument )
    /// https://tc39.es/ecma262/#sec-touint16
    pub fn to_uint16(self, agent: &mut Agent) -> JsResult<i16> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
        if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
            return Ok(0);
        }

        // 3. Let int be truncate(ℝ(number)).
        let int = number.truncate(agent);

        // 4. Let int16bit be int modulo 2^16.
        let int16bit = int % 2i64.pow(16);

        // Return 𝔽(int16bit).
        Ok(int16bit as i16)
    }

    /// 7.1.10 ToInt8 ( argument )
    /// https://tc39.es/ecma262/#sec-toint8
    pub fn to_int8(self, agent: &mut Agent) -> JsResult<i8> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
        if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
            return Ok(0);
        }

        // 3. Let int be truncate(ℝ(number)).
        let int = number.truncate(agent);

        // 4. Let int8bit be int modulo 2^8.
        let int8bit = int % 2i64.pow(8);

        // 5. If int8bit ≥ 2^7, return 𝔽(int8bit - 2^8); otherwise return 𝔽(int8bit).
        Ok(if int8bit >= 2i64.pow(7) {
            int8bit - 2i64.pow(8)
        } else {
            int8bit
        } as i8)
    }

    /// 7.1.11 ToUint8 ( argument )
    /// https://tc39.es/ecma262/#sec-touint8
    pub fn to_uint8(self, agent: &mut Agent) -> JsResult<u8> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
        if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
            return Ok(0);
        }

        // 3. Let int be truncate(ℝ(number)).
        let int = number.truncate(agent);

        // 4. Let int8bit be int modulo 2^8.
        let int8bit = int % 2i64.pow(8);

        // 5. Return 𝔽(int8bit).
        Ok(int8bit as u8)
    }

    /// 7.1.12 ToUint8Clamp ( argument )
    /// https://tc39.es/ecma262/#sec-touint8clamp
    pub fn to_uint8_clamp(self, agent: &mut Agent) -> JsResult<u8> {
        let argument = self;

        // 1. Let number be ? ToNumber(argument).
        let number = argument.to_number(agent)?;

        // 2. If number is NaN, return +0𝔽.
        if number.is_nan(agent) {
            return Ok(0);
        }

        // 3. Let mv be the extended mathematical value of number.
        // TODO: Is there a better way?
        let mv = number.into_f64(agent);

        // 4. Let clamped be the result of clamping mv between 0 and 255.
        let clamped = mv.clamp(0.0, 255.0);

        // 5. Let f be floor(clamped).
        let f = clamped.floor();

        Ok(
            // 6. If clamped < f + 0.5, return 𝔽(f).
            if clamped < f + 0.5 {
                f as u8
            }
            // 7. If clamped > f + 0.5, return 𝔽(f + 1).
            else if clamped > f + 0.5 {
                f as u8 + 1
            }
            // 8. If f is even, return 𝔽(f). Otherwise, return 𝔽(f + 1).
            else if f % 2.0 == 0.0 {
                f as u8
            } else {
                f as u8 + 1
            },
        )
    }

    /// 7.1.13 ToBigInt ( argument )
    /// https://tc39.es/ecma262/#sec-tobigint
    pub fn to_big_int(self, agent: &mut Agent) -> JsResult<BigInt> {
        let argument = self;

        // 1. Let prim be ? ToPrimitive(argument, number).
        let prim = argument.to_primitive(agent, Some(PreferredType::Number))?;

        // 2. Return the value that prim corresponds to in Table 12.
        todo!()
    }

    /// 7.1.17 ToString ( argument )
    /// https://tc39.es/ecma262/#sec-tostring
    pub fn to_string(self, agent: &mut Agent) -> JsResult<String> {
        let argument = self;

        // TODO: 1. If argument is a String, return argument.
        // 2. If argument is a Symbol, throw a TypeError exception.
        // 3. If argument is undefined, return "undefined".
        // 4. If argument is null, return "null".
        // 5. If argument is true, return "true".
        // 6. If argument is false, return "false".
        // 7. If argument is a Number, return Number::toString(argument, 10).
        // 8. If argument is a BigInt, return BigInt::toString(argument, 10).
        // 9. Assert: argument is an Object.
        // 10. Let primValue be ? ToPrimitive(argument, string).
        // 11. Assert: primValue is not an Object.
        // 12. Return ? ToString(primValue).

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
