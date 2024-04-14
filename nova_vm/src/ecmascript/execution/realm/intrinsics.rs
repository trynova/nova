use crate::{
    ecmascript::{
        builtins::{
            indexed_collections::{
                array_objects::{
                    array_constructor::ArrayConstructor, array_prototype::ArrayPrototype,
                },
                typed_array_objects::{
                    typed_array_constructors::{TypedArrayConstructors, TypedArrayPrototypes},
                    typed_array_intrinsic_object::{
                        TypedArrayIntrinsicObject, TypedArrayPrototype,
                    },
                },
            },
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
            BuiltinFunction,
        },
        execution::Agent,
        fundamental_objects::{
            boolean_objects::{
                boolean_constructor::BooleanConstructor, boolean_prototype::BooleanPrototype,
            },
            error_objects::{
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
            date_objects::{date_constructor::DateConstructor, date_prototype::DatePrototype},
            math_object::MathObject,
            number_objects::{
                number_constructor::NumberConstructor, number_prototype::NumberPrototype,
            },
        },
        types::{Object, OrdinaryObject},
    },
    heap::{
        indexes::{BuiltinFunctionIndex, ObjectIndex},
        intrinsic_function_count, intrinsic_object_count, IntrinsicConstructorIndexes,
        IntrinsicFunctionIndexes, IntrinsicObjectIndexes,
    },
};

use super::RealmIdentifier;

#[derive(Debug, Clone)]
pub(crate) struct Intrinsics {
    pub(crate) object_index_base: ObjectIndex,
    pub(crate) builtin_function_index_base: BuiltinFunctionIndex,
}

/// Enumeration of intrinsics intended to be used as the \[\[Prototype\]\] value of
/// an object. Used in GetPrototypeFromConstructor.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ProtoIntrinsics {
    Array,
    ArrayBuffer,
    BigInt,
    Boolean,
    Date,
    Error,
    EvalError,
    Function,
    Number,
    Object,
    RangeError,
    ReferenceError,
    String,
    Symbol,
    SyntaxError,
    TypeError,
    UriError,
}

impl Intrinsics {
    pub(crate) fn new(agent: &mut Agent) -> Self {
        // Use from_usize to index "one over the edge", ie. where new intrinsics will be created.
        let object_index_base = ObjectIndex::from_index(agent.heap.objects.len());
        let builtin_function_index_base =
            BuiltinFunctionIndex::from_index(agent.heap.builtin_functions.len());

        agent
            .heap
            .objects
            .extend((0..intrinsic_object_count()).map(|_| None));
        agent
            .heap
            .builtin_functions
            .extend((0..intrinsic_function_count()).map(|_| None));

        Self {
            object_index_base,
            builtin_function_index_base,
        }
    }

    pub(crate) fn create_intrinsics(agent: &mut Agent, realm: RealmIdentifier) {
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
        NumberPrototype::create_intrinsic(agent, realm);
        NumberConstructor::create_intrinsic(agent, realm);
        BigIntPrototype::create_intrinsic(agent, realm);
        BigIntConstructor::create_intrinsic(agent, realm);
        MathObject::create_intrinsic(agent, realm);
        DatePrototype::create_intrinsic(agent, realm);
        DateConstructor::create_intrinsic(agent, realm);
        StringPrototype::create_intrinsic(agent, realm);
        StringConstructor::create_intrinsic(agent, realm);
        StringIteratorPrototype::create_intrinsic(agent, realm);
        RegExpPrototype::create_intrinsic(agent, realm);
        RegExpConstructor::create_intrinsic(agent, realm);
        RegExpStringIteratorPrototype::create_intrinsic(agent, realm);
        ArrayPrototype::create_intrinsic(agent, realm);
        ArrayConstructor::create_intrinsic(agent, realm);
        TypedArrayPrototype::create_intrinsic(agent, realm);
        TypedArrayIntrinsicObject::create_intrinsic(agent, realm);
        TypedArrayPrototypes::create_intrinsic(agent, realm);
        TypedArrayConstructors::create_intrinsic(agent, realm);
    }

