// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Heap constants for initializing the heap
//!
//! These define the order in which built-in prototypes and constructors
//! are placed into the heap vectors. The order is based on the [ECMAScript
//! specification](https://tc39.es/ecma262/).

use std::num::NonZeroU32;

use crate::{
    ecmascript::{ObjectShape, PropertyKey, ProtoIntrinsics, Symbol, Value},
    heap::HeapIndexHandle,
};

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum IntrinsicObjectIndexes {
    // +==================================================================+
    // | First the list of built-in prototypes and non-prototypal objects |
    // +==================================================================+

    // Fundamental objects
    ObjectPrototype,
    SymbolPrototype,
    ErrorPrototype,

    // Numbers and dates
    BigIntPrototype,
    #[cfg(feature = "math")]
    MathObject,
    #[cfg(feature = "date")]
    DatePrototype,
    #[cfg(feature = "temporal")]
    Temporal,
    #[cfg(feature = "temporal")]
    TemporalInstantPrototype,
    #[cfg(feature = "temporal")]
    TemporalDurationPrototype,

    // Text processing
    #[cfg(feature = "regexp")]
    RegExpPrototype,

    // Indexed collections
    ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    TypedArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Int8ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Uint8ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Int16ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Uint16ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Int32ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Uint32ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    BigInt64ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    BigUint64ArrayPrototype,
    #[cfg(feature = "proposal-float16array")]
    Float16ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Float32ArrayPrototype,
    #[cfg(feature = "array-buffer")]
    Float64ArrayPrototype,

    // Keyed collections
    MapPrototype,
    #[cfg(feature = "set")]
    SetPrototype,
    #[cfg(feature = "weak-refs")]
    WeakMapPrototype,
    #[cfg(feature = "weak-refs")]
    WeakSetPrototype,

    // Structured data
    #[cfg(feature = "array-buffer")]
    ArrayBufferPrototype,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBufferPrototype,
    #[cfg(feature = "array-buffer")]
    DataViewPrototype,
    #[cfg(feature = "atomics")]
    AtomicsObject,
    #[cfg(feature = "json")]
    JSONObject,

    // Managing memory
    #[cfg(feature = "weak-refs")]
    WeakRefPrototype,
    FinalizationRegistryPrototype,

    // Control abstraction objects
    IteratorPrototype,
    ArrayIteratorPrototype,
    // For-In Iterator objects are never directly accessible to ECMAScript code
    // ForInIteratorPrototype,
    AsyncIteratorPrototype,
    // Note: The AsyncFromSyncIteratorPrototype cannot be observed.
    // AsyncFromSyncIteratorPrototype,
    // The %AsyncGeneratorPrototype% object is %AsyncGeneratorFunction.prototype.prototype%.
    // AsyncGeneratorFunctionPrototypePrototype,
    MapIteratorPrototype,
    #[cfg(feature = "set")]
    SetIteratorPrototype,
    PromisePrototype,
    StringIteratorPrototype,
    GeneratorFunctionPrototype,
    // The %GeneratorPrototype% object is %GeneratorFunction.prototype.prototype%.
    // GeneratorFunctionPrototypePrototype,
    AsyncGeneratorFunctionPrototype,
    GeneratorPrototype,
    AsyncGeneratorPrototype,
    AsyncFunctionPrototype,

    // Reflection
    ReflectObject,

    // Errors subtypes
    AggregateErrorPrototype,
    EvalErrorPrototype,
    RangeErrorPrototype,
    ReferenceErrorPrototype,
    SyntaxErrorPrototype,
    TypeErrorPrototype,

    // Others
    URIErrorPrototype,
    #[cfg(feature = "regexp")]
    RegExpStringIteratorPrototype,
}
#[cfg(feature = "regexp")]
pub(crate) const LAST_INTRINSIC_OBJECT_INDEX: IntrinsicObjectIndexes =
    IntrinsicObjectIndexes::RegExpStringIteratorPrototype;
