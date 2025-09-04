// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cmp::Ordering;
use std::borrow::Cow;

use wtf8::{CodePoint, Wtf8};

/// Maximum number of bytes a [SmallString] can inline.
const MAX_LEN: usize = 7;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SmallString {
    /// The string will be padded to 7 bytes with the 0xFF byte, which is never
    /// contained in valid UTF-8 or WTF-8.
    bytes: [u8; MAX_LEN],
}

impl Ord for SmallString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_wtf8().cmp(other.as_wtf8())
    }
}

impl PartialOrd for SmallString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<str> for SmallString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_bytes().eq(other.as_bytes())
    }
}

impl PartialEq<&str> for SmallString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl core::fmt::Debug for SmallString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "\"{}\"", self.to_string_lossy())
    }
}

impl SmallString {
    pub const EMPTY: SmallString = Self {
        bytes: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    };

    pub const fn len(&self) -> usize {
        // Find the first 0xFF byte. Small strings must be valid UTF-8, and
        // UTF-8 can never contain 0xFF, so that must mark the end of the
        // string.
        let mut position: u8 = 0;
        loop {
            let is_end_byte = self.bytes[position as usize] == 0xFF;
            if is_end_byte {
                break;
            }
            position += 1;
            if position == MAX_LEN as u8 {
                break;
            }
        }
        position as usize
    }

    /// Returns true if the SmallString contains only ASCII characters.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use small_string::SmallString;
    /// assert!(SmallString::try_from("abc").unwrap().is_ascii());
    /// assert!(!SmallString::try_from("📦").unwrap().is_ascii());
    /// ```
    pub const fn is_ascii(&self) -> bool {
        let mut i = 0;
        while i < self.bytes.len() {
            let byte = self.bytes[i];
            // Padding byte means end of string.
            if byte == 0xFF {
                break;
            }
            if !byte.is_ascii() {
                return false;
            }
            i += 1;
        }
        true
    }

    pub fn utf16_len(&self) -> usize {
        if self.is_ascii() {
            return self.len();
        }
        self.as_wtf8()
            .code_points()
            .map(|cp| {
                let cp = cp.to_u32();
                if (cp & 0xFFFF) == cp { 1 } else { 2 }
            })
            .sum()
    }

    /// Find a CodePoint at a given u16 index; this will give a full CodePoint
    /// even when the index points at a latter surrogate pair half: in this
    /// case the returned boolean will be `true`.
    fn get_code_point_at(&self, idx: usize) -> (CodePoint, bool) {
        let buf = self.as_wtf8();
        let mut i = 0;
        for char in buf.code_points() {
            if i == idx {
                return (char, false);
            }
            let code = char.to_u32();
            // SAFETY: the WTF-16 index cannot overflow as otherwise the WTF-8
            // index would've overflowed a long time ago.
            i = unsafe { i.unchecked_add(if (code & 0xFFFF) == code { 1 } else { 2 }) };
            if i > idx {
                // We're asking for the latter half of this surrogate pair.
                return (char, true);
            }
            // Our match is still further in the buffer.
        }
        unreachable!("Could not find code point index");
    }

    // TODO: This should return a wtf8::CodePoint.
    pub fn char_code_at(&self, idx: usize) -> CodePoint {
        if self.is_ascii() {
            // SAFETY: ASCII is valid UTF-8.
            return unsafe {
                CodePoint::from_u32_unchecked(self.as_str_unchecked().as_bytes()[idx] as u32)
            };
        }
        let (char, take_latter_half) = self.get_code_point_at(idx);
        let code = char.to_u32();
        if (code & 0xFFFF) == code {
            // Single-char character.
            return char;
        }
        let char = char
            .to_char()
            .expect("Surrogate pair did not map to a char");
        let encoded = &mut [0; 2];
        let enc = char.encode_utf16(encoded);
        // Note: since this is a surrogate pair, it should always
        // encode into two u16s.
        debug_assert_eq!(enc.len(), 2);
        let surrogate = encoded[if take_latter_half { 1 } else { 0 }];
        // SAFETY: 0..0xFFFF is always less than 0x10FFFF.
        unsafe { CodePoint::from_u32_unchecked(surrogate as u32) }
    }

    /// Get the CodePoint at a given WTF-16 index.
    pub fn code_point_at(self, idx: usize) -> CodePoint {
        if self.is_ascii() {
            // SAFETY: ASCII is valid UTF-8.
            return unsafe {
                CodePoint::from_u32_unchecked(self.as_str_unchecked().as_bytes()[idx] as u32)
            };
        }
        let encoded = &mut [0; 2];
        let (char, take_latter_half) = self.get_code_point_at(idx);
        if take_latter_half {
            // We're asking for the latter half of this surrogate pair.
            let char = char
                .to_char()
                .expect("Surrogate pair did not map to a char");
            let encoded = char.encode_utf16(encoded);
            // Note: since this is a surrogate pair, it should always
            // encode into two u16s.
            debug_assert_eq!(encoded.len(), 2);
            let surrogate = encoded[1];
            // SAFETY: 0..0xFFFF is always less than 0x10FFFF.
            unsafe { CodePoint::from_u32_unchecked(surrogate as u32) }
        } else {
            char
        }
    }

