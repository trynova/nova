// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::RealmIdentifier;
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::keyed_collections::{
    weak_map_objects::{
        weak_map_constructor::WeakMapConstructor, weak_map_prototype::WeakMapPrototype,
    },
    weak_set_objects::{
        weak_set_constructor::WeakSetConstructor, weak_set_prototype::WeakSetPrototype,
    },
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::managing_memory::weak_ref_objects::{
    weak_ref_constructor::WeakRefConstructor, weak_ref_prototype::WeakRefPrototype,
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::numbers_and_dates::date_objects::{
    date_constructor::DateConstructor, date_prototype::DatePrototype,
};
#[cfg(feature = "math")]
use crate::ecmascript::builtins::numbers_and_dates::math_object::MathObject;
#[cfg(feature = "atomics")]
use crate::ecmascript::builtins::structured_data::atomics_object::AtomicsObject;
#[cfg(feature = "json")]
use crate::ecmascript::builtins::structured_data::json_object::JSONObject;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::structured_data::shared_array_buffer_objects::{
    shared_array_buffer_constructor::SharedArrayBufferConstructor,
    shared_array_buffer_prototype::SharedArrayBufferPrototype,
};
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{
    indexed_collections::typed_array_objects::{
        typed_array_constructors::{TypedArrayConstructors, TypedArrayPrototypes},
        typed_array_intrinsic_object::{TypedArrayIntrinsicObject, TypedArrayPrototype},
    },
    structured_data::{
        array_buffer_objects::{
            array_buffer_constructor::ArrayBufferConstructor,
            array_buffer_prototype::ArrayBufferPrototype,
        },
        data_view_objects::{
            data_view_constructor::DataViewConstructor, data_view_prototype::DataViewPrototype,
        },
    },
};
use crate::{
    ecmascript::{
        builtins::{
            control_abstraction_objects::{
                async_function_objects::{
                    async_function_constructor::AsyncFunctionConstructor,
                    async_function_prototype::AsyncFunctionPrototype,
                },
                async_generator_function_objects::{
                    async_generator_function_constructor::AsyncGeneratorFunctionConstructor,
                    async_generator_function_prototype::AsyncGeneratorFunctionPrototype,
                },
                async_generator_objects::AsyncGeneratorPrototype,
                generator_function_objects::{
                    generator_function_constructor::GeneratorFunctionConstructor,
                    generator_function_prototype::GeneratorFunctionPrototype,
                },
                generator_prototype::GeneratorPrototype,
                iteration::{
                    async_from_sync_iterator_prototype::AsyncFromSyncIteratorPrototype,
                    async_iterator_prototype::AsyncIteratorPrototype,
                    iterator_prototype::IteratorPrototype,
                },
                promise_objects::{
                    promise_constructor::PromiseConstructor, promise_prototype::PromisePrototype,
                },
            },
            global_object::GlobalObject,
            indexed_collections::array_objects::{
                array_constructor::ArrayConstructor,
                array_iterator_objects::array_iterator_prototype::ArrayIteratorPrototype,
                array_prototype::ArrayPrototype,
            },
            keyed_collections::{
                map_objects::{
                    map_constructor::MapConstructor,
                    map_iterator_objects::map_iterator_prototype::MapIteratorPrototype,
                    map_prototype::MapPrototype,
                },
                set_objects::{
                    set_constructor::SetConstructor,
                    set_iterator_objects::set_iterator_prototype::SetIteratorPrototype,
                    set_prototype::SetPrototype,
                },
            },
            managing_memory::finalization_registry_objects::{
                finalization_registry_constructor::FinalizationRegistryConstructor,
                finalization_registry_prototype::FinalizationRegistryPrototype,
            },
            primitive_objects::PrimitiveObject,
            reflection::{proxy_constructor::ProxyConstructor, reflect_object::ReflectObject},
            text_processing::{
                regexp_objects::{
                    regexp_constructor::RegExpConstructor, regexp_prototype::RegExpPrototype,
                    regexp_string_iterator_prototype::RegExpStringIteratorPrototype,
                },
                string_objects::{
                    string_constructor::StringConstructor,
                    string_iterator_objects::StringIteratorPrototype,
                    string_prototype::StringPrototype,
                },
            },
            Array, BuiltinFunction,
        },
        execution::Agent,
        fundamental_objects::{
            boolean_objects::{
                boolean_constructor::BooleanConstructor, boolean_prototype::BooleanPrototype,
            },
            error_objects::{
                aggregate_error_constructors::AggregateErrorConstructor,
                aggregate_error_prototypes::AggregateErrorPrototype,
                error_constructor::ErrorConstructor, error_prototype::ErrorPrototype,
                native_error_constructors::NativeErrorConstructors,
                native_error_prototypes::NativeErrorPrototypes,
            },
            function_objects::{
                function_constructor::FunctionConstructor, function_prototype::FunctionPrototype,
            },
            object_objects::{
                object_constructor::ObjectConstructor, object_prototype::ObjectPrototype,
            },
            symbol_objects::{
                symbol_constructor::SymbolConstructor, symbol_prototype::SymbolPrototype,
            },
        },
        numbers_and_dates::{
            bigint_objects::{
                bigint_constructor::BigIntConstructor, bigint_prototype::BigIntPrototype,
            },
            number_objects::{
                number_constructor::NumberConstructor, number_prototype::NumberPrototype,
            },
        },
        types::{Object, OrdinaryObject},
    },
    heap::{
        indexes::{ArrayIndex, BuiltinFunctionIndex, ObjectIndex, PrimitiveObjectIndex},
        intrinsic_function_count, intrinsic_object_count, intrinsic_primitive_object_count,
        CompactionLists, HeapMarkAndSweep, IntrinsicConstructorIndexes, IntrinsicFunctionIndexes,
        IntrinsicObjectIndexes, IntrinsicPrimitiveObjectIndexes, WorkQueues,
    },
};
#[derive(Debug, Clone)]
pub(crate) struct Intrinsics {
    pub(crate) object_index_base: ObjectIndex,
    pub(crate) primitive_object_index_base: PrimitiveObjectIndex,
    /// Array prototype object is an Array exotic object. It is the only one
    /// in the ECMAScript spec so we do not need to store the Array index base.
    pub(crate) array_prototype: Array,
    pub(crate) builtin_function_index_base: BuiltinFunctionIndex,
}

/// Enumeration of intrinsics intended to be used as the \[\[Prototype\]\] value of
/// an object. Used in GetPrototypeFromConstructor.
#[derive(Debug, Clone, Copy)]
pub enum ProtoIntrinsics {
    AggregateError,
    Array,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer,
    ArrayIterator,
    AsyncFunction,
    AsyncGeneratorFunction,
    BigInt,
    #[cfg(feature = "array-buffer")]
    BigInt64Array,
    #[cfg(feature = "array-buffer")]
    BigUint64Array,
    Boolean,
    #[cfg(feature = "array-buffer")]
    DataView,
    #[cfg(feature = "date")]
    Date,
    Error,
    EvalError,
    FinalizationRegistry,
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
    Map,
    MapIterator,
    Number,
    Object,
    Promise,
    RangeError,
    ReferenceError,
    RegExp,
    Set,
    SetIterator,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer,
    String,
    Symbol,
    SyntaxError,
    TypeError,
    #[cfg(feature = "array-buffer")]
    Uint16Array,
    #[cfg(feature = "array-buffer")]
    Uint32Array,
    #[cfg(feature = "array-buffer")]
    Uint8Array,
    UriError,
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
        let object_index_base = ObjectIndex::from_index(agent.heap.objects.len());
        let primitive_object_index_base =
            PrimitiveObjectIndex::from_index(agent.heap.primitive_objects.len());
        let builtin_function_index_base =
            BuiltinFunctionIndex::from_index(agent.heap.builtin_functions.len());
        let array_prototype = Array::from(ArrayIndex::from_index(agent.heap.arrays.len()));

        agent
            .heap
            .objects
            .extend((0..intrinsic_object_count()).map(|_| None));
        agent
            .heap
            .primitive_objects
            .extend((0..intrinsic_primitive_object_count()).map(|_| None));
        agent
            .heap
            .builtin_functions
            .extend((0..intrinsic_function_count()).map(|_| None));
        agent.heap.arrays.push(None);

        Self {
            object_index_base,
            primitive_object_index_base,
            builtin_function_index_base,
            array_prototype,
        }
    }

    pub(crate) fn create_intrinsics(agent: &mut Agent, realm: RealmIdentifier) {
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
        NumberConstructor::create_intrinsic(agent, realm);
        BigIntPrototype::create_intrinsic(agent, realm);
        BigIntConstructor::create_intrinsic(agent, realm);
        #[cfg(feature = "math")]
        MathObject::create_intrinsic(agent, realm);
        #[cfg(feature = "date")]
        DatePrototype::create_intrinsic(agent, realm);
        #[cfg(feature = "date")]
        DateConstructor::create_intrinsic(agent, realm);
        StringPrototype::create_intrinsic(agent, realm);
        StringConstructor::create_intrinsic(agent, realm);
        StringIteratorPrototype::create_intrinsic(agent, realm);
        RegExpPrototype::create_intrinsic(agent, realm);
        RegExpConstructor::create_intrinsic(agent, realm);
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
        SetPrototype::create_intrinsic(agent, realm);
        SetConstructor::create_intrinsic(agent, realm);
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
        AsyncFromSyncIteratorPrototype::create_intrinsic(agent, realm);
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
    }

    // Suggest to inline this: The intrinsic default proto is often statically
    // known.
    #[inline]
    pub(crate) fn get_intrinsic_default_proto(
        &self,
        intrinsic_default_proto: ProtoIntrinsics,
    ) -> Object {
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
            ProtoIntrinsics::String => self.string_prototype().into(),
            ProtoIntrinsics::Symbol => self.symbol_prototype().into(),
            ProtoIntrinsics::SyntaxError => self.syntax_error_prototype().into(),
            ProtoIntrinsics::TypeError => self.type_error_prototype().into(),
            ProtoIntrinsics::UriError => self.uri_error_prototype().into(),
            ProtoIntrinsics::AggregateError => self.aggregate_error_prototype().into(),
            ProtoIntrinsics::AsyncFunction => self.async_function_prototype().into(),
            ProtoIntrinsics::AsyncGeneratorFunction => {
                self.async_generator_function_prototype().into()
            }
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigInt64Array => self.big_int64_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigUint64Array => self.big_int64_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::DataView => self.data_view_prototype().into(),
            ProtoIntrinsics::FinalizationRegistry => self.finalization_registry_prototype().into(),
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
            ProtoIntrinsics::Map => self.map_prototype().into(),
            ProtoIntrinsics::MapIterator => self.map_iterator_prototype().into(),
            ProtoIntrinsics::Promise => self.promise_prototype().into(),
            ProtoIntrinsics::RegExp => self.reg_exp_prototype().into(),
            ProtoIntrinsics::Set => self.set_prototype().into(),
            ProtoIntrinsics::SetIterator => self.set_iterator_prototype().into(),
            #[cfg(feature = "shared-array-buffer")]
            ProtoIntrinsics::SharedArrayBuffer => self.shared_array_buffer_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint16Array => self.uint16_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint32Array => self.uint32_array_prototype().into(),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint8Array => self.uint8_array_prototype().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakMap => self.weak_map_prototype().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakRef => self.weak_ref_prototype().into(),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakSet => self.weak_set_prototype().into(),
        }
    }

    pub(crate) fn intrinsic_function_index_to_builtin_function(
        &self,
        index: IntrinsicFunctionIndexes,
    ) -> BuiltinFunction {
        index
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn intrinsic_constructor_index_to_builtin_function(
        &self,
        index: IntrinsicConstructorIndexes,
    ) -> BuiltinFunction {
        index
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn intrinsic_constructor_index_to_object_index(
        &self,
        index: IntrinsicConstructorIndexes,
    ) -> ObjectIndex {
        index.get_object_index(self.object_index_base)
    }

    /// %AggregateError.prototype%
    pub(crate) fn aggregate_error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AggregateErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %AggregateError%
    pub(crate) fn aggregate_error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::AggregateError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn aggregate_error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::AggregateError.get_object_index(self.object_index_base)
    }

    /// %Array.prototype.sort%
    pub(crate) fn array_prototype_sort(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::ArrayPrototypeSort
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Array.prototype.toString%
    pub(crate) fn array_prototype_to_string(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::ArrayPrototypeToString
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Array.prototype.values%
    pub(crate) fn array_prototype_values(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::ArrayPrototypeValues
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Array.prototype%
    pub(crate) fn array_prototype(&self) -> Array {
        self.array_prototype
    }

    /// %Array.prototype%
    pub(crate) fn array_prototype_base_object(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Array%
    pub(crate) fn array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Array.get_object_index(self.object_index_base)
    }

    #[cfg(feature = "array-buffer")]
    /// %ArrayBuffer.prototype%
    pub(crate) fn array_buffer_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ArrayBufferPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    /// %ArrayBuffer%
    pub(crate) fn array_buffer(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::ArrayBuffer
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn array_buffer_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::ArrayBuffer.get_object_index(self.object_index_base)
    }

    /// %ArrayIteratorPrototype%
    pub(crate) fn array_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ArrayIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %AsyncFromSyncIteratorPrototype%
    pub(crate) fn async_from_sync_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AsyncFromSyncIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %AsyncFunction.prototype%
    pub(crate) fn async_function_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AsyncFunctionPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %AsyncFunction%
    pub(crate) fn async_function(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::AsyncFunction
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn async_function_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::AsyncFunction.get_object_index(self.object_index_base)
    }

    /// %AsyncGeneratorFunction.prototype.prototype%
    ///
    /// The %AsyncGeneratorPrototype% object is %AsyncGeneratorFunction.prototype.prototype%.
    pub(crate) fn async_generator_function_prototype_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AsyncGeneratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %AsyncGeneratorFunction.prototype%
    pub(crate) fn async_generator_function_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AsyncGeneratorFunctionPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %AsyncGeneratorFunction%
    pub(crate) fn async_generator_function(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::AsyncGeneratorFunction
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn async_generator_function_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::AsyncGeneratorFunction.get_object_index(self.object_index_base)
    }

    /// %AsyncGeneratorPrototype%
    pub(crate) fn async_generator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AsyncGeneratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %AsyncIteratorPrototype%
    pub(crate) fn async_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AsyncIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Atomics%
    #[cfg(feature = "atomics")]
    pub(crate) fn atomics(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AtomicsObject
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %BigInt.prototype%
    pub(crate) fn big_int_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::BigIntPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %BigInt%
    pub(crate) fn big_int(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::BigInt
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn big_int_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::BigInt.get_object_index(self.object_index_base)
    }

    /// %BigInt64Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn big_int64_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::BigInt64ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn big_int64_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::BigInt64Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn big_int64_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::BigInt64Array.get_object_index(self.object_index_base)
    }

    /// %BigUint64Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn big_uint64_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::BigUint64ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn big_uint64_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::BigUint64Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn big_uint64_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::BigUint64Array.get_object_index(self.object_index_base)
    }

    /// %Boolean.prototype%
    pub(crate) fn boolean_prototype(&self) -> PrimitiveObject {
        IntrinsicPrimitiveObjectIndexes::BooleanPrototype
            .get_primitive_object_index(self.primitive_object_index_base)
            .into()
    }

    pub(crate) fn boolean_prototype_base_object(&self) -> ObjectIndex {
        IntrinsicPrimitiveObjectIndexes::BooleanPrototype.get_object_index(self.object_index_base)
    }

    /// %Boolean%
    pub(crate) fn boolean(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Boolean
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn boolean_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Boolean.get_object_index(self.object_index_base)
    }

    /// %DataView.prototype%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn data_view_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::DataViewPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %DataView%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn data_view(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::DataView
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn data_view_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::DataView.get_object_index(self.object_index_base)
    }

    #[cfg(feature = "date")]
    /// %Date.prototype.toUTCString%
    pub(crate) fn date_prototype_to_utcstring(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::DatePrototypeToUTCString
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "date")]
    /// %Date.prototype%
    pub(crate) fn date_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::DatePrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "date")]
    /// %Date%
    pub(crate) fn date(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Date
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "date")]
    pub(crate) fn date_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Date.get_object_index(self.object_index_base)
    }

    /// %decodeURI%
    pub(crate) fn decode_uri(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::DecodeURI
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %decodeURIComponent%
    pub(crate) fn decode_uri_component(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::DecodeURIComponent
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %encodeURI%
    pub(crate) fn encode_uri(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::EncodeURI
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %encodeURIComponent%
    pub(crate) fn encode_uri_component(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::EncodeURIComponent
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Error.prototype%
    pub(crate) fn error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Error%
    pub(crate) fn error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Error
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Error.get_object_index(self.object_index_base)
    }

    /// %escape%
    pub(crate) fn escape(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::Escape
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %eval%
    pub(crate) fn eval(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::Eval
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %EvalError.prototype%
    pub(crate) fn eval_error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::EvalErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %EvalError%
    pub(crate) fn eval_error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::EvalError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn eval_error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::EvalError.get_object_index(self.object_index_base)
    }

    /// %FinalizationRegistry.prototype%
    pub(crate) fn finalization_registry_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::FinalizationRegistryPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %FinalizationRegistry%
    pub(crate) fn finalization_registry(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::FinalizationRegistry
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn finalization_registry_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::FinalizationRegistry.get_object_index(self.object_index_base)
    }

    /// %Float32Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn float32_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Float32ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn float32_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Float32Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn float32_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Float32Array.get_object_index(self.object_index_base)
    }

    /// %Float64Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn float64_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Float64ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn float64_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Float64Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn float64_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Float64Array.get_object_index(self.object_index_base)
    }

    pub(crate) fn function_prototype(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::FunctionPrototype
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn function_prototype_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::FunctionPrototype.get_object_index(self.object_index_base)
    }

    /// %Function%
    pub(crate) fn function(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Function
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn function_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Function.get_object_index(self.object_index_base)
    }

    /// %GeneratorFunction.prototype.prototype.next%
    pub(crate) fn generator_function_prototype_prototype_next(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::GeneratorFunctionPrototypePrototypeNext
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    // %GeneratorFunction.prototype.prototype%
    //
    // The %GeneratorPrototype% object is %GeneratorFunction.prototype.prototype%.
    pub(crate) fn generator_function_prototype_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::GeneratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %GeneratorFunction.prototype%
    pub(crate) fn generator_function_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::GeneratorFunctionPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %GeneratorFunction%
    pub(crate) fn generator_function(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::GeneratorFunction
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn generator_function_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::GeneratorFunction.get_object_index(self.object_index_base)
    }

    /// %GeneratorPrototype%
    pub(crate) fn generator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::GeneratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Int16Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn int16_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Int16ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn int16_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Int16Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn int16_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Int16Array.get_object_index(self.object_index_base)
    }

    /// %Int32Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn int32_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Int32ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn int32_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Int32Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn int32_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Int32Array.get_object_index(self.object_index_base)
    }

    /// %Int8Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn int8_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Int8ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn int8_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Int8Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn int8_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Int8Array.get_object_index(self.object_index_base)
    }

    /// %isFinite%
    pub(crate) fn is_finite(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::IsFinite
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %isNaN%
    pub(crate) fn is_nan(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::IsNaN
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %IteratorPrototype%
    pub(crate) fn iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::IteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "json")]
    /// %JSON%
    pub(crate) fn json(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::JSONObject
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Map.prototype.entries%
    pub(crate) fn map_prototype_entries(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::MapPrototypeEntries
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Map.prototype%
    pub(crate) fn map_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::MapPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Map%
    pub(crate) fn map(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Map
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn map_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Map.get_object_index(self.object_index_base)
    }

    /// %MapIteratorPrototype%
    pub(crate) fn map_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::MapIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Math%
    #[cfg(feature = "math")]
    pub(crate) fn math(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::MathObject
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Number.prototype%
    pub(crate) fn number_prototype(&self) -> PrimitiveObject {
        IntrinsicPrimitiveObjectIndexes::NumberPrototype
            .get_primitive_object_index(self.primitive_object_index_base)
            .into()
    }

    pub(crate) fn number_prototype_base_object(&self) -> ObjectIndex {
        IntrinsicPrimitiveObjectIndexes::NumberPrototype.get_object_index(self.object_index_base)
    }

    /// %Number%
    pub(crate) fn number(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Number
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn number_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Number.get_object_index(self.object_index_base)
    }

    /// %Object.prototype.toString%
    pub(crate) fn object_prototype_to_string(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::ObjectPrototypeToString
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Object.prototype%
    pub(crate) fn object_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ObjectPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Object%
    pub(crate) fn object(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Object
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn object_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Object.get_object_index(self.object_index_base)
    }

    /// %parseFloat%
    pub(crate) fn parse_float(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::ParseFloat
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %parseInt%
    pub(crate) fn parse_int(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::ParseInt
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Promise.prototype%
    pub(crate) fn promise_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::PromisePrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Promise%
    pub(crate) fn promise(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Promise
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn promise_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Promise.get_object_index(self.object_index_base)
    }

    /// %Proxy%
    pub(crate) fn proxy(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Proxy
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn proxy_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Proxy.get_object_index(self.object_index_base)
    }

    /// %RangeError.prototype%
    pub(crate) fn range_error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::RangeErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %RangeError%
    pub(crate) fn range_error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::RangeError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn range_error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::RangeError.get_object_index(self.object_index_base)
    }

    /// %ReferenceError.prototype%
    pub(crate) fn reference_error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ReferenceErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %ReferenceError%
    pub(crate) fn reference_error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::ReferenceError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn reference_error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::ReferenceError.get_object_index(self.object_index_base)
    }

    /// %Reflect%
    pub(crate) fn reflect(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ReflectObject
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %RegExp.prototype.exec%
    pub(crate) fn reg_exp_prototype_exec(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::RegExpPrototypeExec
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %RegExp.prototype%
    pub(crate) fn reg_exp_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::RegExpPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %RegExp%
    pub(crate) fn reg_exp(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::RegExp
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn reg_exp_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::RegExp.get_object_index(self.object_index_base)
    }

    /// %RegExpStringIteratorPrototype%
    pub(crate) fn reg_exp_string_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::RegExpStringIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Set.prototype.values%
    pub(crate) fn set_prototype_values(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::SetPrototypeValues
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Set.prototype%
    pub(crate) fn set_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::SetPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Set%
    pub(crate) fn set(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Set
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn set_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Set.get_object_index(self.object_index_base)
    }

    /// %SetIteratorPrototype%
    pub(crate) fn set_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::SetIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %SharedArrayBuffer.prototype%
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) fn shared_array_buffer_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::SharedArrayBufferPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %SharedArrayBuffer%
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) fn shared_array_buffer(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::SharedArrayBuffer
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "shared-array-buffer")]
    pub(crate) fn shared_array_buffer_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::SharedArrayBuffer.get_object_index(self.object_index_base)
    }

    /// %String.prototype.trimEnd%
    pub(crate) fn string_prototype_trim_end(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::StringPrototypeTrimEnd
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %String.prototype.trimStart%
    pub(crate) fn string_prototype_trim_start(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::StringPrototypeTrimStart
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %String.prototype%
    pub(crate) fn string_prototype(&self) -> PrimitiveObject {
        IntrinsicPrimitiveObjectIndexes::StringPrototype
            .get_primitive_object_index(self.primitive_object_index_base)
            .into()
    }

    pub(crate) fn string_prototype_base_object(&self) -> ObjectIndex {
        IntrinsicPrimitiveObjectIndexes::StringPrototype.get_object_index(self.object_index_base)
    }

    /// %String%
    pub(crate) fn string(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::String
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn string_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::String.get_object_index(self.object_index_base)
    }

    /// %StringIteratorPrototype%
    pub(crate) fn string_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::StringIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Symbol.prototype%
    pub(crate) fn symbol_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::SymbolPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Symbol%
    pub(crate) fn symbol(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Symbol
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn symbol_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Symbol.get_object_index(self.object_index_base)
    }

    /// %SyntaxError.prototype%
    pub(crate) fn syntax_error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::SyntaxErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %SyntaxError%
    pub(crate) fn syntax_error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::SyntaxError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn syntax_error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::SyntaxError.get_object_index(self.object_index_base)
    }

    /// %ThrowTypeError%
    pub(crate) fn throw_type_error(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::ThrowTypeError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %TypedArray.prototype.values%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn typed_array_prototype_values(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::TypedArrayPrototypeValues
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %TypedArray.prototype%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn typed_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::TypedArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %TypedArray%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn typed_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::TypedArray
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn typed_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::TypedArray.get_object_index(self.object_index_base)
    }

    /// %TypeError.prototype%
    pub(crate) fn type_error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::TypeErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %TypeError%
    pub(crate) fn type_error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::TypeError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn type_error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::TypeError.get_object_index(self.object_index_base)
    }

    /// %Uint16Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint16_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint16ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint16_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint16Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint16_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Uint16Array.get_object_index(self.object_index_base)
    }

    /// %Uint32Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint32_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint32ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint32_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint32Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint32_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Uint32Array.get_object_index(self.object_index_base)
    }

    /// %Uint8Array%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint8_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint8ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint8_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint8Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint8_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Uint8Array.get_object_index(self.object_index_base)
    }

    /// %Uint8ClampedArray%
    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint8_clamped_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint8ClampedArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint8_clamped_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint8ClampedArray
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "array-buffer")]
    pub(crate) fn uint8_clamped_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Uint8ClampedArray.get_object_index(self.object_index_base)
    }

    /// %unescape%
    pub(crate) fn unescape(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::Unescape
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %URIError.prototype%
    pub(crate) fn uri_error_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::URIErrorPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %URIError%
    pub(crate) fn uri_error(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::URIError
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn uri_error_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::URIError.get_object_index(self.object_index_base)
    }

    /// %WeakMap.prototype%
    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_map_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::WeakMapPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %WeakMap%
    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_map(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::WeakMap
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_map_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::WeakMap.get_object_index(self.object_index_base)
    }

    /// %WeakRef.prototype%
    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_ref_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::WeakRefPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %WeakRef%
    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_ref(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::WeakRef
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_ref_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::WeakRef.get_object_index(self.object_index_base)
    }

    /// %WeakSet.prototype%
    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_set_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::WeakSetPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %WeakSet%
    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_set(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::WeakSet
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    #[cfg(feature = "weak-refs")]
    pub(crate) fn weak_set_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::WeakSet.get_object_index(self.object_index_base)
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
        self.async_from_sync_iterator_prototype()
            .mark_values(queues);
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
        self.reg_exp_prototype_exec().mark_values(queues);
        self.reg_exp_prototype().mark_values(queues);
        self.reg_exp().mark_values(queues);
        self.reg_exp_string_iterator_prototype().mark_values(queues);
        self.set_prototype_values().mark_values(queues);
        self.set_prototype().mark_values(queues);
        self.set().mark_values(queues);
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
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index_base,
            primitive_object_index_base,
            array_prototype,
            builtin_function_index_base,
        } = self;
        compactions.objects.shift_index(object_index_base);
        compactions
            .primitive_objects
            .shift_index(primitive_object_index_base);
        array_prototype.sweep_values(compactions);
        compactions
            .builtin_functions
            .shift_index(builtin_function_index_base);
    }
}
