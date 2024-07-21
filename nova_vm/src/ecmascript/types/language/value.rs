// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::mem::size_of;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int, to_int32, to_number, to_numeric, to_string, to_uint32,
        },
        builtins::{
            bound_function::BoundFunction, control_abstraction_objects::promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction, data_view::DataView, date::Date, embedder_object::EmbedderObject, error::Error, finalization_registry::FinalizationRegistry, map::Map, module::Module, primitive_objects::PrimitiveObject, promise::Promise, proxy::Proxy, regexp::RegExp, set::Set, shared_array_buffer::SharedArrayBuffer, weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet, Array, ArrayBuffer, BuiltinFunction, ECMAScriptFunction
        },
        execution::{Agent, JsResult},
        types::BUILTIN_STRING_MEMORY,
    }, engine::small_f64::SmallF64, heap::{indexes::TypedArrayIndex, CompactionLists, HeapMarkAndSweep, WorkQueues}, SmallInteger, SmallString
};

use super::{
    bigint::{HeapBigInt, SmallBigInt},
    number::HeapNumber,
    string::HeapString,
    BigInt, IntoValue, Number, Numeric, OrdinaryObject, String, Symbol,
};

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
    ///
    /// UTF-8 string on the heap. Accessing the data must be done through the
    /// Agent. ECMAScript specification compliant UTF-16 indexing is
    /// implemented through an index mapping.
    String(HeapString),
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// 7-byte UTF-8 string on the stack. End of the string is determined by
    /// the first 0xFF byte in the data. UTF-16 indexing is calculated on
    /// demand from the data.
    SmallString(SmallString),

    /// ### [6.1.5 The Symbol Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type)
    Symbol(Symbol),

    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber),
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 53-bit signed integer on the stack.
    Integer(SmallInteger),
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 56-bit f64 on the stack. The missing byte is a zero least significant
    /// byte.
    SmallF64(SmallF64),

    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// Unlimited size integer data on the heap. Accessing the data must be
    /// done through the Agent.
    BigInt(HeapBigInt),
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// 56-bit signed integer on the stack.
    SmallBigInt(SmallBigInt),

    /// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
    Object(OrdinaryObject),

    // Functions
    BoundFunction(BoundFunction),
    BuiltinFunction(BuiltinFunction),
    ECMAScriptFunction(ECMAScriptFunction),
    // TODO: Figure out if all the special function types are wanted or if we'd
    // prefer to just keep them as internal variants of the three above ones.
    BuiltinGeneratorFunction,
    BuiltinConstructorFunction,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction),
    BuiltinPromiseCollectorFunction,
    BuiltinProxyRevokerFunction,
    ECMAScriptAsyncFunction,
    ECMAScriptAsyncGeneratorFunction,
    ECMAScriptConstructorFunction,
    ECMAScriptGeneratorFunction,

    // Boolean, Number, String, Symbol, BigInt objects
    PrimitiveObject(PrimitiveObject),

    // Well-known object types
    // Roughly corresponding to 6.1.7.4 Well-Known Intrinsic Objects
    // https://tc39.es/ecma262/#sec-well-known-intrinsic-objects
    // and 18 ECMAScript Standard Built-in Objects
    // https://tc39.es/ecma262/#sec-ecmascript-standard-built-in-objects
    Arguments,
    Array(Array),
    ArrayBuffer(ArrayBuffer),
    DataView(DataView),
    Date(Date),
    Error(Error),
    FinalizationRegistry(FinalizationRegistry),
    Map(Map),
    Promise(Promise),
    Proxy(Proxy),
    RegExp(RegExp),
    Set(Set),
    SharedArrayBuffer(SharedArrayBuffer),
    WeakMap(WeakMap),
    WeakRef(WeakRef),
    WeakSet(WeakSet),

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
    Module(Module),

    // Embedder objects
    EmbedderObject(EmbedderObject) = 0x7f,
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
pub(crate) const STRING_DISCRIMINANT: u8 = value_discriminant(Value::String(HeapString::_def()));
pub(crate) const SMALL_STRING_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallString(SmallString::EMPTY));
pub(crate) const SYMBOL_DISCRIMINANT: u8 = value_discriminant(Value::Symbol(Symbol::_def()));
pub(crate) const NUMBER_DISCRIMINANT: u8 = value_discriminant(Value::Number(HeapNumber::_def()));
pub(crate) const INTEGER_DISCRIMINANT: u8 =
    value_discriminant(Value::Integer(SmallInteger::zero()));
