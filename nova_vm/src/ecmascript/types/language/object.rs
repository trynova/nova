mod data;
mod internal_methods;
mod internal_slots;
mod into_object;
mod property_key;
mod property_storage;

use super::{
    value::{
        ARGUMENTS_DISCRIMINANT, ARRAY_BUFFER_DISCRIMINANT, ARRAY_DISCRIMINANT,
        ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT, ASYNC_ITERATOR_DISCRIMINANT,
        BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT, BOUND_FUNCTION_DISCRIMINANT,
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
        OBJECT_DISCRIMINANT, PRIMITIVE_OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT,
        PROXY_DISCRIMINANT, REGEXP_DISCRIMINANT, SET_DISCRIMINANT,
        SHARED_ARRAY_BUFFER_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT,
        UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT, WEAK_MAP_DISCRIMINANT,
        WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT,
    },
    Function, IntoValue, Value,
};
use crate::{
    ecmascript::{
        builtins::{
            bound_function::BoundFunction, data_view::DataView, date::Date, error::Error, map::Map,
            primitive_objects::PrimitiveObject, set::Set, shared_array_buffer::SharedArrayBuffer,
            ArgumentsList, Array, ArrayBuffer, BuiltinFunction, ECMAScriptFunction,
        },
        execution::{Agent, JsResult},
        scripts_and_modules::module::ModuleIdentifier,
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{
            ArrayIndex, EmbedderObjectIndex, FinalizationRegistryIndex, MapIndex, ObjectIndex,
            PromiseIndex, ProxyIndex, RegExpIndex, SetIndex, TypedArrayIndex, WeakMapIndex,
            WeakRefIndex, WeakSetIndex,
        },
        CompactionLists, HeapMarkAndSweep, WorkQueues,
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
    Object(OrdinaryObject) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
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
    PrimitiveObject(PrimitiveObject) = PRIMITIVE_OBJECT_DISCRIMINANT,
    Arguments = ARGUMENTS_DISCRIMINANT,
    Array(Array) = ARRAY_DISCRIMINANT,
    ArrayBuffer(ArrayBuffer) = ARRAY_BUFFER_DISCRIMINANT,
    DataView(DataView) = DATA_VIEW_DISCRIMINANT,
    Date(Date) = DATE_DISCRIMINANT,
    Error(Error) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistryIndex) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(MapIndex) = MAP_DISCRIMINANT,
    Promise(PromiseIndex) = PROMISE_DISCRIMINANT,
    Proxy(ProxyIndex) = PROXY_DISCRIMINANT,
    RegExp(RegExpIndex) = REGEXP_DISCRIMINANT,
    Set(SetIndex) = SET_DISCRIMINANT,
    SharedArrayBuffer(SharedArrayBuffer) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    WeakMap(WeakMapIndex) = WEAK_MAP_DISCRIMINANT,
    WeakRef(WeakRefIndex) = WEAK_REF_DISCRIMINANT,
    WeakSet(WeakSetIndex) = WEAK_SET_DISCRIMINANT,
    Int8Array(TypedArrayIndex) = INT_8_ARRAY_DISCRIMINANT,
    Uint8Array(TypedArrayIndex) = UINT_8_ARRAY_DISCRIMINANT,
    Uint8ClampedArray(TypedArrayIndex) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    Int16Array(TypedArrayIndex) = INT_16_ARRAY_DISCRIMINANT,
    Uint16Array(TypedArrayIndex) = UINT_16_ARRAY_DISCRIMINANT,
    Int32Array(TypedArrayIndex) = INT_32_ARRAY_DISCRIMINANT,
    Uint32Array(TypedArrayIndex) = UINT_32_ARRAY_DISCRIMINANT,
    BigInt64Array(TypedArrayIndex) = BIGINT_64_ARRAY_DISCRIMINANT,
    BigUint64Array(TypedArrayIndex) = BIGUINT_64_ARRAY_DISCRIMINANT,
    Float32Array(TypedArrayIndex) = FLOAT_32_ARRAY_DISCRIMINANT,
    Float64Array(TypedArrayIndex) = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncIterator = ASYNC_ITERATOR_DISCRIMINANT,
    Iterator = ITERATOR_DISCRIMINANT,
    Module(ModuleIdentifier) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObjectIndex) = EMBEDDER_OBJECT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
        Self::Object(value)
    }
}