#[cfg(not(feature = "regexp"))]
pub(crate) const LAST_INTRINSIC_OBJECT_INDEX: IntrinsicObjectIndexes =
    IntrinsicObjectIndexes::URIErrorPrototype;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum IntrinsicPrimitiveObjectIndexes {
    BooleanPrototype,
    NumberPrototype,
    StringPrototype,
}
const LAST_INTRINSIC_PRIMITIVE_OBJECT_INDEX: IntrinsicPrimitiveObjectIndexes =
    IntrinsicPrimitiveObjectIndexes::StringPrototype;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum IntrinsicConstructorIndexes {
    // +===============================================+
    // | Then the list of constructor function objects |
    // +===============================================+

    // Fundamental objects
    Object,
    Function,
    FunctionPrototype,
    Boolean,
    Symbol,
    Error,

    // Numbers and dates
    Number,
    BigInt,
    #[cfg(feature = "date")]
    Date,
    #[cfg(feature = "temporal")]
    TemporalInstant,
    #[cfg(feature = "temporal")]
    TemporalDuration,

    // Text processing
    String,
    #[cfg(feature = "regexp")]
    RegExp,

    // Indexed collections
    Array,
    #[cfg(feature = "array-buffer")]
    TypedArray,
    #[cfg(feature = "array-buffer")]
    Int8Array,
    #[cfg(feature = "array-buffer")]
    Uint8Array,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray,
    #[cfg(feature = "array-buffer")]
    Int16Array,
    #[cfg(feature = "array-buffer")]
    Uint16Array,
    #[cfg(feature = "array-buffer")]
    Int32Array,
    #[cfg(feature = "array-buffer")]
    Uint32Array,
    #[cfg(feature = "array-buffer")]
    BigInt64Array,
    #[cfg(feature = "array-buffer")]
    BigUint64Array,
    #[cfg(feature = "proposal-float16array")]
    Float16Array,
    #[cfg(feature = "array-buffer")]
    Float32Array,
    #[cfg(feature = "array-buffer")]
    Float64Array,

    // Keyed collections
    Map,
    #[cfg(feature = "set")]
    Set,
    #[cfg(feature = "weak-refs")]
    WeakMap,
    #[cfg(feature = "weak-refs")]
    WeakSet,

    // Structured data
    #[cfg(feature = "array-buffer")]
    ArrayBuffer,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer,
    #[cfg(feature = "array-buffer")]
    DataView,

    // Managing memory
    #[cfg(feature = "weak-refs")]
    WeakRef,
    FinalizationRegistry,

    // Control abstraction objects
    Iterator,
    Promise,
    GeneratorFunction,
    AsyncGeneratorFunction,
    AsyncFunction,

    // Reflection
    Proxy,

    // Errors subtypes
    AggregateError,
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,

    // Others
    URIError,
}
pub(crate) const LAST_INTRINSIC_CONSTRUCTOR_INDEX: IntrinsicConstructorIndexes =
    IntrinsicConstructorIndexes::URIError;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum IntrinsicFunctionIndexes {
    // +===================================================================================+
    // | Plain functions: These do not have a corresponding object index reserved for them |
    // +===================================================================================+
    ArrayPrototypeSort,
    ArrayPrototypeToString,
    ArrayPrototypeValues,
    #[cfg(feature = "date")]
    DatePrototypeToUTCString,
    DecodeURI,
    DecodeURIComponent,
    EncodeURI,
    EncodeURIComponent,
    #[cfg(feature = "annex-b-global")]
    Escape,
    Eval,
    GeneratorFunctionPrototypePrototypeNext,
    IsFinite,
    IsNaN,
    MapPrototypeEntries,
    ObjectPrototypeToString,
    ParseFloat,
    ParseInt,
    #[cfg(feature = "regexp")]
    RegExpPrototypeExec,
    #[cfg(feature = "set")]
    SetPrototypeValues,
    StringPrototypeTrimEnd,
    StringPrototypeTrimStart,
    ThrowTypeError,
    #[cfg(feature = "array-buffer")]
    TypedArrayPrototypeValues,
    #[cfg(feature = "annex-b-global")]
    Unescape,
}
#[cfg(feature = "annex-b-global")]
pub(crate) const LAST_INTRINSIC_FUNCTION_INDEX: IntrinsicFunctionIndexes =
    IntrinsicFunctionIndexes::Unescape;
#[cfg(all(not(feature = "annex-b-global"), feature = "array-buffer"))]
pub(crate) const LAST_INTRINSIC_FUNCTION_INDEX: IntrinsicFunctionIndexes =
    IntrinsicFunctionIndexes::TypedArrayPrototypeValues;
#[cfg(all(not(feature = "annex-b-global"), not(feature = "array-buffer")))]
pub(crate) const LAST_INTRINSIC_FUNCTION_INDEX: IntrinsicFunctionIndexes =
    IntrinsicFunctionIndexes::ThrowTypeError;

impl IntrinsicObjectIndexes {
    pub(crate) const OBJECT_INDEX_OFFSET: u32 = 0;
}

impl IntrinsicPrimitiveObjectIndexes {
    pub(crate) const OBJECT_INDEX_OFFSET: u32 =
        IntrinsicObjectIndexes::OBJECT_INDEX_OFFSET + LAST_INTRINSIC_OBJECT_INDEX as u32 + 1;
    pub(crate) const PRIMITIVE_OBJECT_INDEX_OFFSET: u32 = 0;
}

impl IntrinsicConstructorIndexes {
    pub(crate) const OBJECT_INDEX_OFFSET: u32 = IntrinsicPrimitiveObjectIndexes::OBJECT_INDEX_OFFSET
        + LAST_INTRINSIC_PRIMITIVE_OBJECT_INDEX as u32
        + 1;
    pub(crate) const BUILTIN_FUNCTION_INDEX_OFFSET: u32 = 0;
}

impl IntrinsicFunctionIndexes {
    pub(crate) const BUILTIN_FUNCTION_INDEX_OFFSET: u32 =
        IntrinsicConstructorIndexes::BUILTIN_FUNCTION_INDEX_OFFSET
            + LAST_INTRINSIC_CONSTRUCTOR_INDEX as u32
            + 1;
}

