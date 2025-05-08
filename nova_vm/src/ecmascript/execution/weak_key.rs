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
            WeakKey::Symbol(d) => Self::Symbol(d),
            WeakKey::Object(d) => Self::Object(d),
            WeakKey::BoundFunction(d) => Self::BoundFunction(d),
            WeakKey::BuiltinFunction(d) => Self::BuiltinFunction(d),
            WeakKey::ECMAScriptFunction(d) => Self::ECMAScriptFunction(d),
            WeakKey::BuiltinGeneratorFunction => Self::BuiltinGeneratorFunction,
            WeakKey::BuiltinConstructorFunction(d) => Self::BuiltinConstructorFunction(d),
            WeakKey::BuiltinPromiseResolvingFunction(d) => Self::BuiltinPromiseResolvingFunction(d),
            WeakKey::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            WeakKey::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
            WeakKey::PrimitiveObject(d) => Self::PrimitiveObject(d),
            WeakKey::Arguments(d) => Self::Arguments(d),
            WeakKey::Array(d) => Self::Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::ArrayBuffer(d) => Self::ArrayBuffer(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::DataView(d) => Self::DataView(d),
            #[cfg(feature = "date")]
            WeakKey::Date(d) => Self::Date(d),
            WeakKey::Error(d) => Self::Error(d),
            WeakKey::FinalizationRegistry(d) => Self::FinalizationRegistry(d),
            WeakKey::Map(d) => Self::Map(d),
            WeakKey::Promise(d) => Self::Promise(d),
            WeakKey::Proxy(d) => Self::Proxy(d),
            #[cfg(feature = "regexp")]
            WeakKey::RegExp(d) => Self::RegExp(d),
            #[cfg(feature = "set")]
            WeakKey::Set(d) => Self::Set(d),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedArrayBuffer(d) => Self::SharedArrayBuffer(d),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakMap(d) => Self::WeakMap(d),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakRef(d) => Self::WeakRef(d),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakSet(d) => Self::WeakSet(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int8Array(d) => Self::Int8Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8Array(d) => Self::Uint8Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8ClampedArray(d) => Self::Uint8ClampedArray(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int16Array(d) => Self::Int16Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint16Array(d) => Self::Uint16Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int32Array(d) => Self::Int32Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint32Array(d) => Self::Uint32Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigInt64Array(d) => Self::BigInt64Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigUint64Array(d) => Self::BigUint64Array(d),
            #[cfg(feature = "proposal-float16array")]
            WeakKey::Float16Array(d) => Self::Float16Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float32Array(d) => Self::Float32Array(d),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float64Array(d) => Self::Float64Array(d),
            WeakKey::AsyncFromSyncIterator => Self::AsyncFromSyncIterator,
            WeakKey::AsyncGenerator(d) => Self::AsyncGenerator(d),
            WeakKey::ArrayIterator(d) => Self::ArrayIterator(d),
            #[cfg(feature = "set")]
            WeakKey::SetIterator(d) => Self::SetIterator(d),
            WeakKey::MapIterator(d) => Self::MapIterator(d),
            WeakKey::StringIterator(d) => Self::StringIterator(d),
            WeakKey::Generator(d) => Self::Generator(d),
            WeakKey::Module(d) => Self::Module(d),
            WeakKey::EmbedderObject(d) => Self::EmbedderObject(d),
        }
    }
}

