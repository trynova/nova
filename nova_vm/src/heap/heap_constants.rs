//! Heap constants for initializing the heap
//!
//! These define the order in which built-in prototypes and constructors
//! are placed into the heap vectors. The order is based on the ECMAScript
//! definition found in https://tc39.es/ecma262/

// +==================================================================+
// | First the list of built-in prototypes and non-prototypal objects |
// +==================================================================+

use super::indexes::{BuiltinFunctionIndex, ObjectIndex, SymbolIndex};

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum BuiltinObjectIndexes {
    // Fundamental objects
    ObjectPrototype,
    FunctionPrototype,
    BooleanPrototype,
    SymbolPrototype,
    ErrorPrototype,

    // Numbers and dates
    NumberPrototype,
    BigintPrototype,
    MathObject,
    DatePrototype,

    // Text processing
    StringPrototype,
    RegExpPrototype,

    // Indexed collections
    ArrayPrototype,
    Int8ArrayPrototype,
    Uint8ArrayPrototype,
    Uint8ClampedArrayPrototype,
    Int16ArrayPrototype,
    Uint16ArrayPrototype,
    Int32ArrayPrototype,
    Uint32ArrayPrototype,
    BigInt64ArrayPrototype,
    BigUint64ArrayPrototype,
    Float32ArrayPrototype,
    Float64ArrayPrototype,

    // Keyed collections
    MapPrototype,
    SetPrototype,
    WeakMapPrototype,
    WeakSetPrototype,

    // Structured data
    ArrayBufferPrototype,
    SharedArrayBufferPrototype,
    DataViewPrototype,
    AtomicsObject,
    JsonObject,

    // Managing memory
    WeakRefPrototype,
    FinalizationRegistryPrototype,

    // Control abstraction objects
    IteratorPrototype,
    AsyncIteratorPrototype,
    PromisePrototype,
    GeneratorFunctionPrototype,
    AsyncGeneratorFunctionPrototype,
    GeneratorPrototype,
    AsyncGeneratorPrototype,
    AsyncFunctionPrototype,

    // Reflection
    ReflectObject,
    ModulePrototype,

    // +===============================================+
    // | Then the list of constructor function objects |
    // +===============================================+

    // Fundamental objects
    ObjectConstructor,
    FunctionConstructor,
    BooleanConstructor,
    SymbolConstructor,
    ErrorConstructor,

    // Numbers and dates
    NumberConstructor,
    BigintConstructor,
    DateConstructor,

    // Text processing
    StringConstructor,
    RegExpConstructor,

    // Indexed collections
    ArrayConstructor,
    Int8ArrayConstructor,
    Uint8ArrayConstructor,
    Uint8ClampedArrayConstructor,
    Int16ArrayConstructor,
    Uint16ArrayConstructor,
    Int32ArrayConstructor,
    Uint32ArrayConstructor,
    BigInt64ArrayConstructor,
    BigUint64ArrayConstructor,
    Float32ArrayConstructor,
    Float64ArrayConstructor,

    // Keyed collections
    MapConstructor,
    SetConstructor,
    WeakMapConstructor,
    WeakSetConstructor,

    // Structured data
    ArrayBufferConstructor,
    SharedArrayBufferConstructor,
    DataViewConstructor,

    // Managing memory
    WeakRefConstructor,
    FinalizationRegistryConstructor,

    // Control abstraction objects
    PromiseConstructor,
    GeneratorFunctionConstructor,
    AsyncGeneratorFunctionConstructor,
    AsyncFunctionConstructor,

    // Reflection
    ProxyConstructor,
}

impl From<BuiltinObjectIndexes> for ObjectIndex {
    fn from(value: BuiltinObjectIndexes) -> ObjectIndex {
        ObjectIndex::from_u32_index(value as u32)
    }
}

impl From<BuiltinObjectIndexes> for BuiltinFunctionIndex {
    fn from(value: BuiltinObjectIndexes) -> BuiltinFunctionIndex {
        // We do not allow more than 16 777 216 functions to exist.
        assert!(value as u32 <= u32::pow(2, 24));
        BuiltinFunctionIndex::from_u32_index(value as u32)
    }
}

impl Default for BuiltinObjectIndexes {
    fn default() -> Self {
        Self::ObjectPrototype
    }
}

pub const LAST_BUILTIN_OBJECT_INDEX: u32 = BuiltinObjectIndexes::ProxyConstructor as u32;
pub const FIRST_CONSTRUCTOR_INDEX: u32 = BuiltinObjectIndexes::ObjectConstructor as u32;

pub const fn get_constructor_index(object_index: BuiltinObjectIndexes) -> BuiltinFunctionIndex {
    BuiltinFunctionIndex::from_u32_index(object_index as u32 - FIRST_CONSTRUCTOR_INDEX)
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub(crate) enum WellKnownSymbolIndexes {
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

impl From<WellKnownSymbolIndexes> for SymbolIndex {
    fn from(value: WellKnownSymbolIndexes) -> Self {
        SymbolIndex::from_u32_index(value as u32)
    }
}

pub const LAST_WELL_KNOWN_SYMBOL_INDEX: u32 = WellKnownSymbolIndexes::Unscopables as u32;
