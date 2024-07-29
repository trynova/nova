// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        DATA_VIEW_DISCRIMINANT, DATE_DISCRIMINANT, ECMASCRIPT_FUNCTION_DISCRIMINANT,
        EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT, FINALIZATION_REGISTRY_DISCRIMINANT,
        FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, GENERATOR_DISCRIMINANT,
        INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT,
        ITERATOR_DISCRIMINANT, MAP_DISCRIMINANT, MODULE_DISCRIMINANT, OBJECT_DISCRIMINANT,
        PRIMITIVE_OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT,
        REGEXP_DISCRIMINANT, SET_DISCRIMINANT, SHARED_ARRAY_BUFFER_DISCRIMINANT,
        UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
        UINT_8_CLAMPED_ARRAY_DISCRIMINANT, WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT,
        WEAK_SET_DISCRIMINANT,
    },
    Function, IntoValue, Value,
};
use crate::{
    ecmascript::{
        builtins::{
            bound_function::BoundFunction,
            control_abstraction_objects::{
                generator_objects::Generator,
                promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction,
            },
            data_view::DataView,
            date::Date,
            embedder_object::EmbedderObject,
            error::Error,
            finalization_registry::FinalizationRegistry,
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            proxy::Proxy,
            regexp::RegExp,
            set::Set,
            shared_array_buffer::SharedArrayBuffer,
            typed_array::TypedArray,
            weak_map::WeakMap,
            weak_ref::WeakRef,
            weak_set::WeakSet,
            ArgumentsList, Array, ArrayBuffer, BuiltinFunction, ECMAScriptFunction,
        },
        execution::{Agent, JsResult},
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{ArrayIndex, ObjectIndex, TypedArrayIndex},
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
pub enum Object<'gen> {
    Object(OrdinaryObject<'gen>) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction<'gen>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'gen>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'gen>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction = BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'gen>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject<'gen>) = PRIMITIVE_OBJECT_DISCRIMINANT,
    Arguments(OrdinaryObject<'gen>) = ARGUMENTS_DISCRIMINANT,
    Array(Array<'gen>) = ARRAY_DISCRIMINANT,
    ArrayBuffer(ArrayBuffer<'gen>) = ARRAY_BUFFER_DISCRIMINANT,
    DataView(DataView<'gen>) = DATA_VIEW_DISCRIMINANT,
    Date(Date<'gen>) = DATE_DISCRIMINANT,
    Error(Error<'gen>) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistry<'gen>) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(Map<'gen>) = MAP_DISCRIMINANT,
    Promise(Promise<'gen>) = PROMISE_DISCRIMINANT,
    Proxy(Proxy<'gen>) = PROXY_DISCRIMINANT,
    RegExp(RegExp<'gen>) = REGEXP_DISCRIMINANT,
    Set(Set<'gen>) = SET_DISCRIMINANT,
    SharedArrayBuffer(SharedArrayBuffer<'gen>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    WeakMap(WeakMap<'gen>) = WEAK_MAP_DISCRIMINANT,
    WeakRef(WeakRef<'gen>) = WEAK_REF_DISCRIMINANT,
    WeakSet(WeakSet<'gen>) = WEAK_SET_DISCRIMINANT,
    Int8Array(TypedArrayIndex<'gen>) = INT_8_ARRAY_DISCRIMINANT,
    Uint8Array(TypedArrayIndex<'gen>) = UINT_8_ARRAY_DISCRIMINANT,
    Uint8ClampedArray(TypedArrayIndex<'gen>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    Int16Array(TypedArrayIndex<'gen>) = INT_16_ARRAY_DISCRIMINANT,
    Uint16Array(TypedArrayIndex<'gen>) = UINT_16_ARRAY_DISCRIMINANT,
    Int32Array(TypedArrayIndex<'gen>) = INT_32_ARRAY_DISCRIMINANT,
    Uint32Array(TypedArrayIndex<'gen>) = UINT_32_ARRAY_DISCRIMINANT,
    BigInt64Array(TypedArrayIndex<'gen>) = BIGINT_64_ARRAY_DISCRIMINANT,
    BigUint64Array(TypedArrayIndex<'gen>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    Float32Array(TypedArrayIndex<'gen>) = FLOAT_32_ARRAY_DISCRIMINANT,
    Float64Array(TypedArrayIndex<'gen>) = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncIterator = ASYNC_ITERATOR_DISCRIMINANT,
    Iterator = ITERATOR_DISCRIMINANT,
    Generator(Generator<'gen>) = GENERATOR_DISCRIMINANT,
    Module(Module<'gen>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject) = EMBEDDER_OBJECT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrdinaryObject<'gen>(pub(crate) ObjectIndex<'gen>);

impl<'gen> IntoValue<'gen> for Object<'gen> {
    fn into_value(self) -> Value<'gen> {
        match self {
            Object::Object(data) => Value::Object(data),
            Object::BoundFunction(data) => Value::BoundFunction(data),
            Object::BuiltinFunction(data) => Value::BuiltinFunction(data),
            Object::ECMAScriptFunction(data) => Value::ECMAScriptFunction(data),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data),
            Object::Arguments(data) => Value::Arguments(data),
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
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => Value::Generator(data),
            Object::Module(data) => Value::Module(data),
            Object::EmbedderObject(data) => Value::EmbedderObject(data),
        }
    }
}

impl<'gen> IntoObject<'gen> for Object<'gen> {
    #[inline(always)]
    fn into_object(self) -> Object<'gen> {
        self
    }
}

impl<'gen> IntoObject<'gen> for OrdinaryObject<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> IntoValue<'gen> for OrdinaryObject<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> From<OrdinaryObject<'gen>> for Object<'gen> {
    fn from(value: OrdinaryObject<'gen>) -> Self {
        Self::Object(value)
    }
}

impl<'gen> From<ObjectIndex<'gen>> for OrdinaryObject<'gen> {
    fn from(value: ObjectIndex<'gen>) -> Self {
        OrdinaryObject(value)
    }
}

impl<'gen> From<OrdinaryObject<'gen>> for Value<'gen> {
    fn from(value: OrdinaryObject<'gen>) -> Self {
        Self::Object(value)
    }
}

impl<'gen> TryFrom<Value<'gen>> for OrdinaryObject<'gen> {
    type Error = ();

    fn try_from(value: Value<'gen>) -> Result<Self, Self::Error> {
        match value {
            Value::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'gen> TryFrom<Object<'gen>> for OrdinaryObject<'gen> {
    type Error = ();

    fn try_from(value: Object<'gen>) -> Result<Self, Self::Error> {
        match value {
            Object::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'gen> InternalSlots<'gen> for OrdinaryObject<'gen> {
    #[inline(always)]
    fn get_backing_object(self, _: &Agent<'gen>) -> Option<OrdinaryObject<'gen>> {
        Some(self)
    }

    fn create_backing_object(self, _: &mut Agent<'gen>) -> OrdinaryObject<'gen> {
        unreachable!();
    }

    fn internal_extensible(self, agent: &Agent<'gen>) -> bool {
        agent[self].extensible
    }

    fn internal_set_extensible(self, agent: &mut Agent<'gen>, value: bool) {
        agent[self].extensible = value;
    }

    fn internal_prototype(self, agent: &Agent<'gen>) -> Option<Object<'gen>> {
        agent[self].prototype
    }

    fn internal_set_prototype(self, agent: &mut Agent<'gen>, prototype: Option<Object<'gen>>) {
        agent[self].prototype = prototype;
    }
}

impl<'gen> OrdinaryObject<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(ObjectIndex::from_u32_index(0))
    }
    pub(crate) const fn new(value: ObjectIndex<'gen>) -> Self {
        Self(value)
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<ObjectIndex<'gen>> for Object<'gen> {
    fn from(value: ObjectIndex<'gen>) -> Self {
        Object::Object(value.into())
    }
}

impl<'gen> From<ArrayIndex<'gen>> for Object<'gen> {
    fn from(value: ArrayIndex<'gen>) -> Self {
        Object::Array(value.into())
    }
}

impl<'gen> From<BoundFunction<'gen>> for Object<'gen> {
    fn from(value: BoundFunction<'gen>) -> Self {
        Object::BoundFunction(value)
    }
}

impl<'gen> From<Object<'gen>> for Value<'gen> {
    fn from(value: Object<'gen>) -> Self {
        match value {
            Object::Object(data) => Value::Object(data),
            Object::BoundFunction(data) => Value::BoundFunction(data),
            Object::BuiltinFunction(data) => Value::BuiltinFunction(data),
            Object::ECMAScriptFunction(data) => Value::ECMAScriptFunction(data),
            Object::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            Object::BuiltinConstructorFunction => Value::BuiltinConstructorFunction,
            Object::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data)
            }
            Object::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data),
            Object::Arguments(data) => Value::Arguments(data),
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
            Object::Generator(data) => Value::Generator(data),
            Object::Module(data) => Value::Module(data),
            Object::EmbedderObject(data) => Value::EmbedderObject(data),
        }
    }
}

impl<'gen> TryFrom<Value<'gen>> for Object<'gen> {
    type Error = ();
    fn try_from(value: Value<'gen>) -> Result<Self, ()> {
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
            Value::Date(x) => Ok(Object::Date(x)),
            Value::Error(x) => Ok(Object::from(x)),
            Value::BoundFunction(x) => Ok(Object::from(x)),
            Value::BuiltinFunction(x) => Ok(Object::from(x)),
            Value::ECMAScriptFunction(x) => Ok(Object::from(x)),
            Value::BuiltinGeneratorFunction => Ok(Object::BuiltinGeneratorFunction),
            Value::BuiltinConstructorFunction => Ok(Object::BuiltinConstructorFunction),
            Value::BuiltinPromiseResolvingFunction(data) => {
                Ok(Object::BuiltinPromiseResolvingFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Object::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Object::BuiltinProxyRevokerFunction),
            Value::PrimitiveObject(data) => Ok(Object::PrimitiveObject(data)),
            Value::Arguments(data) => Ok(Object::Arguments(data)),
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
            Value::Generator(data) => Ok(Object::Generator(data)),
            Value::Module(data) => Ok(Object::Module(data)),
            Value::EmbedderObject(data) => Ok(Object::EmbedderObject(data)),
        }
    }
}

impl<'gen> Object<'gen> {
    pub fn property_storage(self) -> PropertyStorage<'gen> {
        PropertyStorage::new(self)
    }
}

impl<'gen> InternalSlots<'gen> for Object<'gen> {
    fn get_backing_object(self, _: &Agent<'gen>) -> Option<OrdinaryObject<'gen>> {
        unreachable!("Object should not try to access its backing object");
    }

    fn create_backing_object(self, _: &mut Agent<'gen>) -> OrdinaryObject<'gen> {
        unreachable!("Object should not try to create its backing object");
    }

    fn internal_extensible(self, agent: &Agent<'gen>) -> bool {
        match self {
            Object::Object(data) => data.internal_extensible(agent),
            Object::Array(data) => data.internal_extensible(agent),
            Object::ArrayBuffer(data) => data.internal_extensible(agent),
            Object::Date(data) => data.internal_extensible(agent),
            Object::Error(data) => data.internal_extensible(agent),
            Object::BoundFunction(data) => data.internal_extensible(agent),
            Object::BuiltinFunction(data) => data.internal_extensible(agent),
            Object::ECMAScriptFunction(data) => data.internal_extensible(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_extensible(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_extensible(agent),
            Object::Arguments(data) => data.internal_extensible(agent),
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
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_extensible(agent),
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_extensible(agent),
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_extensible(agent)
            }
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_extensible(agent),
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).internal_extensible(agent),
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_extensible(agent),
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).internal_extensible(agent),
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_extensible(agent)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_extensible(agent)
            }
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_extensible(agent),
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_extensible(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_extensible(agent),
            Object::Module(data) => data.internal_extensible(agent),
            Object::EmbedderObject(data) => data.internal_extensible(agent),
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent<'gen>, value: bool) {
        match self {
            Object::Object(data) => data.internal_set_extensible(agent, value),
            Object::Array(data) => data.internal_set_extensible(agent, value),
            Object::ArrayBuffer(data) => data.internal_set_extensible(agent, value),
            Object::Date(data) => data.internal_set_extensible(agent, value),
            Object::Error(data) => data.internal_set_extensible(agent, value),
            Object::BoundFunction(data) => data.internal_set_extensible(agent, value),
            Object::BuiltinFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::ECMAScriptFunction(idx) => idx.internal_set_extensible(agent, value),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_extensible(agent, value)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_extensible(agent, value),
            Object::Arguments(data) => data.internal_set_extensible(agent, value),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_extensible(agent, value)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_extensible(agent, value)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_extensible(agent, value)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_extensible(agent, value)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_extensible(agent, value)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_extensible(agent, value)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_extensible(agent, value)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_extensible(agent, value)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_extensible(agent, value)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_extensible(agent, value)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_extensible(agent, value)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_set_extensible(agent, value),
            Object::Module(data) => data.internal_set_extensible(agent, value),
            Object::EmbedderObject(data) => data.internal_set_extensible(agent, value),
        }
    }

    fn internal_prototype(self, agent: &Agent<'gen>) -> Option<Object<'gen>> {
        match self {
            Object::Object(data) => data.internal_prototype(agent),
            Object::Array(data) => data.internal_prototype(agent),
            Object::ArrayBuffer(data) => data.internal_prototype(agent),
            Object::Date(data) => data.internal_prototype(agent),
            Object::Error(data) => data.internal_prototype(agent),
            Object::BoundFunction(data) => data.internal_prototype(agent),
            Object::BuiltinFunction(data) => data.internal_prototype(agent),
            Object::ECMAScriptFunction(data) => data.internal_prototype(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_prototype(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_prototype(agent),
            Object::Arguments(data) => data.internal_prototype(agent),
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
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_prototype(agent),
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_prototype(agent),
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_prototype(agent)
            }
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_prototype(agent),
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).internal_prototype(agent),
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_prototype(agent),
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).internal_prototype(agent),
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_prototype(agent)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_prototype(agent)
            }
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_prototype(agent),
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_prototype(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_prototype(agent),
            Object::Module(data) => data.internal_prototype(agent),
            Object::EmbedderObject(data) => data.internal_prototype(agent),
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent<'gen>, prototype: Option<Object<'gen>>) {
        match self {
            Object::Object(data) => data.internal_set_prototype(agent, prototype),
            Object::Array(data) => data.internal_set_prototype(agent, prototype),
            Object::ArrayBuffer(data) => data.internal_set_prototype(agent, prototype),
            Object::Date(data) => data.internal_set_prototype(agent, prototype),
            Object::Error(data) => data.internal_set_prototype(agent, prototype),
            Object::BoundFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::BuiltinFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::ECMAScriptFunction(data) => data.internal_set_prototype(agent, prototype),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_prototype(agent, prototype)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_prototype(agent, prototype),
            Object::Arguments(data) => data.internal_set_prototype(agent, prototype),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_prototype(agent, prototype)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_prototype(agent, prototype)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_prototype(agent, prototype)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_prototype(agent, prototype)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_prototype(agent, prototype)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_prototype(agent, prototype)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_prototype(agent, prototype)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_prototype(agent, prototype)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_prototype(agent, prototype)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_prototype(agent, prototype)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_prototype(agent, prototype)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_set_prototype(agent, prototype),
            Object::Module(data) => data.internal_set_prototype(agent, prototype),
            Object::EmbedderObject(data) => data.internal_set_prototype(agent, prototype),
        }
    }
}

impl<'gen> InternalMethods<'gen> for Object<'gen> {
    fn internal_get_prototype_of(self, agent: &mut Agent<'gen>) -> JsResult<'gen, Option<Object<'gen>>> {
        match self {
            Object::Object(data) => data.internal_get_prototype_of(agent),
            Object::Array(data) => data.internal_get_prototype_of(agent),
            Object::ArrayBuffer(data) => data.internal_get_prototype_of(agent),
            Object::Date(data) => data.internal_get_prototype_of(agent),
            Object::Error(data) => data.internal_get_prototype_of(agent),
            Object::BoundFunction(data) => data.internal_get_prototype_of(agent),
            Object::BuiltinFunction(data) => data.internal_get_prototype_of(agent),
            Object::ECMAScriptFunction(data) => data.internal_get_prototype_of(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_get_prototype_of(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get_prototype_of(agent),
            Object::Arguments(data) => data.internal_get_prototype_of(agent),
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
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_get_prototype_of(agent),
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_prototype_of(agent)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get_prototype_of(agent)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_prototype_of(agent)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_prototype_of(agent)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_prototype_of(agent)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_prototype_of(agent)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_prototype_of(agent)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_prototype_of(agent)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_prototype_of(agent)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_prototype_of(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_get_prototype_of(agent),
            Object::Module(data) => data.internal_get_prototype_of(agent),
            Object::EmbedderObject(data) => data.internal_get_prototype_of(agent),
        }
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent<'gen>,
        prototype: Option<Object<'gen>>,
    ) -> JsResult<'gen, bool> {
        match self {
            Object::Object(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Array(data) => data.internal_set_prototype_of(agent, prototype),
            Object::ArrayBuffer(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Date(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Error(data) => data.internal_set_prototype_of(agent, prototype),
            Object::BoundFunction(data) => data.internal_set_prototype_of(agent, prototype),
            Object::BuiltinFunction(data) => data.internal_set_prototype_of(agent, prototype),
            Object::ECMAScriptFunction(data) => data.internal_set_prototype_of(agent, prototype),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_prototype_of(agent, prototype)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Arguments(data) => data.internal_set_prototype_of(agent, prototype),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_prototype_of(agent, prototype)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_set_prototype_of(agent, prototype),
            Object::Module(data) => data.internal_set_prototype_of(agent, prototype),
            Object::EmbedderObject(data) => data.internal_set_prototype_of(agent, prototype),
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent<'gen>) -> JsResult<'gen, bool> {
        match self {
            Object::Object(data) => data.internal_is_extensible(agent),
            Object::Array(data) => data.internal_is_extensible(agent),
            Object::ArrayBuffer(data) => data.internal_is_extensible(agent),
            Object::Date(data) => data.internal_is_extensible(agent),
            Object::Error(data) => data.internal_is_extensible(agent),
            Object::BoundFunction(data) => data.internal_is_extensible(agent),
            Object::BuiltinFunction(data) => data.internal_is_extensible(agent),
            Object::ECMAScriptFunction(data) => data.internal_is_extensible(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_is_extensible(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_is_extensible(agent),
            Object::Arguments(data) => data.internal_is_extensible(agent),
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
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_is_extensible(agent),
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_is_extensible(agent),
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_is_extensible(agent)
            }
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_is_extensible(agent),
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_is_extensible(agent)
            }
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_is_extensible(agent),
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_is_extensible(agent)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_is_extensible(agent)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_is_extensible(agent)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_is_extensible(agent)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_is_extensible(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_is_extensible(agent),
            Object::Module(data) => data.internal_is_extensible(agent),
            Object::EmbedderObject(data) => data.internal_is_extensible(agent),
        }
    }

    fn internal_prevent_extensions(self, agent: &mut Agent<'gen>) -> JsResult<'gen, bool> {
        match self {
            Object::Object(data) => data.internal_prevent_extensions(agent),
            Object::Array(data) => data.internal_prevent_extensions(agent),
            Object::ArrayBuffer(data) => data.internal_prevent_extensions(agent),
            Object::Date(data) => data.internal_prevent_extensions(agent),
            Object::Error(data) => data.internal_prevent_extensions(agent),
            Object::BoundFunction(data) => data.internal_prevent_extensions(agent),
            Object::BuiltinFunction(data) => data.internal_prevent_extensions(agent),
            Object::ECMAScriptFunction(data) => data.internal_prevent_extensions(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_prevent_extensions(agent)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_prevent_extensions(agent),
            Object::Arguments(data) => data.internal_prevent_extensions(agent),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_prevent_extensions(agent)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_prevent_extensions(agent)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_prevent_extensions(agent)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_prevent_extensions(agent)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_prevent_extensions(agent)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_prevent_extensions(agent)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_prevent_extensions(agent)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_prevent_extensions(agent)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_prevent_extensions(agent)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_prevent_extensions(agent)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_prevent_extensions(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_prevent_extensions(agent),
            Object::Module(data) => data.internal_prevent_extensions(agent),
            Object::EmbedderObject(data) => data.internal_prevent_extensions(agent),
        }
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
    ) -> JsResult<'gen, Option<PropertyDescriptor<'gen>>> {
        match self {
            Object::Object(data) => data.internal_get_own_property(agent, property_key),
            Object::Array(data) => data.internal_get_own_property(agent, property_key),
            Object::ArrayBuffer(data) => data.internal_get_own_property(agent, property_key),
            Object::Date(data) => data.internal_get_own_property(agent, property_key),
            Object::Error(data) => data.internal_get_own_property(agent, property_key),
            Object::BoundFunction(data) => data.internal_get_own_property(agent, property_key),
            Object::BuiltinFunction(data) => data.internal_get_own_property(agent, property_key),
            Object::ECMAScriptFunction(data) => data.internal_get_own_property(agent, property_key),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get_own_property(agent, property_key)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get_own_property(agent, property_key),
            Object::Arguments(data) => data.internal_get_own_property(agent, property_key),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get_own_property(agent, property_key)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_own_property(agent, property_key)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get_own_property(agent, property_key)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_own_property(agent, property_key)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_own_property(agent, property_key)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_own_property(agent, property_key)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_own_property(agent, property_key)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_own_property(agent, property_key)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_own_property(agent, property_key)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_own_property(agent, property_key)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_own_property(agent, property_key)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_get_own_property(agent, property_key),
            Object::Module(data) => data.internal_get_own_property(agent, property_key),
            Object::EmbedderObject(data) => data.internal_get_own_property(agent, property_key),
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        property_descriptor: PropertyDescriptor<'gen>,
    ) -> JsResult<'gen, bool> {
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
            Object::BuiltinConstructorFunction => todo!(),
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
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::Uint16Array(data) => TypedArray::Uint16Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::Uint32Array(data) => TypedArray::Uint32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::Float32Array(data) => TypedArray::Float32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::Float64Array(data) => TypedArray::Float64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
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

    fn internal_has_property(self, agent: &mut Agent<'gen>, property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
        match self {
            Object::Object(data) => data.internal_has_property(agent, property_key),
            Object::Array(data) => data.internal_has_property(agent, property_key),
            Object::ArrayBuffer(data) => data.internal_has_property(agent, property_key),
            Object::Date(data) => data.internal_has_property(agent, property_key),
            Object::Error(data) => data.internal_has_property(agent, property_key),
            Object::BoundFunction(data) => data.internal_has_property(agent, property_key),
            Object::BuiltinFunction(data) => data.internal_has_property(agent, property_key),
            Object::ECMAScriptFunction(data) => data.internal_has_property(agent, property_key),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_has_property(agent, property_key)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_has_property(agent, property_key),
            Object::Arguments(data) => data.internal_has_property(agent, property_key),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_has_property(agent, property_key)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_has_property(agent, property_key)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_has_property(agent, property_key)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_has_property(agent, property_key)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_has_property(agent, property_key)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_has_property(agent, property_key)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_has_property(agent, property_key)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_has_property(agent, property_key)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_has_property(agent, property_key)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_has_property(agent, property_key)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_has_property(agent, property_key)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_has_property(agent, property_key),
            Object::Module(data) => data.internal_has_property(agent, property_key),
            Object::EmbedderObject(data) => data.internal_has_property(agent, property_key),
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        receiver: Value<'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        match self {
            Object::Object(data) => data.internal_get(agent, property_key, receiver),
            Object::Array(data) => data.internal_get(agent, property_key, receiver),
            Object::ArrayBuffer(data) => data.internal_get(agent, property_key, receiver),
            Object::Date(data) => data.internal_get(agent, property_key, receiver),
            Object::Error(data) => data.internal_get(agent, property_key, receiver),
            Object::BoundFunction(data) => data.internal_get(agent, property_key, receiver),
            Object::BuiltinFunction(data) => data.internal_get(agent, property_key, receiver),
            Object::ECMAScriptFunction(data) => data.internal_get(agent, property_key, receiver),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get(agent, property_key, receiver)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get(agent, property_key, receiver),
            Object::Arguments(data) => data.internal_get(agent, property_key, receiver),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get(agent, property_key, receiver)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get(agent, property_key, receiver)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get(agent, property_key, receiver)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get(agent, property_key, receiver)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get(agent, property_key, receiver)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get(agent, property_key, receiver)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get(agent, property_key, receiver)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get(agent, property_key, receiver)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get(agent, property_key, receiver)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get(agent, property_key, receiver)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get(agent, property_key, receiver)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_get(agent, property_key, receiver),
            Object::Module(data) => data.internal_get(agent, property_key, receiver),
            Object::EmbedderObject(data) => data.internal_get(agent, property_key, receiver),
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        value: Value<'gen>,
        receiver: Value<'gen>,
    ) -> JsResult<'gen, bool> {
        match self {
            Object::Object(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Array(data) => data.internal_set(agent, property_key, value, receiver),
            Object::ArrayBuffer(data) => data.internal_set(agent, property_key, value, receiver),
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
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_set(agent, property_key, value, receiver)
            }
            Object::Arguments(data) => data.internal_set(agent, property_key, value, receiver),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
            ),
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set(agent, property_key, value, receiver)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_set(agent, property_key, value, receiver),
            Object::Module(data) => data.internal_set(agent, property_key, value, receiver),
            Object::EmbedderObject(data) => data.internal_set(agent, property_key, value, receiver),
        }
    }

    fn internal_delete(self, agent: &mut Agent<'gen>, property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
        match self {
            Object::Object(data) => data.internal_delete(agent, property_key),
            Object::Array(data) => data.internal_delete(agent, property_key),
            Object::ArrayBuffer(data) => data.internal_delete(agent, property_key),
            Object::Date(data) => data.internal_delete(agent, property_key),
            Object::Error(data) => data.internal_delete(agent, property_key),
            Object::BoundFunction(data) => data.internal_delete(agent, property_key),
            Object::BuiltinFunction(data) => data.internal_delete(agent, property_key),
            Object::ECMAScriptFunction(data) => data.internal_delete(agent, property_key),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_delete(agent, property_key)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_delete(agent, property_key),
            Object::Arguments(data) => data.internal_delete(agent, property_key),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_delete(agent, property_key)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_delete(agent, property_key)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_delete(agent, property_key)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_delete(agent, property_key)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_delete(agent, property_key)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_delete(agent, property_key)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_delete(agent, property_key)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_delete(agent, property_key)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_delete(agent, property_key)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_delete(agent, property_key)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_delete(agent, property_key)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_delete(agent, property_key),
            Object::Module(data) => data.internal_delete(agent, property_key),
            Object::EmbedderObject(data) => data.internal_delete(agent, property_key),
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent<'gen>) -> JsResult<'gen, Vec<PropertyKey<'gen>>> {
        match self {
            Object::Object(data) => data.internal_own_property_keys(agent),
            Object::Array(data) => data.internal_own_property_keys(agent),
            Object::ArrayBuffer(data) => data.internal_own_property_keys(agent),
            Object::Date(data) => data.internal_own_property_keys(agent),
            Object::Error(data) => data.internal_own_property_keys(agent),
            Object::BoundFunction(data) => data.internal_own_property_keys(agent),
            Object::BuiltinFunction(data) => data.internal_own_property_keys(agent),
            Object::ECMAScriptFunction(data) => data.internal_own_property_keys(agent),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_own_property_keys(agent),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_own_property_keys(agent),
            Object::Arguments(data) => data.internal_own_property_keys(agent),
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
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_own_property_keys(agent)
            }
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_own_property_keys(agent)
            }
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_own_property_keys(agent)
            }
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_own_property_keys(agent)
            }
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_own_property_keys(agent)
            }
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_own_property_keys(agent)
            }
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_own_property_keys(agent)
            }
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_own_property_keys(agent)
            }
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_own_property_keys(agent)
            }
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_own_property_keys(agent)
            }
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_own_property_keys(agent)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.internal_own_property_keys(agent),
            Object::Module(data) => data.internal_own_property_keys(agent),
            Object::EmbedderObject(data) => data.internal_own_property_keys(agent),
        }
    }

    fn internal_call(
        self,
        agent: &mut Agent<'gen>,
        this_value: Value<'gen>,
        arguments_list: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
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
        agent: &mut Agent<'gen>,
        arguments_list: ArgumentsList<'_, 'gen>,
        new_target: Function<'gen>,
    ) -> JsResult<'gen, Object<'gen>> {
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

impl<'gen> HeapMarkAndSweep<'gen> for Object<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        match self {
            Object::Object(data) => data.mark_values(queues),
            Object::Array(data) => data.mark_values(queues),
            Object::ArrayBuffer(data) => data.mark_values(queues),
            Object::Date(data) => data.mark_values(queues),
            Object::Error(data) => data.mark_values(queues),
            Object::BoundFunction(data) => data.mark_values(queues),
            Object::BuiltinFunction(data) => data.mark_values(queues),
            Object::ECMAScriptFunction(data) => data.mark_values(queues),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => data.mark_values(queues),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.mark_values(queues),
            Object::Arguments(data) => data.mark_values(queues),
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
            Object::Int8Array(data) => data.mark_values(queues),
            Object::Uint8Array(data) => data.mark_values(queues),
            Object::Uint8ClampedArray(data) => data.mark_values(queues),
            Object::Int16Array(data) => data.mark_values(queues),
            Object::Uint16Array(data) => data.mark_values(queues),
            Object::Int32Array(data) => data.mark_values(queues),
            Object::Uint32Array(data) => data.mark_values(queues),
            Object::BigInt64Array(data) => data.mark_values(queues),
            Object::BigUint64Array(data) => data.mark_values(queues),
            Object::Float32Array(data) => data.mark_values(queues),
            Object::Float64Array(data) => data.mark_values(queues),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
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
            Object::BuiltinConstructorFunction => todo!(),
            Object::BuiltinPromiseResolvingFunction(data) => data.sweep_values(compactions),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.sweep_values(compactions),
            Object::Arguments(data) => data.sweep_values(compactions),
            Object::Array(data) => data.sweep_values(compactions),
            Object::ArrayBuffer(data) => data.sweep_values(compactions),
            Object::DataView(data) => data.sweep_values(compactions),
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
            Object::Int8Array(data) => data.sweep_values(compactions),
            Object::Uint8Array(data) => data.sweep_values(compactions),
            Object::Uint8ClampedArray(data) => data.sweep_values(compactions),
            Object::Int16Array(data) => data.sweep_values(compactions),
            Object::Uint16Array(data) => data.sweep_values(compactions),
            Object::Int32Array(data) => data.sweep_values(compactions),
            Object::Uint32Array(data) => data.sweep_values(compactions),
            Object::BigInt64Array(data) => data.sweep_values(compactions),
            Object::BigUint64Array(data) => data.sweep_values(compactions),
            Object::Float32Array(data) => data.sweep_values(compactions),
            Object::Float64Array(data) => data.sweep_values(compactions),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::Generator(data) => data.sweep_values(compactions),
            Object::Module(data) => data.sweep_values(compactions),
            Object::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}

impl<'gen> CreateHeapData<ObjectHeapData, OrdinaryObject<'gen>> for Heap<'gen> {
    fn create(&mut self, data: ObjectHeapData) -> OrdinaryObject<'gen> {
        self.objects.push(Some(data));
        OrdinaryObject(ObjectIndex::last(&self.objects))
    }
}
