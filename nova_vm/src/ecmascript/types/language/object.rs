mod data;
mod internal_methods;
mod internal_slots;
mod into_object;
mod property_key;
mod property_storage;
use std::ops::Deref;

use super::{
    value::{
        ARGUMENTS_DISCRIMINANT, ARRAY_BUFFER_DISCRIMINANT, ARRAY_DISCRIMINANT,
        ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT, ASYNC_ITERATOR_DISCRIMINANT,
        BIGINT_64_ARRAY_DISCRIMINANT, BIGINT_OBJECT_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
        BOOLEAN_OBJECT_DISCRIMINANT, BOUND_FUNCTION_DISCRIMINANT,
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT, BUILTIN_FUNCTION_DISCRIMINANT,
        BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT, BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_REJECT_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVE_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        DATA_VIEW_DISCRIMINANT, DATE_DISCRIMINANT, ECMASCRIPT_ASYNC_FUNCTION_DISCRIMINANT,
        ECMASCRIPT_ASYNC_GENERATOR_FUNCTION_DISCRIMINANT,
        ECMASCRIPT_CONSTRUCTOR_FUNCTION_DISCRIMINANT, ECMASCRIPT_FUNCTION_DISCRIMINANT,
        ECMASCRIPT_GENERATOR_FUNCTION_DISCRIMINANT, EMBEDDER_OBJECT_DISCRIMINANT,
        ERROR_DISCRIMINANT, FINALIZATION_REGISTRY_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT,
        FLOAT_64_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
        INT_8_ARRAY_DISCRIMINANT, ITERATOR_DISCRIMINANT, MAP_DISCRIMINANT, MODULE_DISCRIMINANT,
        NUMBER_OBJECT_DISCRIMINANT, OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT,
        REGEXP_DISCRIMINANT, SET_DISCRIMINANT, SHARED_ARRAY_BUFFER_DISCRIMINANT,
        STRING_OBJECT_DISCRIMINANT, SYMBOL_OBJECT_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
        UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
        WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT,
    },
    Function, IntoValue, Value,
};
use crate::{
    ecmascript::{
        builtins::{
            date::Date, error::Error, ArgumentsList, Array, ArrayBuffer, BuiltinFunction,
            ECMAScriptFunction,
        },
        execution::{Agent, JsResult},
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{
            ArrayBufferIndex, ArrayIndex, BoundFunctionIndex, BuiltinFunctionIndex, DateIndex,
            ECMAScriptFunctionIndex, ErrorIndex, ObjectIndex, RegExpIndex,
        },
        GetHeapData,
    },
};

pub use data::ObjectHeapData;
pub use internal_methods::InternalMethods;
pub use internal_slots::OrdinaryObjectInternalSlots;
pub use into_object::IntoObject;
pub use property_key::PropertyKey;
pub use property_storage::PropertyStorage;

/// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
///
/// In Nova
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Object {
    Object(ObjectIndex) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunctionIndex) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunctionIndex) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunctionIndex) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction = BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolveFunction = BUILTIN_PROMISE_RESOLVE_FUNCTION_DISCRIMINANT,
    BuiltinPromiseRejectFunction = BUILTIN_PROMISE_REJECT_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    ECMAScriptAsyncFunction = ECMASCRIPT_ASYNC_FUNCTION_DISCRIMINANT,
    ECMAScriptAsyncGeneratorFunction = ECMASCRIPT_ASYNC_GENERATOR_FUNCTION_DISCRIMINANT,
    ECMAScriptConstructorFunction = ECMASCRIPT_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    ECMAScriptGeneratorFunction = ECMASCRIPT_GENERATOR_FUNCTION_DISCRIMINANT,
    BigIntObject = BIGINT_OBJECT_DISCRIMINANT,
    BooleanObject = BOOLEAN_OBJECT_DISCRIMINANT,
    NumberObject = NUMBER_OBJECT_DISCRIMINANT,
    StringObject = STRING_OBJECT_DISCRIMINANT,
    SymbolObject = SYMBOL_OBJECT_DISCRIMINANT,
    Arguments = ARGUMENTS_DISCRIMINANT,
    Array(ArrayIndex) = ARRAY_DISCRIMINANT,
    ArrayBuffer(ArrayBufferIndex) = ARRAY_BUFFER_DISCRIMINANT,
    DataView = DATA_VIEW_DISCRIMINANT,
    Date(DateIndex) = DATE_DISCRIMINANT,
    Error(ErrorIndex) = ERROR_DISCRIMINANT,
    FinalizationRegistry = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map = MAP_DISCRIMINANT,
    Promise = PROMISE_DISCRIMINANT,
    Proxy = PROXY_DISCRIMINANT,
    RegExp(RegExpIndex) = REGEXP_DISCRIMINANT,
    Set = SET_DISCRIMINANT,
    SharedArrayBuffer = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    WeakMap = WEAK_MAP_DISCRIMINANT,
    WeakRef = WEAK_REF_DISCRIMINANT,
    WeakSet = WEAK_SET_DISCRIMINANT,
    Int8Array = INT_8_ARRAY_DISCRIMINANT,
    Uint8Array = UINT_8_ARRAY_DISCRIMINANT,
    Uint8ClampedArray = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    Int16Array = INT_16_ARRAY_DISCRIMINANT,
    Uint16Array = UINT_16_ARRAY_DISCRIMINANT,
    Int32Array = INT_32_ARRAY_DISCRIMINANT,
    Uint32Array = UINT_32_ARRAY_DISCRIMINANT,
    BigInt64Array = BIGINT_64_ARRAY_DISCRIMINANT,
    BigUint64Array = BIGUINT_64_ARRAY_DISCRIMINANT,
    Float32Array = FLOAT_32_ARRAY_DISCRIMINANT,
    Float64Array = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncIterator = ASYNC_ITERATOR_DISCRIMINANT,
    Iterator = ITERATOR_DISCRIMINANT,
    Module = MODULE_DISCRIMINANT,
    EmbedderObject = EMBEDDER_OBJECT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
pub struct OrdinaryObject(pub(crate) ObjectIndex);