impl From<ObjectIndex> for OrdinaryObject {
    fn from(value: ObjectIndex) -> Self {
        OrdinaryObject(value)
    }
}

impl From<OrdinaryObject> for Value {
    fn from(value: OrdinaryObject) -> Self {
        Self::Object(value)
    }
}

impl TryFrom<Value> for OrdinaryObject {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for OrdinaryObject {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl OrdinaryObjectInternalSlots for OrdinaryObject {
    fn internal_extensible(self, agent: &Agent) -> bool {
        agent[self].extensible
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        agent[self].extensible = value;
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        agent[self].prototype
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        agent[self].prototype = prototype;
    }
}

impl OrdinaryObject {
    pub(crate) const fn _def() -> Self {
        Self(ObjectIndex::from_u32_index(0))
    }
    pub(crate) const fn new(value: ObjectIndex) -> Self {
        Self(value)
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<ObjectIndex> for Object {
    fn from(value: ObjectIndex) -> Self {
        Object::Object(value.into())
    }
}

impl From<ArrayIndex> for Object {
    fn from(value: ArrayIndex) -> Self {
        Object::Array(value.into())
    }
}

impl From<BoundFunction> for Object {
    fn from(value: BoundFunction) -> Self {
        Object::BoundFunction(value)
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
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data),
            Object::Arguments => Value::Arguments,
            Object::Array(data) => Value::Array(data),
            Object::ArrayBuffer(data) => Value::ArrayBuffer(data),
            Object::DataView(data) => Value::DataView(data),
            Object::Date(data) => Value::Date(data),
            Object::Error(data) => Value::Error(data),
            Object::FinalizationRegistry(data) => Value::FinalizationRegistry(data),
            Object::Map(data) => Value::Map(data),
            Object::Promise(data) => Value::Promise(data),
            Object::Proxy(data) => Value::Proxy(data),
            Object::RegExp(data) => Value::RegExp(data),
            Object::Set(data) => Value::Set(data),
            Object::SharedArrayBuffer(data) => Value::SharedArrayBuffer(data),
            Object::WeakMap(data) => Value::WeakMap(data),
            Object::WeakRef(data) => Value::WeakRef(data),
            Object::WeakSet(data) => Value::WeakSet(data),
            Object::Int8Array(data) => Value::Int8Array(data),
            Object::Uint8Array(data) => Value::Uint8Array(data),
            Object::Uint8ClampedArray(data) => Value::Uint8ClampedArray(data),
            Object::Int16Array(data) => Value::Int16Array(data),
            Object::Uint16Array(data) => Value::Uint16Array(data),
            Object::Int32Array(data) => Value::Int32Array(data),
            Object::Uint32Array(data) => Value::Uint32Array(data),
            Object::BigInt64Array(data) => Value::BigInt64Array(data),
            Object::BigUint64Array(data) => Value::BigUint64Array(data),
            Object::Float32Array(data) => Value::Float32Array(data),
            Object::Float64Array(data) => Value::Float64Array(data),
            Object::AsyncFromSyncIterator => Value::AsyncFromSyncIterator,
            Object::AsyncIterator => Value::AsyncIterator,
            Object::Iterator => Value::Iterator,
            Object::Module(data) => Value::Module(data),
            Object::EmbedderObject(data) => Value::EmbedderObject(data),
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
            Value::PrimitiveObject(data) => Ok(Object::PrimitiveObject(data)),
            Value::Arguments => Ok(Object::Arguments),
            Value::ArrayBuffer(idx) => Ok(Object::ArrayBuffer(idx)),
            Value::DataView(data) => Ok(Object::DataView(data)),
            Value::FinalizationRegistry(data) => Ok(Object::FinalizationRegistry(data)),
            Value::Map(data) => Ok(Object::Map(data)),
            Value::Promise(data) => Ok(Object::Promise(data)),
            Value::Proxy(data) => Ok(Object::Proxy(data)),
            Value::RegExp(idx) => Ok(Object::RegExp(idx)),
            Value::Set(data) => Ok(Object::Set(data)),
            Value::SharedArrayBuffer(data) => Ok(Object::SharedArrayBuffer(data)),
            Value::WeakMap(data) => Ok(Object::WeakMap(data)),
            Value::WeakRef(data) => Ok(Object::WeakRef(data)),
            Value::WeakSet(data) => Ok(Object::WeakSet(data)),
            Value::Int8Array(data) => Ok(Object::Int8Array(data)),
            Value::Uint8Array(data) => Ok(Object::Uint8Array(data)),
            Value::Uint8ClampedArray(data) => Ok(Object::Uint8ClampedArray(data)),
            Value::Int16Array(data) => Ok(Object::Int16Array(data)),
            Value::Uint16Array(data) => Ok(Object::Uint16Array(data)),
            Value::Int32Array(data) => Ok(Object::Int32Array(data)),
            Value::Uint32Array(data) => Ok(Object::Uint32Array(data)),
            Value::BigInt64Array(data) => Ok(Object::BigInt64Array(data)),
            Value::BigUint64Array(data) => Ok(Object::BigUint64Array(data)),
            Value::Float32Array(data) => Ok(Object::Float32Array(data)),
            Value::Float64Array(data) => Ok(Object::Float64Array(data)),
            Value::AsyncFromSyncIterator => Ok(Object::AsyncFromSyncIterator),
            Value::AsyncIterator => Ok(Object::AsyncIterator),
            Value::Iterator => Ok(Object::Iterator),
            Value::Module(data) => Ok(Object::Module(data)),
            Value::EmbedderObject(data) => Ok(Object::EmbedderObject(data)),
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
    fn internal_extensible(self, agent: &Agent) -> bool {
        match self {
            Object::Object(idx) => idx.internal_extensible(agent),
            Object::Array(idx) => idx.internal_extensible(agent),
            Object::ArrayBuffer(idx) => idx.internal_extensible(agent),
            Object::Date(idx) => idx.internal_extensible(agent),
            Object::Error(idx) => idx.internal_extensible(agent),
            Object::BoundFunction(idx) => idx.internal_extensible(agent),
            Object::BuiltinFunction(idx) => idx.internal_extensible(agent),
            Object::ECMAScriptFunction(idx) => idx.internal_extensible(agent),
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_extensible(agent),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_extensible(agent),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        match self {
            Object::Object(idx) => idx.internal_set_extensible(agent, value),
            Object::Array(idx) => idx.internal_set_extensible(agent, value),
            Object::ArrayBuffer(idx) => idx.internal_set_extensible(agent, value),
            Object::Date(idx) => idx.internal_set_extensible(agent, value),
            Object::Error(idx) => idx.internal_set_extensible(agent, value),
            Object::BoundFunction(idx) => Function::from(idx).internal_set_extensible(agent, value),
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_set_extensible(agent, value)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_set_extensible(agent, value)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_set_extensible(agent, value),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_set_extensible(agent, value),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        match self {
            Object::Object(idx) => idx.internal_prototype(agent),
            Object::Array(idx) => idx.internal_prototype(agent),
            Object::ArrayBuffer(idx) => idx.internal_prototype(agent),
            Object::Date(idx) => idx.internal_prototype(agent),
            Object::Error(idx) => idx.internal_prototype(agent),
            Object::BoundFunction(idx) => Function::from(idx).internal_prototype(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).internal_prototype(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).internal_prototype(agent),
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_prototype(agent),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_prototype(agent),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        match self {
            Object::Object(idx) => idx.internal_set_prototype(agent, prototype),
            Object::Array(idx) => idx.internal_set_prototype(agent, prototype),
            Object::ArrayBuffer(idx) => idx.internal_set_prototype(agent, prototype),
            Object::Date(idx) => idx.internal_set_prototype(agent, prototype),
            Object::Error(idx) => idx.internal_set_prototype(agent, prototype),
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_set_prototype(agent, prototype)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_set_prototype(agent, prototype)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_set_prototype(agent, prototype)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_set_prototype(agent, prototype),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_set_prototype(agent, prototype),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }
}

impl InternalMethods for Object {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        match self {
            Object::Object(idx) => idx.internal_get_prototype_of(agent),
            Object::Array(idx) => idx.internal_get_prototype_of(agent),
            Object::ArrayBuffer(idx) => idx.internal_get_prototype_of(agent),
            Object::Date(idx) => idx.internal_get_prototype_of(agent),
            Object::Error(idx) => idx.internal_get_prototype_of(agent),
            Object::BoundFunction(idx) => Function::from(idx).internal_get_prototype_of(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).internal_get_prototype_of(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).internal_get_prototype_of(agent),
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_get_prototype_of(agent),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_get_prototype_of(agent),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => idx.internal_set_prototype_of(agent, prototype),
            Object::Array(idx) => idx.internal_set_prototype_of(agent, prototype),
            Object::ArrayBuffer(idx) => idx.internal_set_prototype_of(agent, prototype),
            Object::Date(idx) => idx.internal_set_prototype_of(agent, prototype),
            Object::Error(idx) => idx.internal_set_prototype_of(agent, prototype),
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_set_prototype_of(agent, prototype)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_set_prototype_of(agent, prototype)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_set_prototype_of(agent, prototype)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_set_prototype_of(agent, prototype),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_set_prototype_of(agent, prototype),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => idx.internal_is_extensible(agent),
            Object::Array(idx) => idx.internal_is_extensible(agent),
            Object::ArrayBuffer(idx) => idx.internal_is_extensible(agent),
            Object::Date(idx) => idx.internal_is_extensible(agent),
            Object::Error(idx) => idx.internal_is_extensible(agent),
            Object::BoundFunction(idx) => Function::from(idx).internal_is_extensible(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).internal_is_extensible(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).internal_is_extensible(agent),
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_is_extensible(agent),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_is_extensible(agent),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => idx.internal_prevent_extensions(agent),
            Object::Array(idx) => idx.internal_prevent_extensions(agent),
            Object::ArrayBuffer(idx) => idx.internal_prevent_extensions(agent),
            Object::Date(idx) => idx.internal_prevent_extensions(agent),
            Object::Error(idx) => idx.internal_prevent_extensions(agent),
            Object::BoundFunction(idx) => Function::from(idx).internal_prevent_extensions(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).internal_prevent_extensions(agent),
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_prevent_extensions(agent)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_prevent_extensions(agent),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_prevent_extensions(agent),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match self {
            Object::Object(idx) => idx.internal_get_own_property(agent, property_key),
            Object::Array(idx) => idx.internal_get_own_property(agent, property_key),
            Object::ArrayBuffer(idx) => idx.internal_get_own_property(agent, property_key),
            Object::Date(idx) => idx.internal_get_own_property(agent, property_key),
            Object::Error(idx) => idx.internal_get_own_property(agent, property_key),
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_get_own_property(agent, property_key)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_get_own_property(agent, property_key)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_get_own_property(agent, property_key)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_get_own_property(agent, property_key),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_get_own_property(agent, property_key),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Array(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::ArrayBuffer(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Date(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Error(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::BoundFunction(idx) => Function::from(idx).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::BuiltinFunction(idx) => Function::from(idx).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::ECMAScriptFunction(idx) => Function::from(idx).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => idx.internal_has_property(agent, property_key),
            Object::Array(idx) => idx.internal_has_property(agent, property_key),
            Object::ArrayBuffer(idx) => idx.internal_has_property(agent, property_key),
            Object::Date(idx) => idx.internal_has_property(agent, property_key),
            Object::Error(idx) => idx.internal_has_property(agent, property_key),
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_has_property(agent, property_key)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_has_property(agent, property_key)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_has_property(agent, property_key)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_has_property(agent, property_key),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_has_property(agent, property_key),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        match self {
            Object::Object(idx) => idx.internal_get(agent, property_key, receiver),
            Object::Array(idx) => idx.internal_get(agent, property_key, receiver),
            Object::ArrayBuffer(idx) => idx.internal_get(agent, property_key, receiver),
            Object::Date(idx) => idx.internal_get(agent, property_key, receiver),
            Object::Error(idx) => idx.internal_get(agent, property_key, receiver),
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_get(agent, property_key, receiver)
            }
            Object::BuiltinFunction(idx) => idx.internal_get(agent, property_key, receiver),
            Object::ECMAScriptFunction(idx) => idx.internal_get(agent, property_key, receiver),
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_get(agent, property_key, receiver),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_get(agent, property_key, receiver),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => idx.internal_set(agent, property_key, value, receiver),
            Object::Array(idx) => idx.internal_set(agent, property_key, value, receiver),
            Object::ArrayBuffer(idx) => idx.internal_set(agent, property_key, value, receiver),
            Object::Date(idx) => idx.internal_set(agent, property_key, value, receiver),
            Object::Error(idx) => idx.internal_set(agent, property_key, value, receiver),
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_set(agent, property_key, value, receiver)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_set(agent, property_key, value, receiver)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_set(agent, property_key, value, receiver)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_set(agent, property_key, value, receiver),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_set(agent, property_key, value, receiver),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => idx.internal_delete(agent, property_key),
            Object::Array(idx) => idx.internal_delete(agent, property_key),
            Object::ArrayBuffer(idx) => idx.internal_delete(agent, property_key),
            Object::Date(idx) => idx.internal_delete(agent, property_key),
            Object::Error(idx) => idx.internal_delete(agent, property_key),
            Object::BoundFunction(idx) => Function::from(idx).internal_delete(agent, property_key),
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_delete(agent, property_key)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_delete(agent, property_key)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_delete(agent, property_key),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_delete(agent, property_key),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self {
            Object::Object(idx) => idx.internal_own_property_keys(agent),
            Object::Array(idx) => idx.internal_own_property_keys(agent),
            Object::ArrayBuffer(idx) => idx.internal_own_property_keys(agent),
            Object::Date(idx) => idx.internal_own_property_keys(agent),
            Object::Error(idx) => idx.internal_own_property_keys(agent),
            Object::BoundFunction(idx) => Function::from(idx).internal_own_property_keys(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).internal_own_property_keys(agent),
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_own_property_keys(agent)
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
            Object::PrimitiveObject(_data) => todo!(),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(data) => Map::from(data).internal_own_property_keys(agent),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(data) => Set::from(data).internal_own_property_keys(agent),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        match self {
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_call(agent, this_value, arguments_list)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_call(agent, this_value, arguments_list)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_call(agent, this_value, arguments_list)
            }
            Object::EmbedderObject(_) => todo!(),
            _ => unreachable!(),
        }
    }

    fn internal_construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        match self {
            Object::BoundFunction(idx) => {
                Function::from(idx).internal_construct(agent, arguments_list, new_target)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).internal_construct(agent, arguments_list, new_target)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).internal_construct(agent, arguments_list, new_target)
            }
            _ => unreachable!(),
        }
    }
}

impl HeapMarkAndSweep for Object {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Object::Object(idx) => idx.mark_values(queues),
            Object::Array(idx) => idx.mark_values(queues),
            Object::ArrayBuffer(idx) => idx.mark_values(queues),
            Object::Date(idx) => idx.mark_values(queues),
            Object::Error(idx) => idx.mark_values(queues),
            Object::BoundFunction(d) => d.mark_values(queues),
            Object::BuiltinFunction(d) => d.mark_values(queues),
            Object::ECMAScriptFunction(d) => d.mark_values(queues),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::ECMAScriptAsyncFunction => todo!(),
            Object::ECMAScriptAsyncGeneratorFunction => todo!(),
            Object::ECMAScriptConstructorFunction => todo!(),
            Object::ECMAScriptGeneratorFunction => todo!(),
            Object::BuiltinPromiseResolveFunction => todo!(),
            Object::BuiltinPromiseRejectFunction => todo!(),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(idx) => idx.mark_values(queues),
            Object::Arguments => todo!(),
            Object::DataView(_) => todo!(),
            Object::FinalizationRegistry(_) => todo!(),
            Object::Map(_) => todo!(),
            Object::Promise(_) => todo!(),
            Object::Proxy(_) => todo!(),
            Object::RegExp(_) => todo!(),
            Object::Set(_) => todo!(),
            Object::SharedArrayBuffer(_) => todo!(),
            Object::WeakMap(_) => todo!(),
            Object::WeakRef(_) => todo!(),
            Object::WeakSet(_) => todo!(),
            Object::Int8Array(_) => todo!(),
            Object::Uint8Array(_) => todo!(),
            Object::Uint8ClampedArray(_) => todo!(),
            Object::Int16Array(_) => todo!(),
            Object::Uint16Array(_) => todo!(),
            Object::Int32Array(_) => todo!(),
            Object::Uint32Array(_) => todo!(),
            Object::BigInt64Array(_) => todo!(),
            Object::BigUint64Array(_) => todo!(),
            Object::Float32Array(_) => todo!(),
            Object::Float64Array(_) => todo!(),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Module(_) => todo!(),
            Object::EmbedderObject(_) => todo!(),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::Object(idx) => idx.sweep_values(compactions),
            Self::Array(idx) => idx.sweep_values(compactions),
            Self::Error(idx) => idx.sweep_values(compactions),
            _ => todo!(),
        }
    }
}