pub(crate) const fn intrinsic_object_count() -> usize {
    LAST_INTRINSIC_OBJECT_INDEX as usize
        + 1
        + LAST_INTRINSIC_PRIMITIVE_OBJECT_INDEX as usize
        + 1
        + LAST_INTRINSIC_CONSTRUCTOR_INDEX as usize
        + 1
}

pub(crate) const fn intrinsic_primitive_object_count() -> usize {
    LAST_INTRINSIC_PRIMITIVE_OBJECT_INDEX as usize + 1
}

pub(crate) const fn intrinsic_function_count() -> usize {
    LAST_INTRINSIC_CONSTRUCTOR_INDEX as usize + 1 + LAST_INTRINSIC_FUNCTION_INDEX as usize + 1
}

/// Most commonly needed Object Shapes; these get created as part of Realm
/// initialisation.
///
/// Other shapes are created on-demand.
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub(crate) enum IntrinsicObjectShapes {
    Object,
    Array,
    Number,
    String,
}

impl IntrinsicObjectShapes {
    pub(crate) const fn get_object_shape_index(self, base: ObjectShape) -> ObjectShape<'static> {
        ObjectShape::from_non_zero(
            NonZeroU32::new(self as u32 + base.get_index_u32_const() + 1).unwrap(),
        )
    }

    pub(crate) const fn get_proto_intrinsic(self) -> ProtoIntrinsics {
        match self {
            Self::Object => ProtoIntrinsics::Object,
            Self::Array => ProtoIntrinsics::Array,
            Self::Number => ProtoIntrinsics::Number,
            Self::String => ProtoIntrinsics::String,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub(crate) enum WellKnownSymbolIndexes {
    AsyncIterator,
    HasInstance,
    IsConcatSpreadable,
    Iterator,
    #[cfg(feature = "regexp")]
    Match,
    #[cfg(feature = "regexp")]
    MatchAll,
    #[cfg(feature = "regexp")]
    Replace,
    #[cfg(feature = "regexp")]
    Search,
    Species,
    #[cfg(feature = "regexp")]
    Split,
    ToPrimitive,
    ToStringTag,
    Unscopables,
}

impl From<WellKnownSymbolIndexes> for Value<'_> {
    fn from(value: WellKnownSymbolIndexes) -> Self {
        Value::Symbol(value.into())
    }
}

impl From<WellKnownSymbolIndexes> for PropertyKey<'static> {
    fn from(value: WellKnownSymbolIndexes) -> Self {
        PropertyKey::Symbol(value.into())
    }
}

impl TryFrom<Symbol<'_>> for WellKnownSymbolIndexes {
    type Error = ();

    fn try_from(value: Symbol<'_>) -> Result<Self, Self::Error> {
        const ASYNCITERATOR: u32 = WellKnownSymbolIndexes::AsyncIterator as u32;
        const HASINSTANCE: u32 = WellKnownSymbolIndexes::HasInstance as u32;
        const ISCONCATSPREADABLE: u32 = WellKnownSymbolIndexes::IsConcatSpreadable as u32;
        const ITERATOR: u32 = WellKnownSymbolIndexes::Iterator as u32;
        const MATCH: u32 = WellKnownSymbolIndexes::Match as u32;
        const MATCHALL: u32 = WellKnownSymbolIndexes::MatchAll as u32;
        const REPLACE: u32 = WellKnownSymbolIndexes::Replace as u32;
        const SEARCH: u32 = WellKnownSymbolIndexes::Search as u32;
        const SPECIES: u32 = WellKnownSymbolIndexes::Species as u32;
        const SPLIT: u32 = WellKnownSymbolIndexes::Split as u32;
        const TOPRIMITIVE: u32 = WellKnownSymbolIndexes::ToPrimitive as u32;
        const TOSTRINGTAG: u32 = WellKnownSymbolIndexes::ToStringTag as u32;
        const UNSCOPABLES: u32 = WellKnownSymbolIndexes::Unscopables as u32;
        match value.get_index_u32() {
            ASYNCITERATOR => Ok(Self::AsyncIterator),
            HASINSTANCE => Ok(Self::HasInstance),
            ISCONCATSPREADABLE => Ok(Self::IsConcatSpreadable),
            ITERATOR => Ok(Self::Iterator),
            #[cfg(feature = "regexp")]
            MATCH => Ok(Self::Match),
            #[cfg(feature = "regexp")]
            MATCHALL => Ok(Self::MatchAll),
            #[cfg(feature = "regexp")]
            REPLACE => Ok(Self::Replace),
            #[cfg(feature = "regexp")]
            SEARCH => Ok(Self::Search),
            SPECIES => Ok(Self::Species),
            #[cfg(feature = "regexp")]
            SPLIT => Ok(Self::Split),
            TOPRIMITIVE => Ok(Self::ToPrimitive),
            TOSTRINGTAG => Ok(Self::ToStringTag),
            UNSCOPABLES => Ok(Self::Unscopables),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
pub(crate) const LAST_WELL_KNOWN_SYMBOL_INDEX: u32 = WellKnownSymbolIndexes::Unscopables as u32;
