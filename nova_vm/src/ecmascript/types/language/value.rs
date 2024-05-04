use std::mem::size_of;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int, to_int32, to_number, to_numeric, to_uint32,
        }, builtins::control_abstraction_objects::promise_objects::promise_abstract_operations::BuiltinPromiseRejectFunctionIndex, execution::{Agent, JsResult}, scripts_and_modules::module::ModuleIdentifier
    },
    heap::indexes::{
        ArrayBufferIndex, ArrayIndex, BigIntIndex, BoundFunctionIndex, BuiltinFunctionIndex,
        DataViewIndex, DateIndex, ECMAScriptFunctionIndex, EmbedderObjectIndex, ErrorIndex,
        FinalizationRegistryIndex, MapIndex, NumberIndex, ObjectIndex, PrimitiveObjectIndex,
        PromiseIndex, ProxyIndex, RegExpIndex, SetIndex, SharedArrayBufferIndex, StringIndex,
        SymbolIndex, TypedArrayIndex, WeakMapIndex, WeakRefIndex, WeakSetIndex,
    },
    SmallInteger, SmallString,
};

use super::{BigInt, IntoValue, Number, Numeric, String};

/// ### [6.1 ECMAScript Language Types](https://tc39.es/ecma262/#sec-ecmascript-language-types)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(u8)]
pub enum Value {
    /// ### [6.1.1 The Undefined Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type)
    #[default]
    Undefined = 1,

    /// ### [6.1.2 The Null Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-null-type)
    Null,

    /// ### [6.1.3 The Boolean Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-boolean-type)
    Boolean(bool),

    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    String(StringIndex),
    SmallString(SmallString),

    /// ### [6.1.5 The Symbol Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type)
    Symbol(SymbolIndex),

    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    Number(NumberIndex),
    Integer(SmallInteger), // 56-bit signed integer.
    Float(f32),

    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    BigInt(BigIntIndex),
    SmallBigInt(SmallInteger),

    /// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
    Object(ObjectIndex),

    // Functions
    BoundFunction(BoundFunctionIndex),
    BuiltinFunction(BuiltinFunctionIndex),
    ECMAScriptFunction(ECMAScriptFunctionIndex),
    // TODO: Figure out if all the special function types are wanted or if we'd
    // prefer to just keep them as internal variants of the three above ones.
    BuiltinGeneratorFunction,
    BuiltinConstructorFunction,
    BuiltinPromiseResolveFunction,
    BuiltinPromiseRejectFunction(BuiltinPromiseRejectFunctionIndex),
    BuiltinPromiseCollectorFunction,
    BuiltinProxyRevokerFunction,
    ECMAScriptAsyncFunction,
    ECMAScriptAsyncGeneratorFunction,
    ECMAScriptConstructorFunction,
    ECMAScriptGeneratorFunction,

    // Boolean, Number, String, Symbol, BigInt objects
    PrimitiveObject(PrimitiveObjectIndex),

    // Well-known object types
    // Roughly corresponding to 6.1.7.4 Well-Known Intrinsic Objects
    // https://tc39.es/ecma262/#sec-well-known-intrinsic-objects
    // and 18 ECMAScript Standard Built-in Objects
    // https://tc39.es/ecma262/#sec-ecmascript-standard-built-in-objects
    Arguments,
    Array(ArrayIndex),
    ArrayBuffer(ArrayBufferIndex),
    DataView(DataViewIndex),
    Date(DateIndex),
    Error(ErrorIndex),
    FinalizationRegistry(FinalizationRegistryIndex),
    Map(MapIndex),
    Promise(PromiseIndex),
    Proxy(ProxyIndex),
    RegExp(RegExpIndex),
    Set(SetIndex),
    SharedArrayBuffer(SharedArrayBufferIndex),
    WeakMap(WeakMapIndex),
    WeakRef(WeakRefIndex),
    WeakSet(WeakSetIndex),

    // TypedArrays
    Int8Array(TypedArrayIndex),
    Uint8Array(TypedArrayIndex),
    Uint8ClampedArray(TypedArrayIndex),
    Int16Array(TypedArrayIndex),
    Uint16Array(TypedArrayIndex),
    Int32Array(TypedArrayIndex),
    Uint32Array(TypedArrayIndex),
    BigInt64Array(TypedArrayIndex),
    BigUint64Array(TypedArrayIndex),
    Float32Array(TypedArrayIndex),
    Float64Array(TypedArrayIndex),

    // Iterator objects
    // TODO: Figure out if these are needed at all.
    AsyncFromSyncIterator,
    AsyncIterator,
    Iterator,

    // ECMAScript Module
    Module(ModuleIdentifier),

    // Embedder objects
    EmbedderObject(EmbedderObjectIndex) = 0x7f,
}

/// We want to guarantee that all handles to JS values are register sized. This
/// assert must never be removed or broken.
const _VALUE_SIZE_IS_WORD: () = assert!(size_of::<Value>() == size_of::<usize>());
/// We may also want to keep Option<Value> register sized so that eg. holes in
/// arrays do not start requiring extra bookkeeping.
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
    value_discriminant(Value::SmallString(SmallString::EMPTY));
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
pub(crate) const BUILTIN_FUNCTION_DISCRIMINANT: u8 = value_discriminant(Value::BuiltinFunction(
    BuiltinFunctionIndex::from_u32_index(0),
));
pub(crate) const ECMASCRIPT_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::ECMAScriptFunction(ECMAScriptFunctionIndex::from_u32_index(0)),
);
pub(crate) const BOUND_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BoundFunction(BoundFunctionIndex::from_u32_index(0)));
pub(crate) const REGEXP_DISCRIMINANT: u8 =
    value_discriminant(Value::RegExp(RegExpIndex::from_u32_index(0)));

