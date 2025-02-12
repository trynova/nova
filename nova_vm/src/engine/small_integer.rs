// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// 56-bit signed integer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SmallInteger {
    data: [u8; 7],
}

impl std::fmt::Debug for SmallInteger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<i64>::into(*self))
    }
}

impl std::hash::Hash for SmallInteger {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.into_i64().hash(state);
    }
}

impl SmallInteger {
    pub const MIN_BIGINT: i64 = -(2i64.pow(55));
    pub const MAX_BIGINT: i64 = 2i64.pow(55) - 1;

    pub const MIN_NUMBER: i64 = -(2i64.pow(53)) + 1;
    pub const MAX_NUMBER: i64 = 2i64.pow(53) - 1;

    #[inline]
    pub fn into_i64(self) -> i64 {
        self.into()
    }

    pub const fn zero() -> SmallInteger {
        Self {
            data: [0, 0, 0, 0, 0, 0, 0],
        }
    }

    fn from_i64_unchecked(value: i64) -> SmallInteger {
        debug_assert!((Self::MIN_BIGINT..=Self::MAX_BIGINT).contains(&value));
        let bytes = i64::to_ne_bytes(value);

        let data = if cfg!(target_endian = "little") {
            [
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
            ]
        } else {
            [
                bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]
        };

        Self { data }
    }
}

impl std::ops::Neg for SmallInteger {
    type Output = Self;

    /// ## Panics
    /// - If the negation overflows.
    fn neg(self) -> Self::Output {
        Self::from_i64_unchecked(-self.into_i64())
    }
}

impl std::ops::Not for SmallInteger {
    type Output = Self;
    fn not(self) -> Self::Output {
        // NOTE: This is safe because the bitwise not of any number in the range
        // will always be in the safe number range.
        Self::from_i64_unchecked(!self.into_i64())
    }
}

impl TryFrom<i64> for SmallInteger {
    type Error = ();
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if (Self::MIN_BIGINT..=Self::MAX_BIGINT).contains(&value) {
            Ok(Self::from_i64_unchecked(value))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for SmallInteger {
    type Error = ();
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.fract() == 0.0 && (Self::MIN_BIGINT..=Self::MAX_BIGINT).contains(&(value as i64)) {
            Ok(Self::from_i64_unchecked(value as i64))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f32> for SmallInteger {
    type Error = ();
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if value.fract() == 0.0 && (Self::MIN_BIGINT..=Self::MAX_BIGINT).contains(&(value as i64)) {
            Ok(Self::from_i64_unchecked(value as i64))
        } else {
            Err(())
        }
    }
}

impl TryFrom<i128> for SmallInteger {
    type Error = ();
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        if (Self::MIN_BIGINT as i128..=Self::MAX_BIGINT as i128).contains(&value) {
            Ok(Self::from_i64_unchecked(value as i64))
        } else {
            Err(())
        }
    }
}

impl TryFrom<u64> for SmallInteger {
    type Error = ();
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value <= (Self::MAX_BIGINT as u64) {
            Ok(Self::from_i64_unchecked(value as i64))
        } else {
            Err(())
        }
    }
}

macro_rules! from_numeric_type {
    ($numtype:ty) => {
        // Checking at compile-time that $numtype fully fits within the range.
        const _: () = {
            assert!(
                <$numtype>::MIN as i64 >= SmallInteger::MIN_BIGINT,
                concat!(
                    stringify!($numtype),
                    " is outside of the SmallInteger range (min)"
                )
            );
            assert!(
                <$numtype>::MAX as i64 <= SmallInteger::MAX_BIGINT,
                concat!(
                    stringify!($numtype),
                    " is outside of the SmallInteger range (max)"
                )
            );
        };
        impl From<$numtype> for SmallInteger {
            fn from(value: $numtype) -> Self {
                Self::from_i64_unchecked(i64::from(value))
            }
        }
    };
}
from_numeric_type!(u8);
from_numeric_type!(i8);
from_numeric_type!(u16);
from_numeric_type!(i16);
from_numeric_type!(u32);
from_numeric_type!(i32);

impl From<SmallInteger> for i64 {
    fn from(value: SmallInteger) -> Self {
        let SmallInteger { data } = value;

        #[repr(u8)]
        enum Repr {
            Data([u8; 7]),
        }

        // SAFETY: This matches the format on the endian platform.
        let number: i64 = unsafe { std::mem::transmute(Repr::Data(data)) };

        if cfg!(target_endian = "little") {
            number >> 8
        } else {
            number << 8 >> 8
        }
    }
}

#[test]
fn valid_small_integers() {
    assert_eq!(0i64, SmallInteger::from(0).into_i64());
    assert_eq!(5i64, SmallInteger::from(5).into_i64());
    assert_eq!(23i64, SmallInteger::from(23).into_i64());
    assert_eq!(
        SmallInteger::MAX_NUMBER + 1,
        SmallInteger::try_from(SmallInteger::MAX_NUMBER + 1)
            .unwrap()
            .into_i64()
    );
    assert_eq!(
        SmallInteger::MAX_BIGINT,
        SmallInteger::try_from(SmallInteger::MAX_BIGINT)
            .unwrap()
            .into_i64()
    );

    assert_eq!(-5i64, SmallInteger::from(-5).into_i64());
    assert_eq!(-59i64, SmallInteger::from(-59).into_i64());
    assert_eq!(
        SmallInteger::MIN_NUMBER - 1,
        SmallInteger::try_from(SmallInteger::MIN_NUMBER - 1)
            .unwrap()
            .into_i64()
    );
    assert_eq!(
        SmallInteger::MIN_BIGINT,
        SmallInteger::try_from(SmallInteger::MIN_BIGINT)
            .unwrap()
            .into_i64()
    );
}

#[test]
fn invalid_small_integers() {
    assert_eq!(
        SmallInteger::try_from(SmallInteger::MAX_BIGINT + 1),
        Err(())
    );
    assert_eq!(SmallInteger::try_from(i64::MAX), Err(()));
    assert_eq!(
        SmallInteger::try_from(SmallInteger::MIN_BIGINT - 1),
        Err(())
    );
    assert_eq!(SmallInteger::try_from(i64::MIN), Err(()));
}
