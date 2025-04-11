#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum SpecificationValue {
    // Bottom types
    Undefined = 1,
    Null,

    // Primitive types
    Boolean,
    String,
    SmallString,
    Symbol,
    Number,
    Integer,
    Float,
    BigInt,
    SmallBigInt,
    
    // Ordinary object
    Object,

    // Primitive value objects, useless things
    BigIntObject,
    BooleanObject,
    NumberObject,
    StringObject,
    SymbolObject,

    // Well-known (exotic) object types
    Arguments,
    Array,
    ArrayBuffer,
    DataView,
    Date,
    Error,
    FinalizationRegistry,
    Map,
    Promise,
    RegExp,
    Set,
    SharedArrayBuffer,
    WeakMap,
    WeakRef,
    WeakSet,

    // TypedArrays
    Int8Array,
    Uint8Array,
    Uint8ClampedArray,
    Int16Array,
    Uint16Array,
    Int32Array,
    Uint32Array,
    BigInt64Array,
    BigUint64Array,
    Float32Array,
    Float64Array,

    // Functions
    BoundFunction,
    BuiltinFunction,
    ECMAScriptAsyncFunction,
    ECMAScriptAsyncGeneratorFunction,
    ECMAScriptConstructorFunction,
    ECMAScriptFunction,
    ECMAScriptGeneratorFunction,
    PromiseResolvingFunction,
    PromisesResolvingFunction,

    // Iterator objects
    AsyncFromSyncIterator,
    AsyncIterator,
    Iterator,

    // ECMAScript Module
    Module,

    // Embedder objects
    EmbedderObject = 0x7f,

    /// ### [6.2.5 The Reference Record Specification Type](https://tc39.es/ecma262/#sec-reference-record-specification-type)
    ReferenceRecord = 0x80,

    // Thrown Values: Any ECMAScript Value can be thrown, hence each Value
    // requires a thrown variant.
    ThrownUndefined = SpecificationValue::Undefined as u8 + 0x80,
    ThrownNull = SpecificationValue::Null as u8 + 0x80,
    ThrownBoolean = SpecificationValue::Boolean as u8 + 0x80,
    ThrownString = SpecificationValue::String as u8 + 0x80,
    ThrownSmallString = SpecificationValue::SmallString as u8 + 0x80,
    ThrownSymbol = SpecificationValue::Symbol as u8 + 0x80,
    ThrownNumber = SpecificationValue::Number as u8 + 0x80,
    ThrownInteger = SpecificationValue::Integer as u8 + 0x80,
    ThrownFloat = SpecificationValue::Float as u8 + 0x80,
    ThrownBigInt = SpecificationValue::BigInt as u8 + 0x80,
    ThrownSmallBigInt = SpecificationValue::SmallBigInt as u8 + 0x80,
    ThrownBigIntObject = SpecificationValue::BigIntObject as u8 + 0x80,
    ThrownBooleanObject = SpecificationValue::BooleanObject as u8 + 0x80,
    ThrownNumberObject = SpecificationValue::NumberObject as u8 + 0x80,
    ThrownStringObject = SpecificationValue::StringObject as u8 + 0x80,
    ThrownSymbolObject = SpecificationValue::SymbolObject as u8 + 0x80,
    ThrownObject = SpecificationValue::Object as u8 + 0x80,
    ThrownArguments = SpecificationValue::Arguments as u8 + 0x80,
    ThrownArray = SpecificationValue::Array as u8 + 0x80,
    ThrownArrayBuffer = SpecificationValue::ArrayBuffer as u8 + 0x80,
    ThrownDataView = SpecificationValue::DataView as u8 + 0x80,
    ThrownDate = SpecificationValue::Date as u8 + 0x80,
    ThrownError = SpecificationValue::Error as u8 + 0x80,
    ThrownFinalizationRegistry = SpecificationValue::FinalizationRegistry as u8 + 0x80,
    ThrownMap = SpecificationValue::Map as u8 + 0x80,
    ThrownPromise = SpecificationValue::Promise as u8 + 0x80,
    ThrownRegExp = SpecificationValue::RegExp as u8 + 0x80,
    ThrownSet = SpecificationValue::Set as u8 + 0x80,
    ThrownSharedArrayBuffer = SpecificationValue::SharedArrayBuffer as u8 + 0x80,
    ThrownWeakMap = SpecificationValue::WeakMap as u8 + 0x80,
    ThrownWeakRef = SpecificationValue::WeakRef as u8 + 0x80,
    ThrownWeakSet = SpecificationValue::WeakSet as u8 + 0x80,
    ThrownInt8Array = SpecificationValue::Int8Array as u8 + 0x80,
    ThrownUint8Array = SpecificationValue::Uint8Array as u8 + 0x80,
    ThrownUint8ClampedArray = SpecificationValue::Uint8ClampedArray as u8 + 0x80,
    ThrownInt16Array = SpecificationValue::Int16Array as u8 + 0x80,
    ThrownUint16Array = SpecificationValue::Uint16Array as u8 + 0x80,
    ThrownInt32Array = SpecificationValue::Int32Array as u8 + 0x80,
    ThrownUint32Array = SpecificationValue::Uint32Array as u8 + 0x80,
    ThrownBigInt64Array = SpecificationValue::BigInt64Array as u8 + 0x80,
    ThrownBigUint64Array = SpecificationValue::BigUint64Array as u8 + 0x80,
    ThrownFloat32Array = SpecificationValue::Float32Array as u8 + 0x80,
    ThrownFloat64Array = SpecificationValue::Float64Array as u8 + 0x80,
    ThrownAsyncGeneratorFunction = SpecificationValue::ECMAScriptAsyncGeneratorFunction as u8 + 0x80,
    ThrownBoundFunction = SpecificationValue::BoundFunction as u8 + 0x80,
    ThrownBuiltinFunction = SpecificationValue::BuiltinFunction as u8 + 0x80,
    ThrownECMASCriptAsyncFunction = SpecificationValue::ECMAScriptAsyncFunction as u8 + 0x80,
    ThrownECMAScriptConstructorFunction = SpecificationValue::ECMAScriptConstructorFunction as u8 + 0x80,
    ThrownECMAScriptFunction = SpecificationValue::ECMAScriptFunction as u8 + 0x80,
    ThrownECMAScriptGeneratorFunction = SpecificationValue::ECMAScriptGeneratorFunction as u8 + 0x80,
    ThrownECMAScriptPromiseResolvingFunction = SpecificationValue::PromiseResolvingFunction as u8 + 0x80,
    ThrownPromisesResolvingFunction = SpecificationValue::PromisesResolvingFunction as u8 + 0x80,
    ThrownAsyncFromSyncIterator = SpecificationValue::AsyncFromSyncIterator as u8 + 0x80,
    ThrownAsyncIterator = SpecificationValue::AsyncIterator as u8 + 0x80,
    ThrownIterator = SpecificationValue::Iterator as u8 + 0x80,
    ThrownModule = SpecificationValue::Module as u8 + 0x80,

}
