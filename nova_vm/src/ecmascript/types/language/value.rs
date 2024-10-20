// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{
    bigint::{HeapBigInt, SmallBigInt},
    number::HeapNumber,
    string::HeapString,
    BigInt, BigIntHeapData, IntoValue, Number, Numeric, OrdinaryObject, String, StringHeapData,
    Symbol,
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
#[cfg(feature = "array-buffer")]
use crate::{
    ecmascript::builtins::{data_view::DataView, ArrayBuffer},
    heap::indexes::TypedArrayIndex,
};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int, to_int16, to_int32, to_number, to_numeric, to_string, to_uint16, to_uint32,
        },
        builtins::{
            bound_function::BoundFunction,
            control_abstraction_objects::{
                generator_objects::Generator,
                promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction,
            },
            embedder_object::EmbedderObject,
            error::Error,
            finalization_registry::FinalizationRegistry,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator,
            keyed_collections::{
                map_objects::map_iterator_objects::map_iterator::MapIterator,
                set_objects::set_iterator_objects::set_iterator::SetIterator,
            },
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            proxy::Proxy,
            regexp::RegExp,
            set::Set,
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
        },
        execution::{Agent, JsResult},
        types::BUILTIN_STRING_MEMORY,
    },
    engine::{
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_f64::SmallF64,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
    SmallInteger, SmallString,
};

use std::{
    hash::{Hash, Hasher},
    mem::size_of,
    ops::Index,
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
    /// Default class constructor created in step 14 of
    /// [ClassDefinitionEvaluation](https://tc39.es/ecma262/#sec-runtime-semantics-classdefinitionevaluation).
    BuiltinConstructorFunction(BuiltinConstructorFunction),
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction),
    BuiltinPromiseCollectorFunction,
    BuiltinProxyRevokerFunction,

    // Boolean, Number, String, Symbol, BigInt objects
    PrimitiveObject(PrimitiveObject),

    // Well-known object types
    // Roughly corresponding to 6.1.7.4 Well-Known Intrinsic Objects
    // https://tc39.es/ecma262/#sec-well-known-intrinsic-objects
    // and 18 ECMAScript Standard Built-in Objects
    // https://tc39.es/ecma262/#sec-ecmascript-standard-built-in-objects
    /// ### [10.4.4 Arguments Exotic Objects](https://tc39.es/ecma262/#sec-arguments-exotic-objects)
    ///
    /// An unmapped arguments object is an ordinary object with an additional
    /// internal slot \[\[ParameterMap]] whose value is always **undefined**.
    Arguments(OrdinaryObject),
    // TODO: MappedArguments(MappedArgumentsObject),
    Array(Array),
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer),
    #[cfg(feature = "array-buffer")]
    DataView(DataView),
    #[cfg(feature = "date")]
    Date(Date),
    Error(Error),
    FinalizationRegistry(FinalizationRegistry),
    Map(Map),
    Promise(Promise),
    Proxy(Proxy),
    RegExp(RegExp),
    Set(Set),
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer),
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap),
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef),
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet),

    // TypedArrays
    #[cfg(feature = "array-buffer")]
    Int8Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Uint8Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Int16Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Uint16Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Int32Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Uint32Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    BigInt64Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    BigUint64Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Float32Array(TypedArrayIndex),
    #[cfg(feature = "array-buffer")]
    Float64Array(TypedArrayIndex),

    // Iterator objects
    // TODO: Figure out if these are needed at all.
    AsyncFromSyncIterator,
    AsyncIterator,
    Iterator,
    ArrayIterator(ArrayIterator),
    SetIterator(SetIterator),
    MapIterator(MapIterator),
    Generator(Generator),

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
#[cfg(feature = "array-buffer")]
pub(crate) const ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayBuffer(ArrayBuffer::_def()));
#[cfg(feature = "date")]
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
pub(crate) const BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinConstructorFunction(BuiltinConstructorFunction::_def()),
);
pub(crate) const BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction::_def()),
);
pub(crate) const BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinPromiseCollectorFunction);
pub(crate) const BUILTIN_PROXY_REVOKER_FUNCTION: u8 =
    value_discriminant(Value::BuiltinProxyRevokerFunction);
pub(crate) const PRIMITIVE_OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::PrimitiveObject(PrimitiveObject::_def()));
pub(crate) const ARGUMENTS_DISCRIMINANT: u8 =
    value_discriminant(Value::Arguments(OrdinaryObject::_def()));
