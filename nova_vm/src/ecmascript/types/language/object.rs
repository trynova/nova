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
#[cfg(feature = "proposal-float16array")]
use super::value::FLOAT_16_ARRAY_DISCRIMINANT;
#[cfg(feature = "regexp")]
use super::value::REGEXP_DISCRIMINANT;
#[cfg(feature = "shared-array-buffer")]
use super::value::SHARED_ARRAY_BUFFER_DISCRIMINANT;
#[cfg(feature = "array-buffer")]
use super::value::{
    ARRAY_BUFFER_DISCRIMINANT, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
    DATA_VIEW_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
    INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT,
    UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
    UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
};
#[cfg(feature = "weak-refs")]
use super::value::{WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT};
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
    },
    Function, IntoValue, Value,
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExp;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
#[cfg(feature = "set")]
use crate::ecmascript::{
    builtins::{
        keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
    },
    types::{SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT},
};
#[cfg(feature = "array-buffer")]
use crate::{
    ecmascript::builtins::{data_view::DataView, typed_array::TypedArray, ArrayBuffer},
    engine::{context::NoGcScope, Scoped},
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
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIterator,
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            proxy::Proxy,
            ArgumentsList, Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
        },
        execution::{Agent, JsResult},
        types::PropertyDescriptor,
    },
    engine::{
        context::GcScope,
        rootable::{HeapRootData, HeapRootRef, Rootable},
        TryResult,
    },
    heap::{
        indexes::ObjectIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

pub use data::ObjectHeapData;
pub use internal_methods::InternalMethods;
pub use internal_slots::InternalSlots;
pub use into_object::IntoObject;
pub use property_key::{
    bind_property_keys, scope_property_keys, unbind_property_keys, PropertyKey,
};
pub use property_storage::PropertyStorage;

/// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
///
/// In Nova
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Object<'a> {
    Object(OrdinaryObject<'a>) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction<'a>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'a>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'a>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject<'a>) = PRIMITIVE_OBJECT_DISCRIMINANT,
    Arguments(OrdinaryObject<'a>) = ARGUMENTS_DISCRIMINANT,
    Array(Array<'a>) = ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'a>) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'a>) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "date")]
    Date(Date<'a>) = DATE_DISCRIMINANT,
    Error(Error<'a>) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistry<'a>) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(Map<'a>) = MAP_DISCRIMINANT,
    Promise(Promise<'a>) = PROMISE_DISCRIMINANT,
    Proxy(Proxy<'a>) = PROXY_DISCRIMINANT,
    #[cfg(feature = "regexp")]
    RegExp(RegExp<'a>) = REGEXP_DISCRIMINANT,
    #[cfg(feature = "set")]
    Set(Set<'a>) = SET_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'a>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'a>) = WEAK_MAP_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'a>) = WEAK_REF_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'a>) = WEAK_SET_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int8Array(TypedArrayIndex<'a>) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(TypedArrayIndex<'a>) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(TypedArrayIndex<'a>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(TypedArrayIndex<'a>) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(TypedArrayIndex<'a>) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(TypedArrayIndex<'a>) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(TypedArrayIndex<'a>) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(TypedArrayIndex<'a>) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(TypedArrayIndex<'a>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(TypedArrayIndex<'a>) = FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(TypedArrayIndex<'a>) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(TypedArrayIndex<'a>) = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncIterator = ASYNC_ITERATOR_DISCRIMINANT,
    Iterator = ITERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator<'a>) = ARRAY_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'a>) = SET_ITERATOR_DISCRIMINANT,
    MapIterator(MapIterator<'a>) = MAP_ITERATOR_DISCRIMINANT,
    Generator(Generator<'a>) = GENERATOR_DISCRIMINANT,
    Module(Module<'a>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject<'a>) = EMBEDDER_OBJECT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrdinaryObject<'a>(pub(crate) ObjectIndex<'a>);

impl IntoValue for Object<'_> {
    fn into_value(self) -> Value {
        match self {
            Object::Object(data) => Value::Object(data.unbind()),
            Object::BoundFunction(data) => Value::BoundFunction(data.unbind()),
            Object::BuiltinFunction(data) => Value::BuiltinFunction(data.unbind()),
            Object::ECMAScriptFunction(data) => Value::ECMAScriptFunction(data.unbind()),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                Value::BuiltinConstructorFunction(data.unbind())
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data.unbind())
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data.unbind()),
            Object::Arguments(data) => Value::Arguments(data.unbind()),
            Object::Array(data) => Value::Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => Value::ArrayBuffer(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => Value::DataView(data.unbind()),
            #[cfg(feature = "date")]
            Object::Date(data) => Value::Date(data.unbind()),
            Object::Error(data) => Value::Error(data.unbind()),
            Object::FinalizationRegistry(data) => Value::FinalizationRegistry(data.unbind()),
            Object::Map(data) => Value::Map(data.unbind()),
            Object::Promise(data) => Value::Promise(data.unbind()),
            Object::Proxy(data) => Value::Proxy(data.unbind()),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => Value::RegExp(data.unbind()),
            #[cfg(feature = "set")]
            Object::Set(data) => Value::Set(data.unbind()),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => Value::SharedArrayBuffer(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => Value::WeakMap(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => Value::WeakRef(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => Value::WeakSet(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => Value::Int8Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => Value::Uint8Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => Value::Uint8ClampedArray(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => Value::Int16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => Value::Uint16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => Value::Int32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => Value::Uint32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => Value::BigInt64Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => Value::BigUint64Array(data.unbind()),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => Value::Float16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => Value::Float32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => Value::Float64Array(data.unbind()),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => Value::ArrayIterator(data.unbind()),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => Value::SetIterator(data.unbind()),
            Object::MapIterator(data) => Value::MapIterator(data.unbind()),
            Object::Generator(data) => Value::Generator(data.unbind()),
            Object::Module(data) => Value::Module(data.unbind()),
            Object::EmbedderObject(data) => Value::EmbedderObject(data.unbind()),
        }
    }
}

impl<'a> IntoObject<'a> for Object<'a> {
    #[inline(always)]
    fn into_object(self) -> Object<'a> {
        self
    }
}

impl<'a> IntoObject<'a> for OrdinaryObject<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl IntoValue for OrdinaryObject<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> From<OrdinaryObject<'a>> for Object<'a> {
    fn from(value: OrdinaryObject<'_>) -> Self {
        Self::Object(value.unbind())
    }
}

impl<'a> From<ObjectIndex<'a>> for OrdinaryObject<'a> {
    fn from(value: ObjectIndex<'a>) -> Self {
        OrdinaryObject(value)
    }
}

impl From<OrdinaryObject<'_>> for Value {
    fn from(value: OrdinaryObject<'_>) -> Self {
        Self::Object(value.unbind())
    }
}

impl TryFrom<Value> for OrdinaryObject<'_> {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for OrdinaryObject<'a> {
    type Error = ();

    #[inline]
    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::Object(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for OrdinaryObject<'a> {
    #[inline(always)]
    fn get_backing_object(self, _: &Agent) -> Option<OrdinaryObject<'static>> {
        Some(self.unbind())
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!();
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!();
    }

    fn internal_extensible(self, agent: &Agent) -> bool {
        agent[self.unbind()].extensible
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        agent[self.unbind()].extensible = value;
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        agent[self.unbind()].prototype
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        agent[self.unbind()].prototype = prototype.map(|p| p.unbind());
    }
}

impl<'a> OrdinaryObject<'a> {
    /// Unbind this OrdinaryObject from its current lifetime. This is necessary to use
    /// the OrdinaryObject as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> OrdinaryObject<'static> {
        unsafe { std::mem::transmute::<OrdinaryObject<'a>, OrdinaryObject<'static>>(self) }
    }

    // Bind this OrdinaryObject to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your OrdinaryObjects cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let number = number.bind(&gc);
    // ```
    // to make sure that the unbound OrdinaryObject cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> OrdinaryObject<'gc> {
        unsafe { std::mem::transmute::<OrdinaryObject<'a>, OrdinaryObject<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, OrdinaryObject<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(ObjectIndex::from_u32_index(0))
    }
    pub(crate) const fn new(value: ObjectIndex<'a>) -> Self {
        Self(value)
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'a> From<ObjectIndex<'a>> for Object<'a> {
    fn from(value: ObjectIndex<'a>) -> Self {
        let value: OrdinaryObject<'a> = value.into();
        Object::Object(value.unbind())
    }
}

impl<'a> From<BoundFunction<'a>> for Object<'a> {
    fn from(value: BoundFunction) -> Self {
        Object::BoundFunction(value.unbind())
    }
}

impl From<Object<'_>> for Value {
    fn from(value: Object) -> Self {
        match value {
            Object::Object(data) => Value::Object(data.unbind()),
            Object::BoundFunction(data) => Value::BoundFunction(data.unbind()),
            Object::BuiltinFunction(data) => Value::BuiltinFunction(data.unbind()),
            Object::ECMAScriptFunction(data) => Value::ECMAScriptFunction(data.unbind()),
            Object::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            Object::BuiltinConstructorFunction(data) => {
                Value::BuiltinConstructorFunction(data.unbind())
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data.unbind())
            }
            Object::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(data) => Value::PrimitiveObject(data.unbind()),
            Object::Arguments(data) => Value::Arguments(data.unbind()),
            Object::Array(data) => Value::Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => Value::ArrayBuffer(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => Value::DataView(data.unbind()),
            #[cfg(feature = "date")]
            Object::Date(data) => Value::Date(data.unbind()),
            Object::Error(data) => Value::Error(data.unbind()),
            Object::FinalizationRegistry(data) => Value::FinalizationRegistry(data.unbind()),
            Object::Map(data) => Value::Map(data.unbind()),
            Object::Promise(data) => Value::Promise(data.unbind()),
            Object::Proxy(data) => Value::Proxy(data.unbind()),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => Value::RegExp(data.unbind()),
            #[cfg(feature = "set")]
            Object::Set(data) => Value::Set(data.unbind()),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => Value::SharedArrayBuffer(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => Value::WeakMap(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => Value::WeakRef(data.unbind()),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => Value::WeakSet(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => Value::Int8Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => Value::Uint8Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => Value::Uint8ClampedArray(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => Value::Int16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => Value::Uint16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => Value::Int32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => Value::Uint32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => Value::BigInt64Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => Value::BigUint64Array(data.unbind()),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => Value::Float16Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => Value::Float32Array(data.unbind()),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => Value::Float64Array(data.unbind()),
            Object::AsyncFromSyncIterator => Value::AsyncFromSyncIterator,
            Object::AsyncIterator => Value::AsyncIterator,
            Object::Iterator => Value::Iterator,
            Object::ArrayIterator(data) => Value::ArrayIterator(data.unbind()),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => Value::SetIterator(data.unbind()),
            Object::MapIterator(data) => Value::MapIterator(data.unbind()),
            Object::Generator(data) => Value::Generator(data.unbind()),
            Object::Module(data) => Value::Module(data.unbind()),
            Object::EmbedderObject(data) => Value::EmbedderObject(data.unbind()),
        }
    }
}

impl TryFrom<Value> for Object<'_> {
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
            #[cfg(feature = "regexp")]
            Value::RegExp(idx) => Ok(Object::RegExp(idx)),
            #[cfg(feature = "set")]
            Value::Set(data) => Ok(Object::Set(data)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => Ok(Object::SharedArrayBuffer(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => Ok(Object::WeakMap(data)),
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => Ok(Object::WeakRef(data)),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(data) => Ok(Object::Float16Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => Ok(Object::Float32Array(data)),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => Ok(Object::Float64Array(data)),
            Value::AsyncFromSyncIterator => Ok(Object::AsyncFromSyncIterator),
            Value::AsyncIterator => Ok(Object::AsyncIterator),
            Value::Iterator => Ok(Object::Iterator),
            Value::ArrayIterator(data) => Ok(Object::ArrayIterator(data)),
            #[cfg(feature = "set")]
            Value::SetIterator(data) => Ok(Object::SetIterator(data)),
            Value::MapIterator(data) => Ok(Object::MapIterator(data)),
            Value::Generator(data) => Ok(Object::Generator(data)),
            Value::Module(data) => Ok(Object::Module(data)),
            Value::EmbedderObject(data) => Ok(Object::EmbedderObject(data)),
        }
    }
}

impl<'a> Object<'a> {
    /// Unbind this Object from its current lifetime. This is necessary to use
    /// the Object as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> Object<'static> {
        unsafe { std::mem::transmute::<Self, Object<'static>>(self) }
    }

    // Bind this Object to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your Objects cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let object = object.bind(&gc);
    // ```
    // to make sure that the unbound Object cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> Object<'gc> {
        unsafe { std::mem::transmute::<Self, Object<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Object<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn property_storage(self) -> PropertyStorage<'a> {
        PropertyStorage::new(self)
    }
}

impl Hash for Object<'_> {
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
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.get_index().hash(state),
            #[cfg(feature = "set")]
            Object::Set(data) => data.get_index().hash(state),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.get_index().hash(state),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.get_index().hash(state),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.get_index().hash(state),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => data.into_index().hash(state),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => data.into_index().hash(state),
            Object::AsyncFromSyncIterator => {}
            Object::AsyncIterator => {}
            Object::Iterator => {}
            Object::ArrayIterator(data) => data.get_index().hash(state),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.get_index().hash(state),
            Object::MapIterator(data) => data.get_index().hash(state),
            Object::Generator(data) => data.get_index().hash(state),
            Object::Module(data) => data.get_index().hash(state),
            Object::EmbedderObject(data) => data.get_index().hash(state),
        }
    }
}

impl<'a> InternalSlots<'a> for Object<'a> {
    fn get_backing_object(self, _: &Agent) -> Option<OrdinaryObject<'static>> {
        unreachable!("Object should not try to access its backing object");
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("Object should not try to create its backing object");
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
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
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_extensible(agent),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_extensible(agent),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_extensible(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_extensible(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_extensible(agent),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_extensible(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_extensible(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_extensible(agent),
            #[cfg(feature = "set")]
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
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set_extensible(agent, value),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_set_extensible(agent, value)
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
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_set_extensible(agent, value),
            Object::MapIterator(data) => data.internal_set_extensible(agent, value),
            Object::Generator(data) => data.internal_set_extensible(agent, value),
            Object::Module(data) => data.internal_set_extensible(agent, value),
            Object::EmbedderObject(data) => data.internal_set_extensible(agent, value),
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
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
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_prototype(agent),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_prototype(agent),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_prototype(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_prototype(agent),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_prototype(agent),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_prototype(agent),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_prototype(agent),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_prototype(agent),
            #[cfg(feature = "set")]
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
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set_prototype(agent, prototype),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_set_prototype(agent, prototype)
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
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::MapIterator(data) => data.internal_set_prototype(agent, prototype),
            Object::Generator(data) => data.internal_set_prototype(agent, prototype),
            Object::Module(data) => data.internal_set_prototype(agent, prototype),
            Object::EmbedderObject(data) => data.internal_set_prototype(agent, prototype),
        }
    }
}

impl<'a> InternalMethods<'a> for Object<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<Object<'gc>>> {
        match self {
            Object::Object(data) => data.try_get_prototype_of(agent, gc),
            Object::Array(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_get_prototype_of(agent, gc),
            Object::Error(data) => data.try_get_prototype_of(agent, gc),
            Object::BoundFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_get_prototype_of(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_get_prototype_of(agent, gc),
            Object::Arguments(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_get_prototype_of(agent, gc),
            Object::FinalizationRegistry(data) => data.try_get_prototype_of(agent, gc),
            Object::Map(data) => data.try_get_prototype_of(agent, gc),
            Object::Promise(data) => data.try_get_prototype_of(agent, gc),
            Object::Proxy(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_get_prototype_of(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_get_prototype_of(agent, gc),
            Object::MapIterator(data) => data.try_get_prototype_of(agent, gc),
            Object::Generator(data) => data.try_get_prototype_of(agent, gc),
            Object::Module(data) => data.try_get_prototype_of(agent, gc),
            Object::EmbedderObject(data) => data.try_get_prototype_of(agent, gc),
        }
    }

    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Option<Object<'gc>>> {
        match self {
            Object::Object(data) => data.internal_get_prototype_of(agent, gc),
            Object::Array(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get_prototype_of(agent, gc),
            Object::Error(data) => data.internal_get_prototype_of(agent, gc),
            Object::BoundFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::BuiltinFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_get_prototype_of(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get_prototype_of(agent, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get_prototype_of(agent, gc),
            Object::Arguments(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get_prototype_of(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_get_prototype_of(agent, gc),
            Object::Map(data) => data.internal_get_prototype_of(agent, gc),
            Object::Promise(data) => data.internal_get_prototype_of(agent, gc),
            Object::Proxy(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_prototype_of(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_prototype_of(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_get_prototype_of(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_get_prototype_of(agent, gc),
            Object::MapIterator(data) => data.internal_get_prototype_of(agent, gc),
            Object::Generator(data) => data.internal_get_prototype_of(agent, gc),
            Object::Module(data) => data.internal_get_prototype_of(agent, gc),
            Object::EmbedderObject(data) => data.internal_get_prototype_of(agent, gc),
        }
    }

    fn try_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Array(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Error(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::BoundFunction(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::BuiltinFunction(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::ECMAScriptFunction(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Arguments(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::FinalizationRegistry(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Map(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Promise(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Proxy(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_set_prototype_of(agent, prototype, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::MapIterator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Generator(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::Module(data) => data.try_set_prototype_of(agent, prototype, gc),
            Object::EmbedderObject(data) => data.try_set_prototype_of(agent, prototype, gc),
        }
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Array(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Error(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::BoundFunction(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::BuiltinFunction(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::ECMAScriptFunction(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Arguments(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_set_prototype_of(agent, prototype, gc)
            }
            Object::Map(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Promise(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Proxy(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_set_prototype_of(agent, prototype, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::MapIterator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Generator(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::Module(data) => data.internal_set_prototype_of(agent, prototype, gc),
            Object::EmbedderObject(data) => data.internal_set_prototype_of(agent, prototype, gc),
        }
    }

    fn try_is_extensible(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_is_extensible(agent, gc),
            Object::Array(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_is_extensible(agent, gc),
            Object::Error(data) => data.try_is_extensible(agent, gc),
            Object::BoundFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinFunction(data) => data.try_is_extensible(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_is_extensible(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_is_extensible(agent, gc),
            Object::Arguments(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_is_extensible(agent, gc),
            Object::FinalizationRegistry(data) => data.try_is_extensible(agent, gc),
            Object::Map(data) => data.try_is_extensible(agent, gc),
            Object::Promise(data) => data.try_is_extensible(agent, gc),
            Object::Proxy(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).try_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_is_extensible(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_is_extensible(agent, gc),
            Object::MapIterator(data) => data.try_is_extensible(agent, gc),
            Object::Generator(data) => data.try_is_extensible(agent, gc),
            Object::Module(data) => data.try_is_extensible(agent, gc),
            Object::EmbedderObject(data) => data.try_is_extensible(agent, gc),
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent, gc: GcScope) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_is_extensible(agent, gc),
            Object::Array(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_is_extensible(agent, gc),
            Object::Error(data) => data.internal_is_extensible(agent, gc),
            Object::BoundFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinFunction(data) => data.internal_is_extensible(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.internal_is_extensible(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_is_extensible(agent, gc),
            Object::Arguments(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_is_extensible(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_is_extensible(agent, gc),
            Object::Map(data) => data.internal_is_extensible(agent, gc),
            Object::Promise(data) => data.internal_is_extensible(agent, gc),
            Object::Proxy(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_is_extensible(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_is_extensible(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_is_extensible(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_is_extensible(agent, gc),
            Object::MapIterator(data) => data.internal_is_extensible(agent, gc),
            Object::Generator(data) => data.internal_is_extensible(agent, gc),
            Object::Module(data) => data.internal_is_extensible(agent, gc),
            Object::EmbedderObject(data) => data.internal_is_extensible(agent, gc),
        }
    }

    fn try_prevent_extensions(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_prevent_extensions(agent, gc),
            Object::Array(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_prevent_extensions(agent, gc),
            Object::Error(data) => data.try_prevent_extensions(agent, gc),
            Object::BoundFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_prevent_extensions(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_prevent_extensions(agent, gc),
            Object::Arguments(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_prevent_extensions(agent, gc),
            Object::FinalizationRegistry(data) => data.try_prevent_extensions(agent, gc),
            Object::Map(data) => data.try_prevent_extensions(agent, gc),
            Object::Promise(data) => data.try_prevent_extensions(agent, gc),
            Object::Proxy(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_prevent_extensions(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_prevent_extensions(agent, gc),
            Object::MapIterator(data) => data.try_prevent_extensions(agent, gc),
            Object::Generator(data) => data.try_prevent_extensions(agent, gc),
            Object::Module(data) => data.try_prevent_extensions(agent, gc),
            Object::EmbedderObject(data) => data.try_prevent_extensions(agent, gc),
        }
    }

    fn internal_prevent_extensions(self, agent: &mut Agent, gc: GcScope) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_prevent_extensions(agent, gc),
            Object::Array(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_prevent_extensions(agent, gc),
            Object::Error(data) => data.internal_prevent_extensions(agent, gc),
            Object::BoundFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::BuiltinFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_prevent_extensions(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_prevent_extensions(agent, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_prevent_extensions(agent, gc),
            Object::Arguments(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_prevent_extensions(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_prevent_extensions(agent, gc),
            Object::Map(data) => data.internal_prevent_extensions(agent, gc),
            Object::Promise(data) => data.internal_prevent_extensions(agent, gc),
            Object::Proxy(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_prevent_extensions(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_prevent_extensions(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_prevent_extensions(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_prevent_extensions(agent, gc),
            Object::MapIterator(data) => data.internal_prevent_extensions(agent, gc),
            Object::Generator(data) => data.internal_prevent_extensions(agent, gc),
            Object::Module(data) => data.internal_prevent_extensions(agent, gc),
            Object::EmbedderObject(data) => data.internal_prevent_extensions(agent, gc),
        }
    }

    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<Option<PropertyDescriptor>> {
        match self {
            Object::Object(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Array(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Error(data) => data.try_get_own_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.try_get_own_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.try_get_own_property(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.try_get_own_property(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Arguments(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_get_own_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => {
                data.try_get_own_property(agent, property_key, gc)
            }
            Object::Map(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Promise(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Proxy(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_get_own_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::MapIterator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Generator(data) => data.try_get_own_property(agent, property_key, gc),
            Object::Module(data) => data.try_get_own_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.try_get_own_property(agent, property_key, gc),
        }
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match self {
            Object::Object(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Array(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Error(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::Arguments(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            Object::Map(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Promise(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Proxy(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get_own_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get_own_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_get_own_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::MapIterator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Generator(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::Module(data) => data.internal_get_own_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.internal_get_own_property(agent, property_key, gc),
        }
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Array(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "date")]
            Object::Date(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Error(idx) => {
                idx.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BoundFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Arguments(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::FinalizationRegistry(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Map(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Promise(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Proxy(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::Set(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .try_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data)
                .try_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).try_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::SetIterator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::MapIterator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Generator(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Module(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::EmbedderObject(data) => {
                data.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Array(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "date")]
            Object::Date(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Error(idx) => {
                idx.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BoundFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Arguments(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::FinalizationRegistry(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Map(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Promise(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Proxy(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::Set(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => TypedArray::Uint8Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => TypedArray::Int16Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => TypedArray::Uint16Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => TypedArray::Int32Array(data).internal_define_own_property(
                agent,
                property_key,
                property_descriptor,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => TypedArray::Uint32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data)
                .internal_define_own_property(agent, property_key, property_descriptor, gc),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            #[cfg(feature = "set")]
            Object::SetIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::MapIterator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Generator(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::Module(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Object::EmbedderObject(data) => {
                data.internal_define_own_property(agent, property_key, property_descriptor, gc)
            }
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_has_property(agent, property_key, gc),
            Object::Array(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_has_property(agent, property_key, gc),
            Object::Error(data) => data.try_has_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.try_has_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.try_has_property(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.try_has_property(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_has_property(agent, property_key, gc),
            Object::Arguments(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_has_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => data.try_has_property(agent, property_key, gc),
            Object::Map(data) => data.try_has_property(agent, property_key, gc),
            Object::Promise(data) => data.try_has_property(agent, property_key, gc),
            Object::Proxy(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_has_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_has_property(agent, property_key, gc),
            Object::MapIterator(data) => data.try_has_property(agent, property_key, gc),
            Object::Generator(data) => data.try_has_property(agent, property_key, gc),
            Object::Module(data) => data.try_has_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.try_has_property(agent, property_key, gc),
        }
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_has_property(agent, property_key, gc),
            Object::Array(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_has_property(agent, property_key, gc),
            Object::Error(data) => data.internal_has_property(agent, property_key, gc),
            Object::BoundFunction(data) => data.internal_has_property(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.internal_has_property(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.internal_has_property(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_has_property(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_has_property(agent, property_key, gc),
            Object::Arguments(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_has_property(agent, property_key, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_has_property(agent, property_key, gc)
            }
            Object::Map(data) => data.internal_has_property(agent, property_key, gc),
            Object::Promise(data) => data.internal_has_property(agent, property_key, gc),
            Object::Proxy(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_has_property(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_has_property(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_has_property(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_has_property(agent, property_key, gc),
            Object::MapIterator(data) => data.internal_has_property(agent, property_key, gc),
            Object::Generator(data) => data.internal_has_property(agent, property_key, gc),
            Object::Module(data) => data.internal_has_property(agent, property_key, gc),
            Object::EmbedderObject(data) => data.internal_has_property(agent, property_key, gc),
        }
    }

    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<Value> {
        match self {
            Object::Object(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Array(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Error(data) => data.try_get(agent, property_key, receiver, gc),
            Object::BoundFunction(data) => data.try_get(agent, property_key, receiver, gc),
            Object::BuiltinFunction(data) => data.try_get(agent, property_key, receiver, gc),
            Object::ECMAScriptFunction(data) => data.try_get(agent, property_key, receiver, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Arguments(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_get(agent, property_key, receiver, gc),
            Object::FinalizationRegistry(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Map(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Promise(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Proxy(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_get(agent, property_key, receiver, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::MapIterator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Generator(data) => data.try_get(agent, property_key, receiver, gc),
            Object::Module(data) => data.try_get(agent, property_key, receiver, gc),
            Object::EmbedderObject(data) => data.try_get(agent, property_key, receiver, gc),
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<Value> {
        match self {
            Object::Object(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Array(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Error(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::BoundFunction(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::BuiltinFunction(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::ECMAScriptFunction(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Arguments(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_get(agent, property_key, receiver, gc)
            }
            Object::Map(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Promise(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Proxy(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_get(agent, property_key, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_get(agent, property_key, receiver, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_get(agent, property_key, receiver, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::MapIterator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Generator(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::Module(data) => data.internal_get(agent, property_key, receiver, gc),
            Object::EmbedderObject(data) => data.internal_get(agent, property_key, receiver, gc),
        }
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Array(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Error(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::BoundFunction(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::BuiltinFunction(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::ECMAScriptFunction(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Arguments(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::FinalizationRegistry(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            Object::Map(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Promise(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Proxy(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data).try_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_set(agent, property_key, value, receiver, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::MapIterator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Generator(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::Module(data) => data.try_set(agent, property_key, value, receiver, gc),
            Object::EmbedderObject(data) => data.try_set(agent, property_key, value, receiver, gc),
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Array(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Error(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::BoundFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::Arguments(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::FinalizationRegistry(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::Map(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Promise(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Proxy(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_set(agent, property_key, value, receiver, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => TypedArray::Uint8ClampedArray(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => TypedArray::BigInt64Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => TypedArray::BigUint64Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => TypedArray::Float16Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => TypedArray::Float32Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => TypedArray::Float64Array(data).internal_set(
                agent,
                property_key,
                value,
                receiver,
                gc,
            ),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            #[cfg(feature = "set")]
            Object::SetIterator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::MapIterator(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
            Object::Generator(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::Module(data) => data.internal_set(agent, property_key, value, receiver, gc),
            Object::EmbedderObject(data) => {
                data.internal_set(agent, property_key, value, receiver, gc)
            }
        }
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Object::Object(data) => data.try_delete(agent, property_key, gc),
            Object::Array(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_delete(agent, property_key, gc),
            Object::Error(data) => data.try_delete(agent, property_key, gc),
            Object::BoundFunction(data) => data.try_delete(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.try_delete(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.try_delete(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_delete(agent, property_key, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.try_delete(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_delete(agent, property_key, gc),
            Object::Arguments(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_delete(agent, property_key, gc),
            Object::FinalizationRegistry(data) => data.try_delete(agent, property_key, gc),
            Object::Map(data) => data.try_delete(agent, property_key, gc),
            Object::Promise(data) => data.try_delete(agent, property_key, gc),
            Object::Proxy(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_delete(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_delete(agent, property_key, gc),
            Object::MapIterator(data) => data.try_delete(agent, property_key, gc),
            Object::Generator(data) => data.try_delete(agent, property_key, gc),
            Object::Module(data) => data.try_delete(agent, property_key, gc),
            Object::EmbedderObject(data) => data.try_delete(agent, property_key, gc),
        }
    }

    fn internal_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            Object::Object(data) => data.internal_delete(agent, property_key, gc),
            Object::Array(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_delete(agent, property_key, gc),
            Object::Error(data) => data.internal_delete(agent, property_key, gc),
            Object::BoundFunction(data) => data.internal_delete(agent, property_key, gc),
            Object::BuiltinFunction(data) => data.internal_delete(agent, property_key, gc),
            Object::ECMAScriptFunction(data) => data.internal_delete(agent, property_key, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => {
                data.internal_delete(agent, property_key, gc)
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_delete(agent, property_key, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_delete(agent, property_key, gc),
            Object::Arguments(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_delete(agent, property_key, gc),
            Object::FinalizationRegistry(data) => data.internal_delete(agent, property_key, gc),
            Object::Map(data) => data.internal_delete(agent, property_key, gc),
            Object::Promise(data) => data.internal_delete(agent, property_key, gc),
            Object::Proxy(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_delete(agent, property_key, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_delete(agent, property_key, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_delete(agent, property_key, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_delete(agent, property_key, gc),
            Object::MapIterator(data) => data.internal_delete(agent, property_key, gc),
            Object::Generator(data) => data.internal_delete(agent, property_key, gc),
            Object::Module(data) => data.internal_delete(agent, property_key, gc),
            Object::EmbedderObject(data) => data.internal_delete(agent, property_key, gc),
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        match self {
            Object::Object(data) => data.try_own_property_keys(agent, gc),
            Object::Array(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.try_own_property_keys(agent, gc),
            Object::Error(data) => data.try_own_property_keys(agent, gc),
            Object::BoundFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinFunction(data) => data.try_own_property_keys(agent, gc),
            Object::ECMAScriptFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => data.try_own_property_keys(agent, gc),
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.try_own_property_keys(agent, gc),
            Object::Arguments(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.try_own_property_keys(agent, gc),
            Object::FinalizationRegistry(data) => data.try_own_property_keys(agent, gc),
            Object::Map(data) => data.try_own_property_keys(agent, gc),
            Object::Promise(data) => data.try_own_property_keys(agent, gc),
            Object::Proxy(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => TypedArray::Int8Array(data).try_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).try_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).try_own_property_keys(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.try_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.try_own_property_keys(agent, gc),
            Object::MapIterator(data) => data.try_own_property_keys(agent, gc),
            Object::Generator(data) => data.try_own_property_keys(agent, gc),
            Object::Module(data) => data.try_own_property_keys(agent, gc),
            Object::EmbedderObject(data) => data.try_own_property_keys(agent, gc),
        }
    }

    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Vec<PropertyKey<'gc>>> {
        match self {
            Object::Object(data) => data.internal_own_property_keys(agent, gc),
            Object::Array(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "date")]
            Object::Date(data) => data.internal_own_property_keys(agent, gc),
            Object::Error(data) => data.internal_own_property_keys(agent, gc),
            Object::BoundFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::BuiltinFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::ECMAScriptFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::BuiltinGeneratorFunction => todo!(),
            Object::BuiltinConstructorFunction(data) => data.internal_own_property_keys(agent, gc),
            Object::BuiltinPromiseResolvingFunction(data) => {
                data.internal_own_property_keys(agent, gc)
            }
            Object::BuiltinPromiseCollectorFunction => todo!(),
            Object::BuiltinProxyRevokerFunction => todo!(),
            Object::PrimitiveObject(data) => data.internal_own_property_keys(agent, gc),
            Object::Arguments(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::DataView(data) => data.internal_own_property_keys(agent, gc),
            Object::FinalizationRegistry(data) => data.internal_own_property_keys(agent, gc),
            Object::Map(data) => data.internal_own_property_keys(agent, gc),
            Object::Promise(data) => data.internal_own_property_keys(agent, gc),
            Object::Proxy(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::Set(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(data) => {
                TypedArray::Int8Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(data) => {
                TypedArray::Uint8Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(data) => {
                TypedArray::Uint8ClampedArray(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(data) => {
                TypedArray::Int16Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(data) => {
                TypedArray::Uint16Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(data) => {
                TypedArray::Int32Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(data) => {
                TypedArray::Uint32Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(data) => {
                TypedArray::BigInt64Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(data) => {
                TypedArray::BigUint64Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => {
                TypedArray::Float16Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => {
                TypedArray::Float32Array(data).internal_own_property_keys(agent, gc)
            }
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => {
                TypedArray::Float64Array(data).internal_own_property_keys(agent, gc)
            }
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.internal_own_property_keys(agent, gc),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.internal_own_property_keys(agent, gc),
            Object::MapIterator(data) => data.internal_own_property_keys(agent, gc),
            Object::Generator(data) => data.internal_own_property_keys(agent, gc),
            Object::Module(data) => data.internal_own_property_keys(agent, gc),
            Object::EmbedderObject(data) => data.internal_own_property_keys(agent, gc),
        }
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
        gc: GcScope,
    ) -> JsResult<Value> {
        match self {
            Object::BoundFunction(data) => {
                data.internal_call(agent, this_value, arguments_list, gc)
            }
            Object::BuiltinFunction(data) => {
                data.internal_call(agent, this_value, arguments_list, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_call(agent, this_value, arguments_list, gc)
            }
            Object::EmbedderObject(_) => todo!(),
            _ => unreachable!(),
        }
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Object<'gc>> {
        match self {
            Object::BoundFunction(data) => {
                data.internal_construct(agent, arguments_list, new_target, gc)
            }
            Object::BuiltinFunction(data) => {
                data.internal_construct(agent, arguments_list, new_target, gc)
            }
            Object::ECMAScriptFunction(data) => {
                data.internal_construct(agent, arguments_list, new_target, gc)
            }
            _ => unreachable!(),
        }
    }
}

impl HeapMarkAndSweep for Object<'static> {
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
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
            Object::Set(data) => data.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => data.mark_values(queues),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
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
            #[cfg(feature = "regexp")]
            Object::RegExp(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Object::Set(data) => data.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
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
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(data) => data.sweep_values(compactions),
            Object::AsyncFromSyncIterator => todo!(),
            Object::AsyncIterator => todo!(),
            Object::Iterator => todo!(),
            Object::ArrayIterator(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Object::SetIterator(data) => data.sweep_values(compactions),
            Object::MapIterator(data) => data.sweep_values(compactions),
            Object::Generator(data) => data.sweep_values(compactions),
            Object::Module(data) => data.sweep_values(compactions),
            Object::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}

impl CreateHeapData<ObjectHeapData, OrdinaryObject<'static>> for Heap {
    fn create(&mut self, data: ObjectHeapData) -> OrdinaryObject<'static> {
        self.objects.push(Some(data));
        OrdinaryObject(ObjectIndex::last(&self.objects))
    }
}

impl Rootable for OrdinaryObject<'static> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Object(value))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Object(object) => Some(object),
            _ => None,
        }
    }
}

impl TryFrom<HeapRootData> for Object<'_> {
    type Error = ();

    fn try_from(value: HeapRootData) -> Result<Self, ()> {
        match value {
            HeapRootData::String(_) => Err(()),
            HeapRootData::Symbol(_) => Err(()),
            HeapRootData::Number(_) => Err(()),
            HeapRootData::BigInt(_) => Err(()),
            HeapRootData::Object(ordinary_object) => Ok(Self::Object(ordinary_object)),
            HeapRootData::BoundFunction(bound_function) => Ok(Self::BoundFunction(bound_function)),
            HeapRootData::BuiltinFunction(builtin_function) => {
                Ok(Self::BuiltinFunction(builtin_function))
            }
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                Ok(Self::ECMAScriptFunction(ecmascript_function))
            }
            HeapRootData::BuiltinGeneratorFunction => Ok(Self::BuiltinGeneratorFunction),
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => Ok(
                Self::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Ok(Self::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function,
                ))
            }
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Ok(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            HeapRootData::PrimitiveObject(primitive_object) => {
                Ok(Self::PrimitiveObject(primitive_object))
            }
            HeapRootData::Arguments(ordinary_object) => Ok(Self::Arguments(ordinary_object)),
            HeapRootData::Array(array) => Ok(Self::Array(array)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(array_buffer) => Ok(Self::ArrayBuffer(array_buffer)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(data_view) => Ok(Self::DataView(data_view)),
            #[cfg(feature = "date")]
            HeapRootData::Date(date) => Ok(Self::Date(date)),
            HeapRootData::Error(error) => Ok(Self::Error(error)),
            HeapRootData::FinalizationRegistry(finalization_registry) => {
                Ok(Self::FinalizationRegistry(finalization_registry))
            }
            HeapRootData::Map(map) => Ok(Self::Map(map)),
            HeapRootData::Promise(promise) => Ok(Self::Promise(promise)),
            HeapRootData::Proxy(proxy) => Ok(Self::Proxy(proxy)),
            #[cfg(feature = "regexp")]
            HeapRootData::RegExp(reg_exp) => Ok(Self::RegExp(reg_exp)),
            #[cfg(feature = "set")]
            HeapRootData::Set(set) => Ok(Self::Set(set)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(shared_array_buffer) => {
                Ok(Self::SharedArrayBuffer(shared_array_buffer))
            }
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => Ok(Self::WeakMap(weak_map)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => Ok(Self::WeakRef(weak_ref)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => Ok(Self::WeakSet(weak_set)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(base_index) => Ok(Self::Int8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(base_index) => Ok(Self::Uint8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(base_index) => Ok(Self::Uint8ClampedArray(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(base_index) => Ok(Self::Int16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(base_index) => Ok(Self::Uint16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(base_index) => Ok(Self::Int32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(base_index) => Ok(Self::Uint32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(base_index) => Ok(Self::BigInt64Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(base_index) => Ok(Self::BigUint64Array(base_index)),
            #[cfg(feature = "proposal-float16array")]
            HeapRootData::Float16Array(base_index) => Ok(Self::Float16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(base_index) => Ok(Self::Float32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(base_index) => Ok(Self::Float64Array(base_index)),
            HeapRootData::AsyncFromSyncIterator => Ok(Self::AsyncFromSyncIterator),
            HeapRootData::AsyncIterator => Ok(Self::AsyncIterator),
            HeapRootData::Iterator => Ok(Self::Iterator),
            HeapRootData::ArrayIterator(array_iterator) => Ok(Self::ArrayIterator(array_iterator)),
            #[cfg(feature = "set")]
            HeapRootData::SetIterator(set_iterator) => Ok(Self::SetIterator(set_iterator)),
            HeapRootData::MapIterator(map_iterator) => Ok(Self::MapIterator(map_iterator)),
            HeapRootData::Generator(generator) => Ok(Self::Generator(generator)),
            HeapRootData::Module(module) => Ok(Self::Module(module)),
            HeapRootData::EmbedderObject(embedder_object) => {
                Ok(Self::EmbedderObject(embedder_object))
            }
            HeapRootData::PromiseReaction(_) => Err(()),
            // Note: Do not use _ => Err(()) to make sure any added
            // HeapRootData Value variants cause compile errors if not handled.
        }
    }
}
