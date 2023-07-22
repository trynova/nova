use std::fmt::Debug;

/// Small ASCII string of up to 7 bytes in length
///
/// The string is terminated at either the first null
/// byte or at the end of the byte slice. In essence
/// this is a mix between a CStr and a str.
#[derive(Copy, Clone)]
pub struct SmallAsciiString([u8; 7]);

impl SmallAsciiString {
    pub fn len(&self) -> usize {
        self.0
            .as_slice()
            .iter()
            .position(|&x| x == 0)
            .or(Some(7))
            .unwrap()
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: Guaranteed to be ASCII, which is a subset of UTF-8.
        unsafe { &std::str::from_utf8_unchecked(self.as_slice().split_at(self.len()).0) }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub(crate) fn try_from_str(string: &str) -> Option<Self> {
        let bytes_iterator = string.as_bytes();
        if bytes_iterator.len() > 7 {
            return None;
        };
        let mut this = SmallAsciiString([0, 0, 0, 0, 0, 0, 0]);
        let mut is_ascii = true;
        bytes_iterator.iter().enumerate().for_each(|(i, &c)| {
            this.0[i] = c;
            is_ascii = is_ascii && c != 0 && c <= 126
        });
        if is_ascii {
            Some(this)
        } else {
            None
        }
    }
}

impl Debug for SmallAsciiString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

#[test]
fn small_ascii_strings() {
    assert!(SmallAsciiString::try_from_str("asd").is_some());
    assert!(SmallAsciiString::try_from_str("asdasd").is_some());
    assert!(SmallAsciiString::try_from_str("asdasda").is_some());
    assert!(SmallAsciiString::try_from_str("asd76fd").is_some());
}

#[test]
fn not_small_ascii_strings() {
    assert!(SmallAsciiString::try_from_str("asd asd r 547 gdfg").is_none());
    assert!(SmallAsciiString::try_from_str("asd\0foo").is_none());
    assert!(SmallAsciiString::try_from_str("💩").is_none());
}
