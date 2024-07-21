// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::SmallInteger;

/// 56-bit double, the implied bottom 8 bits are zero.
#[derive(Clone, Copy, PartialEq)]
pub struct SmallF64 {
    data: [u8; 7],
}

impl std::fmt::Debug for SmallF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<f64>::into(*self))
    }
}

impl SmallF64 {
    pub(crate) const fn _def() -> SmallF64 {
        Self {
            data: [1, 2, 3, 4, 5, 6, 7],
        }
    }

    #[inline]
    pub fn into_f64(self) -> f64 {
        self.into()
    }

    #[inline(always)]
    fn can_convert(value: f64) -> bool {
        if value.to_bits().trailing_zeros() < 8 {
            false
        } else if value.is_nan() {
            true
        } else if value.fract() == 0.0 {
            // SmallF64 is not allowed to be an integer: It should become a
            // SmallInteger.
            if value == -0.0 {
                // Well that's not representable as an integer!
                true
            } else {
                let range =
                    const { (SmallInteger::MIN_NUMBER as f64)..=(SmallInteger::MAX_NUMBER as f64) };
                // If the integer is in the JS SafeInteger range, it should
                // become a SmallInteger: We don't want it here.
                !range.contains(&value)
            }
        } else {
            true
        }
    }

    /// SAFETY: f64 must have 8 or more trailing zeros
    #[inline]
    unsafe fn from_f64_unchecked(mut value: f64) -> SmallF64 {
        if value.is_nan() {
            // Canonicalize NaNs.
            value = f64::NAN;
        }
        let bytes = u64::to_ne_bytes(value.to_bits());

        let data = if cfg!(target_endian = "little") {
            [
                bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]
        } else {
            [
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
            ]
        };

        Self { data }
    }
}

impl TryFrom<f64> for SmallF64 {
    type Error = ();
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if SmallF64::can_convert(value) {
            // SAFETY: Checked conversion is safe.
            Ok(unsafe { Self::from_f64_unchecked(value) })
        } else {
            Err(())
        }
    }
}

impl From<f32> for SmallF64 {
    fn from(value: f32) -> Self {
        // SAFETY: All floats have 8 trailing zeros when converted to double.
        unsafe { Self::from_f64_unchecked(value as f64) }
    }
}

impl From<SmallF64> for f64 {
    fn from(value: SmallF64) -> Self {
        let SmallF64 { data } = value;

        #[repr(u8)]
        enum Repr {
            Data([u8; 7]) = 0,
        }

        // SAFETY: This matches the format on the endian platform.
        let number: u64 = unsafe { std::mem::transmute(Repr::Data(data)) };

        // The enum repr zero is the first byte
        if cfg!(target_endian = "little") {
            // In little endian that is the least significant byte, which is
            // exactly where we want it.
            f64::from_bits(number)
        } else {
            // In big endian that is the most significant byte. We want to shift
            // up one byte to "pop off" the zero and bring a zero byte in to
            // the least significant byte.
            f64::from_bits(number << 8)
        }
    }
}

#[test]
fn valid_small_integers() {
    assert_eq!(1.0 / 2.0, SmallF64::from(1.0 / 2.0).into());
    assert_eq!(1.0 / 4.0, SmallF64::from(1.0 / 4.0).into());
    assert!(SmallF64::try_from(f64::NAN).unwrap().into_f64().is_nan());
    assert!(
        SmallF64::try_from(f64::INFINITY)
            .unwrap()
            .into_f64()
            .is_infinite()
            && SmallF64::try_from(f64::INFINITY)
                .unwrap()
                .into_f64()
                .is_sign_positive()
    );
    assert!(
        SmallF64::try_from(f64::NEG_INFINITY)
            .unwrap()
            .into_f64()
            .is_infinite()
            && SmallF64::try_from(f64::NEG_INFINITY)
                .unwrap()
                .into_f64()
                .is_sign_negative()
    );
    assert_eq!(
        f64::EPSILON,
        SmallF64::try_from(f64::EPSILON).unwrap().into()
    );
}

#[test]
fn invalid_small_integers() {
    assert_eq!(SmallF64::try_from(1.0 / 3.0), Err(()));
    assert_eq!(SmallF64::try_from(f64::MAX), Err(()));
    assert_eq!(SmallF64::try_from(f64::MIN), Err(()));
}
