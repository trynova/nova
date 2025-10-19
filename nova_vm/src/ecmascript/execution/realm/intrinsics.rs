// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::num::NonZeroU32;

use super::Realm;
#[cfg(feature = "atomics")]
use crate::ecmascript::AtomicsObject;
#[cfg(feature = "json")]
use crate::ecmascript::JSONObject;
#[cfg(feature = "math")]
use crate::ecmascript::MathObject;
#[cfg(feature = "temporal")]
use crate::ecmascript::builtins::{
    TemporalObject, instant::InstantConstructor, instant::InstantPrototype,
};
#[cfg(feature = "array-buffer")]
use crate::ecmascript::{
    ArrayBufferConstructor, ArrayBufferPrototype, DataViewConstructor, DataViewPrototype,
    TypedArrayConstructors, TypedArrayIntrinsicObject, TypedArrayPrototype, TypedArrayPrototypes,
};
#[cfg(feature = "date")]
use crate::ecmascript::{DateConstructor, DatePrototype};
#[cfg(feature = "regexp")]
use crate::ecmascript::{RegExpConstructor, RegExpPrototype, RegExpStringIteratorPrototype};
#[cfg(feature = "set")]
use crate::ecmascript::{SetConstructor, SetIteratorPrototype, SetPrototype};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::{SharedArrayBufferConstructor, SharedArrayBufferPrototype};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{
    WeakMapConstructor, WeakMapPrototype, WeakSetConstructor, WeakSetPrototype,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WeakRefConstructor, WeakRefPrototype};
use crate::{
    ecmascript::{
        Agent, AggregateErrorConstructor, AggregateErrorPrototype, Array, ArrayConstructor,
        ArrayIteratorPrototype, ArrayPrototype, AsyncFunctionConstructor, AsyncFunctionPrototype,
        AsyncGeneratorFunctionConstructor, AsyncGeneratorFunctionPrototype,
        AsyncGeneratorPrototype, AsyncIteratorPrototype, BigIntConstructor, BigIntPrototype,
        BooleanConstructor, BooleanPrototype, BuiltinFunction, BuiltinFunctionHeapData,
        ErrorConstructor, ErrorPrototype, FinalizationRegistryConstructor,
        FinalizationRegistryPrototype, Function, FunctionConstructor, FunctionPrototype,
        GeneratorFunctionConstructor, GeneratorFunctionPrototype, GeneratorPrototype, GlobalObject,
        IteratorConstructor, IteratorPrototype, MapConstructor, MapIteratorPrototype, MapPrototype,
        NativeErrorConstructors, NativeErrorPrototypes, NumberConstructor, NumberPrototype, Object,
        ObjectConstructor, ObjectPrototype, ObjectRecord, ObjectShape, OrdinaryObject,
        PrimitiveObject, PrimitiveObjectRecord, PromiseConstructor, PromisePrototype,
        ProxyConstructor, ReflectObject, StringConstructor, StringIteratorPrototype,
        StringPrototype, SymbolConstructor, SymbolPrototype,
    },
    engine::NoGcScope,
    heap::{
        CompactionLists, HeapMarkAndSweep, IntrinsicConstructorIndexes, IntrinsicFunctionIndexes,
        IntrinsicObjectIndexes, IntrinsicObjectShapes, IntrinsicPrimitiveObjectIndexes, WorkQueues,
        intrinsic_function_count, intrinsic_object_count, intrinsic_primitive_object_count,
        {BaseIndex, HeapIndexHandle},
    },
};
#[derive(Debug, Clone)]
pub(crate) struct Intrinsics {
    object_index_base: BaseIndex<'static, ObjectRecord<'static>>,
    object_shape_base: ObjectShape<'static>,
    primitive_object_index_base: BaseIndex<'static, PrimitiveObjectRecord<'static>>,
    /// Array prototype object is an Array exotic object. It is the only one
    /// in the ECMAScript spec so we do not need to store the Array index base.
    array_prototype: Array<'static>,
    builtin_function_index_base: BaseIndex<'static, BuiltinFunctionHeapData<'static>>,
}

/// Enumeration of intrinsics intended to be used as the \[\[Prototype\]\] value of
/// an object. Used in GetPrototypeFromConstructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtoIntrinsics {
    AggregateError,
    Array,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer,
    ArrayIterator,
    AsyncFunction,
    AsyncGenerator,
    AsyncGeneratorFunction,
    BigInt,
    #[cfg(feature = "array-buffer")]
    BigInt64Array,
    #[cfg(feature = "array-buffer")]
    BigUint64Array,
    Boolean,
    #[cfg(feature = "array-buffer")]
    DataView,
    #[cfg(feature = "shared-array-buffer")]
    SharedDataView,
    #[cfg(feature = "date")]
    Date,
    Error,
    EvalError,
    FinalizationRegistry,
    #[cfg(feature = "proposal-float16array")]
    Float16Array,
    #[cfg(feature = "array-buffer")]
    Float32Array,
    #[cfg(feature = "array-buffer")]
    Float64Array,
    Function,
    Generator,
    GeneratorFunction,
    #[cfg(feature = "array-buffer")]
    Int16Array,
    #[cfg(feature = "array-buffer")]
    Int32Array,
    #[cfg(feature = "array-buffer")]
    Int8Array,
    Iterator,
    Map,
    MapIterator,
    Number,
    Object,
    Promise,
    RangeError,
    ReferenceError,
    #[cfg(feature = "regexp")]
    RegExp,
    #[cfg(feature = "set")]
    Set,
    #[cfg(feature = "set")]
    SetIterator,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer,
    String,
    StringIterator,
    #[cfg(feature = "regexp")]
    RegExpStringIterator,
    Symbol,
    SyntaxError,
    #[cfg(feature = "temporal")]
    TemporalInstant,
    TypeError,
    #[cfg(feature = "array-buffer")]
    Uint16Array,
    #[cfg(feature = "array-buffer")]
    Uint32Array,
    #[cfg(feature = "array-buffer")]
    Uint8Array,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray,
    URIError,
    #[cfg(feature = "weak-refs")]
    WeakMap,
    #[cfg(feature = "weak-refs")]
    WeakRef,
    #[cfg(feature = "weak-refs")]
    WeakSet,
}

