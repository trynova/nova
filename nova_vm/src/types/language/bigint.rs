use super::value::{BIGINT_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT};
use crate::{heap::indexes::BigIntIndex, SmallInteger};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum BigInt {
    BigInt(BigIntIndex) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallInteger) = SMALL_BIGINT_DISCRIMINANT,
}
