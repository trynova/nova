// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Weakly holdable JavaScript Value.

#[cfg(feature = "date")]
use crate::ecmascript::DATE_DISCRIMINANT;
#[cfg(feature = "date")]
use crate::ecmascript::Date;
use crate::ecmascript::UnmappedArguments;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::{
    ARRAY_BUFFER_DISCRIMINANT, ArrayBuffer, BIGINT_64_ARRAY_DISCRIMINANT,
    BIGUINT_64_ARRAY_DISCRIMINANT, BigInt64Array, BigUint64Array, DATA_VIEW_DISCRIMINANT, DataView,
    FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, Float32Array, Float64Array,
    INT_8_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, Int8Array,
    Int16Array, Int32Array, UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, Uint8Array, Uint8ClampedArray,
    Uint16Array, Uint32Array,
};
#[cfg(feature = "temporal")]
use crate::ecmascript::{
    DURATION_DISCRIMINANT, INSTANT_DISCRIMINANT, TemporalDuration, TemporalInstant,
};
#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::{FLOAT_16_ARRAY_DISCRIMINANT, Float16Array};
#[cfg(feature = "regexp")]
use crate::ecmascript::{
    REGEXP_DISCRIMINANT, REGEXP_STRING_ITERATOR_DISCRIMINANT, RegExp, RegExpStringIterator,
};
#[cfg(feature = "set")]
use crate::ecmascript::{SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT, Set, SetIterator};
#[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
use crate::ecmascript::{SHARED_FLOAT_16_ARRAY_DISCRIMINANT, SharedFloat16Array};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WeakMap, WeakRef, WeakSet};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::{
    builtins::{
        SharedArrayBuffer, SharedBigInt64Array, SharedBigUint64Array, SharedDataView,
        SharedFloat32Array, SharedFloat64Array, SharedInt8Array, SharedInt16Array,
        SharedInt32Array, SharedUint8Array, SharedUint8ClampedArray, SharedUint16Array,
        SharedUint32Array,
    },
    types::{
        SHARED_ARRAY_BUFFER_DISCRIMINANT, SHARED_BIGINT_64_ARRAY_DISCRIMINANT,
        SHARED_BIGUINT_64_ARRAY_DISCRIMINANT, SHARED_DATA_VIEW_DISCRIMINANT,
        SHARED_FLOAT_32_ARRAY_DISCRIMINANT, SHARED_FLOAT_64_ARRAY_DISCRIMINANT,
        SHARED_INT_8_ARRAY_DISCRIMINANT, SHARED_INT_16_ARRAY_DISCRIMINANT,
        SHARED_INT_32_ARRAY_DISCRIMINANT, SHARED_UINT_8_ARRAY_DISCRIMINANT,
        SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT, SHARED_UINT_16_ARRAY_DISCRIMINANT,
        SHARED_UINT_32_ARRAY_DISCRIMINANT,
    },
};
use crate::{
    ecmascript::{
        ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
        ASYNC_GENERATOR_DISCRIMINANT, Array, ArrayIterator, AsyncGenerator,
        BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        BoundFunction, BuiltinConstructorFunction, BuiltinFunction, BuiltinPromiseFinallyFunction,
        BuiltinPromiseResolvingFunction, ECMASCRIPT_FUNCTION_DISCRIMINANT, ECMAScriptFunction,
        EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT, EmbedderObject, Error,
        FINALIZATION_REGISTRY_DISCRIMINANT, FinalizationRegistry, GENERATOR_DISCRIMINANT,
        Generator, MAP_DISCRIMINANT, MAP_ITERATOR_DISCRIMINANT, MODULE_DISCRIMINANT, Map,
        MapIterator, Module, OBJECT_DISCRIMINANT, Object, OrdinaryObject,
        PRIMITIVE_OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT, PrimitiveObject,
        Promise, Proxy, STRING_ITERATOR_DISCRIMINANT, SYMBOL_DISCRIMINANT, StringIterator, Symbol,
        Value,
    },
    engine::{Bindable, HeapRootData, HeapRootRef, Rootable, bindable_handle},
    heap::{
        CompactionLists, HeapMarkAndSweep, HeapSweepWeakReference, WellKnownSymbols, WorkQueues,
    },
};