impl Intrinsics {
    pub(crate) fn new(agent: &mut Agent) -> Self {
        // Use from_usize to index "one over the edge", ie. where new intrinsics will be created.
        let object_index_base = BaseIndex::from_index(agent.heap.objects.len());
        let object_shape_base = ObjectShape::from_non_zero(
            NonZeroU32::new((agent.heap.object_shapes.len() + 1) as u32).unwrap(),
        );
        let primitive_object_index_base = BaseIndex::from_index(agent.heap.primitive_objects.len());
        let builtin_function_index_base = BaseIndex::from_index(agent.heap.builtin_functions.len());
        // SAFETY: we're creating the intrinsics.
        let array_prototype = unsafe { Array::next_array(agent) };

        agent
            .heap
            .objects
            .extend((0..intrinsic_object_count()).map(|_| ObjectRecord::BLANK));
        agent
            .heap
            .primitive_objects
            .extend((0..intrinsic_primitive_object_count()).map(|_| PrimitiveObjectRecord::BLANK));
        agent
            .heap
            .builtin_functions
            .extend((0..intrinsic_function_count()).map(|_| BuiltinFunctionHeapData::BLANK));
        agent
            .heap
            .arrays
            .push(Default::default())
            .expect("Failed to allocate");

        Self {
            object_index_base,
            object_shape_base,
            primitive_object_index_base,
            builtin_function_index_base,
            array_prototype,
        }
    }

