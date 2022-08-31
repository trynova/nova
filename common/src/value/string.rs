use gc::{Finalize, Gc, Trace};
use std::borrow::Cow;

#[derive(Trace, Finalize)]
pub enum JsString {
    Latin1(Box<[u8]>),
    Utf16(Box<[u16]>),
}
impl JsString {
    pub fn from_latin1(src: &[u8]) -> Gc<JsString> {
        let mut copy = Vec::with_capacity(src.len());
        copy.extend_from_slice(src);
        Self::from_latin1_owned(copy)
    }
    pub fn from_utf8(src: &str) -> Gc<JsString> {
        if src.is_ascii() {
            Self::from_latin1(src.as_bytes())
        } else {
            let copy = src.encode_utf16().collect();
            Self::from_utf16_owned(copy)
        }
    }
    pub fn from_utf16(src: &[u16]) -> Gc<JsString> {
        let mut copy = Vec::with_capacity(src.len());
        copy.extend_from_slice(src);
        Self::from_utf16_owned(copy)
    }
    pub fn from_latin1_owned(src: Vec<u8>) -> Gc<JsString> {
        Gc::new(JsString::Latin1(src.into_boxed_slice()))
    }
    pub fn from_utf16_owned(src: Vec<u16>) -> Gc<JsString> {
        Gc::new(JsString::Utf16(src.into_boxed_slice()))
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Latin1(str) => str.len(),
            Self::Utf16(str) => str.len(),
        }
    }

    pub fn as_latin1(&self) -> Result<Cow<[u8]>, ()> {
        match self {
            Self::Latin1(str) => Ok(Cow::Borrowed(str)),
            Self::Utf16(str) => {
                let mut owned = Vec::with_capacity(str.len());
                for ch in str.iter() {
                    if *ch > 0xFF {
                        return Err(());
                    }
                    owned.push(*ch as u8);
                }
                Ok(Cow::Owned(owned))
            }
        }
    }
    fn as_utf8(&self) -> Result<Cow<str>, ()> {
        match self {
            Self::Latin1(str) => {
                match std::str::from_utf8(str) {
                    Ok(utf8_str) => Ok(Cow::Borrowed(utf8_str)),
                    Err(e) => {
                        let mut string = String::with_capacity(str.len());
                        // TODO(andreubotella): Look into using std::str::from_utf8_unchecked.
                        string.push_str(std::str::from_utf8(&str[..e.valid_up_to()]).unwrap());
                        Ok(Cow::Owned(string))
                    }
                }
            }
            Self::Utf16(str) => {
                let mut owned = String::with_capacity(str.len());
                for ch in char::decode_utf16(str.iter().copied()) {
                    if ch.is_ok() {
                        owned.push(ch.unwrap());
                    } else {
                        return Err(());
                    }
                }
                Ok(Cow::Owned(owned))
            }
        }
    }
    fn as_utf8_lossy(&self) -> Cow<str> {
        match self {
            Self::Latin1(str) => {
                match std::str::from_utf8(str) {
                    Ok(utf8_str) => Cow::Borrowed(utf8_str),
                    Err(e) => {
                        let mut string = String::with_capacity(str.len());
                        // TODO(andreubotella): Look into using std::str::from_utf8_unchecked.
                        string.push_str(std::str::from_utf8(&str[..e.valid_up_to()]).unwrap());
                        Cow::Owned(string)
                    }
                }
            }
            Self::Utf16(str) => {
                let mut owned = String::with_capacity(str.len());
                for ch in char::decode_utf16(str.iter().copied()) {
                    if ch.is_ok() {
                        owned.push(ch.unwrap());
                    } else {
                        owned.push(char::REPLACEMENT_CHARACTER);
                    }
                }
                Cow::Owned(owned)
            }
        }
    }
    fn as_utf16(&self) -> Cow<[u16]> {
        match self {
            Self::Latin1(str) => {
                let owned = str.iter().map(|b| *b as u16).collect();
                Cow::Owned(owned)
            }
            Self::Utf16(str) => Cow::Borrowed(str),
        }
    }
}

impl PartialEq for JsString {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Latin1(l0), Self::Latin1(r0)) => l0 == r0,
            (Self::Latin1(l0), Self::Utf16(r0)) => {
                if l0.len() != r0.len() {
                    return false;
                }
                for (l, r) in l0.iter().copied().zip(r0.iter().copied()) {
                    if l as u16 != r {
                        return false;
                    }
                }
                true
            }
            (Self::Utf16(l0), Self::Utf16(r0)) => l0 == r0,
            (Self::Utf16(l0), Self::Latin1(r0)) => {
                if l0.len() != r0.len() {
                    return false;
                }
                for (l, r) in l0.iter().copied().zip(r0.iter().copied()) {
                    if l != r as u16 {
                        return false;
                    }
                }
                true
            }
        }
    }
}
impl Eq for JsString {}
