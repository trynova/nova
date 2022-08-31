use std::cmp::Ordering;

#[derive(Clone)]
pub enum JsNumber {
    U32(u32),
    F64(f64),
}
impl JsNumber {
    pub const ZERO: JsNumber = JsNumber::U32(0);
    pub const INFINITY: JsNumber = JsNumber::F64(f64::INFINITY);
    pub const NEG_INFINITY: JsNumber = JsNumber::F64(f64::NEG_INFINITY);
    pub const NAN: JsNumber = JsNumber::F64(f64::NAN);

    pub fn from_u32(v: u32) -> JsNumber {
        JsNumber::U32(v)
    }
    pub fn from_f64(v: f64) -> JsNumber {
        if v.is_nan() {
            Self::NAN
        } else {
            JsNumber::F64(v)
        }
    }

    pub fn as_u8_clamp(&self) -> u8 {
        match self {
            Self::U32(s) => *s.min(&255) as u8,
            Self::F64(s) => {
                if s.is_nan() || *s >= 255. {
                    255
                } else if *s <= 0. {
                    0
                } else {
                    let f = s.floor();
                    if f + 0.5 < *s || (f + 0.5 == *s && f % 2. == 1.) {
                        f as u8 + 1
                    } else {
                        f as u8
                    }
                }
            }
        }
    }
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::U32(s) => *s as f64,
            Self::F64(s) => *s,
        }
    }
}
impl PartialEq for JsNumber {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::U32(l), Self::U32(r)) => l == r,
            (Self::F64(l), Self::F64(r)) => l == r,
            _ => self.as_f64() == other.as_f64(),
        }
    }
}
impl PartialOrd for JsNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::U32(l), Self::U32(r)) => l.partial_cmp(r),
            (Self::F64(l), Self::F64(r)) => l.partial_cmp(r),
            _ => self.as_f64().partial_cmp(&other.as_f64()),
        }
    }
}

macro_rules! js_number_as {
    ($unsigned_ty:ty, $unsigned_method:ident, $signed_ty:ty, $signed_method:ident) => {
        impl JsNumber {
            pub fn $unsigned_method(&self) -> $unsigned_ty {
                match self {
                    Self::U32(s) => *s as $unsigned_ty,
                    Self::F64(s) => {
                        if !s.is_finite() || *s == 0.0 {
                            0
                        } else {
                            let modulus = (<$unsigned_ty>::MAX as f64) + 1.0;
                            let possibly_negative = s.trunc() % modulus;
                            let modulo = if possibly_negative < 0.0 {
                                possibly_negative + modulus
                            } else {
                                possibly_negative
                            };
                            modulo as $unsigned_ty
                        }
                    }
                }
            }
            #[inline]
            pub fn $signed_method(&self) -> $signed_ty {
                self.$unsigned_method() as $signed_ty
            }
        }
    };
}
js_number_as!(u8, as_u8, i8, as_i8);
js_number_as!(u16, as_u16, i16, as_i16);
js_number_as!(u32, as_u32, i32, as_i32);

macro_rules! js_number_from {
    ($type:ty, u32) => {
        impl From<$type> for JsNumber {
            fn from(v: $type) -> Self {
                JsNumber::U32(v as u32)
            }
        }
    };
    ($type:ty, f64) => {
        impl From<$type> for JsNumber {
            fn from(v: $type) -> Self {
                JsNumber::F64(v as f64)
            }
        }
    };
}
js_number_from!(u8, u32);
js_number_from!(u16, u32);
js_number_from!(u32, u32);
js_number_from!(u64, f64);
js_number_from!(i8, f64);
js_number_from!(i16, f64);
js_number_from!(i32, f64);
js_number_from!(i64, f64);
js_number_from!(f32, f64);
js_number_from!(f64, f64);