    pub(crate) fn create_intrinsics(agent: &mut Agent, realm: Realm<'static>, gc: NoGcScope) {
        ObjectShape::create_intrinsic(agent, realm);
        GlobalObject::create_intrinsic(agent, realm);
        ObjectPrototype::create_intrinsic(agent, realm);
        ObjectConstructor::create_intrinsic(agent, realm);
        FunctionPrototype::create_intrinsic(agent, realm);
        FunctionConstructor::create_intrinsic(agent, realm);
        BooleanPrototype::create_intrinsic(agent, realm);
        BooleanConstructor::create_intrinsic(agent, realm);
        SymbolPrototype::create_intrinsic(agent, realm);
        SymbolConstructor::create_intrinsic(agent, realm);
        ErrorConstructor::create_intrinsic(agent, realm);
        ErrorPrototype::create_intrinsic(agent, realm);
        NativeErrorPrototypes::create_intrinsic(agent, realm);
        NativeErrorConstructors::create_intrinsic(agent, realm);
        AggregateErrorPrototype::create_intrinsic(agent, realm);
        AggregateErrorConstructor::create_intrinsic(agent, realm);
        NumberPrototype::create_intrinsic(agent, realm);
        NumberConstructor::create_intrinsic(agent, realm, gc);
        BigIntPrototype::create_intrinsic(agent, realm);
        BigIntConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "math")]
        MathObject::create_intrinsic(agent, realm, gc);

        #[cfg(feature = "temporal")]
        {
            TemporalObject::create_intrinsic(agent, realm, gc);
            // Instant
            InstantConstructor::create_intrinsic(agent, realm, gc);
            InstantPrototype::create_intrinsic(agent, realm, gc);
        }

        #[cfg(feature = "date")]
        DatePrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "date")]
        DateConstructor::create_intrinsic(agent, realm);
        StringPrototype::create_intrinsic(agent, realm);
        StringConstructor::create_intrinsic(agent, realm);
        StringIteratorPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "regexp")]
        RegExpPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "regexp")]
        RegExpConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "regexp")]
        RegExpStringIteratorPrototype::create_intrinsic(agent, realm);
        ArrayPrototype::create_intrinsic(agent, realm);
        ArrayConstructor::create_intrinsic(agent, realm);
        ArrayIteratorPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        TypedArrayPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        TypedArrayIntrinsicObject::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        TypedArrayPrototypes::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        TypedArrayConstructors::create_intrinsic(agent, realm);
        MapPrototype::create_intrinsic(agent, realm);
        MapConstructor::create_intrinsic(agent, realm);
        MapIteratorPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "set")]
        SetPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "set")]
        SetConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "set")]
        SetIteratorPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "weak-refs")]
        WeakMapPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "weak-refs")]
        WeakMapConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "weak-refs")]
        WeakSetPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "weak-refs")]
        WeakSetConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        ArrayBufferPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        ArrayBufferConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "shared-array-buffer")]
        SharedArrayBufferPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "shared-array-buffer")]
        SharedArrayBufferConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        DataViewPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "array-buffer")]
        DataViewConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "atomics")]
        AtomicsObject::create_intrinsic(agent, realm);
        #[cfg(feature = "json")]
        JSONObject::create_intrinsic(agent, realm);
        #[cfg(feature = "weak-refs")]
        WeakRefPrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "weak-refs")]
        WeakRefConstructor::create_intrinsic(agent, realm);
        FinalizationRegistryPrototype::create_intrinsic(agent, realm);
        FinalizationRegistryConstructor::create_intrinsic(agent, realm);
        IteratorPrototype::create_intrinsic(agent, realm);
        AsyncIteratorPrototype::create_intrinsic(agent, realm);
        PromisePrototype::create_intrinsic(agent, realm);
        PromiseConstructor::create_intrinsic(agent, realm);
        GeneratorFunctionPrototype::create_intrinsic(agent, realm);
        GeneratorFunctionConstructor::create_intrinsic(agent, realm);
        AsyncGeneratorFunctionPrototype::create_intrinsic(agent, realm);
        AsyncGeneratorFunctionConstructor::create_intrinsic(agent, realm);
        GeneratorPrototype::create_intrinsic(agent, realm);
        AsyncGeneratorPrototype::create_intrinsic(agent, realm);
        AsyncFunctionPrototype::create_intrinsic(agent, realm);
        AsyncFunctionConstructor::create_intrinsic(agent, realm);
        ReflectObject::create_intrinsic(agent, realm);
        ProxyConstructor::create_intrinsic(agent, realm);
        IteratorConstructor::create_intrinsic(agent, realm);
    }

    // Suggest to inline this: The intrinsic default proto is often statically
    // known.
    #[inline]
    pub(crate) fn get_intrinsic_default_constructor(
        &self,
        intrinsic_default_proto: ProtoIntrinsics,
    ) -> Function<'static> {
        match intrinsic_default_proto {
            ProtoIntrinsics::Array => self.array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::ArrayBuffer => self.array_buffer().into(),
            ProtoIntrinsics::ArrayIterator => unreachable!(),
            ProtoIntrinsics::BigInt => self.big_int().into(),
            ProtoIntrinsics::Boolean => self.boolean().into(),
            ProtoIntrinsics::Error => self.error().into(),
            #[cfg(feature = "date")]
            ProtoIntrinsics::Date => self.date().into(),
            ProtoIntrinsics::EvalError => self.eval_error().into(),
            ProtoIntrinsics::Function => self.function().into(),
            ProtoIntrinsics::Number => self.number().into(),
            ProtoIntrinsics::Object => self.object().into(),
            ProtoIntrinsics::RangeError => self.range_error().into(),
            ProtoIntrinsics::ReferenceError => self.reference_error().into(),
            ProtoIntrinsics::StringIterator => unreachable!(),
            #[cfg(feature = "regexp")]
            ProtoIntrinsics::RegExpStringIterator => unreachable!(),
            ProtoIntrinsics::String => self.string().into(),
            ProtoIntrinsics::Symbol => self.symbol().into(),
            ProtoIntrinsics::SyntaxError => self.syntax_error().into(),
            ProtoIntrinsics::TemporalInstant => self.temporal_instant().into(),
            ProtoIntrinsics::TypeError => self.type_error().into(),
            ProtoIntrinsics::URIError => self.uri_error().into(),
            ProtoIntrinsics::AggregateError => self.aggregate_error().into(),
            ProtoIntrinsics::AsyncFunction => self.async_function().into(),
            ProtoIntrinsics::AsyncGenerator => self.async_generator_function().into(),
            ProtoIntrinsics::AsyncGeneratorFunction => self.async_generator_function().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigInt64Array => self.big_int64_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigUint64Array => self.big_uint64_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::DataView => self.data_view().into(),
            #[cfg(feature = "shared-array-buffer")]
            ProtoIntrinsics::SharedDataView => self.data_view().into(),
            ProtoIntrinsics::FinalizationRegistry => self.finalization_registry().into(),
            #[cfg(feature = "proposal-float16array")]
            ProtoIntrinsics::Float16Array => self.float16_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Float32Array => self.float32_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Float64Array => self.float64_array().into(),
            ProtoIntrinsics::Generator => self.generator_function().into(),
            ProtoIntrinsics::GeneratorFunction => self.generator_function().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int16Array => self.int16_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int32Array => self.int32_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int8Array => self.int8_array().into(),
            ProtoIntrinsics::Iterator => self.iterator().into(),
            ProtoIntrinsics::Map => self.map().into(),
            ProtoIntrinsics::MapIterator => unreachable!(),
            ProtoIntrinsics::Promise => self.promise().into(),
            #[cfg(feature = "regexp")]
            ProtoIntrinsics::RegExp => self.reg_exp().into(),
            #[cfg(feature = "set")]
            ProtoIntrinsics::Set => self.set().into(),
            #[cfg(feature = "set")]
            ProtoIntrinsics::SetIterator => unreachable!(),
            #[cfg(feature = "shared-array-buffer")]
            ProtoIntrinsics::SharedArrayBuffer => self.shared_array_buffer().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint16Array => self.uint16_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint32Array => self.uint32_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint8Array => self.uint8_array().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint8ClampedArray => self.uint8_clamped_array().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakMap => self.weak_map().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakRef => self.weak_ref().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakSet => self.weak_set().into(),
        }
    }

    // Suggest to inline this: The intrinsic default proto is often statically
    // known.
    #[inline]
    pub(crate) fn get_intrinsic_default_proto(
        &self,
        intrinsic_default_proto: ProtoIntrinsics,
    ) -> Object<'static> {
        match intrinsic_default_proto {
            ProtoIntrinsics::Array => self.array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::ArrayBuffer => self.array_buffer_prototype().into(),
            ProtoIntrinsics::ArrayIterator => self.array_iterator_prototype().into(),
            ProtoIntrinsics::BigInt => self.big_int_prototype().into(),
            ProtoIntrinsics::Boolean => self.boolean_prototype().into(),
            ProtoIntrinsics::Error => self.error_prototype().into(),
            #[cfg(feature = "date")]
            ProtoIntrinsics::Date => self.date_prototype().into(),
            ProtoIntrinsics::EvalError => self.eval_error_prototype().into(),
            ProtoIntrinsics::Function => self.function_prototype().into(),
            ProtoIntrinsics::Number => self.number_prototype().into(),
            ProtoIntrinsics::Object => self.object_prototype().into(),
            ProtoIntrinsics::RangeError => self.range_error_prototype().into(),
            ProtoIntrinsics::ReferenceError => self.reference_error_prototype().into(),
            ProtoIntrinsics::StringIterator => self.string_iterator_prototype().into(),
            #[cfg(feature = "regexp")]
            ProtoIntrinsics::RegExpStringIterator => {
                self.reg_exp_string_iterator_prototype().into()
            }
            ProtoIntrinsics::String => self.string_prototype().into(),
            ProtoIntrinsics::Symbol => self.symbol_prototype().into(),
            ProtoIntrinsics::SyntaxError => self.syntax_error_prototype().into(),
            ProtoIntrinsics::TemporalInstant => self.temporal().into(),
            ProtoIntrinsics::TypeError => self.type_error_prototype().into(),
            ProtoIntrinsics::URIError => self.uri_error_prototype().into(),
            ProtoIntrinsics::AggregateError => self.aggregate_error_prototype().into(),
            ProtoIntrinsics::AsyncFunction => self.async_function_prototype().into(),
            ProtoIntrinsics::AsyncGenerator => self.async_generator_prototype().into(),
            ProtoIntrinsics::AsyncGeneratorFunction => {
                self.async_generator_function_prototype().into()
            }
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigInt64Array => self.big_int64_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigUint64Array => self.big_uint64_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::DataView => self.data_view_prototype().into(),
            #[cfg(feature = "shared-array-buffer")]
            ProtoIntrinsics::SharedDataView => self.data_view_prototype().into(),
            ProtoIntrinsics::FinalizationRegistry => self.finalization_registry_prototype().into(),
            #[cfg(feature = "proposal-float16array")]
            ProtoIntrinsics::Float16Array => self.float16_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Float32Array => self.float32_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Float64Array => self.float64_array_prototype().into(),
            ProtoIntrinsics::Generator => self.generator_prototype().into(),
            ProtoIntrinsics::GeneratorFunction => self.generator_function_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int16Array => self.int16_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int32Array => self.int32_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int8Array => self.int8_array_prototype().into(),
            ProtoIntrinsics::Iterator => self.iterator_prototype().into(),
            ProtoIntrinsics::Map => self.map_prototype().into(),
            ProtoIntrinsics::MapIterator => self.map_iterator_prototype().into(),
            ProtoIntrinsics::Promise => self.promise_prototype().into(),
            #[cfg(feature = "regexp")]
            ProtoIntrinsics::RegExp => self.reg_exp_prototype().into(),
            #[cfg(feature = "set")]
            ProtoIntrinsics::Set => self.set_prototype().into(),
            #[cfg(feature = "set")]
            ProtoIntrinsics::SetIterator => self.set_iterator_prototype().into(),
            #[cfg(feature = "shared-array-buffer")]
            ProtoIntrinsics::SharedArrayBuffer => self.shared_array_buffer_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint16Array => self.uint16_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint32Array => self.uint32_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint8Array => self.uint8_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint8ClampedArray => self.uint8_clamped_array_prototype().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakMap => self.weak_map_prototype().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakRef => self.weak_ref_prototype().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakSet => self.weak_set_prototype().into(),
        }
    }

    pub(crate) const fn get_intrinsic_object_shape(
        &self,
        intrinsic_default_proto: ProtoIntrinsics,
    ) -> Option<ObjectShape<'static>> {
        match intrinsic_default_proto {
            ProtoIntrinsics::Array => Some(self.array_shape()),
            ProtoIntrinsics::Number => Some(self.number_shape()),
            ProtoIntrinsics::Object => Some(self.object_shape()),
            ProtoIntrinsics::String => Some(self.string_shape()),
            _ => None,
        }
    }

    pub(crate) const fn intrinsic_function_index_to_builtin_function(
        &self,
        index: IntrinsicFunctionIndexes,
    ) -> BuiltinFunction<'static> {
        index.get_builtin_function(self.builtin_function_index_base)
    }

    pub(crate) const fn intrinsic_constructor_index_to_builtin_function(
        &self,
        index: IntrinsicConstructorIndexes,
    ) -> BuiltinFunction<'static> {
        index.get_builtin_function(self.builtin_function_index_base)
    }

    pub(crate) fn get_intrinsic_constructor_backing_object(
        &self,
        index: IntrinsicConstructorIndexes,
    ) -> OrdinaryObject<'static> {
        index.get_backing_object(self.object_index_base)
    }

    /// %AggregateError.prototype%
    pub(crate) const fn aggregate_error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::AggregateErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %AggregateError%
    pub(crate) const fn aggregate_error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::AggregateError
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Array.prototype.sort%
    pub(crate) const fn array_prototype_sort(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::ArrayPrototypeSort
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Array.prototype.toString%
    pub(crate) const fn array_prototype_to_string(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::ArrayPrototypeToString
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Array.prototype.values%
    pub(crate) const fn array_prototype_values(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::ArrayPrototypeValues
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Array.prototype%
    pub(crate) const fn array_prototype(&self) -> Array<'static> {
        self.array_prototype
    }

    /// %Array.prototype%
    pub(crate) const fn array_prototype_backing_object(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::ArrayPrototype.get_backing_object(self.object_index_base)
    }

    /// %Array%
    pub(crate) const fn array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Array.get_builtin_function(self.builtin_function_index_base)
    }

    /// Empty Array shape.
    pub(crate) const fn array_shape(&self) -> ObjectShape<'static> {
        IntrinsicObjectShapes::Array.get_object_shape_index(self.object_shape_base)
    }

    #[cfg(feature = "array-buffer")]
    /// %ArrayBuffer.prototype%
    pub(crate) const fn array_buffer_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::ArrayBufferPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    /// %ArrayBuffer%
    pub(crate) const fn array_buffer(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::ArrayBuffer
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %ArrayIteratorPrototype%
    pub(crate) const fn array_iterator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::ArrayIteratorPrototype.get_backing_object(self.object_index_base)
    }

    /// %AsyncFunction.prototype%
    pub(crate) const fn async_function_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::AsyncFunctionPrototype.get_backing_object(self.object_index_base)
    }

    /// %AsyncFunction%
    pub(crate) const fn async_function(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::AsyncFunction
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %AsyncGeneratorFunction.prototype%
    pub(crate) const fn async_generator_function_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::AsyncGeneratorFunctionPrototype
            .get_backing_object(self.object_index_base)
    }

    /// %AsyncGeneratorFunction%
    pub(crate) const fn async_generator_function(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::AsyncGeneratorFunction
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %AsyncGeneratorPrototype%
    pub(crate) const fn async_generator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::AsyncGeneratorPrototype.get_backing_object(self.object_index_base)
    }

    /// %AsyncIteratorPrototype%
    pub(crate) const fn async_iterator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::AsyncIteratorPrototype.get_backing_object(self.object_index_base)
    }

    /// %Atomics%
    #[cfg(feature = "atomics")]
    pub(crate) const fn atomics(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::AtomicsObject.get_backing_object(self.object_index_base)
    }

    /// %BigInt.prototype%
    pub(crate) const fn big_int_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::BigIntPrototype.get_backing_object(self.object_index_base)
    }

    /// %BigInt%
    pub(crate) const fn big_int(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::BigInt.get_builtin_function(self.builtin_function_index_base)
    }

    /// %BigInt64Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn big_int64_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::BigInt64ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn big_int64_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::BigInt64Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %BigUint64Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn big_uint64_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::BigUint64ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn big_uint64_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::BigUint64Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Boolean.prototype%
    pub(crate) fn boolean_prototype(&self) -> PrimitiveObject<'static> {
        IntrinsicPrimitiveObjectIndexes::BooleanPrototype
            .get_primitive_object(self.primitive_object_index_base)
    }

    pub(crate) fn boolean_prototype_backing_object(&self) -> OrdinaryObject<'static> {
        IntrinsicPrimitiveObjectIndexes::BooleanPrototype.get_backing_object(self.object_index_base)
    }

    /// %Boolean%
    pub(crate) const fn boolean(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Boolean.get_builtin_function(self.builtin_function_index_base)
    }

    /// %DataView.prototype%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn data_view_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::DataViewPrototype.get_backing_object(self.object_index_base)
    }

    /// %DataView%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn data_view(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::DataView.get_builtin_function(self.builtin_function_index_base)
    }

    #[cfg(feature = "date")]
    /// %Date.prototype.toUTCString%
    pub(crate) const fn date_prototype_to_utcstring(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::DatePrototypeToUTCString
            .get_builtin_function(self.builtin_function_index_base)
    }

    #[cfg(feature = "date")]
    /// %Date.prototype%
    pub(crate) const fn date_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::DatePrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "date")]
    /// %Date%
    pub(crate) const fn date(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Date.get_builtin_function(self.builtin_function_index_base)
    }

    /// %decodeURI%
    pub(crate) const fn decode_uri(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::DecodeURI.get_builtin_function(self.builtin_function_index_base)
    }

    /// %decodeURIComponent%
    pub(crate) const fn decode_uri_component(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::DecodeURIComponent
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %encodeURI%
    pub(crate) const fn encode_uri(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::EncodeURI.get_builtin_function(self.builtin_function_index_base)
    }

    /// %encodeURIComponent%
    pub(crate) const fn encode_uri_component(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::EncodeURIComponent
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Error.prototype%
    pub(crate) const fn error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::ErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %Error%
    pub(crate) const fn error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Error.get_builtin_function(self.builtin_function_index_base)
    }

    /// %escape%
    #[cfg(feature = "annex-b-global")]
    pub(crate) const fn escape(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::Escape.get_builtin_function(self.builtin_function_index_base)
    }

    /// %eval%
    pub(crate) const fn eval(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::Eval.get_builtin_function(self.builtin_function_index_base)
    }

    /// %EvalError.prototype%
    pub(crate) const fn eval_error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::EvalErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %EvalError%
    pub(crate) const fn eval_error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::EvalError
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %FinalizationRegistry.prototype%
    pub(crate) const fn finalization_registry_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::FinalizationRegistryPrototype
            .get_backing_object(self.object_index_base)
    }

    /// %FinalizationRegistry%
    pub(crate) const fn finalization_registry(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::FinalizationRegistry
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Float16Array%
    #[cfg(feature = "proposal-float16array")]
    pub(crate) const fn float16_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Float16ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "proposal-float16array")]
    pub(crate) const fn float16_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Float16Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Float32Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn float32_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Float32ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn float32_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Float32Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Float64Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn float64_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Float64ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn float64_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Float64Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    pub(crate) const fn function_prototype(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::FunctionPrototype
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Function%
    pub(crate) const fn function(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Function.get_builtin_function(self.builtin_function_index_base)
    }

    /// %GeneratorFunction.prototype.prototype.next%
    pub(crate) const fn generator_function_prototype_prototype_next(
        &self,
    ) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::GeneratorFunctionPrototypePrototypeNext
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %GeneratorFunction.prototype%
    pub(crate) const fn generator_function_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::GeneratorFunctionPrototype
            .get_backing_object(self.object_index_base)
    }

    /// %GeneratorFunction%
    pub(crate) const fn generator_function(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::GeneratorFunction
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %GeneratorPrototype%
    pub(crate) const fn generator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::GeneratorPrototype.get_backing_object(self.object_index_base)
    }

    /// %Int16Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn int16_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Int16ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn int16_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Int16Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Int32Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn int32_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Int32ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn int32_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Int32Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Int8Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn int8_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Int8ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn int8_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Int8Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %isFinite%
    pub(crate) const fn is_finite(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::IsFinite.get_builtin_function(self.builtin_function_index_base)
    }

    /// %isNaN%
    pub(crate) const fn is_nan(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::IsNaN.get_builtin_function(self.builtin_function_index_base)
    }

    /// %Iterator%
    pub(crate) const fn iterator(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Iterator.get_builtin_function(self.builtin_function_index_base)
    }

    /// %IteratorPrototype%
    pub(crate) const fn iterator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::IteratorPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "json")]
    /// %JSON%
    pub(crate) const fn json(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::JSONObject.get_backing_object(self.object_index_base)
    }

    /// %Map.prototype.entries%
    pub(crate) const fn map_prototype_entries(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::MapPrototypeEntries
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Map.prototype%
    pub(crate) const fn map_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::MapPrototype.get_backing_object(self.object_index_base)
    }

    /// %Map%
    pub(crate) const fn map(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Map.get_builtin_function(self.builtin_function_index_base)
    }

    /// %MapIteratorPrototype%
    pub(crate) const fn map_iterator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::MapIteratorPrototype.get_backing_object(self.object_index_base)
    }

    /// %Math%
    #[cfg(feature = "math")]
    pub(crate) const fn math(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::MathObject.get_backing_object(self.object_index_base)
    }

    /// %Temporal%
    pub(crate) const fn temporal(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::TemporalObject.get_backing_object(self.object_index_base)
    }

    /// %Temporal.Instant%
    pub(crate) const fn temporal_instant(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::TemporalInstant
            .get_builtin_function(self.builtin_function_index_base)
    }
    /// %Temporal.Instant.Prototype%
    pub(crate) const fn temporal_instant_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::TemporalInstantPrototype.get_backing_object(self.object_index_base)
    }

    /// %Number.prototype%
    pub(crate) fn number_prototype(&self) -> PrimitiveObject<'static> {
        IntrinsicPrimitiveObjectIndexes::NumberPrototype
            .get_primitive_object(self.primitive_object_index_base)
    }

    pub(crate) fn number_prototype_backing_object(&self) -> OrdinaryObject<'static> {
        IntrinsicPrimitiveObjectIndexes::NumberPrototype.get_backing_object(self.object_index_base)
    }

    /// %Number%
    pub(crate) const fn number(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Number.get_builtin_function(self.builtin_function_index_base)
    }

    /// Empty Number shape.
    pub(crate) const fn number_shape(&self) -> ObjectShape<'static> {
        IntrinsicObjectShapes::Number.get_object_shape_index(self.object_shape_base)
    }

    /// %Object.prototype.toString%
    pub(crate) const fn object_prototype_to_string(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::ObjectPrototypeToString
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Object.prototype%
    pub(crate) const fn object_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::ObjectPrototype.get_backing_object(self.object_index_base)
    }

    /// %Object%
    pub(crate) const fn object(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Object.get_builtin_function(self.builtin_function_index_base)
    }

    /// Empty Object shape.
    pub(crate) const fn object_shape(&self) -> ObjectShape<'static> {
        IntrinsicObjectShapes::Object.get_object_shape_index(self.object_shape_base)
    }

    /// %parseFloat%
    pub(crate) const fn parse_float(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::ParseFloat.get_builtin_function(self.builtin_function_index_base)
    }

    /// %parseInt%
    pub(crate) const fn parse_int(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::ParseInt.get_builtin_function(self.builtin_function_index_base)
    }

    /// %Promise.prototype%
    pub(crate) const fn promise_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::PromisePrototype.get_backing_object(self.object_index_base)
    }

    /// %Promise%
    pub(crate) const fn promise(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Promise.get_builtin_function(self.builtin_function_index_base)
    }

    /// %Proxy%
    pub(crate) const fn proxy(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Proxy.get_builtin_function(self.builtin_function_index_base)
    }

    /// %RangeError.prototype%
    pub(crate) const fn range_error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::RangeErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %RangeError%
    pub(crate) const fn range_error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::RangeError
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %ReferenceError.prototype%
    pub(crate) const fn reference_error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::ReferenceErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %ReferenceError%
    pub(crate) const fn reference_error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::ReferenceError
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Reflect%
    pub(crate) const fn reflect(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::ReflectObject.get_backing_object(self.object_index_base)
    }

    /// %RegExp.prototype.exec%
    #[cfg(feature = "regexp")]
    pub(crate) const fn reg_exp_prototype_exec(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::RegExpPrototypeExec
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %RegExp.prototype%
    #[cfg(feature = "regexp")]
    pub(crate) const fn reg_exp_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::RegExpPrototype.get_backing_object(self.object_index_base)
    }

    /// %RegExp%
    #[cfg(feature = "regexp")]
    pub(crate) const fn reg_exp(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::RegExp.get_builtin_function(self.builtin_function_index_base)
    }

    /// %RegExpStringIteratorPrototype%
    #[cfg(feature = "regexp")]
    pub(crate) const fn reg_exp_string_iterator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::RegExpStringIteratorPrototype
            .get_backing_object(self.object_index_base)
    }

    /// %Set.prototype.values%
    #[cfg(feature = "set")]
    pub(crate) const fn set_prototype_values(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::SetPrototypeValues
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Set.prototype%
    #[cfg(feature = "set")]
    pub(crate) const fn set_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::SetPrototype.get_backing_object(self.object_index_base)
    }

    /// %Set%
    #[cfg(feature = "set")]
    pub(crate) const fn set(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Set.get_builtin_function(self.builtin_function_index_base)
    }

    /// %SetIteratorPrototype%
    #[cfg(feature = "set")]
    pub(crate) const fn set_iterator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::SetIteratorPrototype.get_backing_object(self.object_index_base)
    }

    /// %SharedArrayBuffer.prototype%
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) const fn shared_array_buffer_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::SharedArrayBufferPrototype
            .get_backing_object(self.object_index_base)
    }

    /// %SharedArrayBuffer%
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) const fn shared_array_buffer(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::SharedArrayBuffer
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %String.prototype.trimEnd%
    pub(crate) const fn string_prototype_trim_end(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::StringPrototypeTrimEnd
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %String.prototype.trimStart%
    pub(crate) const fn string_prototype_trim_start(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::StringPrototypeTrimStart
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %String.prototype%
    pub(crate) fn string_prototype(&self) -> PrimitiveObject<'static> {
        IntrinsicPrimitiveObjectIndexes::StringPrototype
            .get_primitive_object(self.primitive_object_index_base)
    }

    pub(crate) fn string_prototype_backing_object(&self) -> OrdinaryObject<'static> {
        IntrinsicPrimitiveObjectIndexes::StringPrototype.get_backing_object(self.object_index_base)
    }

    /// %String%
    pub(crate) const fn string(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::String.get_builtin_function(self.builtin_function_index_base)
    }

    /// Empty String shape
    pub(crate) const fn string_shape(&self) -> ObjectShape<'static> {
        IntrinsicObjectShapes::String.get_object_shape_index(self.object_shape_base)
    }

    /// %StringIteratorPrototype%
    pub(crate) const fn string_iterator_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::StringIteratorPrototype.get_backing_object(self.object_index_base)
    }

    /// %Symbol.prototype%
    pub(crate) const fn symbol_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::SymbolPrototype.get_backing_object(self.object_index_base)
    }

    /// %Symbol%
    pub(crate) const fn symbol(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Symbol.get_builtin_function(self.builtin_function_index_base)
    }

    /// %SyntaxError.prototype%
    pub(crate) const fn syntax_error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::SyntaxErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %SyntaxError%
    pub(crate) const fn syntax_error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::SyntaxError
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %ThrowTypeError%
    pub(crate) const fn throw_type_error(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::ThrowTypeError
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %TypedArray.prototype.values%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn typed_array_prototype_values(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::TypedArrayPrototypeValues
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %TypedArray.prototype%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn typed_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::TypedArrayPrototype.get_backing_object(self.object_index_base)
    }

    /// %TypedArray%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn typed_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::TypedArray
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %TypeError.prototype%
    pub(crate) const fn type_error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::TypeErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %TypeError%
    pub(crate) const fn type_error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::TypeError
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Uint16Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint16_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Uint16ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint16_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Uint16Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Uint32Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint32_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Uint32ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint32_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Uint32Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Uint8Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint8_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Uint8ArrayPrototype.get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint8_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Uint8Array
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %Uint8ClampedArray%
    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint8_clamped_array_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::Uint8ClampedArrayPrototype
            .get_backing_object(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) const fn uint8_clamped_array(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::Uint8ClampedArray
            .get_builtin_function(self.builtin_function_index_base)
    }

    /// %unescape%
    #[cfg(feature = "annex-b-global")]
    pub(crate) const fn unescape(&self) -> BuiltinFunction<'static> {
        IntrinsicFunctionIndexes::Unescape.get_builtin_function(self.builtin_function_index_base)
    }

    /// %URIError.prototype%
    pub(crate) const fn uri_error_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::URIErrorPrototype.get_backing_object(self.object_index_base)
    }

    /// %URIError%
    pub(crate) const fn uri_error(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::URIError.get_builtin_function(self.builtin_function_index_base)
    }

    /// %WeakMap.prototype%
    #[cfg(feature = "weak-refs")]
    pub(crate) const fn weak_map_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::WeakMapPrototype.get_backing_object(self.object_index_base)
    }

    /// %WeakMap%
    #[cfg(feature = "weak-refs")]
    pub(crate) const fn weak_map(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::WeakMap.get_builtin_function(self.builtin_function_index_base)
    }

    /// %WeakRef.prototype%
    #[cfg(feature = "weak-refs")]
    pub(crate) const fn weak_ref_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::WeakRefPrototype.get_backing_object(self.object_index_base)
    }

    /// %WeakRef%
    #[cfg(feature = "weak-refs")]
    pub(crate) const fn weak_ref(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::WeakRef.get_builtin_function(self.builtin_function_index_base)
    }

    /// %WeakSet.prototype%
    #[cfg(feature = "weak-refs")]
    pub(crate) const fn weak_set_prototype(&self) -> OrdinaryObject<'static> {
        IntrinsicObjectIndexes::WeakSetPrototype.get_backing_object(self.object_index_base)
    }

    /// %WeakSet%
    #[cfg(feature = "weak-refs")]
    pub(crate) const fn weak_set(&self) -> BuiltinFunction<'static> {
        IntrinsicConstructorIndexes::WeakSet.get_builtin_function(self.builtin_function_index_base)
    }
}