impl<'a> From<Object<'a>> for WeakKey<'a> {
    #[inline]
    fn from(value: Object<'a>) -> Self {
        match value {
            Object::Object(d) => Self::Object(d),
            Object::BoundFunction(d) => Self::BoundFunction(d),
            Object::BuiltinFunction(d) => Self::BuiltinFunction(d),
            Object::ECMAScriptFunction(d) => Self::ECMAScriptFunction(d),
            Object::BuiltinGeneratorFunction => Self::BuiltinGeneratorFunction,
            Object::BuiltinConstructorFunction(d) => Self::BuiltinConstructorFunction(d),
            Object::BuiltinPromiseResolvingFunction(d) => Self::BuiltinPromiseResolvingFunction(d),
            Object::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(d) => Self::PrimitiveObject(d),
            Object::Arguments(d) => Self::Arguments(d),
            Object::Array(d) => Self::Array(d),
            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(d) => Self::ArrayBuffer(d),
            #[cfg(feature = "array-buffer")]
            Object::DataView(d) => Self::DataView(d),
            #[cfg(feature = "date")]
            Object::Date(d) => Self::Date(d),
            Object::Error(d) => Self::Error(d),
            Object::FinalizationRegistry(d) => Self::FinalizationRegistry(d),
            Object::Map(d) => Self::Map(d),
            Object::Promise(d) => Self::Promise(d),
            Object::Proxy(d) => Self::Proxy(d),
            #[cfg(feature = "regexp")]
            Object::RegExp(d) => Self::RegExp(d),
            Object::Set(d) => Self::Set(d),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(d) => Self::SharedArrayBuffer(d),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(d) => Self::WeakMap(d),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(d) => Self::WeakRef(d),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(d) => Self::WeakSet(d),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(d) => Self::Int8Array(d),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(d) => Self::Uint8Array(d),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(d) => Self::Uint8ClampedArray(d),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(d) => Self::Int16Array(d),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(d) => Self::Uint16Array(d),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(d) => Self::Int32Array(d),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(d) => Self::Uint32Array(d),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(d) => Self::BigInt64Array(d),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(d) => Self::BigUint64Array(d),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(d) => Self::Float16Array(d),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(d) => Self::Float32Array(d),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(d) => Self::Float64Array(d),
            Object::AsyncFromSyncIterator => Self::AsyncFromSyncIterator,
            Object::AsyncGenerator(d) => Self::AsyncGenerator(d),
            Object::ArrayIterator(d) => Self::ArrayIterator(d),
            #[cfg(feature = "set")]
            Object::SetIterator(d) => Self::SetIterator(d),
            Object::MapIterator(d) => Self::MapIterator(d),
            Object::StringIterator(d) => Self::StringIterator(d),
            Object::Generator(d) => Self::Generator(d),
            Object::Module(d) => Self::Module(d),
            Object::EmbedderObject(d) => Self::EmbedderObject(d),
        }
    }
}

