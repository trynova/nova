#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SmallString {
    /// The string will be padded to 7 bytes with the 0xFF byte, which is never
    /// contained in valid UTF-8 or WTF-8.
    bytes: [u8; 7],
}

impl std::fmt::Debug for SmallString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

impl SmallString {
    pub const EMPTY: SmallString = Self {
        bytes: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    };

    pub fn len(&self) -> usize {
        // Find the first 0xFF byte. Small strings must be valid UTF-8, and
        // UTF-8 can never contain 0xFF, so that must mark the end of the
        // string.
        self.bytes.iter().position(|&x| x == 0xFF).unwrap_or(7)
    }

    pub fn utf16_len(&self) -> usize {
        self.as_str().chars().map(char::len_utf16).sum()
    }

    // TODO: This should return a wtf8::CodePoint.
    pub fn utf16_char(&self, idx: usize) -> char {
        let mut u16_i = 0;
        for ch in self.as_str().chars() {
            if idx == u16_i {
                // TODO: Deal with surrogates.
                assert_eq!(ch.len_utf16(), 1);
                return ch;
            }
            u16_i += ch.len_utf16();
        }
        panic!("Index out of bounds");
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: Guaranteed to be UTF-8.
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice().split_at(self.len()).0
    }

    #[inline]
    pub fn data(&self) -> &[u8; 7] {
        &self.bytes
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self.bytes, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
    }

    pub const fn from_str_unchecked(string: &str) -> Self {
        let string_bytes = string.as_bytes();

        // We have only 7 bytes to work with, so we must fail to convert if the
        // string is longer than that.
        debug_assert!(string_bytes.len() < 8);

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
}

impl TryFrom<&str> for SmallString {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // We have only 7 bytes to work with, so we must fail to convert if the
        // string is longer than that.
        if value.len() < 8 {
            Ok(Self::from_str_unchecked(value))
        } else {
            Err(())
        }
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