pub(crate) const FLOAT_DISCRIMINANT: u8 = value_discriminant(Value::SmallF64(SmallF64::_def()));
pub(crate) const BIGINT_DISCRIMINANT: u8 = value_discriminant(Value::BigInt(HeapBigInt::_def()));
pub(crate) const SMALL_BIGINT_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallBigInt(SmallBigInt::zero()));
pub(crate) const OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::Object(OrdinaryObject::_def()));
pub(crate) const ARRAY_DISCRIMINANT: u8 = value_discriminant(Value::Array(Array::_def()));
pub(crate) const ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayBuffer(ArrayBuffer::_def()));
pub(crate) const DATE_DISCRIMINANT: u8 = value_discriminant(Value::Date(Date::_def()));
pub(crate) const ERROR_DISCRIMINANT: u8 = value_discriminant(Value::Error(Error::_def()));
pub(crate) const BUILTIN_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinFunction(BuiltinFunction::_def()));
pub(crate) const ECMASCRIPT_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::ECMAScriptFunction(ECMAScriptFunction::_def()));
pub(crate) const BOUND_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BoundFunction(BoundFunction::_def()));
pub(crate) const REGEXP_DISCRIMINANT: u8 = value_discriminant(Value::RegExp(RegExp::_def()));

pub(crate) const BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinGeneratorFunction);
pub(crate) const BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinConstructorFunction);
pub(crate) const BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction::_def()),
);
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
pub(crate) const PRIMITIVE_OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::PrimitiveObject(PrimitiveObject::_def()));
pub(crate) const ARGUMENTS_DISCRIMINANT: u8 = value_discriminant(Value::Arguments);
pub(crate) const DATA_VIEW_DISCRIMINANT: u8 = value_discriminant(Value::DataView(DataView::_def()));
pub(crate) const FINALIZATION_REGISTRY_DISCRIMINANT: u8 =
    value_discriminant(Value::FinalizationRegistry(FinalizationRegistry::_def()));
