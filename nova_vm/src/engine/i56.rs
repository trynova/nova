/// 56-bit signed integer.
#[derive(Clone, Copy, PartialEq)]
pub struct I56 {
    data: [u8; 7],
}

impl std::fmt::Debug for I56 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<i64>::into(*self))
    }
}

impl I56 {
    pub const MIN_BIGINT: i64 = -(2i64.pow(55));
    pub const MAX_BIGINT: i64 = 2i64.pow(55) - 1;

    pub const MIN_NUMBER: i64 = -(2i64.pow(53)) + 1;
    pub const MAX_NUMBER: i64 = 2i64.pow(53) - 1;

    #[inline]
    pub fn into_i64(self) -> i64 {
        self.into()
    }

    pub const fn zero() -> I56 {
        Self {
            data: [0, 0, 0, 0, 0, 0, 0],
        }
    }

    fn from_i64_unchecked(value: i64) -> I56 {
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

impl std::ops::Neg for I56 {
    type Output = Self;

    /// ## Panics
    /// - If the negation overflows.
    fn neg(self) -> Self::Output {
        Self::from_i64_unchecked(-self.into_i64())
    }
}

impl std::ops::Not for I56 {
    type Output = Self;
    fn not(self) -> Self::Output {
        // NOTE: This is safe because the bitwise not of any number in the range
        // will always be in the safe number range.
        Self::from_i64_unchecked(!self.into_i64())
    }
}

impl TryFrom<i64> for I56 {
    type Error = ();
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if (Self::MIN_BIGINT..=Self::MAX_BIGINT).contains(&value) {
            Ok(Self::from_i64_unchecked(value))
        } else {
            Err(())
        }
    }
}

impl TryFrom<i128> for I56 {
    type Error = ();
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        if (Self::MIN_BIGINT as i128..=Self::MAX_BIGINT as i128).contains(&value) {
            Ok(Self::from_i64_unchecked(value as i64))
        } else {
            Err(())
        }
    }
}

impl TryFrom<u64> for I56 {
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
                <$numtype>::MIN as i64 >= I56::MIN_BIGINT,
                concat!(stringify!($numtype), " is outside of the I56 range (min)")
            );
            assert!(
                <$numtype>::MAX as i64 <= I56::MAX_BIGINT,
                concat!(stringify!($numtype), " is outside of the I56 range (max)")
            );
        };
        impl From<$numtype> for I56 {
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

impl From<I56> for i64 {
    fn from(value: I56) -> Self {
        let I56 { data } = value;

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
    assert_eq!(0i64, I56::try_from(0).unwrap().into());
    assert_eq!(5i64, I56::try_from(5).unwrap().into());
    assert_eq!(23i64, I56::try_from(23).unwrap().into());
    assert_eq!(
        I56::MAX_NUMBER + 1,
        I56::try_from(I56::MAX_NUMBER + 1).unwrap().into()
    );
    assert_eq!(
        I56::MAX_BIGINT,
        I56::try_from(I56::MAX_BIGINT).unwrap().into()
    );

    assert_eq!(-5i64, I56::try_from(-5).unwrap().into());
    assert_eq!(-59i64, I56::try_from(-59).unwrap().into());
    assert_eq!(
        I56::MIN_NUMBER - 1,
        I56::try_from(I56::MIN_NUMBER - 1).unwrap().into()
    );
    assert_eq!(
        I56::MIN_BIGINT,
        I56::try_from(I56::MIN_BIGINT).unwrap().into()
    );
}

#[test]
fn invalid_small_integers() {
    assert_eq!(I56::try_from(I56::MAX_BIGINT + 1), Err(()));
    assert_eq!(I56::try_from(i64::MAX), Err(()));
    assert_eq!(I56::try_from(I56::MIN_BIGINT - 1), Err(()));
    assert_eq!(I56::try_from(i64::MIN), Err(()));
}