impl HeapMarkAndSweep for Intrinsics {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.aggregate_error_prototype().mark_values(queues);
        self.aggregate_error().mark_values(queues);
        self.array_prototype_sort().mark_values(queues);
        self.array_prototype_to_string().mark_values(queues);
        self.array_prototype_values().mark_values(queues);
        self.array_prototype().mark_values(queues);
        self.array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.array_buffer_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.array_buffer().mark_values(queues);
        self.array_iterator_prototype().mark_values(queues);
        self.async_function_prototype().mark_values(queues);
        self.async_function().mark_values(queues);
        self.async_generator_function_prototype()
            .mark_values(queues);
        self.async_generator_function().mark_values(queues);
        self.async_generator_prototype().mark_values(queues);
        self.async_iterator_prototype().mark_values(queues);
        #[cfg(feature = "atomics")]
        self.atomics().mark_values(queues);
        self.big_int_prototype().mark_values(queues);
        self.big_int().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.big_int64_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.big_int64_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.big_uint64_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.big_uint64_array_prototype().mark_values(queues);
        self.boolean_prototype().mark_values(queues);
        self.boolean().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.data_view_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.data_view().mark_values(queues);
        #[cfg(feature = "date")]
        self.date_prototype_to_utcstring().mark_values(queues);
        #[cfg(feature = "date")]
        self.date_prototype().mark_values(queues);
        #[cfg(feature = "date")]
        self.date().mark_values(queues);
        self.decode_uri().mark_values(queues);
        self.decode_uri_component().mark_values(queues);
        self.encode_uri().mark_values(queues);
        self.encode_uri_component().mark_values(queues);
        self.error_prototype().mark_values(queues);
        self.error().mark_values(queues);
        #[cfg(feature = "annex-b-global")]
        self.escape().mark_values(queues);
        self.eval().mark_values(queues);
        self.eval_error_prototype().mark_values(queues);
        self.eval_error().mark_values(queues);
        self.finalization_registry_prototype().mark_values(queues);
        self.finalization_registry().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.float32_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.float32_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.float64_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.float64_array_prototype().mark_values(queues);
        self.function_prototype().mark_values(queues);
        self.function().mark_values(queues);
        self.generator_function_prototype_prototype_next()
            .mark_values(queues);
        self.generator_function_prototype().mark_values(queues);
        self.generator_function().mark_values(queues);
        self.generator_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.int16_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.int16_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.int32_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.int32_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.int8_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.int8_array_prototype().mark_values(queues);
        self.is_finite().mark_values(queues);
        self.is_nan().mark_values(queues);
        self.iterator_prototype().mark_values(queues);
        #[cfg(feature = "json")]
        self.json().mark_values(queues);
        self.map_prototype_entries().mark_values(queues);
        self.map_prototype().mark_values(queues);
        self.map().mark_values(queues);
        self.map_iterator_prototype().mark_values(queues);
        #[cfg(feature = "math")]
        self.math().mark_values(queues);
        self.number_prototype().mark_values(queues);
        self.number().mark_values(queues);
        self.object_prototype_to_string().mark_values(queues);
        self.object_prototype().mark_values(queues);
        self.object().mark_values(queues);
        self.parse_float().mark_values(queues);
        self.parse_int().mark_values(queues);
        self.promise_prototype().mark_values(queues);
        self.promise().mark_values(queues);
        self.proxy().mark_values(queues);
        self.range_error_prototype().mark_values(queues);
        self.range_error().mark_values(queues);
        self.reference_error_prototype().mark_values(queues);
        self.reference_error().mark_values(queues);
        self.reflect().mark_values(queues);
        #[cfg(feature = "regexp")]
        self.reg_exp_prototype_exec().mark_values(queues);
        #[cfg(feature = "regexp")]
        self.reg_exp_prototype().mark_values(queues);
        #[cfg(feature = "regexp")]
        self.reg_exp().mark_values(queues);
        #[cfg(feature = "regexp")]
        self.reg_exp_string_iterator_prototype().mark_values(queues);
        #[cfg(feature = "set")]
        self.set_prototype_values().mark_values(queues);
        #[cfg(feature = "set")]
        self.set_prototype().mark_values(queues);
        #[cfg(feature = "set")]
        self.set().mark_values(queues);
        #[cfg(feature = "set")]
        self.set_iterator_prototype().mark_values(queues);
        #[cfg(feature = "shared-array-buffer")]
        self.shared_array_buffer_prototype().mark_values(queues);
        #[cfg(feature = "shared-array-buffer")]
        self.shared_array_buffer().mark_values(queues);
        self.string_prototype_trim_end().mark_values(queues);
        self.string_prototype_trim_start().mark_values(queues);
        self.string_prototype().mark_values(queues);
        self.string().mark_values(queues);
        self.string_iterator_prototype().mark_values(queues);
        self.symbol_prototype().mark_values(queues);
        self.symbol().mark_values(queues);
        self.syntax_error_prototype().mark_values(queues);
        self.syntax_error().mark_values(queues);
        self.throw_type_error().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.typed_array_prototype_values().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.typed_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.typed_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.typed_array_prototype().mark_values(queues);
        self.type_error_prototype().mark_values(queues);
        self.type_error().mark_values(queues);
        self.type_error_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint16_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint16_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint32_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint32_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint8_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint8_array_prototype().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint8_clamped_array().mark_values(queues);
        #[cfg(feature = "array-buffer")]
        self.uint8_clamped_array_prototype().mark_values(queues);
        #[cfg(feature = "annex-b-global")]
        self.unescape().mark_values(queues);
        self.uri_error_prototype().mark_values(queues);
        self.uri_error().mark_values(queues);
        #[cfg(feature = "weak-refs")]
        self.weak_map_prototype().mark_values(queues);
        #[cfg(feature = "weak-refs")]
        self.weak_map().mark_values(queues);
        #[cfg(feature = "weak-refs")]
        self.weak_ref_prototype().mark_values(queues);
        #[cfg(feature = "weak-refs")]
        self.weak_ref().mark_values(queues);
        #[cfg(feature = "weak-refs")]
        self.weak_set_prototype().mark_values(queues);
        #[cfg(feature = "weak-refs")]
        self.weak_set().mark_values(queues);
        IntrinsicObjectShapes::Object
            .get_object_shape_index(self.object_shape_base)
            .mark_values(queues);
        IntrinsicObjectShapes::Number
            .get_object_shape_index(self.object_shape_base)
            .mark_values(queues);
        IntrinsicObjectShapes::String
            .get_object_shape_index(self.object_shape_base)
            .mark_values(queues);
        IntrinsicObjectShapes::Array
            .get_object_shape_index(self.object_shape_base)
            .mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index_base,
            object_shape_base,
            primitive_object_index_base,
            array_prototype,
            builtin_function_index_base,
        } = self;
        compactions.objects.shift_index(object_index_base);
        object_shape_base.sweep_values(compactions);
        compactions
            .primitive_objects
            .shift_index(primitive_object_index_base);
        array_prototype.sweep_values(compactions);
        compactions
            .builtin_functions
            .shift_index(builtin_function_index_base);
    }
}