pub(crate) const MAP_DISCRIMINANT: u8 = value_discriminant(Value::Map(Map::_def()));
pub(crate) const PROMISE_DISCRIMINANT: u8 = value_discriminant(Value::Promise(Promise::_def()));
pub(crate) const PROXY_DISCRIMINANT: u8 = value_discriminant(Value::Proxy(Proxy::_def()));
pub(crate) const SET_DISCRIMINANT: u8 = value_discriminant(Value::Set(Set::_def()));
pub(crate) const SHARED_ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedArrayBuffer(SharedArrayBuffer::_def()));
pub(crate) const WEAK_MAP_DISCRIMINANT: u8 = value_discriminant(Value::WeakMap(WeakMap::_def()));
pub(crate) const WEAK_REF_DISCRIMINANT: u8 = value_discriminant(Value::WeakRef(WeakRef::_def()));
pub(crate) const WEAK_SET_DISCRIMINANT: u8 = value_discriminant(Value::WeakSet(WeakSet::_def()));
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
pub(crate) const MODULE_DISCRIMINANT: u8 = value_discriminant(Value::Module(Module::_def()));
pub(crate) const EMBEDDER_OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::EmbedderObject(EmbedderObject::_def()));

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
        Number::from_f64(agent, value).into_value()
    }

    pub fn nan() -> Self {
        Number::nan().into_value()
    }

    pub fn pos_inf() -> Self {
        Number::pos_inf().into_value()
    }

    pub fn neg_inf() -> Self {
        Number::neg_inf().into_value()
    }

    pub fn pos_zero() -> Self {
        Number::pos_zero().into_value()
    }

    pub fn neg_zero() -> Self {
        Number::neg_zero().into_value()
    }

    pub fn is_true(self) -> bool {
        matches!(self, Value::Boolean(true))
    }

    pub fn is_false(self) -> bool {
        matches!(self, Value::Boolean(false))
    }

    pub fn is_object(self) -> bool {
        super::Object::try_from(self).is_ok()
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
        matches!(self, Value::Boolean(_))
    }

    pub fn is_null(self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_undefined(self) -> bool {
        matches!(self, Value::Undefined)
    }

    pub fn is_pos_zero(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_pos_zero(agent))
            .unwrap_or(false)
    }

    pub fn is_neg_zero(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_neg_zero(agent))
            .unwrap_or(false)
    }

    pub fn is_pos_infinity(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_pos_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_neg_infinity(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_neg_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_nan(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_nan(agent))
            .unwrap_or(false)
    }

    pub fn is_bigint(self) -> bool {
        matches!(self, Value::BigInt(_) | Value::SmallBigInt(_))
    }

    pub fn is_symbol(self) -> bool {
        matches!(self, Value::Symbol(_))
    }

    pub fn is_number(self) -> bool {
        matches!(
            self,
            Value::Number(_) | Value::SmallF64(_) | Value::Integer(_)
        )
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

    pub fn to_string(self, agent: &mut Agent) -> JsResult<String> {
        to_string(agent, self)
    }

    /// A string conversion that will never throw, meant for things like
    /// displaying exceptions.
    pub fn string_repr(self, agent: &mut Agent) -> String {
        if let Value::Symbol(symbol_idx) = self {
            // ToString of a symbol always throws. We use the descriptive
            // string instead (the result of `String(symbol)`).
            return symbol_idx.descriptive_string(agent);
        };
        match self.to_string(agent) {
            Ok(result) => result,
            Err(_) => {
                debug_assert!(self.is_object());
                BUILTIN_STRING_MEMORY.Object
            }
        }
    }

    /// ### [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub fn to_real(self, agent: &mut Agent) -> JsResult<f64> {
        Ok(match self {
            Value::Number(n) => agent[n],
            Value::Integer(i) => i.into_i64() as f64,
            Value::SmallF64(f) => f.into_f64(),
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
        Value::SmallF64(SmallF64::from(value))
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

impl IntoValue for Value {
    #[inline(always)]
    fn into_value(self) -> Value {
        self
    }
}

impl HeapMarkAndSweep for Value {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::SmallString(_)
            | Value::Integer(_)
            | Value::SmallF64(_)
            | Value::SmallBigInt(_) => {
                // Stack values: Nothing to mark
            }
            Value::String(idx) => idx.mark_values(queues),
            Value::Symbol(idx) => idx.mark_values(queues),
            Value::Number(idx) => idx.mark_values(queues),
            Value::BigInt(idx) => idx.mark_values(queues),
            Value::Object(idx) => idx.mark_values(queues),
            Value::Array(idx) => idx.mark_values(queues),
            Value::ArrayBuffer(idx) => idx.mark_values(queues),
            Value::Date(idx) => idx.mark_values(queues),
            Value::Error(idx) => idx.mark_values(queues),
            Value::BoundFunction(idx) => idx.mark_values(queues),
            Value::BuiltinFunction(idx) => idx.mark_values(queues),
            Value::ECMAScriptFunction(idx) => idx.mark_values(queues),
            Value::RegExp(idx) => idx.mark_values(queues),
            Value::PrimitiveObject(idx) => idx.mark_values(queues),
            Value::Arguments => todo!(),
            Value::DataView(data) => data.mark_values(queues),
            Value::FinalizationRegistry(data) => data.mark_values(queues),
            Value::Map(data) => data.mark_values(queues),
            Value::Proxy(data) => data.mark_values(queues),
            Value::Promise(data) => data.mark_values(queues),
            Value::Set(data) => data.mark_values(queues),
            Value::SharedArrayBuffer(data) => data.mark_values(queues),
            Value::WeakMap(data) => data.mark_values(queues),
            Value::WeakRef(data) => data.mark_values(queues),
            Value::WeakSet(data) => data.mark_values(queues),
            Value::Int8Array(data) => data.mark_values(queues),
            Value::Uint8Array(data) => data.mark_values(queues),
            Value::Uint8ClampedArray(data) => data.mark_values(queues),
            Value::Int16Array(data) => data.mark_values(queues),
            Value::Uint16Array(data) => data.mark_values(queues),
            Value::Int32Array(data) => data.mark_values(queues),
            Value::Uint32Array(data) => data.mark_values(queues),
            Value::BigInt64Array(data) => data.mark_values(queues),
            Value::BigUint64Array(data) => data.mark_values(queues),
            Value::Float32Array(data) => data.mark_values(queues),
            Value::Float64Array(data) => data.mark_values(queues),
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction => todo!(),
            Value::BuiltinPromiseResolvingFunction(data) => data.mark_values(queues),
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::ECMAScriptAsyncFunction => todo!(),
            Value::ECMAScriptAsyncGeneratorFunction => todo!(),
            Value::ECMAScriptConstructorFunction => todo!(),
            Value::ECMAScriptGeneratorFunction => todo!(),
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncIterator => todo!(),
            Value::Iterator => todo!(),
            Value::Module(data) => data.mark_values(queues),
            Value::EmbedderObject(data) => data.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::SmallString(_)
            | Value::Integer(_)
            | Value::SmallF64(_)
            | Value::SmallBigInt(_) => {
                // Stack values: Nothing to sweep
            }
            Value::String(idx) => idx.sweep_values(compactions),
            Value::Symbol(idx) => idx.sweep_values(compactions),
            Value::Number(idx) => idx.sweep_values(compactions),
            Value::BigInt(idx) => idx.sweep_values(compactions),
            Value::Object(idx) => idx.sweep_values(compactions),
            Value::Array(idx) => idx.sweep_values(compactions),
            Value::ArrayBuffer(idx) => idx.sweep_values(compactions),
            Value::Date(idx) => idx.sweep_values(compactions),
            Value::Error(idx) => idx.sweep_values(compactions),
            Value::BoundFunction(idx) => idx.sweep_values(compactions),
            Value::BuiltinFunction(idx) => idx.sweep_values(compactions),
            Value::ECMAScriptFunction(idx) => idx.sweep_values(compactions),
            Value::RegExp(idx) => idx.sweep_values(compactions),
            Value::PrimitiveObject(idx) => idx.sweep_values(compactions),
            Value::Arguments => todo!(),
            Value::DataView(data) => data.sweep_values(compactions),
            Value::FinalizationRegistry(data) => data.sweep_values(compactions),
            Value::Map(data) => data.sweep_values(compactions),
            Value::Proxy(data) => data.sweep_values(compactions),
            Value::Promise(data) => data.sweep_values(compactions),
            Value::Set(data) => data.sweep_values(compactions),
            Value::SharedArrayBuffer(data) => data.sweep_values(compactions),
            Value::WeakMap(data) => data.sweep_values(compactions),
            Value::WeakRef(data) => data.sweep_values(compactions),
            Value::WeakSet(data) => data.sweep_values(compactions),
            Value::Int8Array(data) => data.sweep_values(compactions),
            Value::Uint8Array(data) => data.sweep_values(compactions),
            Value::Uint8ClampedArray(data) => data.sweep_values(compactions),
            Value::Int16Array(data) => data.sweep_values(compactions),
            Value::Uint16Array(data) => data.sweep_values(compactions),
            Value::Int32Array(data) => data.sweep_values(compactions),
            Value::Uint32Array(data) => data.sweep_values(compactions),
            Value::BigInt64Array(data) => data.sweep_values(compactions),
            Value::BigUint64Array(data) => data.sweep_values(compactions),
            Value::Float32Array(data) => data.sweep_values(compactions),
            Value::Float64Array(data) => data.sweep_values(compactions),
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction => todo!(),
            Value::BuiltinPromiseResolvingFunction(data) => data.sweep_values(compactions),
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::ECMAScriptAsyncFunction => todo!(),
            Value::ECMAScriptAsyncGeneratorFunction => todo!(),
            Value::ECMAScriptConstructorFunction => todo!(),
            Value::ECMAScriptGeneratorFunction => todo!(),
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncIterator => todo!(),
            Value::Iterator => todo!(),
            Value::Module(data) => data.sweep_values(compactions),
            Value::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}
