use small_string::SmallString;

use crate::{
    ecmascript::types::{
        BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT,
        NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT,
        STRING_DISCRIMINANT,
    },
    heap::indexes::{BigIntIndex, NumberIndex, ObjectIndex, StringIndex},
    SmallInteger,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum PrimitiveObjectData {
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(StringIndex) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Number(NumberIndex) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
    BigInt(BigIntIndex) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallInteger) = SMALL_BIGINT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PrimitiveObjectHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) data: PrimitiveObjectData,
}
