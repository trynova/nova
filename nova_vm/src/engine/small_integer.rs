/// 56-bit signed integer.
#[derive(Clone, Copy, PartialEq)]
pub struct SmallInteger {
    data: [u8; 7],
}

impl std::fmt::Debug for SmallInteger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<i64>::into(*self))
    }
}

impl SmallInteger {
    pub const MIN_BIGINT: i64 = -2i64.pow(55);
    pub const MAX_BIGINT: i64 = 2i64.pow(55) - 1;

    pub const MIN_NUMBER: i64 = -2i64.pow(53) + 1;
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

    pub fn from_i64_unchecked(value: i64) -> SmallInteger {
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
    assert_eq!(0i64, SmallInteger::try_from(0).unwrap().into());
    assert_eq!(5i64, SmallInteger::try_from(5).unwrap().into());
    assert_eq!(23i64, SmallInteger::try_from(23).unwrap().into());
    assert_eq!(
        SmallInteger::MAX_NUMBER + 1,
        SmallInteger::try_from(SmallInteger::MAX_NUMBER + 1)
            .unwrap()
            .into()
    );
    assert_eq!(
        SmallInteger::MAX_BIGINT,
        SmallInteger::try_from(SmallInteger::MAX_BIGINT)
            .unwrap()
            .into()
    );

    assert_eq!(-5i64, SmallInteger::try_from(-5).unwrap().into());
    assert_eq!(-59i64, SmallInteger::try_from(-59).unwrap().into());
    assert_eq!(
        SmallInteger::MIN_NUMBER - 1,
        SmallInteger::try_from(SmallInteger::MIN_NUMBER - 1)
            .unwrap()
            .into()
    );
    assert_eq!(
        SmallInteger::MIN_BIGINT,
        SmallInteger::try_from(SmallInteger::MIN_BIGINT)
            .unwrap()
            .into()
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
