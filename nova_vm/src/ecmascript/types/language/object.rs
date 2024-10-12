// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod internal_methods;
mod internal_slots;
mod into_object;
mod property_key;
mod property_storage;

use std::hash::Hash;

#[cfg(feature = "date")]
use super::value::DATE_DISCRIMINANT;
#[cfg(feature = "array-buffer")]
use super::value::{
    ARRAY_BUFFER_DISCRIMINANT, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
    DATA_VIEW_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
    INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT,
    UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
    UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
};
use super::{
    value::{
        ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
        ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT, ASYNC_ITERATOR_DISCRIMINANT,
        BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        ECMASCRIPT_FUNCTION_DISCRIMINANT, EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT,
        FINALIZATION_REGISTRY_DISCRIMINANT, GENERATOR_DISCRIMINANT, ITERATOR_DISCRIMINANT,
        MAP_DISCRIMINANT, MAP_ITERATOR_DISCRIMINANT, MODULE_DISCRIMINANT, OBJECT_DISCRIMINANT,
        PRIMITIVE_OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT,
        REGEXP_DISCRIMINANT, SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT,
        SHARED_ARRAY_BUFFER_DISCRIMINANT, WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT,
        WEAK_SET_DISCRIMINANT,
    },
    Function, IntoValue, Value,
};

#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "array-buffer")]
use crate::{
    ecmascript::builtins::{data_view::DataView, typed_array::TypedArray, ArrayBuffer},
    heap::indexes::TypedArrayIndex,
};
use crate::{
    ecmascript::{
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
            shared_array_buffer::SharedArrayBuffer,
            weak_map::WeakMap,
            weak_ref::WeakRef,
            weak_set::WeakSet,
            ArgumentsList, Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
        },
        execution::{Agent, JsResult},
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{ArrayIndex, ObjectIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

pub use data::ObjectHeapData;
pub use internal_methods::InternalMethods;
pub use internal_slots::InternalSlots;
pub use into_object::IntoObject;
pub use property_key::PropertyKey;
pub use property_storage::PropertyStorage;

/// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
///
/// In Nova
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Object {
    Object(OrdinaryObject) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject) = PRIMITIVE_OBJECT_DISCRIMINANT,
    Arguments(OrdinaryObject) = ARGUMENTS_DISCRIMINANT,
    Array(Array) = ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    DataView(DataView) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "date")]
    Date(Date) = DATE_DISCRIMINANT,
    Error(Error) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistry) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(Map) = MAP_DISCRIMINANT,
    Promise(Promise) = PROMISE_DISCRIMINANT,
    Proxy(Proxy) = PROXY_DISCRIMINANT,
    RegExp(RegExp) = REGEXP_DISCRIMINANT,
    Set(Set) = SET_DISCRIMINANT,
    SharedArrayBuffer(SharedArrayBuffer) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    WeakMap(WeakMap) = WEAK_MAP_DISCRIMINANT,
    WeakRef(WeakRef) = WEAK_REF_DISCRIMINANT,
    WeakSet(WeakSet) = WEAK_SET_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int8Array(TypedArrayIndex) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(TypedArrayIndex) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(TypedArrayIndex) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(TypedArrayIndex) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(TypedArrayIndex) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(TypedArrayIndex) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(TypedArrayIndex) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(TypedArrayIndex) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(TypedArrayIndex) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(TypedArrayIndex) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(TypedArrayIndex) = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncIterator = ASYNC_ITERATOR_DISCRIMINANT,
    Iterator = ITERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator) = ARRAY_ITERATOR_DISCRIMINANT,
    SetIterator(SetIterator) = SET_ITERATOR_DISCRIMINANT,
    MapIterator(MapIterator) = MAP_ITERATOR_DISCRIMINANT,
    Generator(Generator) = GENERATOR_DISCRIMINANT,
    Module(Module) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject) = EMBEDDER_OBJECT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrdinaryObject(pub(crate) ObjectIndex);

impl IntoValue for Object {
    fn into_value(self) -> Value {
        match self {
            Object::Object(data) => Value::Object(data),
            Object::BoundFunction(data) => Value::BoundFunction(data),
            Object::BuiltinFunction(data) => Value::BuiltinFunction(data),
            Object::ECMAScriptFunction(data) => Value::ECMAScriptFunction(data),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => Value::BuiltinConstructorFunction(data),
            Object::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data),
            Object::Arguments(data) => Value::Arguments(data),
            Object::Array(data) => Value::Array(data),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => Value::ArrayBuffer(data),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => Value::DataView(data),
            #[cfg(feature = "date")]
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
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => Value::Int8Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => Value::Uint8Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => Value::Uint8ClampedArray(data),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => Value::Int16Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => Value::Uint16Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => Value::Int32Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => Value::Uint32Array(data),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => Value::BigInt64Array(data),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => Value::BigUint64Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => Value::Float32Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => Value::Float64Array(data),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => Value::ArrayIterator(data),
            Object::SetIterator(data) => Value::SetIterator(data),
            Object::MapIterator(data) => Value::MapIterator(data),
            Object::Generator(data) => Value::Generator(data),
            Object::Module(data) => Value::Module(data),
            Object::EmbedderObject(data) => Value::EmbedderObject(data),
        }
    }
}

