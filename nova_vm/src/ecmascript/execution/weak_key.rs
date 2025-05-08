// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Weakly holdable JavaScript Value.

#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExp;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
#[cfg(feature = "date")]
use crate::ecmascript::types::DATE_DISCRIMINANT;
#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::types::FLOAT_16_ARRAY_DISCRIMINANT;
#[cfg(feature = "regexp")]
use crate::ecmascript::types::REGEXP_DISCRIMINANT;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::types::SHARED_ARRAY_BUFFER_DISCRIMINANT;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::types::{
    ARRAY_BUFFER_DISCRIMINANT, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
    DATA_VIEW_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
    INT_8_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
    UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
    UINT_32_ARRAY_DISCRIMINANT,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::types::{
    WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT,
};
#[cfg(feature = "set")]
use crate::ecmascript::{
    builtins::{
        keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
    },
    types::{SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT},
};
#[cfg(feature = "array-buffer")]
use crate::{
    ecmascript::builtins::{ArrayBuffer, data_view::DataView},
    heap::indexes::TypedArrayIndex,
};

use crate::{
    ecmascript::{
        builtins::{
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
            async_generator_objects::AsyncGenerator,
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
            text_processing::string_objects::string_iterator_objects::StringIterator,
        },
        types::{
            ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
            ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT, ASYNC_GENERATOR_DISCRIMINANT,
            BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
            BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
            BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
            BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
            ECMASCRIPT_FUNCTION_DISCRIMINANT, EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT,
            FINALIZATION_REGISTRY_DISCRIMINANT, GENERATOR_DISCRIMINANT, IntoValue,
            MAP_DISCRIMINANT, MAP_ITERATOR_DISCRIMINANT, MODULE_DISCRIMINANT, OBJECT_DISCRIMINANT,
            Object, OrdinaryObject, PRIMITIVE_OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT,
            PROXY_DISCRIMINANT, STRING_ITERATOR_DISCRIMINANT, Symbol, Value,
        },
    },
    engine::{
        context::{Bindable, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{CompactionLists, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues},
};

/// ### [6.1 ECMAScript Language Types](https://tc39.es/ecma262/#sec-ecmascript-language-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum WeakKey<'a> {
    Symbol(Symbol<'a>),
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
    AsyncGenerator(AsyncGenerator<'a>) = ASYNC_GENERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator<'a>) = ARRAY_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'a>) = SET_ITERATOR_DISCRIMINANT,
    MapIterator(MapIterator<'a>) = MAP_ITERATOR_DISCRIMINANT,
    StringIterator(StringIterator<'a>) = STRING_ITERATOR_DISCRIMINANT,
    Generator(Generator<'a>) = GENERATOR_DISCRIMINANT,
    Module(Module<'a>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject<'a>) = EMBEDDER_OBJECT_DISCRIMINANT,
}

impl core::hash::Hash for WeakKey<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.into_value().try_hash(state).unwrap()
    }
}

impl<'a> From<WeakKey<'a>> for Value<'a> {
    #[inline]
    fn from(value: WeakKey<'a>) -> Self {
        match value {
            WeakKey::Symbol(d) => Value::Symbol(d),
            WeakKey::Object(d) => Value::Object(d),
            WeakKey::BoundFunction(d) => Value::BoundFunction(d),
            WeakKey::BuiltinFunction(d) => Value::BuiltinFunction(d),
            WeakKey::ECMAScriptFunction(d) => Value::ECMAScriptFunction(d),
            WeakKey::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            WeakKey::BuiltinConstructorFunction(d) => Value::BuiltinConstructorFunction(d),
            WeakKey::BuiltinPromiseResolvingFunction(d) => {
                Value::BuiltinPromiseResolvingFunction(d)
            }
            WeakKey::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            WeakKey::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            WeakKey::PrimitiveObject(d) => Value::PrimitiveObject(d),
            WeakKey::Arguments(d) => Value::Arguments(d),
            WeakKey::Array(d) => Value::Array(d),
            WeakKey::ArrayBuffer(d) => Value::ArrayBuffer(d),
            WeakKey::DataView(d) => Value::DataView(d),
            WeakKey::Date(d) => Value::Date(d),
            WeakKey::Error(d) => Value::Error(d),
            WeakKey::FinalizationRegistry(d) => Value::FinalizationRegistry(d),
            WeakKey::Map(d) => Value::Map(d),
            WeakKey::Promise(d) => Value::Promise(d),
            WeakKey::Proxy(d) => Value::Proxy(d),
            WeakKey::RegExp(d) => Value::RegExp(d),
            WeakKey::Set(d) => Value::Set(d),
            WeakKey::SharedArrayBuffer(d) => Value::SharedArrayBuffer(d),
            WeakKey::WeakMap(d) => Value::WeakMap(d),
            WeakKey::WeakRef(d) => Value::WeakRef(d),
            WeakKey::WeakSet(d) => Value::WeakSet(d),
            WeakKey::Int8Array(d) => Value::Int8Array(d),
            WeakKey::Uint8Array(d) => Value::Uint8Array(d),
            WeakKey::Uint8ClampedArray(d) => Value::Uint8ClampedArray(d),
            WeakKey::Int16Array(d) => Value::Int16Array(d),
            WeakKey::Uint16Array(d) => Value::Uint16Array(d),
            WeakKey::Int32Array(d) => Value::Int32Array(d),
            WeakKey::Uint32Array(d) => Value::Uint32Array(d),
            WeakKey::BigInt64Array(d) => Value::BigInt64Array(d),
            WeakKey::BigUint64Array(d) => Value::BigUint64Array(d),
            WeakKey::Float32Array(d) => Value::Float32Array(d),
            WeakKey::Float64Array(d) => Value::Float64Array(d),
            WeakKey::AsyncFromSyncIterator => Value::AsyncFromSyncIterator,
            WeakKey::AsyncGenerator(d) => Value::AsyncGenerator(d),
            WeakKey::ArrayIterator(d) => Value::ArrayIterator(d),
            WeakKey::SetIterator(d) => Value::SetIterator(d),
            WeakKey::MapIterator(d) => Value::MapIterator(d),
            WeakKey::StringIterator(d) => Value::StringIterator(d),
            WeakKey::Generator(d) => Value::Generator(d),
            WeakKey::Module(d) => Value::Module(d),
            WeakKey::EmbedderObject(d) => Value::EmbedderObject(d),
        }
    }
}

impl<'a> From<Object<'a>> for WeakKey<'a> {
    #[inline]
    fn from(value: Object<'a>) -> Self {
        match value {
            Object::Object(d) => WeakKey::Object(d),
            Object::BoundFunction(d) => WeakKey::BoundFunction(d),
            Object::BuiltinFunction(d) => WeakKey::BuiltinFunction(d),
            Object::ECMAScriptFunction(d) => WeakKey::ECMAScriptFunction(d),
            Object::BuiltinGeneratorFunction => WeakKey::BuiltinGeneratorFunction,
            Object::BuiltinConstructorFunction(d) => WeakKey::BuiltinConstructorFunction(d),
            Object::BuiltinPromiseResolvingFunction(d) => {
                WeakKey::BuiltinPromiseResolvingFunction(d)
            }
            Object::BuiltinPromiseCollectorFunction => WeakKey::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => WeakKey::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(d) => WeakKey::PrimitiveObject(d),
            Object::Arguments(d) => WeakKey::Arguments(d),
            Object::Array(d) => WeakKey::Array(d),
            Object::ArrayBuffer(d) => WeakKey::ArrayBuffer(d),
            Object::DataView(d) => WeakKey::DataView(d),
            Object::Date(d) => WeakKey::Date(d),
            Object::Error(d) => WeakKey::Error(d),
            Object::FinalizationRegistry(d) => WeakKey::FinalizationRegistry(d),
            Object::Map(d) => WeakKey::Map(d),
            Object::Promise(d) => WeakKey::Promise(d),
            Object::Proxy(d) => WeakKey::Proxy(d),
            Object::RegExp(d) => WeakKey::RegExp(d),
            Object::Set(d) => WeakKey::Set(d),
            Object::SharedArrayBuffer(d) => WeakKey::SharedArrayBuffer(d),
            Object::WeakMap(d) => WeakKey::WeakMap(d),
            Object::WeakRef(d) => WeakKey::WeakRef(d),
            Object::WeakSet(d) => WeakKey::WeakSet(d),
            Object::Int8Array(d) => WeakKey::Int8Array(d),
            Object::Uint8Array(d) => WeakKey::Uint8Array(d),
            Object::Uint8ClampedArray(d) => WeakKey::Uint8ClampedArray(d),
            Object::Int16Array(d) => WeakKey::Int16Array(d),
            Object::Uint16Array(d) => WeakKey::Uint16Array(d),
            Object::Int32Array(d) => WeakKey::Int32Array(d),
            Object::Uint32Array(d) => WeakKey::Uint32Array(d),
            Object::BigInt64Array(d) => WeakKey::BigInt64Array(d),
            Object::BigUint64Array(d) => WeakKey::BigUint64Array(d),
            Object::Float32Array(d) => WeakKey::Float32Array(d),
            Object::Float64Array(d) => WeakKey::Float64Array(d),
            Object::AsyncFromSyncIterator => WeakKey::AsyncFromSyncIterator,
            Object::AsyncGenerator(d) => WeakKey::AsyncGenerator(d),
            Object::ArrayIterator(d) => WeakKey::ArrayIterator(d),
            Object::SetIterator(d) => WeakKey::SetIterator(d),
            Object::MapIterator(d) => WeakKey::MapIterator(d),
            Object::StringIterator(d) => WeakKey::StringIterator(d),
            Object::Generator(d) => WeakKey::Generator(d),
            Object::Module(d) => WeakKey::Module(d),
            Object::EmbedderObject(d) => WeakKey::EmbedderObject(d),
        }
    }
}

impl<'a> TryFrom<WeakKey<'a>> for Object<'a> {
    type Error = Symbol<'a>;

    fn try_from(value: WeakKey<'a>) -> Result<Self, Symbol<'a>> {
        match value {
            WeakKey::Symbol(d) => Err(d),
            WeakKey::Object(d) => Ok(Object::Object(d)),
            WeakKey::BoundFunction(d) => Ok(Object::BoundFunction(d)),
            WeakKey::BuiltinFunction(d) => Ok(Object::BuiltinFunction(d)),
            WeakKey::ECMAScriptFunction(d) => Ok(Object::ECMAScriptFunction(d)),
            WeakKey::BuiltinGeneratorFunction => Ok(Object::BuiltinGeneratorFunction),
            WeakKey::BuiltinConstructorFunction(d) => Ok(Object::BuiltinConstructorFunction(d)),
            WeakKey::BuiltinPromiseResolvingFunction(d) => {
                Ok(Object::BuiltinPromiseResolvingFunction(d))
            }
            WeakKey::BuiltinPromiseCollectorFunction => Ok(Object::BuiltinPromiseCollectorFunction),
            WeakKey::BuiltinProxyRevokerFunction => Ok(Object::BuiltinProxyRevokerFunction),
            WeakKey::PrimitiveObject(d) => Ok(Object::PrimitiveObject(d)),
            WeakKey::Arguments(d) => Ok(Object::Arguments(d)),
            WeakKey::Array(d) => Ok(Object::Array(d)),
            WeakKey::ArrayBuffer(d) => Ok(Object::ArrayBuffer(d)),
            WeakKey::DataView(d) => Ok(Object::DataView(d)),
            WeakKey::Date(d) => Ok(Object::Date(d)),
            WeakKey::Error(d) => Ok(Object::Error(d)),
            WeakKey::FinalizationRegistry(d) => Ok(Object::FinalizationRegistry(d)),
            WeakKey::Map(d) => Ok(Object::Map(d)),
            WeakKey::Promise(d) => Ok(Object::Promise(d)),
            WeakKey::Proxy(d) => Ok(Object::Proxy(d)),
            WeakKey::RegExp(d) => Ok(Object::RegExp(d)),
            WeakKey::Set(d) => Ok(Object::Set(d)),
            WeakKey::SharedArrayBuffer(d) => Ok(Object::SharedArrayBuffer(d)),
            WeakKey::WeakMap(d) => Ok(Object::WeakMap(d)),
            WeakKey::WeakRef(d) => Ok(Object::WeakRef(d)),
            WeakKey::WeakSet(d) => Ok(Object::WeakSet(d)),
            WeakKey::Int8Array(d) => Ok(Object::Int8Array(d)),
            WeakKey::Uint8Array(d) => Ok(Object::Uint8Array(d)),
            WeakKey::Uint8ClampedArray(d) => Ok(Object::Uint8ClampedArray(d)),
            WeakKey::Int16Array(d) => Ok(Object::Int16Array(d)),
            WeakKey::Uint16Array(d) => Ok(Object::Uint16Array(d)),
            WeakKey::Int32Array(d) => Ok(Object::Int32Array(d)),
            WeakKey::Uint32Array(d) => Ok(Object::Uint32Array(d)),
            WeakKey::BigInt64Array(d) => Ok(Object::BigInt64Array(d)),
            WeakKey::BigUint64Array(d) => Ok(Object::BigUint64Array(d)),
            WeakKey::Float32Array(d) => Ok(Object::Float32Array(d)),
            WeakKey::Float64Array(d) => Ok(Object::Float64Array(d)),
            WeakKey::AsyncFromSyncIterator => Ok(Object::AsyncFromSyncIterator),
            WeakKey::AsyncGenerator(d) => Ok(Object::AsyncGenerator(d)),
            WeakKey::ArrayIterator(d) => Ok(Object::ArrayIterator(d)),
            WeakKey::SetIterator(d) => Ok(Object::SetIterator(d)),
            WeakKey::MapIterator(d) => Ok(Object::MapIterator(d)),
            WeakKey::StringIterator(d) => Ok(Object::StringIterator(d)),
            WeakKey::Generator(d) => Ok(Object::Generator(d)),
            WeakKey::Module(d) => Ok(Object::Module(d)),
            WeakKey::EmbedderObject(d) => Ok(Object::EmbedderObject(d)),
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for WeakKey<'_> {
    type Of<'a> = WeakKey<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl Rootable for WeakKey<'_> {
    type RootRepr = HeapRootRef;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match Object::try_from(value) {
            Ok(object) => Err(object.unbind().into()),
            Err(symbol) => Err(HeapRootData::Symbol(symbol.unbind())),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        if let HeapRootData::Symbol(symbol) = heap_data {
            Some(Self::Symbol(symbol))
        } else if let Ok(object) = Object::try_from(heap_data) {
            Some(Self::from(object))
        } else {
            None
        }
    }
}

impl HeapMarkAndSweep for WeakKey<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Self::Symbol(d) => d.mark_values(queues),
            Self::Object(d) => d.mark_values(queues),
            Self::BoundFunction(d) => d.mark_values(queues),
            Self::BuiltinFunction(d) => d.mark_values(queues),
            Self::ECMAScriptFunction(d) => d.mark_values(queues),
            Self::BuiltinGeneratorFunction => {}
            Self::BuiltinConstructorFunction(d) => d.mark_values(queues),
            Self::BuiltinPromiseResolvingFunction(d) => d.mark_values(queues),
            Self::BuiltinPromiseCollectorFunction => {}
            Self::BuiltinProxyRevokerFunction => {}
            Self::PrimitiveObject(d) => d.mark_values(queues),
            Self::Arguments(d) => d.mark_values(queues),
            Self::Array(d) => d.mark_values(queues),
            Self::ArrayBuffer(d) => d.mark_values(queues),
            Self::DataView(d) => d.mark_values(queues),
            Self::Date(d) => d.mark_values(queues),
            Self::Error(d) => d.mark_values(queues),
            Self::FinalizationRegistry(d) => d.mark_values(queues),
            Self::Map(d) => d.mark_values(queues),
            Self::Promise(d) => d.mark_values(queues),
            Self::Proxy(d) => d.mark_values(queues),
            Self::RegExp(d) => d.mark_values(queues),
            Self::Set(d) => d.mark_values(queues),
            Self::SharedArrayBuffer(d) => d.mark_values(queues),
            Self::WeakMap(d) => d.mark_values(queues),
            Self::WeakRef(d) => d.mark_values(queues),
            Self::WeakSet(d) => d.mark_values(queues),
            Self::Int8Array(d) => d.mark_values(queues),
            Self::Uint8Array(d) => d.mark_values(queues),
            Self::Uint8ClampedArray(d) => d.mark_values(queues),
            Self::Int16Array(d) => d.mark_values(queues),
            Self::Uint16Array(d) => d.mark_values(queues),
            Self::Int32Array(d) => d.mark_values(queues),
            Self::Uint32Array(d) => d.mark_values(queues),
            Self::BigInt64Array(d) => d.mark_values(queues),
            Self::BigUint64Array(d) => d.mark_values(queues),
            Self::Float32Array(d) => d.mark_values(queues),
            Self::Float64Array(d) => d.mark_values(queues),
            Self::AsyncFromSyncIterator => {}
            Self::AsyncGenerator(d) => d.mark_values(queues),
            Self::ArrayIterator(d) => d.mark_values(queues),
            Self::SetIterator(d) => d.mark_values(queues),
            Self::MapIterator(d) => d.mark_values(queues),
            Self::StringIterator(d) => d.mark_values(queues),
            Self::Generator(d) => d.mark_values(queues),
            Self::Module(d) => d.mark_values(queues),
            Self::EmbedderObject(d) => d.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::Symbol(d) => d.sweep_values(compactions),
            Self::Object(d) => d.sweep_values(compactions),
            Self::BoundFunction(d) => d.sweep_values(compactions),
            Self::BuiltinFunction(d) => d.sweep_values(compactions),
            Self::ECMAScriptFunction(d) => d.sweep_values(compactions),
            Self::BuiltinGeneratorFunction => {}
            Self::BuiltinConstructorFunction(d) => d.sweep_values(compactions),
            Self::BuiltinPromiseResolvingFunction(d) => d.sweep_values(compactions),
            Self::BuiltinPromiseCollectorFunction => {}
            Self::BuiltinProxyRevokerFunction => {}
            Self::PrimitiveObject(d) => d.sweep_values(compactions),
            Self::Arguments(d) => d.sweep_values(compactions),
            Self::Array(d) => d.sweep_values(compactions),
            Self::ArrayBuffer(d) => d.sweep_values(compactions),
            Self::DataView(d) => d.sweep_values(compactions),
            Self::Date(d) => d.sweep_values(compactions),
            Self::Error(d) => d.sweep_values(compactions),
            Self::FinalizationRegistry(d) => d.sweep_values(compactions),
            Self::Map(d) => d.sweep_values(compactions),
            Self::Promise(d) => d.sweep_values(compactions),
            Self::Proxy(d) => d.sweep_values(compactions),
            Self::RegExp(d) => d.sweep_values(compactions),
            Self::Set(d) => d.sweep_values(compactions),
            Self::SharedArrayBuffer(d) => d.sweep_values(compactions),
            Self::WeakMap(d) => d.sweep_values(compactions),
            Self::WeakRef(d) => d.sweep_values(compactions),
            Self::WeakSet(d) => d.sweep_values(compactions),
            Self::Int8Array(d) => d.sweep_values(compactions),
            Self::Uint8Array(d) => d.sweep_values(compactions),
            Self::Uint8ClampedArray(d) => d.sweep_values(compactions),
            Self::Int16Array(d) => d.sweep_values(compactions),
            Self::Uint16Array(d) => d.sweep_values(compactions),
            Self::Int32Array(d) => d.sweep_values(compactions),
            Self::Uint32Array(d) => d.sweep_values(compactions),
            Self::BigInt64Array(d) => d.sweep_values(compactions),
            Self::BigUint64Array(d) => d.sweep_values(compactions),
            Self::Float32Array(d) => d.sweep_values(compactions),
            Self::Float64Array(d) => d.sweep_values(compactions),
            Self::AsyncFromSyncIterator => {}
            Self::AsyncGenerator(d) => d.sweep_values(compactions),
            Self::ArrayIterator(d) => d.sweep_values(compactions),
            Self::SetIterator(d) => d.sweep_values(compactions),
            Self::MapIterator(d) => d.sweep_values(compactions),
            Self::StringIterator(d) => d.sweep_values(compactions),
            Self::Generator(d) => d.sweep_values(compactions),
            Self::Module(d) => d.sweep_values(compactions),
            Self::EmbedderObject(d) => d.sweep_values(compactions),
        }
    }
}

impl HeapSweepWeakReference for WeakKey<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        match self {
            Self::Symbol(data) => data.sweep_weak_reference(compactions).map(Self::Symbol),
            Self::Object(data) => data.sweep_weak_reference(compactions).map(Self::Object),
            Self::BoundFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BoundFunction),
            Self::BuiltinFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinFunction),
            Self::ECMAScriptFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::ECMAScriptFunction),
            Self::BuiltinGeneratorFunction => Some(Self::BuiltinGeneratorFunction),
            Self::BuiltinConstructorFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinConstructorFunction),
            Self::BuiltinPromiseResolvingFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinPromiseResolvingFunction),
            Self::BuiltinPromiseCollectorFunction => Some(Self::BuiltinPromiseCollectorFunction),
            Self::BuiltinProxyRevokerFunction => Some(Self::BuiltinProxyRevokerFunction),
            Self::PrimitiveObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::PrimitiveObject),
            Self::Arguments(data) => data.sweep_weak_reference(compactions).map(Self::Arguments),
            Self::Array(data) => data.sweep_weak_reference(compactions).map(Self::Array),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::ArrayBuffer),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data) => data.sweep_weak_reference(compactions).map(Self::DataView),
            #[cfg(feature = "date")]
            Self::Date(data) => data.sweep_weak_reference(compactions).map(Self::Date),
            Self::Error(data) => data.sweep_weak_reference(compactions).map(Self::Error),
            Self::FinalizationRegistry(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::FinalizationRegistry),
            Self::Map(data) => data.sweep_weak_reference(compactions).map(Self::Map),
            Self::Promise(data) => data.sweep_weak_reference(compactions).map(Self::Promise),
            Self::Proxy(data) => data.sweep_weak_reference(compactions).map(Self::Proxy),
            #[cfg(feature = "regexp")]
            Self::RegExp(data) => data.sweep_weak_reference(compactions).map(Self::RegExp),
            #[cfg(feature = "set")]
            Self::Set(data) => data.sweep_weak_reference(compactions).map(Self::Set),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::SharedArrayBuffer),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.sweep_weak_reference(compactions).map(Self::WeakMap),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.sweep_weak_reference(compactions).map(Self::WeakRef),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.sweep_weak_reference(compactions).map(Self::WeakSet),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(data) => data.sweep_weak_reference(compactions).map(Self::Int8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(data) => data.sweep_weak_reference(compactions).map(Self::Uint8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Uint8ClampedArray),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(data) => data.sweep_weak_reference(compactions).map(Self::Int16Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Uint16Array),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(data) => data.sweep_weak_reference(compactions).map(Self::Int32Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Uint32Array),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BigInt64Array),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BigUint64Array),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(data) => data.sweep_weak_values(compactions).map(Self::Float16Array),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Float32Array),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Float64Array),
            Self::AsyncFromSyncIterator => Some(Self::AsyncFromSyncIterator),
            Self::AsyncGenerator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::AsyncGenerator),
            Self::ArrayIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::ArrayIterator),
            #[cfg(feature = "set")]
            Self::SetIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::SetIterator),
            Self::MapIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::MapIterator),
            Self::StringIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::StringIterator),
            Self::Generator(data) => data.sweep_weak_reference(compactions).map(Self::Generator),
            Self::Module(data) => data.sweep_weak_reference(compactions).map(Self::Module),
            Self::EmbedderObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::EmbedderObject),
        }
    }
}
