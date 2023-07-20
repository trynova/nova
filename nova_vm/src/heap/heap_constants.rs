//! Heap constants for initializing the heap
//!
//! These define the order in which built-in prototypes and constructors
//! are placed into the heap vectors. The order is based on the ECMAScript
//! definition found in https://tc39.es/ecma262/

// +==================================================================+
// | First the list of built-in prototypes and non-prototypal objects |
// +==================================================================+

#[repr(u32)]
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
    RegexpPrototypeIndex,

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
    RegexpConstructorIndex,

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

pub const LAST_BUILTIN_OBJECT_INDEX: u32 = BuiltinObjectIndexes::ProxyConstructorIndex as u32;
pub const FIRST_CONSTRUCTOR_INDEX: u32 = BuiltinObjectIndexes::ObjectConstructorIndex as u32;

pub const fn get_constructor_index(object_index: BuiltinObjectIndexes) -> u32 {
    object_index as u32 - FIRST_CONSTRUCTOR_INDEX
}