pub(crate) const BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinGeneratorFunction);
pub(crate) const BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinConstructorFunction);
pub(crate) const BUILTIN_PROMISE_RESOLVE_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinPromiseResolveFunction);
pub(crate) const BUILTIN_PROMISE_REJECT_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinPromiseRejectFunction(BuiltinPromiseRejectFunctionIndex::from_u32_index(0)));
pub(crate) const BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinPromiseCollectorFunction);
pub(crate) const BUILTIN_PROXY_REVOKER_FUNCTION: u8 =
    value_discriminant(Value::BuiltinProxyRevokerFunction);
pub(crate) const ECMASCRIPT_ASYNC_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::ECMAScriptAsyncFunction);
pub(crate) const ECMASCRIPT_ASYNC_GENERATOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::ECMAScriptAsyncGeneratorFunction);
pub(crate) const ECMASCRIPT_CONSTRUCTOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::ECMAScriptConstructorFunction);
pub(crate) const ECMASCRIPT_GENERATOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::ECMAScriptGeneratorFunction);
pub(crate) const PRIMITIVE_OBJECT_DISCRIMINANT: u8 = value_discriminant(Value::PrimitiveObject(
    PrimitiveObjectIndex::from_u32_index(0),
));
pub(crate) const ARGUMENTS_DISCRIMINANT: u8 = value_discriminant(Value::Arguments);
pub(crate) const DATA_VIEW_DISCRIMINANT: u8 =
    value_discriminant(Value::DataView(DataViewIndex::from_u32_index(0)));
pub(crate) const FINALIZATION_REGISTRY_DISCRIMINANT: u8 = value_discriminant(
    Value::FinalizationRegistry(FinalizationRegistryIndex::from_u32_index(0)),
);
pub(crate) const MAP_DISCRIMINANT: u8 = value_discriminant(Value::Map(MapIndex::from_u32_index(0)));
pub(crate) const PROMISE_DISCRIMINANT: u8 =
    value_discriminant(Value::Promise(PromiseIndex::from_u32_index(0)));
pub(crate) const PROXY_DISCRIMINANT: u8 =
    value_discriminant(Value::Proxy(ProxyIndex::from_u32_index(0)));
pub(crate) const SET_DISCRIMINANT: u8 = value_discriminant(Value::Set(SetIndex::from_u32_index(0)));
pub(crate) const SHARED_ARRAY_BUFFER_DISCRIMINANT: u8 = value_discriminant(
    Value::SharedArrayBuffer(SharedArrayBufferIndex::from_u32_index(0)),
);
pub(crate) const WEAK_MAP_DISCRIMINANT: u8 =
    value_discriminant(Value::WeakMap(WeakMapIndex::from_u32_index(0)));
pub(crate) const WEAK_REF_DISCRIMINANT: u8 =
    value_discriminant(Value::WeakRef(WeakRefIndex::from_u32_index(0)));
pub(crate) const WEAK_SET_DISCRIMINANT: u8 =
    value_discriminant(Value::WeakSet(WeakSetIndex::from_u32_index(0)));
pub(crate) const INT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int8Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const UINT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const UINT_8_CLAMPED_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8ClampedArray(TypedArrayIndex::from_u32_index(0)));
pub(crate) const INT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int16Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const UINT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint16Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const INT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int32Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const UINT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint32Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const BIGINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigInt64Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const BIGUINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigUint64Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const FLOAT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float32Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const FLOAT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float64Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::AsyncFromSyncIterator);
pub(crate) const ASYNC_ITERATOR_DISCRIMINANT: u8 = value_discriminant(Value::AsyncIterator);
pub(crate) const ITERATOR_DISCRIMINANT: u8 = value_discriminant(Value::Iterator);
pub(crate) const MODULE_DISCRIMINANT: u8 =
    value_discriminant(Value::Module(ModuleIdentifier::from_u32(0)));
pub(crate) const EMBEDDER_OBJECT_DISCRIMINANT: u8 = value_discriminant(Value::EmbedderObject(
    EmbedderObjectIndex::from_u32_index(0),
));

impl Value {
    pub fn from_str(agent: &mut Agent, str: &str) -> Value {
        String::from_str(agent, str).into_value()
    }

    pub fn from_string(agent: &mut Agent, string: std::string::String) -> Value {
        String::from_string(agent, string).into_value()
    }

    pub fn from_static_str(agent: &mut Agent, str: &'static str) -> Value {
        String::from_static_str(agent, str).into_value()
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
                | Value::BuiltinFunction(_)
                | Value::ECMAScriptFunction(_)
                | Value::BoundFunction(_)
                | Value::Error(_)
                | Value::RegExp(_)
        )
    }

    pub fn is_function(self) -> bool {
        matches!(
            self,
            Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_)
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

    pub fn to_numeric(self, agent: &mut Agent) -> JsResult<Numeric> {
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
            Value::Number(n) => agent[n],
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

impl TryFrom<Value> for bool {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Boolean(bool) => Ok(bool),
            _ => Err(()),
        }
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