    pub fn utf8_index(&self, utf16_idx: usize) -> Option<usize> {
        if self.is_ascii() {
            return Some(utf16_idx);
        }
        let mut current_utf16_index = 0;
        let mut scratch = [0u16; 2];
        for (idx, ch) in self.as_wtf8().code_points().enumerate() {
            match current_utf16_index.cmp(&utf16_idx) {
                Ordering::Equal => return Some(idx),
                Ordering::Greater => return None,
                Ordering::Less => {
                    current_utf16_index += ch
                        .to_char()
                        .map(|ch| ch.encode_utf16(&mut scratch).len())
                        .unwrap_or(1)
                }
            }
        }
        if current_utf16_index > utf16_idx {
            return None;
        }
        debug_assert_eq!(utf16_idx, current_utf16_index);
        Some(self.len())
    }

    pub fn utf16_index(&self, utf8_idx: usize) -> usize {
        if self.is_ascii() {
            return utf8_idx;
        }
        let mut utf16_idx = 0;
        for (idx, ch) in self.to_string_lossy().char_indices() {
            if idx == utf8_idx {
                return utf16_idx;
            }
            assert!(idx < utf8_idx);
            utf16_idx += ch.len_utf16();
        }

        assert_eq!(utf8_idx, self.len());
        utf16_idx
    }

    /// Lossily convert the string to UTF-8.
    /// Return an UTF-8 `&str` slice if the contents are well-formed in UTF-8.
    ///
    /// Surrogates are replaced with `"\u{FFFD}"` (the replacement character “�”).
    ///
    /// This only copies the data if necessary (if it contains any surrogate).
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.as_wtf8().to_string_lossy()
    }

    /// Try to convert the string to UTF-8 and return a `&str` slice.
    ///
    /// Return `None` if the string contains surrogates.
    ///
    /// This does not copy the data.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        self.as_wtf8().as_str()
    }

    /// Get the SmallString data as a string slice without checking that the SmallString contains valid UTF-8.
    ///
    /// ## Safety
    ///
    /// The SmallString must contain valid UTF-8.
    #[inline]
    pub const unsafe fn as_str_unchecked(&self) -> &str {
        // SAFETY: caller promises they've checked the UTF-8 validity.
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub const fn as_wtf8(&self) -> &Wtf8 {
        // SAFETY: guaranteed to be WTF-8.
        unsafe { core::mem::transmute::<&[u8], &Wtf8>(self.as_bytes()) }
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice().split_at(self.len()).0
    }

    #[inline]
    pub const fn data(&self) -> &[u8; MAX_LEN] {
        &self.bytes
    }

    #[inline]
    pub const fn data_mut(&mut self) -> &mut [u8; MAX_LEN] {
        &mut self.bytes
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        matches!(self.bytes, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
    }

    /// Create a [SmallString] from a [str] without checking that it is small
    /// enough to fit in the inline buffer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `string` 7 bytes or fewer long.
    pub const unsafe fn from_str_unchecked(string: &str) -> Self {
        let string_bytes = string.as_bytes();

        // We have only 7 bytes to work with, so we must fail to convert if the
        // string is longer than that.
        unsafe { std::hint::assert_unchecked(string_bytes.len() <= MAX_LEN) };

        match string_bytes.len() {
            0 => Self {
                bytes: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            },
            1 => Self {
                bytes: [string_bytes[0], 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            },
            2 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    0xFF,
                    0xFF,
                    0xFF,
                    0xFF,
                    0xFF,
                ],
            },
            3 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    0xFF,
                    0xFF,
                    0xFF,
                    0xFF,
                ],
            },
            4 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    0xFF,
                    0xFF,
                    0xFF,
                ],
            },
            5 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    string_bytes[4],
                    0xFF,
                    0xFF,
                ],
            },
            6 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    string_bytes[4],
                    string_bytes[5],
                    0xFF,
                ],
            },
            7 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    string_bytes[4],
                    string_bytes[5],
                    string_bytes[6],
                ],
            },
            _ => unreachable!(),
        }
    }

    /// Inline a [Wtf8] into a [SmallString].
    ///
    /// # Panics
    ///
    /// If `string` is longer than 7 bytes.
    #[inline]
    pub fn from_wtf8(string: &Wtf8) -> Self {
        assert!(string.len() <= MAX_LEN);
        unsafe { Self::from_wtf8_unchecked(string) }
    }

    /// Create a [SmallString] from a [Wtf8] without checking that it is small
    /// enough to fit in the inline buffer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `string` 7 bytes or fewer long.
    pub const unsafe fn from_wtf8_unchecked(string: &Wtf8) -> Self {
        // SAFETY: The backing data of a WTF8 buffer is indeed a u8 buffer.
        // This is very sketchy but completely safe.
        let string_bytes = unsafe { core::mem::transmute::<&Wtf8, &[u8]>(string) };

        // We have only 7 bytes to work with, so we must fail to convert if the
        // string is longer than that.
        unsafe { std::hint::assert_unchecked(string_bytes.len() <= MAX_LEN) };

        match string_bytes.len() {
            0 => Self {
                bytes: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            },
            1 => Self {
                bytes: [string_bytes[0], 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            },
            2 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    0xFF,
                    0xFF,
                    0xFF,
                    0xFF,
                    0xFF,
                ],
            },
            3 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    0xFF,
                    0xFF,
                    0xFF,
                    0xFF,
                ],
            },
            4 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    0xFF,
                    0xFF,
                    0xFF,
                ],
            },
            5 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    string_bytes[4],
                    0xFF,
                    0xFF,
                ],
            },
            6 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    string_bytes[4],
                    string_bytes[5],
                    0xFF,
                ],
            },
            7 => Self {
                bytes: [
                    string_bytes[0],
                    string_bytes[1],
                    string_bytes[2],
                    string_bytes[3],
                    string_bytes[4],
                    string_bytes[5],
                    string_bytes[6],
                ],
            },
            _ => unreachable!(),
        }
    }

    pub fn from_char(ch: char) -> Self {
        let mut bytes = [0xFF; MAX_LEN];
        ch.encode_utf8(&mut bytes);
        SmallString { bytes }
    }

    pub fn from_code_point(ch: CodePoint) -> Self {
        if let Some(char) = ch.to_char() {
            Self::from_char(char)
        } else {
            let mut bytes = [0xFFu8; MAX_LEN];

            // Lone surrogate: these are U+D800 to U+DFFF.
            let p = ch.to_u32();
            debug_assert!(p <= 0xFFFF);
            let p = p as u16;
            bytes[0] = (0xE0 | (p >> 12)) as u8;
            bytes[1] = (0x80 | ((p >> 6) & 0x3F)) as u8;
            bytes[2] = (0x80 | (p & 0x3F)) as u8;
            SmallString { bytes }
        }
    }
}