impl IntoObject for OrdinaryObject {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoValue for OrdinaryObject {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl From<OrdinaryObject> for Object {
    fn from(value: OrdinaryObject) -> Self {
        Self::Object(value.0)
    }
}

impl From<ObjectIndex> for OrdinaryObject {
    fn from(value: ObjectIndex) -> Self {
        OrdinaryObject(value)
    }
}

impl From<OrdinaryObject> for Value {
    fn from(value: OrdinaryObject) -> Self {
        Self::Object(value.0)
    }
}

impl TryFrom<Value> for OrdinaryObject {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(data) => Ok(OrdinaryObject(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for OrdinaryObject {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::Object(data) => Ok(OrdinaryObject(data)),
            _ => Err(()),
        }
    }
}

impl Deref for OrdinaryObject {
    type Target = ObjectIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OrdinaryObjectInternalSlots for OrdinaryObject {
    fn extensible(self, agent: &Agent) -> bool {
        agent.heap.get(*self).extensible
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        agent.heap.get_mut(*self).extensible = value;
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        agent.heap.get(*self).prototype
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        agent.heap.get_mut(*self).prototype = prototype;
    }
}

impl OrdinaryObject {
    pub(crate) const fn new(value: ObjectIndex) -> Self {
        Self(value)
    }
}

impl From<ObjectIndex> for Object {
    fn from(value: ObjectIndex) -> Self {
        Object::Object(value)
    }
}

impl From<ArrayIndex> for Object {
    fn from(value: ArrayIndex) -> Self {
        Object::Array(value)
    }
}

impl From<BoundFunctionIndex> for Object {
    fn from(value: BoundFunctionIndex) -> Self {
        Object::BoundFunction(value)
    }
}

impl From<BuiltinFunctionIndex> for Object {
    fn from(value: BuiltinFunctionIndex) -> Self {
        Object::BuiltinFunction(value)
    }
}

impl From<ECMAScriptFunctionIndex> for Object {
    fn from(value: ECMAScriptFunctionIndex) -> Self {
        Object::ECMAScriptFunction(value)
    }
}

impl From<ErrorIndex> for Object {
    fn from(value: ErrorIndex) -> Self {
        Object::Error(value)
    }
}

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        match value {
            Object::Object(data) => Value::Object(data),
            Object::BoundFunction(data) => Value::BoundFunction(data),
            Object::BuiltinFunction(data) => Value::BuiltinFunction(data),
            Object::ECMAScriptFunction(data) => Value::ECMAScriptFunction(data),
            Object::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            Object::BuiltinConstructorFunction => Value::BuiltinConstructorFunction,
            Object::BuiltinPromiseResolveFunction => Value::BuiltinPromiseResolveFunction,
            Object::BuiltinPromiseRejectFunction => Value::BuiltinPromiseRejectFunction,
            Object::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            Object::ECMAScriptAsyncFunction => Value::ECMAScriptAsyncFunction,
            Object::ECMAScriptAsyncGeneratorFunction => Value::ECMAScriptAsyncGeneratorFunction,
            Object::ECMAScriptConstructorFunction => Value::ECMAScriptConstructorFunction,
            Object::ECMAScriptGeneratorFunction => Value::ECMAScriptGeneratorFunction,
            Object::BigIntObject => Value::BigIntObject,
            Object::BooleanObject => Value::BooleanObject,
            Object::NumberObject => Value::NumberObject,
            Object::StringObject => Value::StringObject,
            Object::SymbolObject => Value::SymbolObject,
            Object::Arguments => Value::Arguments,
            Object::Array(data) => Value::Array(data),
            Object::ArrayBuffer(data) => Value::ArrayBuffer(data),
            Object::DataView => Value::DataView,
            Object::Date(data) => Value::Date(data),
            Object::Error(data) => Value::Error(data),
            Object::FinalizationRegistry => Value::FinalizationRegistry,
            Object::Map => Value::Map,
            Object::Promise => Value::Promise,
            Object::Proxy => Value::Proxy,
            Object::RegExp(data) => Value::RegExp(data),
            Object::Set => Value::Set,
            Object::SharedArrayBuffer => Value::SharedArrayBuffer,
            Object::WeakMap => Value::WeakMap,
            Object::WeakRef => Value::WeakRef,
            Object::WeakSet => Value::WeakSet,
            Object::Int8Array => Value::Int8Array,
            Object::Uint8Array => Value::Uint8Array,
            Object::Uint8ClampedArray => Value::Uint8ClampedArray,
            Object::Int16Array => Value::Int16Array,
            Object::Uint16Array => Value::Uint16Array,
            Object::Int32Array => Value::Int32Array,
            Object::Uint32Array => Value::Uint32Array,
            Object::BigInt64Array => Value::BigInt64Array,
            Object::BigUint64Array => Value::BigUint64Array,
            Object::Float32Array => Value::Float32Array,
            Object::Float64Array => Value::Float64Array,
            Object::AsyncFromSyncIterator => Value::AsyncFromSyncIterator,
            Object::AsyncIterator => Value::AsyncIterator,
            Object::Iterator => Value::Iterator,
            Object::Module => Value::Module,
            Object::EmbedderObject => Value::EmbedderObject,
        }
    }
}

impl TryFrom<Value> for Object {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, ()> {
        match value {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::String(_)
            | Value::SmallString(_)
            | Value::Symbol(_)
            | Value::Number(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::BigInt(_)
            | Value::SmallBigInt(_) => Err(()),
            Value::Object(x) => Ok(Object::from(x)),
            Value::Array(x) => Ok(Object::from(x)),
            Value::Date(x) => Ok(Object::Date(x)),
            Value::Error(x) => Ok(Object::from(x)),
            Value::BoundFunction(x) => Ok(Object::from(x)),
            Value::BuiltinFunction(x) => Ok(Object::from(x)),
            Value::ECMAScriptFunction(x) => Ok(Object::from(x)),
            Value::BuiltinGeneratorFunction => Ok(Object::BuiltinGeneratorFunction),
            Value::BuiltinConstructorFunction => Ok(Object::BuiltinConstructorFunction),
            Value::BuiltinPromiseResolveFunction => Ok(Object::BuiltinPromiseResolveFunction),
            Value::BuiltinPromiseRejectFunction => Ok(Object::BuiltinPromiseRejectFunction),
            Value::BuiltinPromiseCollectorFunction => Ok(Object::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Object::BuiltinProxyRevokerFunction),
            Value::ECMAScriptAsyncFunction => Ok(Object::ECMAScriptAsyncFunction),
            Value::ECMAScriptAsyncGeneratorFunction => Ok(Object::ECMAScriptAsyncGeneratorFunction),
            Value::ECMAScriptConstructorFunction => Ok(Object::ECMAScriptConstructorFunction),
            Value::ECMAScriptGeneratorFunction => Ok(Object::ECMAScriptGeneratorFunction),
            Value::BigIntObject => Ok(Object::BigIntObject),
            Value::BooleanObject => Ok(Object::BooleanObject),
            Value::NumberObject => Ok(Object::NumberObject),
            Value::StringObject => Ok(Object::StringObject),
            Value::SymbolObject => Ok(Object::SymbolObject),
            Value::Arguments => Ok(Object::Arguments),
            Value::ArrayBuffer(idx) => Ok(Object::ArrayBuffer(idx)),
            Value::DataView => Ok(Object::DataView),
            Value::FinalizationRegistry => Ok(Object::FinalizationRegistry),
            Value::Map => Ok(Object::Map),
            Value::Promise => Ok(Object::Promise),
            Value::Proxy => Ok(Object::Proxy),
            Value::RegExp(idx) => Ok(Object::RegExp(idx)),
            Value::Set => Ok(Object::Set),
            Value::SharedArrayBuffer => Ok(Object::SharedArrayBuffer),
            Value::WeakMap => Ok(Object::WeakMap),
            Value::WeakRef => Ok(Object::WeakRef),
            Value::WeakSet => Ok(Object::WeakSet),
            Value::Int8Array => Ok(Object::Int8Array),
            Value::Uint8Array => Ok(Object::Uint8Array),
            Value::Uint8ClampedArray => Ok(Object::Uint8ClampedArray),
            Value::Int16Array => Ok(Object::Int16Array),
            Value::Uint16Array => Ok(Object::Uint16Array),
            Value::Int32Array => Ok(Object::Int32Array),
            Value::Uint32Array => Ok(Object::Uint32Array),
            Value::BigInt64Array => Ok(Object::BigInt64Array),
            Value::BigUint64Array => Ok(Object::BigUint64Array),
            Value::Float32Array => Ok(Object::Float32Array),
            Value::Float64Array => Ok(Object::Float64Array),
            Value::AsyncFromSyncIterator => Ok(Object::AsyncFromSyncIterator),
            Value::AsyncIterator => Ok(Object::AsyncIterator),
            Value::Iterator => Ok(Object::Iterator),
            Value::Module => Ok(Object::Module),
            Value::EmbedderObject => Ok(Object::EmbedderObject),
        }
    }
}

impl Object {
    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn property_storage(self) -> PropertyStorage {
        PropertyStorage::new(self)
    }
}

impl OrdinaryObjectInternalSlots for Object {
    fn extensible(self, agent: &Agent) -> bool {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).extensible(agent),
            Object::Array(idx) => Array::from(idx).extensible(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).extensible(agent),
            Object::Date(idx) => Date::from(idx).extensible(agent),
            Object::Error(idx) => Error::from(idx).extensible(agent),
            Object::BoundFunction(idx) => Function::from(idx).extensible(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).extensible(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).extensible(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_extensible(agent, value),
            Object::Array(idx) => Array::from(idx).set_extensible(agent, value),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_extensible(agent, value),
            Object::Date(idx) => Date::from(idx).set_extensible(agent, value),
            Object::Error(idx) => Error::from(idx).set_extensible(agent, value),
            Object::BoundFunction(idx) => Function::from(idx).set_extensible(agent, value),
            Object::BuiltinFunction(idx) => Function::from(idx).set_extensible(agent, value),
            Object::ECMAScriptFunction(idx) => Function::from(idx).set_extensible(agent, value),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).prototype(agent),
            Object::Array(idx) => Array::from(idx).prototype(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).prototype(agent),
            Object::Date(idx) => Date::from(idx).prototype(agent),
            Object::Error(idx) => Error::from(idx).prototype(agent),
            Object::BoundFunction(idx) => Function::from(idx).prototype(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).prototype(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).prototype(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_prototype(agent, prototype),
            Object::Array(idx) => Array::from(idx).set_prototype(agent, prototype),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_prototype(agent, prototype),
            Object::Date(idx) => Date::from(idx).set_prototype(agent, prototype),
            Object::Error(idx) => Error::from(idx).set_prototype(agent, prototype),
            Object::BoundFunction(idx) => Function::from(idx).set_prototype(agent, prototype),
            Object::BuiltinFunction(idx) => Function::from(idx).set_prototype(agent, prototype),
            Object::ECMAScriptFunction(idx) => Function::from(idx).set_prototype(agent, prototype),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }
}

impl InternalMethods for Object {
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get_prototype_of(agent),
            Object::Array(idx) => Array::from(idx).get_prototype_of(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).get_prototype_of(agent),
            Object::Date(idx) => Date::from(idx).get_prototype_of(agent),
            Object::Error(idx) => Error::from(idx).get_prototype_of(agent),
            Object::BoundFunction(idx) => Function::from(idx).get_prototype_of(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).get_prototype_of(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).get_prototype_of(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_prototype_of(agent, prototype),
            Object::Array(idx) => Array::from(idx).set_prototype_of(agent, prototype),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_prototype_of(agent, prototype),
            Object::Date(idx) => Date::from(idx).set_prototype_of(agent, prototype),
            Object::Error(idx) => Error::from(idx).set_prototype_of(agent, prototype),
            Object::BoundFunction(idx) => Function::from(idx).set_prototype_of(agent, prototype),
            Object::BuiltinFunction(idx) => Function::from(idx).set_prototype_of(agent, prototype),
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).set_prototype_of(agent, prototype)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).is_extensible(agent),
            Object::Array(idx) => Array::from(idx).is_extensible(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).is_extensible(agent),
            Object::Date(idx) => Date::from(idx).is_extensible(agent),
            Object::Error(idx) => Error::from(idx).is_extensible(agent),
            Object::BoundFunction(idx) => Function::from(idx).is_extensible(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).is_extensible(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).is_extensible(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).prevent_extensions(agent),
            Object::Array(idx) => Array::from(idx).prevent_extensions(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).prevent_extensions(agent),
            Object::Date(idx) => Date::from(idx).prevent_extensions(agent),
            Object::Error(idx) => Error::from(idx).prevent_extensions(agent),
            Object::BoundFunction(idx) => Function::from(idx).prevent_extensions(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).prevent_extensions(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).prevent_extensions(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get_own_property(agent, property_key),
            Object::Array(idx) => Array::from(idx).get_own_property(agent, property_key),
            Object::ArrayBuffer(idx) => {
                ArrayBuffer::from(idx).get_own_property(agent, property_key)
            }
            Object::Date(idx) => Date::from(idx).get_own_property(agent, property_key),
            Object::Error(idx) => Error::from(idx).get_own_property(agent, property_key),
            Object::BoundFunction(idx) => Function::from(idx).get_own_property(agent, property_key),
            Object::BuiltinFunction(idx) => {
                Function::from(idx).get_own_property(agent, property_key)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).get_own_property(agent, property_key)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::Array(idx) => {
                Array::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::ArrayBuffer(idx) => {
                ArrayBuffer::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::Date(idx) => {
                Date::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::Error(idx) => {
                Error::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::BoundFunction(idx) => {
                Function::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).has_property(agent, property_key),
            Object::Array(idx) => Array::from(idx).has_property(agent, property_key),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).has_property(agent, property_key),
            Object::Date(idx) => Date::from(idx).has_property(agent, property_key),
            Object::Error(idx) => Error::from(idx).has_property(agent, property_key),
            Object::BoundFunction(idx) => Function::from(idx).has_property(agent, property_key),
            Object::BuiltinFunction(idx) => Function::from(idx).has_property(agent, property_key),
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).has_property(agent, property_key)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get(agent, property_key, receiver),
            Object::Array(idx) => Array::from(idx).get(agent, property_key, receiver),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).get(agent, property_key, receiver),
            Object::Date(idx) => Date::from(idx).get(agent, property_key, receiver),
            Object::Error(idx) => Error::from(idx).get(agent, property_key, receiver),
            Object::BoundFunction(idx) => Function::from(idx).get(agent, property_key, receiver),
            Object::BuiltinFunction(idx) => {
                BuiltinFunction::from(idx).get(agent, property_key, receiver)
            }
            Object::ECMAScriptFunction(idx) => {
                ECMAScriptFunction::from(idx).get(agent, property_key, receiver)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => {
                OrdinaryObject::from(idx).set(agent, property_key, value, receiver)
            }
            Object::Array(idx) => Array::from(idx).set(agent, property_key, value, receiver),
            Object::ArrayBuffer(idx) => {
                ArrayBuffer::from(idx).set(agent, property_key, value, receiver)
            }
            Object::Date(idx) => Date::from(idx).set(agent, property_key, value, receiver),
            Object::Error(idx) => Error::from(idx).set(agent, property_key, value, receiver),
            Object::BoundFunction(idx) => {
                Function::from(idx).set(agent, property_key, value, receiver)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).set(agent, property_key, value, receiver)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).set(agent, property_key, value, receiver)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).delete(agent, property_key),
            Object::Array(idx) => Array::from(idx).delete(agent, property_key),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).delete(agent, property_key),
            Object::Date(idx) => Date::from(idx).delete(agent, property_key),
            Object::Error(idx) => Error::from(idx).delete(agent, property_key),
            Object::BoundFunction(idx) => Function::from(idx).delete(agent, property_key),
            Object::BuiltinFunction(idx) => Function::from(idx).delete(agent, property_key),
            Object::ECMAScriptFunction(idx) => Function::from(idx).delete(agent, property_key),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).own_property_keys(agent),
            Object::Array(idx) => Array::from(idx).own_property_keys(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).own_property_keys(agent),
            Object::Date(idx) => Date::from(idx).own_property_keys(agent),
            Object::Error(idx) => Error::from(idx).own_property_keys(agent),
            Object::BoundFunction(idx) => Function::from(idx).own_property_keys(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).own_property_keys(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).own_property_keys(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BigIntObject => todo!(),
            Object::BooleanObject => todo!(),
            Object::NumberObject => todo!(),
            Object::StringObject => todo!(),
            Object::SymbolObject => todo!(),
            Object::Arguments => todo!(),
            Object::DataView => todo!(),
            Object::FinalizationRegistry => todo!(),
            Object::Map => todo!(),
            Object::Promise => todo!(),
            Object::Proxy => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set => todo!(),
            Object::SharedArrayBuffer => todo!(),
            Object::WeakMap => todo!(),
            Object::WeakRef => todo!(),
            Object::WeakSet => todo!(),
            Object::Int8Array => todo!(),
            Object::Uint8Array => todo!(),
            Object::Uint8ClampedArray => todo!(),
            Object::Int16Array => todo!(),
            Object::Uint16Array => todo!(),
            Object::Int32Array => todo!(),
            Object::Uint32Array => todo!(),
            Object::BigInt64Array => todo!(),
            Object::BigUint64Array => todo!(),
            Object::Float32Array => todo!(),
            Object::Float64Array => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module => todo!(),
            Object::EmbedderObject => todo!(),
        }
    }

    fn call(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        match self {
            Object::BoundFunction(idx) => {
                Function::from(idx).call(agent, this_value, arguments_list)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).call(agent, this_value, arguments_list)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).call(agent, this_value, arguments_list)
            }
            Object::EmbedderObject => todo!(),
            _ => unreachable!(),
        }
    }

    fn construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        match self {
            Object::BoundFunction(idx) => {
                Function::from(idx).construct(agent, arguments_list, new_target)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).construct(agent, arguments_list, new_target)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).construct(agent, arguments_list, new_target)
            }
            _ => unreachable!(),
        }
    }
}