/// ## [6.1 ECMAScript Language Types](https://tc39.es/ecma262/#sec-ecmascript-language-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum WeakKey<'a> {
    Symbol(Symbol<'a>) = SYMBOL_DISCRIMINANT,
    Object(OrdinaryObject<'a>) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction<'a>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'a>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'a>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseFinallyFunction(BuiltinPromiseFinallyFunction<'a>) =
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject<'a>) = PRIMITIVE_OBJECT_DISCRIMINANT,
    Arguments(UnmappedArguments<'a>) = ARGUMENTS_DISCRIMINANT,
    Array(Array<'a>) = ARRAY_DISCRIMINANT,
    #[cfg(feature = "date")]
    Date(Date<'a>) = DATE_DISCRIMINANT,
    #[cfg(feature = "temporal")]
    Instant(TemporalInstant<'a>) = INSTANT_DISCRIMINANT,
    #[cfg(feature = "temporal")]
    Duration(TemporalDuration<'a>) = DURATION_DISCRIMINANT,
    Error(Error<'a>) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistry<'a>) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(Map<'a>) = MAP_DISCRIMINANT,
    Promise(Promise<'a>) = PROMISE_DISCRIMINANT,
    Proxy(Proxy<'a>) = PROXY_DISCRIMINANT,
    #[cfg(feature = "regexp")]
    RegExp(RegExp<'a>) = REGEXP_DISCRIMINANT,
    #[cfg(feature = "set")]
    Set(Set<'a>) = SET_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'a>) = WEAK_MAP_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'a>) = WEAK_REF_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'a>) = WEAK_SET_DISCRIMINANT,

    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'a>) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'a>) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int8Array(Int8Array<'a>) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(Uint8Array<'a>) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(Uint8ClampedArray<'a>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(Int16Array<'a>) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(Uint16Array<'a>) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(Int32Array<'a>) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(Uint32Array<'a>) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(BigInt64Array<'a>) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(BigUint64Array<'a>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(Float16Array<'a>) = FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(Float32Array<'a>) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(Float64Array<'a>) = FLOAT_64_ARRAY_DISCRIMINANT,

    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'a>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedDataView(SharedDataView<'a>) = SHARED_DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt8Array(SharedInt8Array<'a>) = SHARED_INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8Array(SharedUint8Array<'a>) = SHARED_UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8ClampedArray(SharedUint8ClampedArray<'a>) = SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt16Array(SharedInt16Array<'a>) = SHARED_INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint16Array(SharedUint16Array<'a>) = SHARED_UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt32Array(SharedInt32Array<'a>) = SHARED_INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint32Array(SharedUint32Array<'a>) = SHARED_UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedBigInt64Array(SharedBigInt64Array<'a>) = SHARED_BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedBigUint64Array(SharedBigUint64Array<'a>) = SHARED_BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
    SharedFloat16Array(SharedFloat16Array<'a>) = SHARED_FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat32Array(SharedFloat32Array<'a>) = SHARED_FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat64Array(SharedFloat64Array<'a>) = SHARED_FLOAT_64_ARRAY_DISCRIMINANT,

    AsyncGenerator(AsyncGenerator<'a>) = ASYNC_GENERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator<'a>) = ARRAY_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'a>) = SET_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    MapIterator(MapIterator<'a>) = MAP_ITERATOR_DISCRIMINANT,
    StringIterator(StringIterator<'a>) = STRING_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "regexp")]
    RegExpStringIterator(RegExpStringIterator<'a>) = REGEXP_STRING_ITERATOR_DISCRIMINANT,
    Generator(Generator<'a>) = GENERATOR_DISCRIMINANT,
    Module(Module<'a>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject<'a>) = EMBEDDER_OBJECT_DISCRIMINANT,
}

impl core::hash::Hash for WeakKey<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let value: Value = (*self).into();
        value.try_hash(state).unwrap()
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
            WeakKey::BuiltinConstructorFunction(d) => Self::BuiltinConstructorFunction(d),
            WeakKey::BuiltinPromiseResolvingFunction(d) => Self::BuiltinPromiseResolvingFunction(d),
            WeakKey::BuiltinPromiseFinallyFunction(d) => Self::BuiltinPromiseFinallyFunction(d),
            WeakKey::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            WeakKey::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
            WeakKey::PrimitiveObject(d) => Self::PrimitiveObject(d),
            WeakKey::Arguments(d) => Self::Arguments(d),
            WeakKey::Array(d) => Self::Array(d),
            #[cfg(feature = "date")]
            WeakKey::Date(d) => Self::Date(d),
            #[cfg(feature = "temporal")]
            WeakKey::Instant(d) => Self::Instant(d),
            #[cfg(feature = "temporal")]
            WeakKey::Duration(d) => Self::Duration(d),
            WeakKey::Error(d) => Self::Error(d),
            WeakKey::FinalizationRegistry(d) => Self::FinalizationRegistry(d),
            WeakKey::Map(d) => Self::Map(d),
            WeakKey::Promise(d) => Self::Promise(d),
            WeakKey::Proxy(d) => Self::Proxy(d),
            #[cfg(feature = "regexp")]
            WeakKey::RegExp(d) => Self::RegExp(d),
            #[cfg(feature = "set")]
            WeakKey::Set(d) => Self::Set(d),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakMap(d) => Self::WeakMap(d),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakRef(d) => Self::WeakRef(d),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakSet(d) => Self::WeakSet(d),

            #[cfg(feature = "array-buffer")]
            WeakKey::ArrayBuffer(ab) => Self::ArrayBuffer(ab),
            #[cfg(feature = "array-buffer")]
            WeakKey::DataView(dv) => Self::DataView(dv),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int8Array(ta) => Self::Int8Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8Array(ta) => Self::Uint8Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8ClampedArray(ta) => Self::Uint8ClampedArray(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int16Array(ta) => Self::Int16Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint16Array(ta) => Self::Uint16Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int32Array(ta) => Self::Int32Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint32Array(ta) => Self::Uint32Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigInt64Array(ta) => Self::BigInt64Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigUint64Array(ta) => Self::BigUint64Array(ta),
            #[cfg(feature = "proposal-float16array")]
            WeakKey::Float16Array(ta) => Self::Float16Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float32Array(ta) => Self::Float32Array(ta),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float64Array(ta) => Self::Float64Array(ta),

            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedArrayBuffer(sab) => Self::SharedArrayBuffer(sab),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedDataView(sdv) => Self::SharedDataView(sdv),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt8Array(sta) => Self::SharedInt8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint8Array(sta) => Self::SharedUint8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint8ClampedArray(sta) => Self::SharedUint8ClampedArray(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt16Array(sta) => Self::SharedInt16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint16Array(sta) => Self::SharedUint16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt32Array(sta) => Self::SharedInt32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint32Array(sta) => Self::SharedUint32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedBigInt64Array(sta) => Self::SharedBigInt64Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedBigUint64Array(sta) => Self::SharedBigUint64Array(sta),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            WeakKey::SharedFloat16Array(sta) => Self::SharedFloat16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedFloat32Array(sta) => Self::SharedFloat32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedFloat64Array(sta) => Self::SharedFloat64Array(sta),

            WeakKey::AsyncGenerator(d) => Self::AsyncGenerator(d),
            WeakKey::ArrayIterator(d) => Self::ArrayIterator(d),
            #[cfg(feature = "set")]
            WeakKey::SetIterator(d) => Self::SetIterator(d),
            WeakKey::MapIterator(d) => Self::MapIterator(d),
            WeakKey::StringIterator(d) => Self::StringIterator(d),
            #[cfg(feature = "regexp")]
            WeakKey::RegExpStringIterator(d) => Self::RegExpStringIterator(d),
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
            Object::BuiltinConstructorFunction(d) => Self::BuiltinConstructorFunction(d),
            Object::BuiltinPromiseResolvingFunction(d) => Self::BuiltinPromiseResolvingFunction(d),
            Object::BuiltinPromiseFinallyFunction(d) => Self::BuiltinPromiseFinallyFunction(d),
            Object::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(d) => Self::PrimitiveObject(d),
            Object::Arguments(d) => Self::Arguments(d),
            Object::Array(d) => Self::Array(d),
            #[cfg(feature = "date")]
            Object::Date(d) => Self::Date(d),
            #[cfg(feature = "temporal")]
            Object::Instant(d) => Self::Instant(d),
            #[cfg(feature = "temporal")]
            Object::Duration(d) => Self::Duration(d),
            Object::Error(d) => Self::Error(d),
            Object::FinalizationRegistry(d) => Self::FinalizationRegistry(d),
            Object::Map(d) => Self::Map(d),
            Object::Promise(d) => Self::Promise(d),
            Object::Proxy(d) => Self::Proxy(d),
            #[cfg(feature = "regexp")]
            Object::RegExp(d) => Self::RegExp(d),
            #[cfg(feature = "set")]
            Object::Set(d) => Self::Set(d),
            #[cfg(feature = "weak-refs")]
            Object::WeakMap(d) => Self::WeakMap(d),
            #[cfg(feature = "weak-refs")]
            Object::WeakRef(d) => Self::WeakRef(d),
            #[cfg(feature = "weak-refs")]
            Object::WeakSet(d) => Self::WeakSet(d),

            #[cfg(feature = "array-buffer")]
            Object::ArrayBuffer(ab) => Self::ArrayBuffer(ab),
            #[cfg(feature = "array-buffer")]
            Object::DataView(dv) => Self::DataView(dv),
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(ta) => Self::Int8Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint8Array(ta) => Self::Uint8Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint8ClampedArray(ta) => Self::Uint8ClampedArray(ta),
            #[cfg(feature = "array-buffer")]
            Object::Int16Array(ta) => Self::Int16Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint16Array(ta) => Self::Uint16Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Int32Array(ta) => Self::Int32Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Uint32Array(ta) => Self::Uint32Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::BigInt64Array(ta) => Self::BigInt64Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::BigUint64Array(ta) => Self::BigUint64Array(ta),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(ta) => Self::Float16Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Float32Array(ta) => Self::Float32Array(ta),
            #[cfg(feature = "array-buffer")]
            Object::Float64Array(ta) => Self::Float64Array(ta),

            #[cfg(feature = "shared-array-buffer")]
            Object::SharedArrayBuffer(sab) => Self::SharedArrayBuffer(sab),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedDataView(sdv) => Self::SharedDataView(sdv),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedInt8Array(sta) => Self::SharedInt8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint8Array(sta) => Self::SharedUint8Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint8ClampedArray(sta) => Self::SharedUint8ClampedArray(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedInt16Array(sta) => Self::SharedInt16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint16Array(sta) => Self::SharedUint16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedInt32Array(sta) => Self::SharedInt32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedUint32Array(sta) => Self::SharedUint32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedBigInt64Array(sta) => Self::SharedBigInt64Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedBigUint64Array(sta) => Self::SharedBigUint64Array(sta),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Object::SharedFloat16Array(sta) => Self::SharedFloat16Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedFloat32Array(sta) => Self::SharedFloat32Array(sta),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedFloat64Array(sta) => Self::SharedFloat64Array(sta),

            Object::AsyncGenerator(d) => Self::AsyncGenerator(d),
            Object::ArrayIterator(d) => Self::ArrayIterator(d),
            #[cfg(feature = "set")]
            Object::SetIterator(d) => Self::SetIterator(d),
            Object::MapIterator(d) => Self::MapIterator(d),
            Object::StringIterator(d) => Self::StringIterator(d),
            #[cfg(feature = "regexp")]
            Object::RegExpStringIterator(d) => Self::RegExpStringIterator(d),
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
            WeakKey::BuiltinConstructorFunction(d) => Ok(Self::BuiltinConstructorFunction(d)),
            WeakKey::BuiltinPromiseResolvingFunction(d) => {
                Ok(Self::BuiltinPromiseResolvingFunction(d))
            }
            WeakKey::BuiltinPromiseFinallyFunction(d) => Ok(Self::BuiltinPromiseFinallyFunction(d)),
            WeakKey::BuiltinPromiseCollectorFunction => Ok(Self::BuiltinPromiseCollectorFunction),
            WeakKey::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            WeakKey::PrimitiveObject(d) => Ok(Self::PrimitiveObject(d)),
            WeakKey::Arguments(d) => Ok(Self::Arguments(d)),
            WeakKey::Array(d) => Ok(Self::Array(d)),
            #[cfg(feature = "date")]
            WeakKey::Date(d) => Ok(Self::Date(d)),
            #[cfg(feature = "temporal")]
            WeakKey::Instant(d) => Ok(Self::Instant(d)),
            #[cfg(feature = "temporal")]
            WeakKey::Duration(d) => Ok(Self::Duration(d)),
            WeakKey::Error(d) => Ok(Self::Error(d)),
            WeakKey::FinalizationRegistry(d) => Ok(Self::FinalizationRegistry(d)),
            WeakKey::Map(d) => Ok(Self::Map(d)),
            WeakKey::Promise(d) => Ok(Self::Promise(d)),
            WeakKey::Proxy(d) => Ok(Self::Proxy(d)),
            #[cfg(feature = "regexp")]
            WeakKey::RegExp(d) => Ok(Self::RegExp(d)),
            #[cfg(feature = "set")]
            WeakKey::Set(d) => Ok(Self::Set(d)),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakMap(d) => Ok(Self::WeakMap(d)),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakRef(d) => Ok(Self::WeakRef(d)),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakSet(d) => Ok(Self::WeakSet(d)),

            #[cfg(feature = "array-buffer")]
            WeakKey::ArrayBuffer(ab) => Ok(Self::ArrayBuffer(ab)),
            #[cfg(feature = "array-buffer")]
            WeakKey::DataView(dv) => Ok(Self::DataView(dv)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int8Array(ta) => Ok(Self::Int8Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8Array(ta) => Ok(Self::Uint8Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8ClampedArray(ta) => Ok(Self::Uint8ClampedArray(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int16Array(ta) => Ok(Self::Int16Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint16Array(ta) => Ok(Self::Uint16Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int32Array(ta) => Ok(Self::Int32Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint32Array(ta) => Ok(Self::Uint32Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigInt64Array(ta) => Ok(Self::BigInt64Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigUint64Array(ta) => Ok(Self::BigUint64Array(ta)),
            #[cfg(feature = "proposal-float16array")]
            WeakKey::Float16Array(ta) => Ok(Self::Float16Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float32Array(ta) => Ok(Self::Float32Array(ta)),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float64Array(ta) => Ok(Self::Float64Array(ta)),

            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedArrayBuffer(sab) => Ok(Self::SharedArrayBuffer(sab)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedDataView(sdv) => Ok(Self::SharedDataView(sdv)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt8Array(sta) => Ok(Self::SharedInt8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint8Array(sta) => Ok(Self::SharedUint8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint8ClampedArray(sta) => Ok(Self::SharedUint8ClampedArray(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt16Array(sta) => Ok(Self::SharedInt16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint16Array(sta) => Ok(Self::SharedUint16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt32Array(sta) => Ok(Self::SharedInt32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint32Array(sta) => Ok(Self::SharedUint32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedBigInt64Array(sta) => Ok(Self::SharedBigInt64Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedBigUint64Array(sta) => Ok(Self::SharedBigUint64Array(sta)),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            WeakKey::SharedFloat16Array(sta) => Ok(Self::SharedFloat16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedFloat32Array(sta) => Ok(Self::SharedFloat32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedFloat64Array(sta) => Ok(Self::SharedFloat64Array(sta)),

            WeakKey::AsyncGenerator(d) => Ok(Self::AsyncGenerator(d)),
            WeakKey::ArrayIterator(d) => Ok(Self::ArrayIterator(d)),
            #[cfg(feature = "set")]
            WeakKey::SetIterator(d) => Ok(Self::SetIterator(d)),
            WeakKey::MapIterator(d) => Ok(Self::MapIterator(d)),
            WeakKey::StringIterator(d) => Ok(Self::StringIterator(d)),
            #[cfg(feature = "regexp")]
            WeakKey::RegExpStringIterator(d) => Ok(Self::RegExpStringIterator(d)),
            WeakKey::Generator(d) => Ok(Self::Generator(d)),
            WeakKey::Module(d) => Ok(Self::Module(d)),
            WeakKey::EmbedderObject(d) => Ok(Self::EmbedderObject(d)),
        }
    }
}

bindable_handle!(WeakKey);

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum WeakKeyRootRerp {
    Symbol(WellKnownSymbols) = SYMBOL_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl Rootable for WeakKey<'_> {
    type RootRepr = WeakKeyRootRerp;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match Object::try_from(value) {
            Ok(object) => Err(object.unbind().into()),
            Err(symbol) => {
                if let Ok(s) = WellKnownSymbols::try_from(symbol) {
                    Ok(WeakKeyRootRerp::Symbol(s))
                } else {
                    Err(HeapRootData::try_from(symbol).unwrap())
                }
            }
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            WeakKeyRootRerp::Symbol(s) => Ok(Self::Symbol(s.into())),
            WeakKeyRootRerp::HeapRef(s) => Err(s),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
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
            Self::BuiltinConstructorFunction(d) => d.mark_values(queues),
            Self::BuiltinPromiseResolvingFunction(d) => d.mark_values(queues),
            Self::BuiltinPromiseFinallyFunction(d) => d.mark_values(queues),
            Self::BuiltinPromiseCollectorFunction => {}
            Self::BuiltinProxyRevokerFunction => {}
            Self::PrimitiveObject(d) => d.mark_values(queues),
            Self::Arguments(d) => d.mark_values(queues),
            Self::Array(d) => d.mark_values(queues),
            #[cfg(feature = "date")]
            Self::Date(d) => d.mark_values(queues),
            #[cfg(feature = "temporal")]
            Self::Instant(d) => d.mark_values(queues),
            #[cfg(feature = "temporal")]
            Self::Duration(d) => d.mark_values(queues),
            Self::Error(d) => d.mark_values(queues),
            Self::FinalizationRegistry(d) => d.mark_values(queues),
            Self::Map(d) => d.mark_values(queues),
            Self::Promise(d) => d.mark_values(queues),
            Self::Proxy(d) => d.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExp(d) => d.mark_values(queues),
            #[cfg(feature = "set")]
            Self::Set(d) => d.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(d) => d.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(d) => d.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(d) => d.mark_values(queues),

            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => ab.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::DataView(dv) => dv.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.mark_values(queues),

            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sab) => sab.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sdv) => sdv.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => sta.mark_values(queues),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => sta.mark_values(queues),

            Self::AsyncGenerator(d) => d.mark_values(queues),
            Self::ArrayIterator(d) => d.mark_values(queues),
            #[cfg(feature = "set")]
            Self::SetIterator(d) => d.mark_values(queues),
            Self::MapIterator(d) => d.mark_values(queues),
            Self::StringIterator(d) => d.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(d) => d.mark_values(queues),
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
            Self::BuiltinConstructorFunction(d) => d.sweep_values(compactions),
            Self::BuiltinPromiseResolvingFunction(d) => d.sweep_values(compactions),
            Self::BuiltinPromiseFinallyFunction(d) => d.sweep_values(compactions),
            Self::BuiltinPromiseCollectorFunction => {}
            Self::BuiltinProxyRevokerFunction => {}
            Self::PrimitiveObject(d) => d.sweep_values(compactions),
            Self::Arguments(d) => d.sweep_values(compactions),
            Self::Array(d) => d.sweep_values(compactions),
            #[cfg(feature = "date")]
            Self::Date(d) => d.sweep_values(compactions),
            #[cfg(feature = "temporal")]
            Self::Instant(d) => d.sweep_values(compactions),
            #[cfg(feature = "temporal")]
            Self::Duration(d) => d.sweep_values(compactions),
            Self::Error(d) => d.sweep_values(compactions),
            Self::FinalizationRegistry(d) => d.sweep_values(compactions),
            Self::Map(d) => d.sweep_values(compactions),
            Self::Promise(d) => d.sweep_values(compactions),
            Self::Proxy(d) => d.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExp(d) => d.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::Set(d) => d.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(d) => d.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(d) => d.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(d) => d.sweep_values(compactions),

            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => ab.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::DataView(dv) => dv.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.sweep_values(compactions),

            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sab) => sab.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sdv) => sdv.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => sta.sweep_values(compactions),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => sta.sweep_values(compactions),

            Self::AsyncGenerator(d) => d.sweep_values(compactions),
            Self::ArrayIterator(d) => d.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::SetIterator(d) => d.sweep_values(compactions),
            Self::MapIterator(d) => d.sweep_values(compactions),
            Self::StringIterator(d) => d.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(d) => d.sweep_values(compactions),
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
            Self::BuiltinConstructorFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinConstructorFunction),
            Self::BuiltinPromiseResolvingFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinPromiseResolvingFunction),
            Self::BuiltinPromiseFinallyFunction(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::BuiltinPromiseFinallyFunction),
            Self::BuiltinPromiseCollectorFunction => Some(Self::BuiltinPromiseCollectorFunction),
            Self::BuiltinProxyRevokerFunction => Some(Self::BuiltinProxyRevokerFunction),
            Self::PrimitiveObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::PrimitiveObject),
            Self::Arguments(data) => data.sweep_weak_reference(compactions).map(Self::Arguments),
            Self::Array(data) => data.sweep_weak_reference(compactions).map(Self::Array),
            #[cfg(feature = "date")]
            Self::Date(data) => data.sweep_weak_reference(compactions).map(Self::Date),
            #[cfg(feature = "temporal")]
            Self::Instant(data) => data.sweep_weak_reference(compactions).map(Self::Instant),
            #[cfg(feature = "temporal")]
            Self::Duration(data) => data.sweep_weak_reference(compactions).map(Self::Duration),
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
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.sweep_weak_reference(compactions).map(Self::WeakMap),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.sweep_weak_reference(compactions).map(Self::WeakRef),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.sweep_weak_reference(compactions).map(Self::WeakSet),

            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => ab.sweep_weak_reference(compactions).map(Self::ArrayBuffer),
            #[cfg(feature = "array-buffer")]
            Self::DataView(dv) => dv.sweep_weak_reference(compactions).map(Self::DataView),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Int8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Uint8Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta
                .sweep_weak_reference(compactions)
                .map(Self::Uint8ClampedArray),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Int16Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Uint16Array),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Int32Array),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Uint32Array),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta
                .sweep_weak_reference(compactions)
                .map(Self::BigInt64Array),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta
                .sweep_weak_reference(compactions)
                .map(Self::BigUint64Array),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Float16Array),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Float32Array),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.sweep_weak_reference(compactions).map(Self::Float64Array),

            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sab) => sab
                .sweep_weak_reference(compactions)
                .map(Self::SharedArrayBuffer),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sdv) => sdv
                .sweep_weak_reference(compactions)
                .map(Self::SharedDataView),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedInt8Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint8Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint8ClampedArray),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedInt16Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint16Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedInt32Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedUint32Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedBigInt64Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedBigUint64Array),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedFloat16Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedFloat32Array),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => sta
                .sweep_weak_reference(compactions)
                .map(Self::SharedFloat64Array),

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
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::RegExpStringIterator),
            Self::Generator(data) => data.sweep_weak_reference(compactions).map(Self::Generator),
            Self::Module(data) => data.sweep_weak_reference(compactions).map(Self::Module),
            Self::EmbedderObject(data) => data
                .sweep_weak_reference(compactions)
                .map(Self::EmbedderObject),
        }
    }
}