impl TryFrom<&str> for SmallString {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // We have only 7 bytes to work with, so we must fail to convert if the
        // string is longer than that.
        if value.len() <= MAX_LEN {
            // SAFETY: we just checked that the string is 7 bytes or fewer.
            Ok(unsafe { Self::from_str_unchecked(value) })
        } else {
            Err(())
        }
    }
}

impl TryFrom<&Wtf8> for SmallString {
    type Error = ();
    fn try_from(value: &Wtf8) -> Result<Self, Self::Error> {
        // We have only 7 bytes to work with, so we must fail to convert if the
        // string is longer than that.
        if value.len() <= MAX_LEN {
            // SAFETY: we just checked that the string is 7 bytes or fewer.
            Ok(unsafe { Self::from_wtf8_unchecked(value) })
        } else {
            Err(())
        }
    }
}

impl From<char> for SmallString {
    fn from(ch: char) -> Self {
        Self::from_char(ch)
    }
}

#[test]
fn valid_stack_strings() {
    assert!(SmallString::try_from("").is_ok());
    assert_eq!(SmallString::try_from("").unwrap().len(), 0);
    assert!(SmallString::try_from("asd").is_ok());
    assert_eq!(SmallString::try_from("asd").unwrap().len(), 3);
    assert!(SmallString::try_from("asdasd").is_ok());
    assert_eq!(SmallString::try_from("asdasd").unwrap().len(), 6);
    assert!(SmallString::try_from("asdasda").is_ok());
    assert_eq!(SmallString::try_from("asdasda").unwrap().len(), 7);
    assert!(SmallString::try_from("asd76fd").is_ok());
    assert_eq!(SmallString::try_from("asd76fd").unwrap().len(), 7);
    assert!(SmallString::try_from("💩").is_ok());
    assert_eq!(SmallString::try_from("💩 ").unwrap().len(), 5);
    assert!(SmallString::try_from("asd\0foo").is_ok());
    assert_eq!(SmallString::try_from("asd\0foo").unwrap().len(), 7);
    assert!(SmallString::try_from("asdfoo\0").is_ok());
    assert_eq!(SmallString::try_from("asdfoo\0").unwrap().len(), 7);
}

#[test]
fn not_valid_stack_strings() {
    assert!(SmallString::try_from("asd asd r 547 gdfg").is_err());
}

#[test]
fn test_ascii() {
    let ascii = ["", "abc", "a\0bc"];
    for s in ascii {
        assert!(SmallString::try_from(s).unwrap().is_ascii());
    }

    let non_ascii = ["📦", "f📦"];
    for s in non_ascii {
        assert!(!SmallString::try_from(s).unwrap().is_ascii());
    }
}

#[test]
fn str_conversion() {
    let unicode = "🤗";
    let str = SmallString::try_from(unicode).unwrap();
    assert_eq!(str.len(), 4);
    assert_eq!(str, unicode);

    let str = SmallString::try_from(Wtf8::from_str(unicode)).unwrap();
    assert_eq!(str.len(), 4);
    assert_eq!(str, unicode);

    // less than 7 characters, but more than 7 bytes
    let too_large_unicode = "🤗🤗🤗";
    assert!(SmallString::try_from(too_large_unicode).is_err());
    assert!(SmallString::try_from(Wtf8::from_str(too_large_unicode)).is_err());
}
