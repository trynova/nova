// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//#[cfg(feature = "temporal")]
//use temporal_rs::Instant as Instant; // shadow regular instant, should probably change name to TemporalInstant

use super::{
    BigInt, BigIntHeapData, IntoValue, Number, Numeric, OrdinaryObject, Primitive, String,
    StringRecord, Symbol, bigint::HeapBigInt, number::HeapNumber, string::HeapString,
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "temporal")]
use crate::ecmascript::builtins::temporal::instant::Instant;
#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::builtins::typed_array::Float16Array;
#[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
use crate::ecmascript::builtins::typed_array::SharedFloat16Array;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{
    ArrayBuffer,
    data_view::DataView,
    typed_array::{
        BigInt64Array, BigUint64Array, Float32Array, Float64Array, Int8Array, Int16Array,
        Int32Array, Uint8Array, Uint8ClampedArray, Uint16Array, Uint32Array,
    },
};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::{
    data_view::SharedDataView,
    shared_array_buffer::SharedArrayBuffer,
    typed_array::{
        SharedBigInt64Array, SharedBigUint64Array, SharedFloat32Array, SharedFloat64Array,
        SharedInt8Array, SharedInt16Array, SharedInt32Array, SharedUint8Array,
        SharedUint8ClampedArray, SharedUint16Array, SharedUint32Array,
    },
};
#[cfg(feature = "set")]
use crate::ecmascript::builtins::{
    keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
};
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::{
    regexp::RegExp,
    text_processing::regexp_objects::regexp_string_iterator_objects::RegExpStringIterator,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
use crate::{
    SmallInteger, SmallString,
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int, to_big_int64, to_big_uint64, to_int8, to_int16, to_int32, to_number,
            to_numeric, to_string, to_uint8, to_uint8_clamp, to_uint16, to_uint32, try_to_string,
        },
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
            promise_objects::promise_abstract_operations::promise_finally_functions::BuiltinPromiseFinallyFunction,
            proxy::Proxy,
            text_processing::string_objects::string_iterator_objects::StringIterator,
        },
        execution::{
            Agent, JsResult,
            agent::{TryResult, try_result_into_js},
        },
        types::{BUILTIN_STRING_MEMORY, Object},
    },
    engine::{
        Scoped,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_bigint::SmallBigInt,
        small_f64::SmallF64,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use core::{
    hash::{Hash, Hasher},
    mem::size_of,
    ops::Index,
};

/// ### [6.1 ECMAScript Language Types](https://tc39.es/ecma262/#sec-ecmascript-language-types)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(u8)]
pub enum Value<'a> {
    /// ### [6.1.1 The Undefined Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type)
    #[default]
    Undefined = 1,

    /// ### [6.1.2 The Null Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-null-type)
    Null,

    /// ### [6.1.3 The Boolean Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-boolean-type)
    Boolean(bool),

    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// UTF-8 string on the heap. Accessing the data must be done through the
    /// Agent. ECMAScript specification compliant UTF-16 indexing is
    /// implemented through an index mapping.
    String(HeapString<'a>),
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// 7-byte UTF-8 string on the stack. End of the string is determined by
    /// the first 0xFF byte in the data. UTF-16 indexing is calculated on
    /// demand from the data.
    SmallString(SmallString),

    /// ### [6.1.5 The Symbol Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type)
    Symbol(Symbol<'a>),

    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber<'a>),
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 53-bit signed integer on the stack.
    Integer(SmallInteger),
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 56-bit f64 on the stack. The missing byte is a zero least significant
    /// byte.
    SmallF64(SmallF64),

    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// Unlimited size integer data on the heap. Accessing the data must be
    /// done through the Agent.
    BigInt(HeapBigInt<'a>),
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// 56-bit signed integer on the stack.
    SmallBigInt(SmallBigInt),

    /// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
    Object(OrdinaryObject<'a>),

    // Functions
    BoundFunction(BoundFunction<'a>),
    BuiltinFunction(BuiltinFunction<'a>),
    ECMAScriptFunction(ECMAScriptFunction<'a>),
    /// Default class constructor created in step 14 of
    /// [ClassDefinitionEvaluation](https://tc39.es/ecma262/#sec-runtime-semantics-classdefinitionevaluation).
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>),
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>),
    BuiltinPromiseFinallyFunction(BuiltinPromiseFinallyFunction<'a>),
    BuiltinPromiseCollectorFunction,
    BuiltinProxyRevokerFunction,

    // Boolean, Number, String, Symbol, BigInt objects
    PrimitiveObject(PrimitiveObject<'a>),

    // Well-known object types
    // Roughly corresponding to 6.1.7.4 Well-Known Intrinsic Objects
    // https://tc39.es/ecma262/#sec-well-known-intrinsic-objects
    // and 18 ECMAScript Standard Built-in Objects
    // https://tc39.es/ecma262/#sec-ecmascript-standard-built-in-objects
    /// ### [10.4.4 Arguments Exotic Objects](https://tc39.es/ecma262/#sec-arguments-exotic-objects)
    ///
    /// An unmapped arguments object is an ordinary object with an additional
    /// internal slot \[\[ParameterMap]] whose value is always **undefined**.
    Arguments(OrdinaryObject<'a>),
    // TODO: MappedArguments(MappedArgumentsObject),
    Array(Array<'a>),
    #[cfg(feature = "date")]
    Date(Date<'a>),
    #[cfg(feature = "temporal")]
    Instant(Instant<'a>),
    Error(Error<'a>),
    FinalizationRegistry(FinalizationRegistry<'a>),
    Map(Map<'a>),
    Promise(Promise<'a>),
    Proxy(Proxy<'a>),
    #[cfg(feature = "regexp")]
    RegExp(RegExp<'a>),
    #[cfg(feature = "set")]
    Set(Set<'a>),
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'a>),
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'a>),
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'a>),

    /// ### [25.1 ArrayBuffer Objects](https://tc39.es/ecma262/#sec-arraybuffer-objects)
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'a>),
    /// ### [25.3 DataView Objects](https://tc39.es/ecma262/#sec-dataview-objects)
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'a>),
    // ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
    #[cfg(feature = "array-buffer")]
    Int8Array(Int8Array<'a>),
    #[cfg(feature = "array-buffer")]
    Uint8Array(Uint8Array<'a>),
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(Uint8ClampedArray<'a>),
    #[cfg(feature = "array-buffer")]
    Int16Array(Int16Array<'a>),
    #[cfg(feature = "array-buffer")]
    Uint16Array(Uint16Array<'a>),
    #[cfg(feature = "array-buffer")]
    Int32Array(Int32Array<'a>),
    #[cfg(feature = "array-buffer")]
    Uint32Array(Uint32Array<'a>),
    #[cfg(feature = "array-buffer")]
    BigInt64Array(BigInt64Array<'a>),
    #[cfg(feature = "array-buffer")]
    BigUint64Array(BigUint64Array<'a>),
    #[cfg(feature = "proposal-float16array")]
    Float16Array(Float16Array<'a>),
    #[cfg(feature = "array-buffer")]
    Float32Array(Float32Array<'a>),
    #[cfg(feature = "array-buffer")]
    Float64Array(Float64Array<'a>),

    /// ### [25.2 SharedArrayBuffer Objects](https://tc39.es/ecma262/#sec-sharedarraybuffer-objects)
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'a>),
    /// ### [25.3 DataView Objects](https://tc39.es/ecma262/#sec-dataview-objects)
    ///
    /// A variant of DataView Objects viewing a SharedArrayBuffer.
    #[cfg(feature = "shared-array-buffer")]
    SharedDataView(SharedDataView<'a>),
    // ### [23.2 TypedArray Objects](https://tc39.es/ecma262/#sec-typedarray-objects)
    //
    // Variants of TypedArray Objects viewing a SharedArrayBuffer.
    #[cfg(feature = "shared-array-buffer")]
    SharedInt8Array(SharedInt8Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8Array(SharedUint8Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8ClampedArray(SharedUint8ClampedArray<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedInt16Array(SharedInt16Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedUint16Array(SharedUint16Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedInt32Array(SharedInt32Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedUint32Array(SharedUint32Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedBigInt64Array(SharedBigInt64Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedBigUint64Array(SharedBigUint64Array<'a>),
    #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
    SharedFloat16Array(SharedFloat16Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat32Array(SharedFloat32Array<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat64Array(SharedFloat64Array<'a>),

    // Iterator objects
    AsyncGenerator(AsyncGenerator<'a>),
    ArrayIterator(ArrayIterator<'a>),
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'a>),
    #[cfg(feature = "set")]
    MapIterator(MapIterator<'a>),
    StringIterator(StringIterator<'a>),
    #[cfg(feature = "regexp")]
    RegExpStringIterator(RegExpStringIterator<'a>),
    Generator(Generator<'a>),

    // ECMAScript Module
    Module(Module<'a>),

    // Embedder objects
    EmbedderObject(EmbedderObject<'a>) = 0x7f,
}

/// We want to guarantee that all handles to JS values are register sized. This
/// assert must never be removed or broken.
const _VALUE_SIZE_IS_WORD: () = assert!(size_of::<Value>() == size_of::<usize>());
/// We may also want to keep Option<Value> register sized so that eg. holes in
/// arrays do not start requiring extra bookkeeping.
const _OPTIONAL_VALUE_SIZE_IS_WORD: () = assert!(size_of::<Option<Value>>() == size_of::<usize>());

const fn value_discriminant(value: Value) -> u8 {
    // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
    // between `repr(C)` structs, each of which has the `u8` discriminant as its first
    // field, so we can read the discriminant without offsetting the pointer.
    unsafe { *(&value as *const Value).cast::<u8>() }
}

pub(crate) const UNDEFINED_DISCRIMINANT: u8 = value_discriminant(Value::Undefined);
pub(crate) const NULL_DISCRIMINANT: u8 = value_discriminant(Value::Null);
pub(crate) const BOOLEAN_DISCRIMINANT: u8 = value_discriminant(Value::Boolean(true));
pub(crate) const STRING_DISCRIMINANT: u8 = value_discriminant(Value::String(HeapString::_def()));
pub(crate) const SMALL_STRING_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallString(SmallString::EMPTY));
pub(crate) const SYMBOL_DISCRIMINANT: u8 = value_discriminant(Value::Symbol(Symbol::_def()));
pub(crate) const NUMBER_DISCRIMINANT: u8 = value_discriminant(Value::Number(HeapNumber::_def()));
pub(crate) const INTEGER_DISCRIMINANT: u8 =
    value_discriminant(Value::Integer(SmallInteger::zero()));
pub(crate) const FLOAT_DISCRIMINANT: u8 = value_discriminant(Value::SmallF64(SmallF64::_def()));
pub(crate) const BIGINT_DISCRIMINANT: u8 = value_discriminant(Value::BigInt(HeapBigInt::_def()));
pub(crate) const SMALL_BIGINT_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallBigInt(SmallBigInt::zero()));
pub(crate) const OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::Object(OrdinaryObject::_def()));
pub(crate) const ARRAY_DISCRIMINANT: u8 = value_discriminant(Value::Array(Array::_def()));
#[cfg(feature = "date")]
pub(crate) const DATE_DISCRIMINANT: u8 = value_discriminant(Value::Date(Date::_def()));
#[cfg(feature = "temporal")]
pub(crate) const INSTANT_DISCRIMINANT: u8 = value_discriminant(Value::Instant(Instant::_def()));
pub(crate) const ERROR_DISCRIMINANT: u8 = value_discriminant(Value::Error(Error::_def()));
pub(crate) const BUILTIN_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinFunction(BuiltinFunction::_def()));
pub(crate) const ECMASCRIPT_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::ECMAScriptFunction(ECMAScriptFunction::_def()));
pub(crate) const BOUND_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BoundFunction(BoundFunction::_def()));
#[cfg(feature = "regexp")]
pub(crate) const REGEXP_DISCRIMINANT: u8 = value_discriminant(Value::RegExp(RegExp::_def()));

pub(crate) const BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinConstructorFunction(BuiltinConstructorFunction::_def()),
);
pub(crate) const BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction::_def()),
);
pub(crate) const BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinPromiseFinallyFunction(BuiltinPromiseFinallyFunction::_def()),
);
pub(crate) const BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinPromiseCollectorFunction);
pub(crate) const BUILTIN_PROXY_REVOKER_FUNCTION: u8 =
    value_discriminant(Value::BuiltinProxyRevokerFunction);
pub(crate) const PRIMITIVE_OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::PrimitiveObject(PrimitiveObject::_def()));
pub(crate) const ARGUMENTS_DISCRIMINANT: u8 =
    value_discriminant(Value::Arguments(OrdinaryObject::_def()));
pub(crate) const FINALIZATION_REGISTRY_DISCRIMINANT: u8 =
    value_discriminant(Value::FinalizationRegistry(FinalizationRegistry::_DEF));
pub(crate) const MAP_DISCRIMINANT: u8 = value_discriminant(Value::Map(Map::_def()));
pub(crate) const PROMISE_DISCRIMINANT: u8 = value_discriminant(Value::Promise(Promise::_def()));
pub(crate) const PROXY_DISCRIMINANT: u8 = value_discriminant(Value::Proxy(Proxy::_def()));
#[cfg(feature = "set")]
pub(crate) const SET_DISCRIMINANT: u8 = value_discriminant(Value::Set(Set::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_MAP_DISCRIMINANT: u8 = value_discriminant(Value::WeakMap(WeakMap::_DEF));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_REF_DISCRIMINANT: u8 = value_discriminant(Value::WeakRef(WeakRef::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_SET_DISCRIMINANT: u8 = value_discriminant(Value::WeakSet(WeakSet::_def()));

#[cfg(feature = "array-buffer")]
pub(crate) const ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayBuffer(ArrayBuffer::_def()));
#[cfg(feature = "array-buffer")]
pub(crate) const DATA_VIEW_DISCRIMINANT: u8 = value_discriminant(Value::DataView(DataView::_def()));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int8Array(Int8Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8Array(Uint8Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_8_CLAMPED_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8ClampedArray(Uint8ClampedArray::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int16Array(Int16Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint16Array(Uint16Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int32Array(Int32Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint32Array(Uint32Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const BIGINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigInt64Array(BigInt64Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const BIGUINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigUint64Array(BigUint64Array::_DEF));
#[cfg(feature = "proposal-float16array")]
pub(crate) const FLOAT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float16Array(Float16Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const FLOAT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float32Array(Float32Array::_DEF));
#[cfg(feature = "array-buffer")]
pub(crate) const FLOAT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float64Array(Float64Array::_DEF));

#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedArrayBuffer(SharedArrayBuffer::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_DATA_VIEW_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedDataView(SharedDataView::_def()));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_INT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedInt8Array(SharedInt8Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_UINT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedUint8Array(SharedUint8Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT: u8 = value_discriminant(
    Value::SharedUint8ClampedArray(SharedUint8ClampedArray::_DEF),
);
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_INT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedInt16Array(SharedInt16Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_UINT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedUint16Array(SharedUint16Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_INT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedInt32Array(SharedInt32Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_UINT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedUint32Array(SharedUint32Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_BIGINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedBigInt64Array(SharedBigInt64Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_BIGUINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedBigUint64Array(SharedBigUint64Array::_DEF));
#[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
pub(crate) const SHARED_FLOAT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedFloat16Array(SharedFloat16Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_FLOAT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedFloat32Array(SharedFloat32Array::_DEF));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_FLOAT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedFloat64Array(SharedFloat64Array::_DEF));

pub(crate) const ASYNC_GENERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::AsyncGenerator(AsyncGenerator::_def()));
pub(crate) const ARRAY_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayIterator(ArrayIterator::_def()));
#[cfg(feature = "set")]
pub(crate) const SET_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::SetIterator(SetIterator::_def()));
#[cfg(feature = "set")]
pub(crate) const MAP_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::MapIterator(MapIterator::_def()));
pub(crate) const STRING_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::StringIterator(StringIterator::_def()));
#[cfg(feature = "regexp")]
pub(crate) const REGEXP_STRING_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::RegExpStringIterator(RegExpStringIterator::_def()));
pub(crate) const GENERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::Generator(Generator::_def()));
pub(crate) const MODULE_DISCRIMINANT: u8 = value_discriminant(Value::Module(Module::_def()));
pub(crate) const EMBEDDER_OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::EmbedderObject(EmbedderObject::_def()));

impl<'a> Value<'a> {
    /// Scope a stack-only Value. Stack-only Values are primitives that do not
    /// need to store any data on the heap, hence scoping them is effectively a
    /// no-op. These Values are also not concerned with the garbage collector.
    ///
    /// ## Panics
    ///
    /// If the Value is not stack-only, this method will panic.
    pub const fn scope_static<'scope>(
        self,
        _gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Value<'static>> {
        let key_root_repr = match self {
            Value::Undefined => ValueRootRepr::Undefined,
            Value::Null => ValueRootRepr::Null,
            Value::Boolean(bool) => ValueRootRepr::Boolean(bool),
            Value::SmallString(small_string) => ValueRootRepr::SmallString(small_string),
            Value::Integer(small_integer) => ValueRootRepr::Integer(small_integer),
            Value::SmallF64(small_string) => ValueRootRepr::SmallF64(small_string),
            Value::SmallBigInt(small_string) => ValueRootRepr::SmallBigInt(small_string),
            _ => panic!("Value required rooting"),
        };
        Scoped::from_root_repr(key_root_repr)
    }

    pub fn from_str(agent: &mut Agent, str: &str, gc: NoGcScope<'a, '_>) -> Value<'a> {
        String::from_str(agent, str, gc).into_value()
    }

    pub fn from_string(
        agent: &mut Agent,
        string: std::string::String,
        gc: NoGcScope<'a, '_>,
    ) -> Value<'a> {
        String::from_string(agent, string, gc).into_value()
    }

    pub fn from_static_str(
        agent: &mut Agent,
        str: &'static str,
        gc: NoGcScope<'a, '_>,
    ) -> Value<'a> {
        String::from_static_str(agent, str, gc).into_value()
    }

    pub fn from_f64(agent: &mut Agent, value: f64, gc: NoGcScope<'a, '_>) -> Value<'a> {
        Number::from_f64(agent, value, gc).into_value()
    }

    pub fn from_i64(agent: &mut Agent, value: i64, gc: NoGcScope<'a, '_>) -> Value<'a> {
        Number::from_i64(agent, value, gc).into_value()
    }

    pub fn nan() -> Self {
        Number::nan().into_value()
    }

    pub fn pos_inf() -> Self {
        Number::pos_inf().into_value()
    }

    pub fn neg_inf() -> Self {
        Number::neg_inf().into_value()
    }

    pub fn pos_zero() -> Self {
        Number::pos_zero().into_value()
    }

    pub fn neg_zero() -> Self {
        Number::neg_zero().into_value()
    }

    pub fn is_true(self) -> bool {
        matches!(self, Value::Boolean(true))
    }

    pub fn is_false(self) -> bool {
        matches!(self, Value::Boolean(false))
    }

    pub fn is_object(self) -> bool {
        super::Object::try_from(self).is_ok()
    }

    pub fn is_function(self) -> bool {
        matches!(
            self,
            Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_)
        )
    }

    pub fn is_primitive(self) -> bool {
        Primitive::try_from(self).is_ok()
    }

    pub fn is_string(self) -> bool {
        matches!(self, Value::String(_) | Value::SmallString(_))
    }

    pub fn is_boolean(self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    pub fn is_null(self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_undefined(self) -> bool {
        matches!(self, Value::Undefined)
    }

    pub fn is_pos_zero(self, agent: &Agent) -> bool {
        Number::try_from(self).is_ok_and(|n| n.is_pos_zero(agent))
            || BigInt::try_from(self).is_ok_and(|n| n.is_zero(agent))
    }

    pub fn is_neg_zero(self, agent: &Agent) -> bool {
        Number::try_from(self).is_ok_and(|n| n.is_neg_zero(agent))
    }

    pub fn is_pos_infinity(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_pos_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_neg_infinity(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_neg_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_nan(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_nan(agent))
            .unwrap_or(false)
    }

    pub fn is_bigint(self) -> bool {
        matches!(self, Value::BigInt(_) | Value::SmallBigInt(_))
    }

    pub fn is_symbol(self) -> bool {
        matches!(self, Value::Symbol(_))
    }

    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Value::Number(_)
                | Value::SmallF64(_)
                | Value::Integer(_)
                | Value::BigInt(_)
                | Value::SmallBigInt(_)
        )
    }

    pub fn is_number(self) -> bool {
        matches!(
            self,
            Value::Number(_) | Value::SmallF64(_) | Value::Integer(_)
        )
    }

    pub fn is_integer(self) -> bool {
        matches!(self, Value::Integer(_))
    }

    pub fn is_empty_string(self) -> bool {
        if let Value::SmallString(s) = self {
            s.is_empty()
        } else {
            false
        }
    }

    pub fn to_number<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Number<'gc>> {
        to_number(agent, self, gc)
    }

    pub fn to_bigint<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, BigInt<'gc>> {
        to_big_int(agent, self, gc)
    }

    pub fn to_numeric<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Numeric<'gc>> {
        to_numeric(agent, self, gc)
    }

    #[inline]
    pub fn to_int32<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, i32> {
        to_int32(agent, self, gc)
    }

    #[inline]
    pub fn to_uint32<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, u32> {
        to_uint32(agent, self, gc)
    }

    #[inline]
    pub fn to_int16<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, i16> {
        to_int16(agent, self, gc)
    }

    #[inline]
    pub fn to_uint16<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, u16> {
        to_uint16(agent, self, gc)
    }

    #[inline]
    pub fn to_int8<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, i8> {
        to_int8(agent, self, gc)
    }

    #[inline]
    pub fn to_uint8<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, u8> {
        to_uint8(agent, self, gc)
    }

    #[inline]
    pub fn to_uint8_clamp<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, u8> {
        to_uint8_clamp(agent, self, gc)
    }

    #[inline]
    pub fn to_big_int64<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, i64> {
        to_big_int64(agent, self, gc)
    }

    #[inline]
    pub fn to_big_uint64<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, u64> {
        to_big_uint64(agent, self, gc)
    }

    pub fn to_string<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, String<'gc>> {
        to_string(agent, self, gc)
    }

    pub fn try_to_string<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, String<'gc>> {
        try_to_string(agent, self, gc)
    }

    /// A string conversion that will never throw, meant for things like
    /// displaying exceptions.
    pub fn string_repr<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> String<'gc> {
        if let Value::Symbol(symbol_idx) = self {
            // ToString of a symbol always throws. We use the descriptive
            // string instead (the result of `String(symbol)`).
            let gc = gc.into_nogc();
            return symbol_idx.unbind().descriptive_string(agent, gc);
        };
        match self.to_string(agent, gc) {
            Ok(result) => result,
            _ => map_object_to_static_string_repr(self),
        }
    }

    /// A string conversion that will never throw, meant for things like
    /// displaying exceptions.
    pub fn try_string_repr<'gc>(self, agent: &mut Agent, gc: NoGcScope<'gc, '_>) -> String<'gc> {
        if let Value::Symbol(symbol_idx) = self {
            // ToString of a symbol always throws. We use the descriptive
            // string instead (the result of `String(symbol)`).
            return symbol_idx.unbind().descriptive_string(agent, gc);
        };
        match try_result_into_js(self.try_to_string(agent, gc)).unwrap() {
            Some(result) => result,
            None => map_object_to_static_string_repr(self),
        }
    }

    /// ### [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub fn to_real<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, f64> {
        Ok(match self {
            Value::Number(n) => agent[n],
            Value::Integer(i) => i.into_i64() as f64,
            Value::SmallF64(f) => f.into_f64(),
            // NOTE: Converting to a number should give us a nice error message.
            _ => to_number(agent, self, gc)?.into_f64(agent),
        })
    }

    pub(crate) fn hash<H, A>(self, arena: &A, hasher: &mut H)
    where
        H: Hasher,
        A: Index<HeapString<'a>, Output = StringRecord>
            + Index<HeapNumber<'a>, Output = f64>
            + Index<HeapBigInt<'a>, Output = BigIntHeapData>,
    {
        let discriminant = core::mem::discriminant(&self);
        match self {
            Value::Undefined => discriminant.hash(hasher),
            Value::Null => discriminant.hash(hasher),
            Value::Boolean(data) => {
                discriminant.hash(hasher);
                data.hash(hasher);
            }
            Value::String(data) => {
                // Skip discriminant hashing in strings
                arena[data].data.hash(hasher);
            }
            Value::SmallString(data) => {
                data.as_wtf8().hash(hasher);
            }
            Value::Symbol(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Number(data) => {
                // Skip discriminant hashing in numbers
                arena[data].to_bits().hash(hasher);
            }
            Value::Integer(data) => {
                data.into_i64().hash(hasher);
            }
            Value::SmallF64(data) => {
                data.into_f64().to_bits().hash(hasher);
            }
            Value::BigInt(data) => {
                // Skip dsciriminant hashing in bigint numbers
                arena[data].data.hash(hasher);
            }
            Value::SmallBigInt(data) => {
                data.into_i64().hash(hasher);
            }
            _ => Object::try_from(self).unwrap().hash(hasher),
        };
    }

    pub(crate) fn try_hash<H>(self, hasher: &mut H) -> Result<(), ()>
    where
        H: Hasher,
    {
        let discriminant = core::mem::discriminant(&self);
        match self {
            Value::String(_) | Value::Number(_) | Value::BigInt(_) => {
                // These values need Agent access to hash.
                return Err(());
            }
            // All other types can be hashed on the stack.
            Value::Undefined => discriminant.hash(hasher),
            Value::Null => discriminant.hash(hasher),
            Value::Boolean(data) => {
                discriminant.hash(hasher);
                data.hash(hasher);
            }
            Value::SmallString(data) => {
                data.to_string_lossy().hash(hasher);
            }
            Value::Symbol(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Integer(data) => {
                data.into_i64().hash(hasher);
            }
            Value::SmallF64(data) => {
                data.into_f64().to_bits().hash(hasher);
            }
            Value::SmallBigInt(data) => {
                data.into_i64().hash(hasher);
            }
            _ => Object::try_from(self).unwrap().hash(hasher),
        }
        Ok(())
    }
}

impl From<bool> for Value<'_> {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

bindable_handle!(Value);

impl<'a, T> From<Option<T>> for Value<'a>
where
    T: Into<Value<'a>>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Value::Undefined, |v| v.into())
    }
}

impl TryFrom<&str> for Value<'static> {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, ()> {
        if let Ok(data) = value.try_into() {
            Ok(Value::SmallString(data))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for Value<'static> {
    type Error = ();
    fn try_from(value: f64) -> Result<Self, ()> {
        Number::try_from(value).map(|v| v.into())
    }
}

impl<'a> From<Number<'a>> for Value<'a> {
    fn from(value: Number<'a>) -> Self {
        match value {
            Number::Number(idx) => Value::Number(idx.unbind()),
            Number::Integer(data) => Value::Integer(data),
            Number::SmallF64(data) => Value::SmallF64(data),
        }
    }
}

impl From<f32> for Value<'static> {
    fn from(value: f32) -> Self {
        Value::SmallF64(SmallF64::from(value))
    }
}

impl TryFrom<i64> for Value<'static> {
    type Error = ();
    fn try_from(value: i64) -> Result<Self, ()> {
        Ok(Value::Integer(SmallInteger::try_from(value)?))
    }
}

impl<'a> TryFrom<Value<'a>> for bool {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Boolean(bool) => Ok(bool),
            _ => Err(()),
        }
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for Value<'_> {
            fn from(value: $size) -> Self {
                Value::Integer(SmallInteger::from(value))
            }
        }
    };
}

impl_value_from_n!(u8);
impl_value_from_n!(i8);
impl_value_from_n!(u16);
impl_value_from_n!(i16);
impl_value_from_n!(u32);
impl_value_from_n!(i32);

impl Rootable for Value<'_> {
    type RootRepr = ValueRootRepr;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Undefined => Ok(Self::RootRepr::Undefined),
            Self::Null => Ok(Self::RootRepr::Null),
            Self::Boolean(bool) => Ok(Self::RootRepr::Boolean(bool)),
            Self::String(heap_string) => Err(HeapRootData::String(heap_string.unbind())),
            Self::SmallString(small_string) => Ok(Self::RootRepr::SmallString(small_string)),
            Self::Symbol(symbol) => Err(HeapRootData::Symbol(symbol.unbind())),
            Self::Number(heap_number) => Err(HeapRootData::Number(heap_number.unbind())),
            Self::Integer(small_integer) => Ok(Self::RootRepr::Integer(small_integer)),
            Self::SmallF64(small_f64) => Ok(Self::RootRepr::SmallF64(small_f64)),
            Self::BigInt(heap_big_int) => Err(HeapRootData::BigInt(heap_big_int.unbind())),
            Self::SmallBigInt(small_big_int) => Ok(Self::RootRepr::SmallBigInt(small_big_int)),
            Self::Object(ordinary_object) => Err(HeapRootData::Object(ordinary_object.unbind())),
            Self::BoundFunction(bound_function) => {
                Err(HeapRootData::BoundFunction(bound_function.unbind()))
            }
            Self::BuiltinFunction(builtin_function) => {
                Err(HeapRootData::BuiltinFunction(builtin_function.unbind()))
            }
            Self::ECMAScriptFunction(ecmascript_function) => Err(HeapRootData::ECMAScriptFunction(
                ecmascript_function.unbind(),
            )),
            Self::BuiltinConstructorFunction(builtin_constructor_function) => Err(
                HeapRootData::BuiltinConstructorFunction(builtin_constructor_function.unbind()),
            ),
            Self::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Err(HeapRootData::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function.unbind(),
                ))
            }
            Self::BuiltinPromiseFinallyFunction(builtin_promise_finally_function) => {
                Err(HeapRootData::BuiltinPromiseFinallyFunction(
                    builtin_promise_finally_function.unbind(),
                ))
            }
            Self::BuiltinPromiseCollectorFunction => {
                Err(HeapRootData::BuiltinPromiseCollectorFunction)
            }
            Self::BuiltinProxyRevokerFunction => Err(HeapRootData::BuiltinProxyRevokerFunction),
            Self::PrimitiveObject(primitive_object) => {
                Err(HeapRootData::PrimitiveObject(primitive_object.unbind()))
            }
            Self::Arguments(ordinary_object) => {
                Err(HeapRootData::Arguments(ordinary_object.unbind()))
            }
            Self::Array(array) => Err(HeapRootData::Array(array.unbind())),
            #[cfg(feature = "date")]
            Self::Date(date) => Err(HeapRootData::Date(date.unbind())),
            #[cfg(feature = "temporal")]
            Self::Instant(instant) => Err(HeapRootData::Instant(instant.unbind())),
            Self::Error(error) => Err(HeapRootData::Error(error.unbind())),
            Self::FinalizationRegistry(finalization_registry) => Err(
                HeapRootData::FinalizationRegistry(finalization_registry.unbind()),
            ),
            Self::Map(map) => Err(HeapRootData::Map(map.unbind())),
            Self::Promise(promise) => Err(HeapRootData::Promise(promise.unbind())),
            Self::Proxy(proxy) => Err(HeapRootData::Proxy(proxy.unbind())),
            #[cfg(feature = "regexp")]
            Self::RegExp(reg_exp) => Err(HeapRootData::RegExp(reg_exp.unbind())),
            #[cfg(feature = "set")]
            Self::Set(set) => Err(HeapRootData::Set(set.unbind())),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(weak_map) => Err(HeapRootData::WeakMap(weak_map.unbind())),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(weak_ref) => Err(HeapRootData::WeakRef(weak_ref.unbind())),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(weak_set) => Err(HeapRootData::WeakSet(weak_set.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => Err(HeapRootData::ArrayBuffer(ab.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::DataView(dv) => Err(HeapRootData::DataView(dv.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => Err(HeapRootData::Int8Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => Err(HeapRootData::Uint8Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => Err(HeapRootData::Uint8ClampedArray(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => Err(HeapRootData::Int16Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => Err(HeapRootData::Uint16Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => Err(HeapRootData::Int32Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => Err(HeapRootData::Uint32Array(ta.unbind())),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => Err(HeapRootData::Float16Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => Err(HeapRootData::Float32Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => Err(HeapRootData::BigInt64Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => Err(HeapRootData::BigUint64Array(ta.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => Err(HeapRootData::Float64Array(ta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sab) => Err(HeapRootData::SharedArrayBuffer(sab.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sdv) => Err(HeapRootData::SharedDataView(sdv.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => Err(HeapRootData::SharedInt8Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => Err(HeapRootData::SharedUint8Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => {
                Err(HeapRootData::SharedUint8ClampedArray(sta.unbind()))
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => Err(HeapRootData::SharedInt16Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => Err(HeapRootData::SharedUint16Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => Err(HeapRootData::SharedInt32Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => Err(HeapRootData::SharedUint32Array(sta.unbind())),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => Err(HeapRootData::SharedFloat16Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => Err(HeapRootData::SharedFloat32Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => Err(HeapRootData::SharedBigInt64Array(sta.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => {
                Err(HeapRootData::SharedBigUint64Array(sta.unbind()))
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => Err(HeapRootData::SharedFloat64Array(sta.unbind())),
            Self::AsyncGenerator(r#gen) => Err(HeapRootData::AsyncGenerator(r#gen.unbind())),

            Self::ArrayIterator(array_iterator) => {
                Err(HeapRootData::ArrayIterator(array_iterator.unbind()))
            }
            #[cfg(feature = "set")]
            Self::SetIterator(set_iterator) => {
                Err(HeapRootData::SetIterator(set_iterator.unbind()))
            }
            Self::MapIterator(map_iterator) => {
                Err(HeapRootData::MapIterator(map_iterator.unbind()))
            }
            Self::Generator(generator) => Err(HeapRootData::Generator(generator.unbind())),
            Self::StringIterator(generator) => {
                Err(HeapRootData::StringIterator(generator.unbind()))
            }
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(data) => {
                Err(HeapRootData::RegExpStringIterator(data.unbind()))
            }
            Self::Module(module) => Err(HeapRootData::Module(module.unbind())),
            Self::EmbedderObject(embedder_object) => {
                Err(HeapRootData::EmbedderObject(embedder_object.unbind()))
            }
        }
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Undefined => Ok(Self::Undefined),
            Self::RootRepr::Null => Ok(Self::Null),
            Self::RootRepr::Boolean(bool) => Ok(Self::Boolean(bool)),
            Self::RootRepr::SmallString(small_string) => Ok(Self::SmallString(small_string)),
            Self::RootRepr::Integer(small_integer) => Ok(Self::Integer(small_integer)),
            Self::RootRepr::SmallF64(small_f64) => Ok(Self::SmallF64(small_f64)),
            Self::RootRepr::SmallBigInt(small_big_int) => Ok(Self::SmallBigInt(small_big_int)),
            Self::RootRepr::HeapRef(heap_root_ref) => Err(heap_root_ref),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Empty => None,
            HeapRootData::String(heap_string) => Some(Self::String(heap_string)),
            HeapRootData::Symbol(symbol) => Some(Self::Symbol(symbol)),
            HeapRootData::Number(heap_number) => Some(Self::Number(heap_number)),
            HeapRootData::BigInt(heap_big_int) => Some(Self::BigInt(heap_big_int)),
            HeapRootData::Object(ordinary_object) => Some(Self::Object(ordinary_object)),
            HeapRootData::BoundFunction(bound_function) => {
                Some(Self::BoundFunction(bound_function))
            }
            HeapRootData::BuiltinFunction(builtin_function) => {
                Some(Self::BuiltinFunction(builtin_function))
            }
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                Some(Self::ECMAScriptFunction(ecmascript_function))
            }
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => Some(
                Self::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Some(Self::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function,
                ))
            }
            HeapRootData::BuiltinPromiseFinallyFunction(builtin_promise_finally_function) => Some(
                Self::BuiltinPromiseFinallyFunction(builtin_promise_finally_function),
            ),
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Some(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Some(Self::BuiltinProxyRevokerFunction),
            HeapRootData::PrimitiveObject(primitive_object) => {
                Some(Self::PrimitiveObject(primitive_object))
            }
            HeapRootData::Arguments(ordinary_object) => Some(Self::Arguments(ordinary_object)),
            HeapRootData::Array(array) => Some(Self::Array(array)),
            #[cfg(feature = "date")]
            HeapRootData::Date(date) => Some(Self::Date(date)),
            #[cfg(feature = "temporal")]
            HeapRootData::Instant(instant) => Some(Self::Instant(instant)),
            HeapRootData::Error(error) => Some(Self::Error(error)),
            HeapRootData::FinalizationRegistry(finalization_registry) => {
                Some(Self::FinalizationRegistry(finalization_registry))
            }
            HeapRootData::Map(map) => Some(Self::Map(map)),
            HeapRootData::Promise(promise) => Some(Self::Promise(promise)),
            HeapRootData::Proxy(proxy) => Some(Self::Proxy(proxy)),
            #[cfg(feature = "regexp")]
            HeapRootData::RegExp(reg_exp) => Some(Self::RegExp(reg_exp)),
            #[cfg(feature = "set")]
            HeapRootData::Set(set) => Some(Self::Set(set)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => Some(Self::WeakMap(weak_map)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => Some(Self::WeakRef(weak_ref)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => Some(Self::WeakSet(weak_set)),

            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(ab) => Some(Self::ArrayBuffer(ab)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(dv) => Some(Self::DataView(dv)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(ta) => Some(Self::Int8Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(ta) => Some(Self::Uint8Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(ta) => Some(Self::Uint8ClampedArray(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(ta) => Some(Self::Int16Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(ta) => Some(Self::Uint16Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(ta) => Some(Self::Int32Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(ta) => Some(Self::Uint32Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(ta) => Some(Self::BigInt64Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(ta) => Some(Self::BigUint64Array(ta)),
            #[cfg(feature = "proposal-float16array")]
            HeapRootData::Float16Array(ta) => Some(Self::Float16Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(ta) => Some(Self::Float32Array(ta)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(ta) => Some(Self::Float64Array(ta)),

            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(sab) => Some(Self::SharedArrayBuffer(sab)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedDataView(sdv) => Some(Self::SharedDataView(sdv)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedInt8Array(sta) => Some(Self::SharedInt8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint8Array(sta) => Some(Self::SharedUint8Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint8ClampedArray(sta) => Some(Self::SharedUint8ClampedArray(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedInt16Array(sta) => Some(Self::SharedInt16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint16Array(sta) => Some(Self::SharedUint16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedInt32Array(sta) => Some(Self::SharedInt32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedUint32Array(sta) => Some(Self::SharedUint32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedBigInt64Array(sta) => Some(Self::SharedBigInt64Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedBigUint64Array(sta) => Some(Self::SharedBigUint64Array(sta)),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            HeapRootData::SharedFloat16Array(sta) => Some(Self::SharedFloat16Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedFloat32Array(sta) => Some(Self::SharedFloat32Array(sta)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedFloat64Array(sta) => Some(Self::SharedFloat64Array(sta)),

            HeapRootData::AsyncGenerator(r#gen) => Some(Self::AsyncGenerator(r#gen)),

            HeapRootData::ArrayIterator(array_iterator) => {
                Some(Self::ArrayIterator(array_iterator))
            }
            #[cfg(feature = "set")]
            HeapRootData::SetIterator(set_iterator) => Some(Self::SetIterator(set_iterator)),
            HeapRootData::MapIterator(map_iterator) => Some(Self::MapIterator(map_iterator)),
            HeapRootData::StringIterator(generator) => Some(Self::StringIterator(generator)),
            #[cfg(feature = "regexp")]
            HeapRootData::RegExpStringIterator(generator) => {
                Some(Self::RegExpStringIterator(generator))
            }
            HeapRootData::Generator(generator) => Some(Self::Generator(generator)),
            HeapRootData::Module(module) => Some(Self::Module(module)),
            HeapRootData::EmbedderObject(embedder_object) => {
                Some(Self::EmbedderObject(embedder_object))
            }
            HeapRootData::Executable(_)
            | HeapRootData::Realm(_)
            | HeapRootData::Script(_)
            | HeapRootData::SourceCode(_)
            | HeapRootData::SourceTextModule(_)
            | HeapRootData::AwaitReaction(_)
            | HeapRootData::PromiseReaction(_)
            | HeapRootData::PromiseGroup(_)
            | HeapRootData::DeclarativeEnvironment(_)
            | HeapRootData::FunctionEnvironment(_)
            | HeapRootData::GlobalEnvironment(_)
            | HeapRootData::ModuleEnvironment(_)
            | HeapRootData::ObjectEnvironment(_)
            | HeapRootData::PrivateEnvironment(_)
            | HeapRootData::PropertyLookupCache(_) => None,
            // Note: Do not use _ => Err(()) to make sure any added
            // HeapRootData Value variants cause compile errors if not handled.
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ValueRootRepr {
    Undefined = UNDEFINED_DISCRIMINANT,
    Null = NULL_DISCRIMINANT,
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl HeapMarkAndSweep for Value<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Self::Undefined
            | Self::Null
            | Self::Boolean(_)
            | Self::SmallString(_)
            | Self::Integer(_)
            | Self::SmallF64(_)
            | Self::SmallBigInt(_) => {
                // Stack values: Nothing to mark
            }
            Self::String(data) => data.mark_values(queues),
            Self::Symbol(data) => data.mark_values(queues),
            Self::Number(data) => data.mark_values(queues),
            Self::BigInt(data) => data.mark_values(queues),
            Self::Object(data) => data.mark_values(queues),
            Self::Array(data) => data.mark_values(queues),
            #[cfg(feature = "date")]
            Self::Date(dv) => dv.mark_values(queues),
            #[cfg(feature = "temporal")]
            Self::Instant(dv) => dv.mark_values(queues),
            Self::Error(data) => data.mark_values(queues),
            Self::BoundFunction(data) => data.mark_values(queues),
            Self::BuiltinFunction(data) => data.mark_values(queues),
            Self::ECMAScriptFunction(data) => data.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExp(data) => data.mark_values(queues),
            Self::PrimitiveObject(data) => data.mark_values(queues),
            Self::Arguments(data) => data.mark_values(queues),
            Self::FinalizationRegistry(data) => data.mark_values(queues),
            Self::Map(data) => data.mark_values(queues),
            Self::Proxy(data) => data.mark_values(queues),
            Self::Promise(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
            Self::Set(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => ab.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data) => data.mark_values(queues),
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
            Self::SharedArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(dv) => dv.mark_values(queues),
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
            Self::BuiltinConstructorFunction(data) => data.mark_values(queues),
            Self::BuiltinPromiseResolvingFunction(data) => data.mark_values(queues),
            Self::BuiltinPromiseFinallyFunction(data) => data.mark_values(queues),
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::AsyncGenerator(data) => data.mark_values(queues),
            Self::ArrayIterator(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
            Self::SetIterator(data) => data.mark_values(queues),
            Self::MapIterator(data) => data.mark_values(queues),
            Self::StringIterator(data) => data.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(data) => data.mark_values(queues),
            Self::Generator(data) => data.mark_values(queues),
            Self::Module(data) => data.mark_values(queues),
            Self::EmbedderObject(data) => data.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::Undefined
            | Self::Null
            | Self::Boolean(_)
            | Self::SmallString(_)
            | Self::Integer(_)
            | Self::SmallF64(_)
            | Self::SmallBigInt(_) => {
                // Stack values: Nothing to sweep
            }
            Self::String(data) => data.sweep_values(compactions),
            Self::Symbol(data) => data.sweep_values(compactions),
            Self::Number(data) => data.sweep_values(compactions),
            Self::BigInt(data) => data.sweep_values(compactions),
            Self::Object(data) => data.sweep_values(compactions),
            Self::Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "date")]
            Self::Date(data) => data.sweep_values(compactions),
            #[cfg(feature = "temporal")]
            Self::Instant(dv) => dv.sweep_values(compactions),
            Self::Error(data) => data.sweep_values(compactions),
            Self::BoundFunction(data) => data.sweep_values(compactions),
            Self::BuiltinFunction(data) => data.sweep_values(compactions),
            Self::ECMAScriptFunction(data) => data.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExp(data) => data.sweep_values(compactions),
            Self::PrimitiveObject(data) => data.sweep_values(compactions),
            Self::Arguments(data) => data.sweep_values(compactions),
            Self::FinalizationRegistry(data) => data.sweep_values(compactions),
            Self::Map(data) => data.sweep_values(compactions),
            Self::Proxy(data) => data.sweep_values(compactions),
            Self::Promise(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::Set(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(data) => data.sweep_values(compactions),

            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(ab) => ab.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data) => data.sweep_values(compactions),
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
            Self::SharedArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(dv) => dv.sweep_values(compactions),
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

            Self::BuiltinConstructorFunction(data) => data.sweep_values(compactions),
            Self::BuiltinPromiseResolvingFunction(data) => data.sweep_values(compactions),
            Self::BuiltinPromiseFinallyFunction(data) => data.sweep_values(compactions),
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::AsyncGenerator(data) => data.sweep_values(compactions),
            Self::ArrayIterator(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::SetIterator(data) => data.sweep_values(compactions),
            Self::MapIterator(data) => data.sweep_values(compactions),
            Self::StringIterator(data) => data.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(data) => data.sweep_values(compactions),
            Self::Generator(data) => data.sweep_values(compactions),
            Self::Module(data) => data.sweep_values(compactions),
            Self::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}

fn map_object_to_static_string_repr(value: Value) -> String<'static> {
    match Object::try_from(value).unwrap() {
        Object::BoundFunction(_)
        | Object::BuiltinFunction(_)
        | Object::ECMAScriptFunction(_)
        | Object::BuiltinConstructorFunction(_)
        | Object::BuiltinPromiseResolvingFunction(_)
        | Object::BuiltinPromiseFinallyFunction(_)
        | Object::BuiltinPromiseCollectorFunction
        | Object::BuiltinProxyRevokerFunction => BUILTIN_STRING_MEMORY._object_Function_,
        Object::Arguments(_) => BUILTIN_STRING_MEMORY._object_Arguments_,
        Object::Array(_) => BUILTIN_STRING_MEMORY._object_Array_,
        Object::Error(_) => BUILTIN_STRING_MEMORY._object_Error_,
        #[cfg(feature = "regexp")]
        Object::RegExp(_) => BUILTIN_STRING_MEMORY._object_RegExp_,
        #[cfg(feature = "regexp")]
        Object::RegExpStringIterator(_) => BUILTIN_STRING_MEMORY._object_Object_,
        Object::Module(_) => BUILTIN_STRING_MEMORY._object_Module_,
        #[cfg(feature = "array-buffer")]
        Object::ArrayBuffer(_)
        | Object::DataView(_)
        | Object::Int8Array(_)
        | Object::Uint8Array(_)
        | Object::Uint8ClampedArray(_)
        | Object::Int16Array(_)
        | Object::Uint16Array(_)
        | Object::Int32Array(_)
        | Object::Uint32Array(_)
        | Object::BigInt64Array(_)
        | Object::BigUint64Array(_)
        | Object::Float32Array(_)
        | Object::Float64Array(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(feature = "proposal-float16array")]
        Object::Float16Array(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(feature = "shared-array-buffer")]
        Object::SharedArrayBuffer(_)
        | Object::SharedDataView(_)
        | Object::SharedInt8Array(_)
        | Object::SharedUint8Array(_)
        | Object::SharedUint8ClampedArray(_)
        | Object::SharedInt16Array(_)
        | Object::SharedUint16Array(_)
        | Object::SharedInt32Array(_)
        | Object::SharedUint32Array(_)
        | Object::SharedBigInt64Array(_)
        | Object::SharedBigUint64Array(_)
        | Object::SharedFloat32Array(_)
        | Object::SharedFloat64Array(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
        Object::SharedFloat16Array(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(feature = "date")]
        Object::Date(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(feature = "temporal")]
        Object::Instant(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(feature = "set")]
        Object::Set(_) | Object::SetIterator(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(feature = "weak-refs")]
        Object::WeakMap(_) | Object::WeakRef(_) | Object::WeakSet(_) => {
            BUILTIN_STRING_MEMORY._object_Object_
        }
        Object::Object(_)
        | Object::PrimitiveObject(_)
        | Object::FinalizationRegistry(_)
        | Object::Map(_)
        | Object::Promise(_)
        | Object::Proxy(_)
        | Object::AsyncGenerator(_)
        | Object::ArrayIterator(_)
        | Object::MapIterator(_)
        | Object::StringIterator(_)
        | Object::Generator(_)
        | Object::EmbedderObject(_) => BUILTIN_STRING_MEMORY._object_Object_,
    }
}