#[cfg(feature = "array-buffer")]
pub(crate) const DATA_VIEW_DISCRIMINANT: u8 = value_discriminant(Value::DataView(DataView::_def()));
pub(crate) const FINALIZATION_REGISTRY_DISCRIMINANT: u8 =
    value_discriminant(Value::FinalizationRegistry(FinalizationRegistry::_def()));
pub(crate) const MAP_DISCRIMINANT: u8 = value_discriminant(Value::Map(Map::_def()));
pub(crate) const PROMISE_DISCRIMINANT: u8 = value_discriminant(Value::Promise(Promise::_def()));
pub(crate) const PROXY_DISCRIMINANT: u8 = value_discriminant(Value::Proxy(Proxy::_def()));
pub(crate) const SET_DISCRIMINANT: u8 = value_discriminant(Value::Set(Set::_def()));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedArrayBuffer(SharedArrayBuffer::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_MAP_DISCRIMINANT: u8 = value_discriminant(Value::WeakMap(WeakMap::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_REF_DISCRIMINANT: u8 = value_discriminant(Value::WeakRef(WeakRef::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_SET_DISCRIMINANT: u8 = value_discriminant(Value::WeakSet(WeakSet::_def()));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int8Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_8_CLAMPED_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8ClampedArray(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int16Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint16Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int32Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint32Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const BIGINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigInt64Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const BIGUINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigUint64Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const FLOAT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float32Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const FLOAT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float64Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::AsyncFromSyncIterator);
pub(crate) const ASYNC_ITERATOR_DISCRIMINANT: u8 = value_discriminant(Value::AsyncIterator);
pub(crate) const ITERATOR_DISCRIMINANT: u8 = value_discriminant(Value::Iterator);
pub(crate) const ARRAY_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayIterator(ArrayIterator::_def()));
pub(crate) const SET_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::SetIterator(SetIterator::_def()));
pub(crate) const MAP_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::MapIterator(MapIterator::_def()));
pub(crate) const GENERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::Generator(Generator::_def()));
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

    pub fn to_int16(self, agent: &mut Agent) -> JsResult<i16> {
        to_int16(agent, self)
    }

    pub fn to_uint16(self, agent: &mut Agent) -> JsResult<u16> {
        to_uint16(agent, self)
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

    pub(crate) fn hash<H, A>(self, arena: &A, hasher: &mut H)
    where
        H: Hasher,
        A: Index<HeapString, Output = StringHeapData>
            + Index<HeapNumber, Output = f64>
            + Index<HeapBigInt, Output = BigIntHeapData>,
    {
        let discriminant = core::mem::discriminant(&self);
        match self {
            Value::Undefined => discriminant.hash(hasher),
            Value::Null => discriminant.hash(hasher),
            Value::Boolean(data) => {
                discriminant.hash(hasher);
                data.hash(hasher);
            }
            Value::String(data) => {
                // Skip discriminant hashing in strings
                arena[data].as_str().hash(hasher);
            }
            Value::SmallString(data) => {
                data.as_str().hash(hasher);
            }
            Value::Symbol(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Number(data) => {
                // Skip discriminant hashing in numbers
                arena[data].to_bits().hash(hasher);
            }
            Value::Integer(data) => {
                data.into_i64().hash(hasher);
            }
            Value::SmallF64(data) => {
                data.into_f64().to_bits().hash(hasher);
            }
            Value::BigInt(data) => {
                // Skip dsciriminant hashing in bigint numbers
                arena[data].data.hash(hasher);
            }
            Value::SmallBigInt(data) => {
                data.into_i64().hash(hasher);
            }
            Value::Object(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BoundFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::ECMAScriptFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseResolvingFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::PrimitiveObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Arguments(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Array(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "date")]
            Value::Date(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Error(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::FinalizationRegistry(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Map(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Promise(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Proxy(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::RegExp(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Set(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncIterator => todo!(),
            Value::Iterator => todo!(),
            Value::ArrayIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::SetIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::MapIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Generator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Module(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::EmbedderObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
        };
    }

    pub(crate) fn try_hash<H>(self, hasher: &mut H) -> Result<(), ()>
    where
        H: Hasher,
    {
        let discriminant = core::mem::discriminant(&self);
        match self {
            Value::String(_) | Value::Number(_) | Value::BigInt(_) => {
                // These values need Agent access to hash.
                return Err(());
            }
            // All other types can be hashed on the stack.
            Value::Undefined => discriminant.hash(hasher),
            Value::Null => discriminant.hash(hasher),
            Value::Boolean(data) => {
                discriminant.hash(hasher);
                data.hash(hasher);
            }
            Value::SmallString(data) => {
                data.as_str().hash(hasher);
            }
            Value::Symbol(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Integer(data) => {
                data.into_i64().hash(hasher);
            }
            Value::SmallF64(data) => {
                data.into_f64().to_bits().hash(hasher);
            }
            Value::SmallBigInt(data) => {
                data.into_i64().hash(hasher);
            }
            Value::Object(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BoundFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::ECMAScriptFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseResolvingFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::PrimitiveObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Arguments(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Array(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "date")]
            Value::Date(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Error(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::FinalizationRegistry(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Map(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Promise(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Proxy(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::RegExp(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Set(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncIterator => todo!(),
            Value::Iterator => todo!(),
            Value::ArrayIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::SetIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::MapIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Generator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Module(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::EmbedderObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
        }
        Ok(())
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

impl Rootable for Value {
    type RootRepr = ValueRootRepr;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Undefined => Ok(Self::RootRepr::Undefined),
            Self::Null => Ok(Self::RootRepr::Null),
            Self::Boolean(bool) => Ok(Self::RootRepr::Boolean(bool)),
            Self::String(heap_string) => Err(HeapRootData::String(heap_string)),
            Self::SmallString(small_string) => Ok(Self::RootRepr::SmallString(small_string)),
            Self::Symbol(symbol) => Err(HeapRootData::Symbol(symbol)),
            Self::Number(heap_number) => Err(HeapRootData::Number(heap_number)),
            Self::Integer(small_integer) => Ok(Self::RootRepr::Integer(small_integer)),
            Self::SmallF64(small_f64) => Ok(Self::RootRepr::SmallF64(small_f64)),
            Self::BigInt(heap_big_int) => Err(HeapRootData::BigInt(heap_big_int)),
            Self::SmallBigInt(small_big_int) => Ok(Self::RootRepr::SmallBigInt(small_big_int)),
            Self::Object(ordinary_object) => Err(HeapRootData::Object(ordinary_object)),
            Self::BoundFunction(bound_function) => Err(HeapRootData::BoundFunction(bound_function)),
            Self::BuiltinFunction(builtin_function) => {
                Err(HeapRootData::BuiltinFunction(builtin_function))
            }
            Self::ECMAScriptFunction(ecmascript_function) => {
                Err(HeapRootData::ECMAScriptFunction(ecmascript_function))
            }
            Self::BuiltinGeneratorFunction => Err(HeapRootData::BuiltinGeneratorFunction),
            Self::BuiltinConstructorFunction(builtin_constructor_function) => Err(
                HeapRootData::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            Self::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => Err(
                HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function),
            ),
            Self::BuiltinPromiseCollectorFunction => {
                Err(HeapRootData::BuiltinPromiseCollectorFunction)
            }
            Self::BuiltinProxyRevokerFunction => Err(HeapRootData::BuiltinProxyRevokerFunction),
            Self::PrimitiveObject(primitive_object) => {
                Err(HeapRootData::PrimitiveObject(primitive_object))
            }
            Self::Arguments(ordinary_object) => Err(HeapRootData::Arguments(ordinary_object)),
            Self::Array(array) => Err(HeapRootData::Array(array)),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(array_buffer) => Err(HeapRootData::ArrayBuffer(array_buffer)),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data_view) => Err(HeapRootData::DataView(data_view)),
            #[cfg(feature = "date")]
            Self::Date(date) => Err(HeapRootData::Date(date)),
            Self::Error(error) => Err(HeapRootData::Error(error)),
            Self::FinalizationRegistry(finalization_registry) => {
                Err(HeapRootData::FinalizationRegistry(finalization_registry))
            }
            Self::Map(map) => Err(HeapRootData::Map(map)),
            Self::Promise(promise) => Err(HeapRootData::Promise(promise)),
            Self::Proxy(proxy) => Err(HeapRootData::Proxy(proxy)),
            Self::RegExp(reg_exp) => Err(HeapRootData::RegExp(reg_exp)),
            Self::Set(set) => Err(HeapRootData::Set(set)),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(shared_array_buffer) => {
                Err(HeapRootData::SharedArrayBuffer(shared_array_buffer))
            }
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(weak_map) => Err(HeapRootData::WeakMap(weak_map)),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(weak_ref) => Err(HeapRootData::WeakRef(weak_ref)),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(weak_set) => Err(HeapRootData::WeakSet(weak_set)),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(base_index) => Err(HeapRootData::Int8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(base_index) => Err(HeapRootData::Uint8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(base_index) => Err(HeapRootData::Uint8ClampedArray(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(base_index) => Err(HeapRootData::Int16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(base_index) => Err(HeapRootData::Uint16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(base_index) => Err(HeapRootData::Int32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(base_index) => Err(HeapRootData::Uint32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(base_index) => Err(HeapRootData::BigInt64Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(base_index) => Err(HeapRootData::BigUint64Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(base_index) => Err(HeapRootData::Float32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(base_index) => Err(HeapRootData::Float64Array(base_index)),
            Self::AsyncFromSyncIterator => Err(HeapRootData::AsyncFromSyncIterator),
            Self::AsyncIterator => Err(HeapRootData::AsyncIterator),
            Self::Iterator => Err(HeapRootData::Iterator),
            Self::ArrayIterator(array_iterator) => Err(HeapRootData::ArrayIterator(array_iterator)),
            Self::SetIterator(set_iterator) => Err(HeapRootData::SetIterator(set_iterator)),
            Self::MapIterator(map_iterator) => Err(HeapRootData::MapIterator(map_iterator)),
            Self::Generator(generator) => Err(HeapRootData::Generator(generator)),
            Self::Module(module) => Err(HeapRootData::Module(module)),
            Self::EmbedderObject(embedder_object) => {
                Err(HeapRootData::EmbedderObject(embedder_object))
            }
        }
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Undefined => Ok(Self::Undefined),
            Self::RootRepr::Null => Ok(Self::Null),
            Self::RootRepr::Boolean(bool) => Ok(Self::Boolean(bool)),
            Self::RootRepr::SmallString(small_string) => Ok(Self::SmallString(small_string)),
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

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::String(heap_string) => Some(Self::String(heap_string)),
            HeapRootData::Symbol(symbol) => Some(Self::Symbol(symbol)),
            HeapRootData::Number(heap_number) => Some(Self::Number(heap_number)),
            HeapRootData::BigInt(heap_big_int) => Some(Self::BigInt(heap_big_int)),
            HeapRootData::Object(ordinary_object) => Some(Self::Object(ordinary_object)),
            HeapRootData::BoundFunction(bound_function) => {
                Some(Self::BoundFunction(bound_function))
            }
            HeapRootData::BuiltinFunction(builtin_function) => {
                Some(Self::BuiltinFunction(builtin_function))
            }
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                Some(Self::ECMAScriptFunction(ecmascript_function))
            }
            HeapRootData::BuiltinGeneratorFunction => Some(Self::BuiltinGeneratorFunction),
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => Some(
                Self::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Some(Self::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function,
                ))
            }
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Some(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Some(Self::BuiltinProxyRevokerFunction),
            HeapRootData::PrimitiveObject(primitive_object) => {
                Some(Self::PrimitiveObject(primitive_object))
            }
            HeapRootData::Arguments(ordinary_object) => Some(Self::Arguments(ordinary_object)),
            HeapRootData::Array(array) => Some(Self::Array(array)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(array_buffer) => Some(Self::ArrayBuffer(array_buffer)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(data_view) => Some(Self::DataView(data_view)),
            #[cfg(feature = "date")]
            HeapRootData::Date(date) => Some(Self::Date(date)),
            HeapRootData::Error(error) => Some(Self::Error(error)),
            HeapRootData::FinalizationRegistry(finalization_registry) => {
                Some(Self::FinalizationRegistry(finalization_registry))
            }
            HeapRootData::Map(map) => Some(Self::Map(map)),
            HeapRootData::Promise(promise) => Some(Self::Promise(promise)),
            HeapRootData::Proxy(proxy) => Some(Self::Proxy(proxy)),
            HeapRootData::RegExp(reg_exp) => Some(Self::RegExp(reg_exp)),
            HeapRootData::Set(set) => Some(Self::Set(set)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(shared_array_buffer) => {
                Some(Self::SharedArrayBuffer(shared_array_buffer))
            }
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => Some(Self::WeakMap(weak_map)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => Some(Self::WeakRef(weak_ref)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => Some(Self::WeakSet(weak_set)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(base_index) => Some(Self::Int8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(base_index) => Some(Self::Uint8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(base_index) => {
                Some(Self::Uint8ClampedArray(base_index))
            }
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(base_index) => Some(Self::Int16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(base_index) => Some(Self::Uint16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(base_index) => Some(Self::Int32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(base_index) => Some(Self::Uint32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(base_index) => Some(Self::BigInt64Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(base_index) => Some(Self::BigUint64Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(base_index) => Some(Self::Float32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(base_index) => Some(Self::Float64Array(base_index)),
            HeapRootData::AsyncFromSyncIterator => Some(Self::AsyncFromSyncIterator),
            HeapRootData::AsyncIterator => Some(Self::AsyncIterator),
            HeapRootData::Iterator => Some(Self::Iterator),
            HeapRootData::ArrayIterator(array_iterator) => {
                Some(Self::ArrayIterator(array_iterator))
            }
            HeapRootData::SetIterator(set_iterator) => Some(Self::SetIterator(set_iterator)),
            HeapRootData::MapIterator(map_iterator) => Some(Self::MapIterator(map_iterator)),
            HeapRootData::Generator(generator) => Some(Self::Generator(generator)),
            HeapRootData::Module(module) => Some(Self::Module(module)),
            HeapRootData::EmbedderObject(embedder_object) => {
                Some(Self::EmbedderObject(embedder_object))
            } // Note: Do not use _ => Err(()) to make sure any added
              // HeapRootData Value variants cause compile errors if not handled.
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ValueRootRepr {
    Undefined = UNDEFINED_DISCRIMINANT,
    Null = NULL_DISCRIMINANT,
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
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
            Value::String(data) => data.mark_values(queues),
            Value::Symbol(data) => data.mark_values(queues),
            Value::Number(data) => data.mark_values(queues),
            Value::BigInt(data) => data.mark_values(queues),
            Value::Object(data) => data.mark_values(queues),
            Value::Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "date")]
            Value::Date(data) => data.mark_values(queues),
            Value::Error(data) => data.mark_values(queues),
            Value::BoundFunction(data) => data.mark_values(queues),
            Value::BuiltinFunction(data) => data.mark_values(queues),
            Value::ECMAScriptFunction(data) => data.mark_values(queues),
            Value::RegExp(data) => data.mark_values(queues),
            Value::PrimitiveObject(data) => data.mark_values(queues),
            Value::Arguments(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => data.mark_values(queues),
            Value::FinalizationRegistry(data) => data.mark_values(queues),
            Value::Map(data) => data.mark_values(queues),
            Value::Proxy(data) => data.mark_values(queues),
            Value::Promise(data) => data.mark_values(queues),
            Value::Set(data) => data.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => data.mark_values(queues),
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => data.mark_values(queues),
            Value::BuiltinPromiseResolvingFunction(data) => data.mark_values(queues),
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncIterator => todo!(),
            Value::Iterator => todo!(),
            Value::ArrayIterator(data) => data.mark_values(queues),
            Value::SetIterator(data) => data.mark_values(queues),
            Value::MapIterator(data) => data.mark_values(queues),
            Value::Generator(data) => data.mark_values(queues),
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
            Value::String(data) => data.sweep_values(compactions),
            Value::Symbol(data) => data.sweep_values(compactions),
            Value::Number(data) => data.sweep_values(compactions),
            Value::BigInt(data) => data.sweep_values(compactions),
            Value::Object(data) => data.sweep_values(compactions),
            Value::Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "date")]
            Value::Date(data) => data.sweep_values(compactions),
            Value::Error(data) => data.sweep_values(compactions),
            Value::BoundFunction(data) => data.sweep_values(compactions),
            Value::BuiltinFunction(data) => data.sweep_values(compactions),
            Value::ECMAScriptFunction(data) => data.sweep_values(compactions),
            Value::RegExp(data) => data.sweep_values(compactions),
            Value::PrimitiveObject(data) => data.sweep_values(compactions),
            Value::Arguments(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => data.sweep_values(compactions),
            Value::FinalizationRegistry(data) => data.sweep_values(compactions),
            Value::Map(data) => data.sweep_values(compactions),
            Value::Proxy(data) => data.sweep_values(compactions),
            Value::Promise(data) => data.sweep_values(compactions),
            Value::Set(data) => data.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => data.sweep_values(compactions),
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => data.sweep_values(compactions),
            Value::BuiltinPromiseResolvingFunction(data) => data.sweep_values(compactions),
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncIterator => todo!(),
            Value::Iterator => todo!(),
            Value::ArrayIterator(data) => data.sweep_values(compactions),
            Value::SetIterator(data) => data.sweep_values(compactions),
            Value::MapIterator(data) => data.sweep_values(compactions),
            Value::Generator(data) => data.sweep_values(compactions),
            Value::Module(data) => data.sweep_values(compactions),
            Value::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}
