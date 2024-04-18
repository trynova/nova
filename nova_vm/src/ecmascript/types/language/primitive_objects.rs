#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum PrimitiveObjectData {
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(StringIndex) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Number(NumberIndex) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
    BigInt(BigIntIndex) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallInteger) = SMALL_BIGINT_DISCRIMINANT,
}

pub(crate) struct PrimitiveObjectHeapData {
    data: PrimitiveObjectData,
}