impl<'a> TryFrom<WeakKey<'a>> for Object<'a> {
    type Error = Symbol<'a>;

    fn try_from(value: WeakKey<'a>) -> Result<Self, Symbol<'a>> {
        match value {
            WeakKey::Symbol(d) => Err(d),
            WeakKey::Object(d) => Ok(Self::Object(d)),
            WeakKey::BoundFunction(d) => Ok(Self::BoundFunction(d)),
            WeakKey::BuiltinFunction(d) => Ok(Self::BuiltinFunction(d)),
            WeakKey::ECMAScriptFunction(d) => Ok(Self::ECMAScriptFunction(d)),
            WeakKey::BuiltinGeneratorFunction => Ok(Self::BuiltinGeneratorFunction),
            WeakKey::BuiltinConstructorFunction(d) => Ok(Self::BuiltinConstructorFunction(d)),
            WeakKey::BuiltinPromiseResolvingFunction(d) => {
                Ok(Self::BuiltinPromiseResolvingFunction(d))
            }
            WeakKey::BuiltinPromiseCollectorFunction => Ok(Self::BuiltinPromiseCollectorFunction),
            WeakKey::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            WeakKey::PrimitiveObject(d) => Ok(Self::PrimitiveObject(d)),
            WeakKey::Arguments(d) => Ok(Self::Arguments(d)),
            WeakKey::Array(d) => Ok(Self::Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::ArrayBuffer(d) => Ok(Self::ArrayBuffer(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::DataView(d) => Ok(Self::DataView(d)),
            #[cfg(feature = "date")]
            WeakKey::Date(d) => Ok(Self::Date(d)),
            WeakKey::Error(d) => Ok(Self::Error(d)),
            WeakKey::FinalizationRegistry(d) => Ok(Self::FinalizationRegistry(d)),
            WeakKey::Map(d) => Ok(Self::Map(d)),
            WeakKey::Promise(d) => Ok(Self::Promise(d)),
            WeakKey::Proxy(d) => Ok(Self::Proxy(d)),
            #[cfg(feature = "regexp")]
            WeakKey::RegExp(d) => Ok(Self::RegExp(d)),
            WeakKey::Set(d) => Ok(Self::Set(d)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedArrayBuffer(d) => Ok(Self::SharedArrayBuffer(d)),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakMap(d) => Ok(Self::WeakMap(d)),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakRef(d) => Ok(Self::WeakRef(d)),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakSet(d) => Ok(Self::WeakSet(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int8Array(d) => Ok(Self::Int8Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8Array(d) => Ok(Self::Uint8Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8ClampedArray(d) => Ok(Self::Uint8ClampedArray(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int16Array(d) => Ok(Self::Int16Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint16Array(d) => Ok(Self::Uint16Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int32Array(d) => Ok(Self::Int32Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint32Array(d) => Ok(Self::Uint32Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigInt64Array(d) => Ok(Self::BigInt64Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigUint64Array(d) => Ok(Self::BigUint64Array(d)),
            #[cfg(feature = "proposal-float16array")]
            WeakKey::Float16Array(d) => Ok(Self::Float16Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float32Array(d) => Ok(Self::Float32Array(d)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float64Array(d) => Ok(Self::Float64Array(d)),
            WeakKey::AsyncFromSyncIterator => Ok(Self::AsyncFromSyncIterator),
            WeakKey::AsyncGenerator(d) => Ok(Self::AsyncGenerator(d)),
            WeakKey::ArrayIterator(d) => Ok(Self::ArrayIterator(d)),
            #[cfg(feature = "set")]
            WeakKey::SetIterator(d) => Ok(Self::SetIterator(d)),
            WeakKey::MapIterator(d) => Ok(Self::MapIterator(d)),
            WeakKey::StringIterator(d) => Ok(Self::StringIterator(d)),
            WeakKey::Generator(d) => Ok(Self::Generator(d)),
            WeakKey::Module(d) => Ok(Self::Module(d)),
            WeakKey::EmbedderObject(d) => Ok(Self::EmbedderObject(d)),
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
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::DataView(d) => d.mark_values(queues),
            #[cfg(feature = "date")]
            Self::Date(d) => d.mark_values(queues),
            Self::Error(d) => d.mark_values(queues),
            Self::FinalizationRegistry(d) => d.mark_values(queues),
            Self::Map(d) => d.mark_values(queues),
            Self::Promise(d) => d.mark_values(queues),
            Self::Proxy(d) => d.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExp(d) => d.mark_values(queues),
            Self::Set(d) => d.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(d) => d.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(d) => d.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(d) => d.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(d) => d.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(d) => d.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(d) => d.mark_values(queues),
            Self::AsyncFromSyncIterator => {}
            Self::AsyncGenerator(d) => d.mark_values(queues),
            Self::ArrayIterator(d) => d.mark_values(queues),
            #[cfg(feature = "set")]
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
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::DataView(d) => d.sweep_values(compactions),
            #[cfg(feature = "date")]
            Self::Date(d) => d.sweep_values(compactions),
            Self::Error(d) => d.sweep_values(compactions),
            Self::FinalizationRegistry(d) => d.sweep_values(compactions),
            Self::Map(d) => d.sweep_values(compactions),
            Self::Promise(d) => d.sweep_values(compactions),
            Self::Proxy(d) => d.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExp(d) => d.sweep_values(compactions),
            Self::Set(d) => d.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(d) => d.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(d) => d.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(d) => d.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(d) => d.sweep_values(compactions),
            Self::AsyncFromSyncIterator => {}
            Self::AsyncGenerator(d) => d.sweep_values(compactions),
            Self::ArrayIterator(d) => d.sweep_values(compactions),
            #[cfg(feature = "set")]
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
            Self::Float16Array(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::Float16Array),
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