    pub(crate) fn get_intrinsic_default_proto(
        &self,
        intrinsic_default_proto: ProtoIntrinsics,
    ) -> Object {
        match intrinsic_default_proto {
            ProtoIntrinsics::Array => self.array_prototype().into(),
            ProtoIntrinsics::ArrayBuffer => self.array_buffer_prototype().into(),
            ProtoIntrinsics::BigInt => self.big_int_prototype().into(),
            ProtoIntrinsics::Boolean => self.boolean_prototype().into(),
            ProtoIntrinsics::Error => self.error_prototype().into(),
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
        }
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
    pub(crate) fn array_prototype(&self) -> OrdinaryObject {
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

    /// %ArrayBuffer.prototype%
    pub(crate) fn array_buffer_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ArrayBufferPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %ArrayBuffer%
    pub(crate) fn array_buffer(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::ArrayBuffer
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

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
    pub(crate) fn async_generator_function_prototype_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::AsyncGeneratorFunctionPrototypePrototype
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
    pub(crate) fn big_int64_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::BigInt64ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn big_int64_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::BigInt64Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn big_int64_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::BigInt64Array.get_object_index(self.object_index_base)
    }

    /// %BigUint64Array%
    pub(crate) fn big_uint64_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::BigUint64ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn big_uint64_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::BigUint64Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn big_uint64_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::BigUint64Array.get_object_index(self.object_index_base)
    }

    /// %Boolean.prototype%
    pub(crate) fn boolean_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::BooleanPrototype
            .get_object_index(self.object_index_base)
            .into()
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
    pub(crate) fn data_view_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::DataViewPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %DataView%
    pub(crate) fn data_view(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::DataView
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn data_view_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::DataView.get_object_index(self.object_index_base)
    }

    /// %Date.prototype.toUTCString%
    pub(crate) fn date_prototype_to_utcstring(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::DatePrototypeToUTCString
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %Date.prototype%
    pub(crate) fn date_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::DatePrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Date%
    pub(crate) fn date(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Date
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

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
    pub(crate) fn decode_uricomponent(&self) -> BuiltinFunction {
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
    pub(crate) fn float32_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Float32ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn float32_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Float32Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn float32_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Float32Array.get_object_index(self.object_index_base)
    }

    /// %Float64Array%
    pub(crate) fn float64_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Float64ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn float64_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Float64Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn float64_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Float64Array.get_object_index(self.object_index_base)
    }

    /// %ForInIteratorPrototype%
    pub(crate) fn for_in_iterator_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::ForInIteratorPrototype
            .get_object_index(self.object_index_base)
            .into()
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

    /// %GeneratorFunction.prototype.prototype%
    pub(crate) fn generator_function_prototype_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::GeneratorFunctionPrototypePrototype
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
    pub(crate) fn int16_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Int16ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn int16_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Int16Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn int16_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Int16Array.get_object_index(self.object_index_base)
    }

    /// %Int32Array%
    pub(crate) fn int32_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Int32ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn int32_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Int32Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn int32_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Int32Array.get_object_index(self.object_index_base)
    }

    /// %Int8Array%
    pub(crate) fn int8_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Int8ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn int8_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Int8Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

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
    pub(crate) fn math(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::MathObject
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %Number.prototype%
    pub(crate) fn number_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::NumberPrototype
            .get_object_index(self.object_index_base)
            .into()
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
    pub(crate) fn shared_array_buffer_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::SharedArrayBufferPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %SharedArrayBuffer%
    pub(crate) fn shared_array_buffer(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::SharedArrayBuffer
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

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
    pub(crate) fn string_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::StringPrototype
            .get_object_index(self.object_index_base)
            .into()
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
    pub(crate) fn typed_array_prototype_values(&self) -> BuiltinFunction {
        IntrinsicFunctionIndexes::TypedArrayPrototypeValues
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    /// %TypedArray.prototype%
    pub(crate) fn typed_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::TypedArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %TypedArray%
    pub(crate) fn typed_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::TypedArray
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

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
    pub(crate) fn uint16_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint16ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn uint16_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint16Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn uint16_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Uint16Array.get_object_index(self.object_index_base)
    }

    /// %Uint32Array%
    pub(crate) fn uint32_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint32ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn uint32_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint32Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn uint32_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Uint32Array.get_object_index(self.object_index_base)
    }

    /// %Uint8Array%
    pub(crate) fn uint8_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint8ArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn uint8_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint8Array
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn uint8_array_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::Uint8Array.get_object_index(self.object_index_base)
    }

    /// %Uint8ClampedArray%
    pub(crate) fn uint8_clamped_array_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::Uint8ClampedArrayPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    pub(crate) fn uint8_clamped_array(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::Uint8ClampedArray
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

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
    pub(crate) fn weak_map_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::WeakMapPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %WeakMap%
    pub(crate) fn weak_map(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::WeakMap
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn weak_map_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::WeakMap.get_object_index(self.object_index_base)
    }

    /// %WeakRef.prototype%
    pub(crate) fn weak_ref_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::WeakRefPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %WeakRef%
    pub(crate) fn weak_ref(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::WeakRef
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn weak_ref_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::WeakRef.get_object_index(self.object_index_base)
    }

    /// %WeakSet.prototype%
    pub(crate) fn weak_set_prototype(&self) -> OrdinaryObject {
        IntrinsicObjectIndexes::WeakSetPrototype
            .get_object_index(self.object_index_base)
            .into()
    }

    /// %WeakSet%
    pub(crate) fn weak_set(&self) -> BuiltinFunction {
        IntrinsicConstructorIndexes::WeakSet
            .get_builtin_function_index(self.builtin_function_index_base)
            .into()
    }

    pub(crate) fn weak_set_base_object(&self) -> ObjectIndex {
        IntrinsicConstructorIndexes::WeakSet.get_object_index(self.object_index_base)
    }
}
