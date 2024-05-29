use crate::{
    ecmascript::types::{
        bigint::{HeapBigInt, SmallBigInt},
        BigInt, HeapNumber, HeapString, Number, OrdinaryObject, String, Symbol,
        BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT,
        NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT,
        STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
    },
    heap::indexes::SymbolIndex,
    SmallInteger,
};
use small_string::SmallString;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum PrimitiveObjectData {
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(HeapString) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Symbol(SymbolIndex) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
pub struct PrimitiveObjectHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) data: PrimitiveObjectData,
}

impl PrimitiveObjectHeapData {
    pub(crate) fn new_big_int_object(big_int: BigInt) -> Self {
        let data = match big_int {
            BigInt::BigInt(data) => PrimitiveObjectData::BigInt(data),
            BigInt::SmallBigInt(data) => PrimitiveObjectData::SmallBigInt(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_boolean_object(boolean: bool) -> Self {
        Self {
            object_index: None,
            data: PrimitiveObjectData::Boolean(boolean),
        }
    }

    pub(crate) fn new_number_object(number: Number) -> Self {
        let data = match number {
            Number::Number(data) => PrimitiveObjectData::Number(data),
            Number::Integer(data) => PrimitiveObjectData::Integer(data),
            Number::Float(data) => PrimitiveObjectData::Float(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_string_object(string: String) -> Self {
        let data = match string {
            String::String(data) => PrimitiveObjectData::String(data),
            String::SmallString(data) => PrimitiveObjectData::SmallString(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_symbol_object(symbol: Symbol) -> Self {
        Self {
            object_index: None,
            data: PrimitiveObjectData::Symbol(symbol.0),
        }
    }
}
