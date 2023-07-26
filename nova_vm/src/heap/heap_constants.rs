//! Heap constants for initializing the heap
//!
//! These define the order in which built-in prototypes and constructors
//! are placed into the heap vectors. The order is based on the ECMAScript
//! definition found in https://tc39.es/ecma262/

// +==================================================================+
// | First the list of built-in prototypes and non-prototypal objects |
// +==================================================================+

use super::indexes::{FunctionIndex, ObjectIndex, SymbolIndex};

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum BuiltinObjectIndexes {
    // Fundamental objects
    ObjectPrototypeIndex,
    FunctionPrototypeIndex,
    BooleanPrototypeIndex,
    SymbolPrototypeIndex,
    ErrorPrototypeIndex,

    // Numbers and dates
    NumberPrototypeIndex,
    BigintPrototypeIndex,
    MathObjectIndex,
    DatePrototypeIndex,

    // Text processing
    StringPrototypeIndex,
    RegExpPrototypeIndex,

    // Indexed collections
    ArrayPrototypeIndex,
    Int8ArrayPrototypeIndex,
    Uint8ArrayPrototypeIndex,
    Uint8ClampedArrayPrototypeIndex,
    Int16ArrayPrototypeIndex,
    Uint16ArrayPrototypeIndex,
    Int32ArrayPrototypeIndex,
    Uint32ArrayPrototypeIndex,
    BigInt64ArrayPrototypeIndex,
    BigUint64ArrayPrototypeIndex,
    Float32ArrayPrototypeIndex,
    Float64ArrayPrototypeIndex,

    // Keyed collections
    MapPrototypeIndex,
    SetPrototypeIndex,
    WeakMapPrototypeIndex,
    WeakSetPrototypeIndex,

    // Structured data
    ArrayBufferPrototypeIndex,
    SharedArrayBufferPrototypeIndex,
    DataViewPrototypeIndex,
    AtomicsObjectIndex,
    JsonObjectIndex,

    // Managing memory
    WeakRefPrototypeIndex,
    FinalizationRegistryPrototypeIndex,

    // Control abstraction objects
    IteratorPrototypeIndex,
    AsyncIteratorPrototypeIndex,
    PromisePrototypeIndex,
    GeneratorFunctionPrototypeIndex,
    AsyncGeneratorFunctionPrototypeIndex,
    GeneratorPrototypeIndex,
    AsyncGeneratorPrototypeIndex,
    AsyncFunctionPrototypeIndex,

    // Reflection
    ReflectObjectIndex,
    ModulePrototypeIndex,

    // +===============================================+
    // | Then the list of constructor function objects |
    // +===============================================+

    // Fundamental objects
    ObjectConstructorIndex,
    FunctionConstructorIndex,
    BooleanConstructorIndex,
    SymbolConstructorIndex,
    ErrorConstructorIndex,

    // Numbers and dates
    NumberConstructorIndex,
    BigintConstructorIndex,
    DateConstructorIndex,

    // Text processing
    StringConstructorIndex,
    RegExpConstructorIndex,

    // Indexed collections
    ArrayConstructorIndex,
    Int8ArrayConstructorIndex,
    Uint8ArrayConstructorIndex,
    Uint8ClampedArrayConstructorIndex,
    Int16ArrayConstructorIndex,
    Uint16ArrayConstructorIndex,
    Int32ArrayConstructorIndex,
    Uint32ArrayConstructorIndex,
    BigInt64ArrayConstructorIndex,
    BigUint64ArrayConstructorIndex,
    Float32ArrayConstructorIndex,
    Float64ArrayConstructorIndex,

    // Keyed collections
    MapConstructorIndex,
    SetConstructorIndex,
    WeakMapConstructorIndex,
    WeakSetConstructorIndex,

    // Structured data
    ArrayBufferConstructorIndex,
    SharedArrayBufferConstructorIndex,
    DataViewConstructorIndex,

    // Managing memory
    WeakRefConstructorIndex,
    FinalizationRegistryConstructorIndex,

    // Control abstraction objects
    PromiseConstructorIndex,
    GeneratorFunctionConstructorIndex,
    AsyncGeneratorFunctionConstructorIndex,
    AsyncFunctionConstructorIndex,

    // Reflection
    ProxyConstructorIndex,
}

impl Into<ObjectIndex> for BuiltinObjectIndexes {
    fn into(self) -> ObjectIndex {
        ObjectIndex::from_u32_index(self as u32)
    }
}

impl Into<FunctionIndex> for BuiltinObjectIndexes {
    fn into(self) -> FunctionIndex {
        // We do not allow more than 16 777 216 functions to exist.
        assert!(self as u32 <= u32::pow(2, 24));
        FunctionIndex::from_u32_index(self as u32)
    }
}

impl Default for BuiltinObjectIndexes {
    fn default() -> Self {
        Self::ObjectPrototypeIndex
    }
}

pub const LAST_BUILTIN_OBJECT_INDEX: u32 = BuiltinObjectIndexes::ProxyConstructorIndex as u32;
pub const FIRST_CONSTRUCTOR_INDEX: u32 = BuiltinObjectIndexes::ObjectConstructorIndex as u32;

pub const fn get_constructor_index(object_index: BuiltinObjectIndexes) -> FunctionIndex {
    FunctionIndex::from_u32_index(object_index as u32 - FIRST_CONSTRUCTOR_INDEX)
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum WellKnownSymbolIndexes {
    AsyncIterator,
    HasInstance,
    IsConcatSpreadable,
    Iterator,
    Match,
    MatchAll,
    Replace,
    Search,
    Species,
    Split,
    ToPrimitive,
    ToStringTag,
    Unscopables,
}

impl Into<SymbolIndex> for WellKnownSymbolIndexes {
    fn into(self) -> SymbolIndex {
        SymbolIndex::from_u32_index(self as u32)
    }
}

pub const LAST_WELL_KNOWN_SYMBOL_INDEX: u32 = WellKnownSymbolIndexes::Unscopables as u32;
