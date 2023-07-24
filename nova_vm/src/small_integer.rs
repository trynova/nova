/// 56-bit signed integer.
#[derive(Clone, Copy)]
pub struct SmallInteger {
    data: [u8; 7],
}

impl std::fmt::Debug for SmallInteger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<i64>::into(*self))
    }
}

impl SmallInteger {
    pub const MIN: i64 = -(2 as i64).pow(56) / 2 + 1;
    pub const MAX: i64 = (2 as i64).pow(56) / 2 - 1;

    pub(crate) fn from_i64_unchecked(value: i64) -> SmallInteger {
        debug_assert!(value >= Self::MIN && value <= Self::MAX);
        let mut data: [u8; 7] = [0, 0, 0, 0, 0, 0, 0];
        let bytes = i64::to_ne_bytes(value);
        if cfg!(target_endian = "little") {
            data.copy_from_slice(&bytes[0..7]);
        } else {
            data.copy_from_slice(&bytes[1..8]);
        }
        Self { data }
    }
}

impl TryFrom<i64> for SmallInteger {
    type Error = ();
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self::from_i64_unchecked(value))
        } else {
            Err(())
        }
    }
}

impl Into<i64> for SmallInteger {
    fn into(self) -> i64 {
        let Self { data } = self;
        let mut bytes: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
        bytes.copy_from_slice(&data);
        if cfg!(target_endian = "big") {
            bytes.copy_within(0..7, 1);
        }
        // SAFETY: The format is guaranteed to match `from_i64_unchecked`.
        unsafe { std::mem::transmute(bytes) }
    }
}