impl IntoObject for Object {
    #[inline(always)]
    fn into_object(self) -> Object {
        self
    }
}

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

impl InternalSlots for OrdinaryObject {
    #[inline(always)]
    fn get_backing_object(self, _: &Agent) -> Option<OrdinaryObject> {
        Some(self)
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject) {
        unreachable!();
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject {
        unreachable!();
    }

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
            Object::BuiltinConstructorFunction(data) => Value::BuiltinConstructorFunction(data),
            Object::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data)
            }
            Object::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data),
            Object::Arguments(data) => Value::Arguments(data),
            Object::Array(data) => Value::Array(data),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => Value::ArrayBuffer(data),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => Value::DataView(data),
            #[cfg(feature = "date")]
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
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => Value::Int8Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => Value::Uint8Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => Value::Uint8ClampedArray(data),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => Value::Int16Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => Value::Uint16Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => Value::Int32Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => Value::Uint32Array(data),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => Value::BigInt64Array(data),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => Value::BigUint64Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => Value::Float32Array(data),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => Value::Float64Array(data),
            Object::AsyncFromSyncIterator => Value::AsyncFromSyncIterator,
            Object::AsyncIterator => Value::AsyncIterator,
            Object::Iterator => Value::Iterator,
            Object::ArrayIterator(data) => Value::ArrayIterator(data),
            Object::SetIterator(data) => Value::SetIterator(data),
            Object::MapIterator(data) => Value::MapIterator(data),
            Object::Generator(data) => Value::Generator(data),
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
            | Value::SmallF64(_)
            | Value::BigInt(_)
            | Value::SmallBigInt(_) => Err(()),
            Value::Object(x) => Ok(Object::from(x)),
            Value::Array(x) => Ok(Object::from(x)),
            #[cfg(feature = "date")]
            Value::Date(x) => Ok(Object::Date(x)),
            Value::Error(x) => Ok(Object::from(x)),
            Value::BoundFunction(x) => Ok(Object::from(x)),
            Value::BuiltinFunction(x) => Ok(Object::from(x)),
            Value::ECMAScriptFunction(x) => Ok(Object::from(x)),
            Value::BuiltinGeneratorFunction => Ok(Object::BuiltinGeneratorFunction),
            Value::BuiltinConstructorFunction(data) => Ok(Object::BuiltinConstructorFunction(data)),
            Value::BuiltinPromiseResolvingFunction(data) => {
                Ok(Object::BuiltinPromiseResolvingFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Object::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Object::BuiltinProxyRevokerFunction),
            Value::PrimitiveObject(data) => Ok(Object::PrimitiveObject(data)),
            Value::Arguments(data) => Ok(Object::Arguments(data)),
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(idx) => Ok(Object::ArrayBuffer(idx)),
            #[cfg(feature = "array-buffer")]
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
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => Ok(Object::Int8Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => Ok(Object::Uint8Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => Ok(Object::Uint8ClampedArray(data)),
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => Ok(Object::Int16Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => Ok(Object::Uint16Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => Ok(Object::Int32Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => Ok(Object::Uint32Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => Ok(Object::BigInt64Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => Ok(Object::BigUint64Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => Ok(Object::Float32Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => Ok(Object::Float64Array(data)),
            Value::AsyncFromSyncIterator => Ok(Object::AsyncFromSyncIterator),
            Value::AsyncIterator => Ok(Object::AsyncIterator),
            Value::Iterator => Ok(Object::Iterator),
            Value::ArrayIterator(data) => Ok(Object::ArrayIterator(data)),
            Value::SetIterator(data) => Ok(Object::SetIterator(data)),
            Value::MapIterator(data) => Ok(Object::MapIterator(data)),
            Value::Generator(data) => Ok(Object::Generator(data)),
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

impl Hash for Object {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Object::Object(data) => data.get_index().hash(state),
            Object::BoundFunction(data) => data.get_index().hash(state),
            Object::BuiltinFunction(data) => data.get_index().hash(state),
            Object::ECMAScriptFunction(data) => data.get_index().hash(state),
            Object::BuiltinGeneratorFunction => {}
            Object::BuiltinConstructorFunction(data) => data.get_index().hash(state),
            Object::BuiltinPromiseResolvingFunction(data) => data.get_index().hash(state),
            Object::BuiltinPromiseCollectorFunction => {}
            Object::BuiltinProxyRevokerFunction => {}
            Object::PrimitiveObject(data) => data.get_index().hash(state),
            Object::Arguments(data) => data.get_index().hash(state),
            Object::Array(data) => data.get_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.get_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.get_index().hash(state),
            #[cfg(feature = "date")]
            Object::Date(data) => data.get_index().hash(state),
            Object::Error(data) => data.get_index().hash(state),
            Object::FinalizationRegistry(data) => data.get_index().hash(state),
            Object::Map(data) => data.get_index().hash(state),
            Object::Promise(data) => data.get_index().hash(state),
            Object::Proxy(data) => data.get_index().hash(state),
            Object::RegExp(data) => data.get_index().hash(state),
            Object::Set(data) => data.get_index().hash(state),
            Object::SharedArrayBuffer(data) => data.get_index().hash(state),
            Object::WeakMap(data) => data.get_index().hash(state),
            Object::WeakRef(data) => data.get_index().hash(state),
            Object::WeakSet(data) => data.get_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => data.into_index().hash(state),
            Object::AsyncFromSyncIterator => {}
            Object::AsyncIterator => {}
            Object::Iterator => {}
            Object::ArrayIterator(data) => data.get_index().hash(state),
            Object::SetIterator(data) => data.get_index().hash(state),
            Object::MapIterator(data) => data.get_index().hash(state),
            Object::Generator(data) => data.get_index().hash(state),
            Object::Module(data) => data.get_index().hash(state),
            Object::EmbedderObject(data) => data.get_index().hash(state),
        }
    }
}

impl InternalSlots for Object {
    fn get_backing_object(self, _: &Agent) -> Option<OrdinaryObject> {
        unreachable!("Object should not try to access its backing object");
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject) {
        unreachable!("Object should not try to create its backing object");
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject {
        unreachable!("Object should not try to create its backing object");
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        match self {
            Object::Object(data) => data.internal_extensible(agent),
            Object::Array(data) => data.internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_extensible(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_extensible(agent),
            Object::Error(data) => data.internal_extensible(agent),
            Object::BoundFunction(data) => data.internal_extensible(agent),
            Object::BuiltinFunction(data) => data.internal_extensible(agent),
            Object::ECMAScriptFunction(data) => data.internal_extensible(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_extensible(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_extensible(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_extensible(agent),
            Object::Arguments(data) => data.internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_extensible(agent),
            Object::FinalizationRegistry(data) => data.internal_extensible(agent),
            Object::Map(data) => data.internal_extensible(agent),
            Object::Promise(data) => data.internal_extensible(agent),
            Object::Proxy(data) => data.internal_extensible(agent),
            Object::RegExp(data) => data.internal_extensible(agent),
            Object::Set(data) => data.internal_extensible(agent),
            Object::SharedArrayBuffer(data) => data.internal_extensible(agent),
            Object::WeakMap(data) => data.internal_extensible(agent),
            Object::WeakRef(data) => data.internal_extensible(agent),
            Object::WeakSet(data) => data.internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_extensible(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_extensible(agent),
            Object::SetIterator(data) => data.internal_extensible(agent),
            Object::MapIterator(data) => data.internal_extensible(agent),
            Object::Generator(data) => data.internal_extensible(agent),
            Object::Module(data) => data.internal_extensible(agent),
            Object::EmbedderObject(data) => data.internal_extensible(agent),
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        match self {
            Object::Object(data) => data.internal_set_extensible(agent, value),
            Object::Array(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set_extensible(agent, value),
            Object::Error(data) => data.internal_set_extensible(agent, value),
            Object::BoundFunction(data) => data.internal_set_extensible(agent, value),
            Object::BuiltinFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::ECMAScriptFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_extensible(agent, value)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_extensible(agent, value),
            Object::Arguments(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set_extensible(agent, value),
            Object::FinalizationRegistry(data) => data.internal_set_extensible(agent, value),
            Object::Map(data) => data.internal_set_extensible(agent, value),
            Object::Promise(data) => data.internal_set_extensible(agent, value),
            Object::Proxy(data) => data.internal_set_extensible(agent, value),
            Object::RegExp(data) => data.internal_set_extensible(agent, value),
            Object::Set(data) => data.internal_set_extensible(agent, value),
            Object::SharedArrayBuffer(data) => data.internal_set_extensible(agent, value),
            Object::WeakMap(data) => data.internal_set_extensible(agent, value),
            Object::WeakRef(data) => data.internal_set_extensible(agent, value),
            Object::WeakSet(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_extensible(agent, value)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_extensible(agent, value)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_set_extensible(agent, value),
            Object::SetIterator(data) => data.internal_set_extensible(agent, value),
            Object::MapIterator(data) => data.internal_set_extensible(agent, value),
            Object::Generator(data) => data.internal_set_extensible(agent, value),
            Object::Module(data) => data.internal_set_extensible(agent, value),
            Object::EmbedderObject(data) => data.internal_set_extensible(agent, value),
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        match self {
            Object::Object(data) => data.internal_prototype(agent),
            Object::Array(data) => data.internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_prototype(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_prototype(agent),
            Object::Error(data) => data.internal_prototype(agent),
            Object::BoundFunction(data) => data.internal_prototype(agent),
            Object::BuiltinFunction(data) => data.internal_prototype(agent),
            Object::ECMAScriptFunction(data) => data.internal_prototype(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_prototype(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_prototype(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_prototype(agent),
            Object::Arguments(data) => data.internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_prototype(agent),
            Object::FinalizationRegistry(data) => data.internal_prototype(agent),
            Object::Map(data) => data.internal_prototype(agent),
            Object::Promise(data) => data.internal_prototype(agent),
            Object::Proxy(data) => data.internal_prototype(agent),
            Object::RegExp(data) => data.internal_prototype(agent),
            Object::Set(data) => data.internal_prototype(agent),
            Object::SharedArrayBuffer(data) => data.internal_prototype(agent),
            Object::WeakMap(data) => data.internal_prototype(agent),
            Object::WeakRef(data) => data.internal_prototype(agent),
            Object::WeakSet(data) => data.internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_prototype(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_prototype(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_prototype(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_prototype(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_prototype(agent),
            Object::SetIterator(data) => data.internal_prototype(agent),
            Object::MapIterator(data) => data.internal_prototype(agent),
            Object::Generator(data) => data.internal_prototype(agent),
            Object::Module(data) => data.internal_prototype(agent),
            Object::EmbedderObject(data) => data.internal_prototype(agent),
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        match self {
            Object::Object(data) => data.internal_set_prototype(agent, prototype),
            Object::Array(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set_prototype(agent, prototype),
            Object::Error(data) => data.internal_set_prototype(agent, prototype),
            Object::BoundFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::BuiltinFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::ECMAScriptFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set_prototype(agent, prototype)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_prototype(agent, prototype)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_prototype(agent, prototype),
            Object::Arguments(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set_prototype(agent, prototype),
            Object::FinalizationRegistry(data) => data.internal_set_prototype(agent, prototype),
            Object::Map(data) => data.internal_set_prototype(agent, prototype),
            Object::Promise(data) => data.internal_set_prototype(agent, prototype),
            Object::Proxy(data) => data.internal_set_prototype(agent, prototype),
            Object::RegExp(data) => data.internal_set_prototype(agent, prototype),
            Object::Set(data) => data.internal_set_prototype(agent, prototype),
            Object::SharedArrayBuffer(data) => data.internal_set_prototype(agent, prototype),
            Object::WeakMap(data) => data.internal_set_prototype(agent, prototype),
            Object::WeakRef(data) => data.internal_set_prototype(agent, prototype),
            Object::WeakSet(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_prototype(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_prototype(agent, prototype)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::SetIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::MapIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::Generator(data) => data.internal_set_prototype(agent, prototype),
            Object::Module(data) => data.internal_set_prototype(agent, prototype),
            Object::EmbedderObject(data) => data.internal_set_prototype(agent, prototype),
        }
    }
}

impl InternalMethods for Object {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        match self {
            Object::Object(data) => data.internal_get_prototype_of(agent),
            Object::Array(data) => data.internal_get_prototype_of(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get_prototype_of(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get_prototype_of(agent),
            Object::Error(data) => data.internal_get_prototype_of(agent),
            Object::BoundFunction(data) => data.internal_get_prototype_of(agent),
            Object::BuiltinFunction(data) => data.internal_get_prototype_of(agent),
            Object::ECMAScriptFunction(data) => data.internal_get_prototype_of(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_get_prototype_of(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_get_prototype_of(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get_prototype_of(agent),
            Object::Arguments(data) => data.internal_get_prototype_of(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get_prototype_of(agent),
            Object::FinalizationRegistry(data) => data.internal_get_prototype_of(agent),
            Object::Map(data) => data.internal_get_prototype_of(agent),
            Object::Promise(data) => data.internal_get_prototype_of(agent),
            Object::Proxy(data) => data.internal_get_prototype_of(agent),
            Object::RegExp(data) => data.internal_get_prototype_of(agent),
            Object::Set(data) => data.internal_get_prototype_of(agent),
            Object::SharedArrayBuffer(data) => data.internal_get_prototype_of(agent),
            Object::WeakMap(data) => data.internal_get_prototype_of(agent),
            Object::WeakRef(data) => data.internal_get_prototype_of(agent),
            Object::WeakSet(data) => data.internal_get_prototype_of(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_get_prototype_of(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_prototype_of(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_prototype_of(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_get_prototype_of(agent),
            Object::SetIterator(data) => data.internal_get_prototype_of(agent),
            Object::MapIterator(data) => data.internal_get_prototype_of(agent),
            Object::Generator(data) => data.internal_get_prototype_of(agent),
            Object::Module(data) => data.internal_get_prototype_of(agent),
            Object::EmbedderObject(data) => data.internal_get_prototype_of(agent),
        }
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Array(data) => data.internal_set_prototype_of(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set_prototype_of(agent, prototype),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Error(data) => data.internal_set_prototype_of(agent, prototype),
            Object::BoundFunction(data) => data.internal_set_prototype_of(agent, prototype),
            Object::BuiltinFunction(data) => data.internal_set_prototype_of(agent, prototype),
            Object::ECMAScriptFunction(data) => data.internal_set_prototype_of(agent, prototype),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set_prototype_of(agent, prototype)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_prototype_of(agent, prototype)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Arguments(data) => data.internal_set_prototype_of(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set_prototype_of(agent, prototype),
            Object::FinalizationRegistry(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Map(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Promise(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Proxy(data) => data.internal_set_prototype_of(agent, prototype),
            Object::RegExp(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Set(data) => data.internal_set_prototype_of(agent, prototype),
            Object::SharedArrayBuffer(data) => data.internal_set_prototype_of(agent, prototype),
            Object::WeakMap(data) => data.internal_set_prototype_of(agent, prototype),
            Object::WeakRef(data) => data.internal_set_prototype_of(agent, prototype),
            Object::WeakSet(data) => data.internal_set_prototype_of(agent, prototype),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_prototype_of(agent, prototype)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_set_prototype_of(agent, prototype),
            Object::SetIterator(data) => data.internal_set_prototype_of(agent, prototype),
            Object::MapIterator(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Generator(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Module(data) => data.internal_set_prototype_of(agent, prototype),
            Object::EmbedderObject(data) => data.internal_set_prototype_of(agent, prototype),
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_is_extensible(agent),
            Object::Array(data) => data.internal_is_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_is_extensible(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_is_extensible(agent),
            Object::Error(data) => data.internal_is_extensible(agent),
            Object::BoundFunction(data) => data.internal_is_extensible(agent),
            Object::BuiltinFunction(data) => data.internal_is_extensible(agent),
            Object::ECMAScriptFunction(data) => data.internal_is_extensible(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_is_extensible(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_is_extensible(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_is_extensible(agent),
            Object::Arguments(data) => data.internal_is_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_is_extensible(agent),
            Object::FinalizationRegistry(data) => data.internal_is_extensible(agent),
            Object::Map(data) => data.internal_is_extensible(agent),
            Object::Promise(data) => data.internal_is_extensible(agent),
            Object::Proxy(data) => data.internal_is_extensible(agent),
            Object::RegExp(data) => data.internal_is_extensible(agent),
            Object::Set(data) => data.internal_is_extensible(agent),
            Object::SharedArrayBuffer(data) => data.internal_is_extensible(agent),
            Object::WeakMap(data) => data.internal_is_extensible(agent),
            Object::WeakRef(data) => data.internal_is_extensible(agent),
            Object::WeakSet(data) => data.internal_is_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_is_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_is_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_is_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_is_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_is_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_is_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_is_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_is_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_is_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_is_extensible(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_is_extensible(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_is_extensible(agent),
            Object::SetIterator(data) => data.internal_is_extensible(agent),
            Object::MapIterator(data) => data.internal_is_extensible(agent),
            Object::Generator(data) => data.internal_is_extensible(agent),
            Object::Module(data) => data.internal_is_extensible(agent),
            Object::EmbedderObject(data) => data.internal_is_extensible(agent),
        }
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_prevent_extensions(agent),
            Object::Array(data) => data.internal_prevent_extensions(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_prevent_extensions(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_prevent_extensions(agent),
            Object::Error(data) => data.internal_prevent_extensions(agent),
            Object::BoundFunction(data) => data.internal_prevent_extensions(agent),
            Object::BuiltinFunction(data) => data.internal_prevent_extensions(agent),
            Object::ECMAScriptFunction(data) => data.internal_prevent_extensions(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_prevent_extensions(agent),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_prevent_extensions(agent)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_prevent_extensions(agent),
            Object::Arguments(data) => data.internal_prevent_extensions(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_prevent_extensions(agent),
            Object::FinalizationRegistry(data) => data.internal_prevent_extensions(agent),
            Object::Map(data) => data.internal_prevent_extensions(agent),
            Object::Promise(data) => data.internal_prevent_extensions(agent),
            Object::Proxy(data) => data.internal_prevent_extensions(agent),
            Object::RegExp(data) => data.internal_prevent_extensions(agent),
            Object::Set(data) => data.internal_prevent_extensions(agent),
            Object::SharedArrayBuffer(data) => data.internal_prevent_extensions(agent),
            Object::WeakMap(data) => data.internal_prevent_extensions(agent),
            Object::WeakRef(data) => data.internal_prevent_extensions(agent),
            Object::WeakSet(data) => data.internal_prevent_extensions(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_prevent_extensions(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_prevent_extensions(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_prevent_extensions(agent),
            Object::SetIterator(data) => data.internal_prevent_extensions(agent),
            Object::MapIterator(data) => data.internal_prevent_extensions(agent),
            Object::Generator(data) => data.internal_prevent_extensions(agent),
            Object::Module(data) => data.internal_prevent_extensions(agent),
            Object::EmbedderObject(data) => data.internal_prevent_extensions(agent),
        }
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match self {
            Object::Object(data) => data.internal_get_own_property(agent, property_key),
            Object::Array(data) => data.internal_get_own_property(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get_own_property(agent, property_key),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get_own_property(agent, property_key),
            Object::Error(data) => data.internal_get_own_property(agent, property_key),
            Object::BoundFunction(data) => data.internal_get_own_property(agent, property_key),
            Object::BuiltinFunction(data) => data.internal_get_own_property(agent, property_key),
            Object::ECMAScriptFunction(data) => data.internal_get_own_property(agent, property_key),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_get_own_property(agent, property_key)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get_own_property(agent, property_key)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get_own_property(agent, property_key),
            Object::Arguments(data) => data.internal_get_own_property(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get_own_property(agent, property_key),
            Object::FinalizationRegistry(data) => {
                data.internal_get_own_property(agent, property_key)
            }
            Object::Map(data) => data.internal_get_own_property(agent, property_key),
            Object::Promise(data) => data.internal_get_own_property(agent, property_key),
            Object::Proxy(data) => data.internal_get_own_property(agent, property_key),
            Object::RegExp(data) => data.internal_get_own_property(agent, property_key),
            Object::Set(data) => data.internal_get_own_property(agent, property_key),
            Object::SharedArrayBuffer(data) => data.internal_get_own_property(agent, property_key),
            Object::WeakMap(data) => data.internal_get_own_property(agent, property_key),
            Object::WeakRef(data) => data.internal_get_own_property(agent, property_key),
            Object::WeakSet(data) => data.internal_get_own_property(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_own_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_own_property(agent, property_key)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_get_own_property(agent, property_key),
            Object::SetIterator(data) => data.internal_get_own_property(agent, property_key),
            Object::MapIterator(data) => data.internal_get_own_property(agent, property_key),
            Object::Generator(data) => data.internal_get_own_property(agent, property_key),
            Object::Module(data) => data.internal_get_own_property(agent, property_key),
            Object::EmbedderObject(data) => data.internal_get_own_property(agent, property_key),
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
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            #[cfg(feature = "date")]
            Object::Date(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Error(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::BoundFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::BuiltinFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Arguments(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::FinalizationRegistry(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Map(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Promise(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Proxy(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::RegExp(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Set(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::SharedArrayBuffer(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::WeakMap(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::WeakRef(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::WeakSet(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::SetIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::MapIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Generator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::Module(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Object::EmbedderObject(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor)
            }
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_has_property(agent, property_key),
            Object::Array(data) => data.internal_has_property(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_has_property(agent, property_key),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_has_property(agent, property_key),
            Object::Error(data) => data.internal_has_property(agent, property_key),
            Object::BoundFunction(data) => data.internal_has_property(agent, property_key),
            Object::BuiltinFunction(data) => data.internal_has_property(agent, property_key),
            Object::ECMAScriptFunction(data) => data.internal_has_property(agent, property_key),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_has_property(agent, property_key)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_has_property(agent, property_key)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_has_property(agent, property_key),
            Object::Arguments(data) => data.internal_has_property(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_has_property(agent, property_key),
            Object::FinalizationRegistry(data) => data.internal_has_property(agent, property_key),
            Object::Map(data) => data.internal_has_property(agent, property_key),
            Object::Promise(data) => data.internal_has_property(agent, property_key),
            Object::Proxy(data) => data.internal_has_property(agent, property_key),
            Object::RegExp(data) => data.internal_has_property(agent, property_key),
            Object::Set(data) => data.internal_has_property(agent, property_key),
            Object::SharedArrayBuffer(data) => data.internal_has_property(agent, property_key),
            Object::WeakMap(data) => data.internal_has_property(agent, property_key),
            Object::WeakRef(data) => data.internal_has_property(agent, property_key),
            Object::WeakSet(data) => data.internal_has_property(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_has_property(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_has_property(agent, property_key)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_has_property(agent, property_key),
            Object::SetIterator(data) => data.internal_has_property(agent, property_key),
            Object::MapIterator(data) => data.internal_has_property(agent, property_key),
            Object::Generator(data) => data.internal_has_property(agent, property_key),
            Object::Module(data) => data.internal_has_property(agent, property_key),
            Object::EmbedderObject(data) => data.internal_has_property(agent, property_key),
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        match self {
            Object::Object(data) => data.internal_get(agent, property_key, receiver),
            Object::Array(data) => data.internal_get(agent, property_key, receiver),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get(agent, property_key, receiver),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get(agent, property_key, receiver),
            Object::Error(data) => data.internal_get(agent, property_key, receiver),
            Object::BoundFunction(data) => data.internal_get(agent, property_key, receiver),
            Object::BuiltinFunction(data) => data.internal_get(agent, property_key, receiver),
            Object::ECMAScriptFunction(data) => data.internal_get(agent, property_key, receiver),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_get(agent, property_key, receiver)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get(agent, property_key, receiver)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get(agent, property_key, receiver),
            Object::Arguments(data) => data.internal_get(agent, property_key, receiver),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get(agent, property_key, receiver),
            Object::FinalizationRegistry(data) => data.internal_get(agent, property_key, receiver),
            Object::Map(data) => data.internal_get(agent, property_key, receiver),
            Object::Promise(data) => data.internal_get(agent, property_key, receiver),
            Object::Proxy(data) => data.internal_get(agent, property_key, receiver),
            Object::RegExp(data) => data.internal_get(agent, property_key, receiver),
            Object::Set(data) => data.internal_get(agent, property_key, receiver),
            Object::SharedArrayBuffer(data) => data.internal_get(agent, property_key, receiver),
            Object::WeakMap(data) => data.internal_get(agent, property_key, receiver),
            Object::WeakRef(data) => data.internal_get(agent, property_key, receiver),
            Object::WeakSet(data) => data.internal_get(agent, property_key, receiver),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get(agent, property_key, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get(agent, property_key, receiver)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_get(agent, property_key, receiver),
            Object::SetIterator(data) => data.internal_get(agent, property_key, receiver),
            Object::MapIterator(data) => data.internal_get(agent, property_key, receiver),
            Object::Generator(data) => data.internal_get(agent, property_key, receiver),
            Object::Module(data) => data.internal_get(agent, property_key, receiver),
            Object::EmbedderObject(data) => data.internal_get(agent, property_key, receiver),
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
            Object::Object(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Array(data) => data.internal_set(agent, property_key, value, receiver),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set(agent, property_key, value, receiver),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Error(data) => data.internal_set(agent, property_key, value, receiver),
            Object::BoundFunction(data) => data.internal_set(agent, property_key, value, receiver),
            Object::BuiltinFunction(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::Arguments(data) => data.internal_set(agent, property_key, value, receiver),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set(agent, property_key, value, receiver),
            Object::FinalizationRegistry(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::Map(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Promise(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Proxy(data) => data.internal_set(agent, property_key, value, receiver),
            Object::RegExp(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Set(data) => data.internal_set(agent, property_key, value, receiver),
            Object::SharedArrayBuffer(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::WeakMap(data) => data.internal_set(agent, property_key, value, receiver),
            Object::WeakRef(data) => data.internal_set(agent, property_key, value, receiver),
            Object::WeakSet(data) => data.internal_set(agent, property_key, value, receiver),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set(agent, property_key, value, receiver)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_set(agent, property_key, value, receiver),
            Object::SetIterator(data) => data.internal_set(agent, property_key, value, receiver),
            Object::MapIterator(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Generator(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Module(data) => data.internal_set(agent, property_key, value, receiver),
            Object::EmbedderObject(data) => data.internal_set(agent, property_key, value, receiver),
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_delete(agent, property_key),
            Object::Array(data) => data.internal_delete(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_delete(agent, property_key),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_delete(agent, property_key),
            Object::Error(data) => data.internal_delete(agent, property_key),
            Object::BoundFunction(data) => data.internal_delete(agent, property_key),
            Object::BuiltinFunction(data) => data.internal_delete(agent, property_key),
            Object::ECMAScriptFunction(data) => data.internal_delete(agent, property_key),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_delete(agent, property_key),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_delete(agent, property_key)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_delete(agent, property_key),
            Object::Arguments(data) => data.internal_delete(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_delete(agent, property_key),
            Object::FinalizationRegistry(data) => data.internal_delete(agent, property_key),
            Object::Map(data) => data.internal_delete(agent, property_key),
            Object::Promise(data) => data.internal_delete(agent, property_key),
            Object::Proxy(data) => data.internal_delete(agent, property_key),
            Object::RegExp(data) => data.internal_delete(agent, property_key),
            Object::Set(data) => data.internal_delete(agent, property_key),
            Object::SharedArrayBuffer(data) => data.internal_delete(agent, property_key),
            Object::WeakMap(data) => data.internal_delete(agent, property_key),
            Object::WeakRef(data) => data.internal_delete(agent, property_key),
            Object::WeakSet(data) => data.internal_delete(agent, property_key),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_delete(agent, property_key)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_delete(agent, property_key)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_delete(agent, property_key),
            Object::SetIterator(data) => data.internal_delete(agent, property_key),
            Object::MapIterator(data) => data.internal_delete(agent, property_key),
            Object::Generator(data) => data.internal_delete(agent, property_key),
            Object::Module(data) => data.internal_delete(agent, property_key),
            Object::EmbedderObject(data) => data.internal_delete(agent, property_key),
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self {
            Object::Object(data) => data.internal_own_property_keys(agent),
            Object::Array(data) => data.internal_own_property_keys(agent),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_own_property_keys(agent),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_own_property_keys(agent),
            Object::Error(data) => data.internal_own_property_keys(agent),
            Object::BoundFunction(data) => data.internal_own_property_keys(agent),
            Object::BuiltinFunction(data) => data.internal_own_property_keys(agent),
            Object::ECMAScriptFunction(data) => data.internal_own_property_keys(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_own_property_keys(agent),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_own_property_keys(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_own_property_keys(agent),
            Object::Arguments(data) => data.internal_own_property_keys(agent),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_own_property_keys(agent),
            Object::FinalizationRegistry(data) => data.internal_own_property_keys(agent),
            Object::Map(data) => data.internal_own_property_keys(agent),
            Object::Promise(data) => data.internal_own_property_keys(agent),
            Object::Proxy(data) => data.internal_own_property_keys(agent),
            Object::RegExp(data) => data.internal_own_property_keys(agent),
            Object::Set(data) => data.internal_own_property_keys(agent),
            Object::SharedArrayBuffer(data) => data.internal_own_property_keys(agent),
            Object::WeakMap(data) => data.internal_own_property_keys(agent),
            Object::WeakRef(data) => data.internal_own_property_keys(agent),
            Object::WeakSet(data) => data.internal_own_property_keys(agent),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_own_property_keys(agent)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_own_property_keys(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_own_property_keys(agent),
            Object::SetIterator(data) => data.internal_own_property_keys(agent),
            Object::MapIterator(data) => data.internal_own_property_keys(agent),
            Object::Generator(data) => data.internal_own_property_keys(agent),
            Object::Module(data) => data.internal_own_property_keys(agent),
            Object::EmbedderObject(data) => data.internal_own_property_keys(agent),
        }
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        match self {
            Object::BoundFunction(data) => data.internal_call(agent, this_value, arguments_list),
            Object::BuiltinFunction(data) => data.internal_call(agent, this_value, arguments_list),
            Object::ECMAScriptFunction(data) => {
                data.internal_call(agent, this_value, arguments_list)
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
            Object::BoundFunction(data) => {
                data.internal_construct(agent, arguments_list, new_target)
            }
            Object::BuiltinFunction(data) => {
                data.internal_construct(agent, arguments_list, new_target)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_construct(agent, arguments_list, new_target)
            }
            _ => unreachable!(),
        }
    }
}

impl HeapMarkAndSweep for Object {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Object::Object(data) => data.mark_values(queues),
            Object::Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "date")]
            Object::Date(data) => data.mark_values(queues),
            Object::Error(data) => data.mark_values(queues),
            Object::BoundFunction(data) => data.mark_values(queues),
            Object::BuiltinFunction(data) => data.mark_values(queues),
            Object::ECMAScriptFunction(data) => data.mark_values(queues),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.mark_values(queues),
            Object::BuiltinPromiseResolvingFunction(data) => data.mark_values(queues),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.mark_values(queues),
            Object::Arguments(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.mark_values(queues),
            Object::FinalizationRegistry(data) => data.mark_values(queues),
            Object::Map(data) => data.mark_values(queues),
            Object::Promise(data) => data.mark_values(queues),
            Object::Proxy(data) => data.mark_values(queues),
            Object::RegExp(data) => data.mark_values(queues),
            Object::Set(data) => data.mark_values(queues),
            Object::SharedArrayBuffer(data) => data.mark_values(queues),
            Object::WeakMap(data) => data.mark_values(queues),
            Object::WeakRef(data) => data.mark_values(queues),
            Object::WeakSet(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => data.mark_values(queues),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.mark_values(queues),
            Object::SetIterator(data) => data.mark_values(queues),
            Object::MapIterator(data) => data.mark_values(queues),
            Object::Generator(data) => data.mark_values(queues),
            Object::Module(data) => data.mark_values(queues),
            Object::EmbedderObject(data) => data.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Object::Object(data) => data.sweep_values(compactions),
            Object::BoundFunction(data) => data.sweep_values(compactions),
            Object::BuiltinFunction(data) => data.sweep_values(compactions),
            Object::ECMAScriptFunction(data) => data.sweep_values(compactions),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.sweep_values(compactions),
            Object::BuiltinPromiseResolvingFunction(data) => data.sweep_values(compactions),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.sweep_values(compactions),
            Object::Arguments(data) => data.sweep_values(compactions),
            Object::Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.sweep_values(compactions),
            #[cfg(feature = "date")]
            Object::Date(data) => data.sweep_values(compactions),
            Object::Error(data) => data.sweep_values(compactions),
            Object::FinalizationRegistry(data) => data.sweep_values(compactions),
            Object::Map(data) => data.sweep_values(compactions),
            Object::Promise(data) => data.sweep_values(compactions),
            Object::Proxy(data) => data.sweep_values(compactions),
            Object::RegExp(data) => data.sweep_values(compactions),
            Object::Set(data) => data.sweep_values(compactions),
            Object::SharedArrayBuffer(data) => data.sweep_values(compactions),
            Object::WeakMap(data) => data.sweep_values(compactions),
            Object::WeakRef(data) => data.sweep_values(compactions),
            Object::WeakSet(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => data.sweep_values(compactions),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.sweep_values(compactions),
            Object::SetIterator(data) => data.sweep_values(compactions),
            Object::MapIterator(data) => data.sweep_values(compactions),
            Object::Generator(data) => data.sweep_values(compactions),
            Object::Module(data) => data.sweep_values(compactions),
            Object::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}

impl CreateHeapData<ObjectHeapData, OrdinaryObject> for Heap {
    fn create(&mut self, data: ObjectHeapData) -> OrdinaryObject {
        self.objects.push(Some(data));
        OrdinaryObject(ObjectIndex::last(&self.objects))
    }
}
