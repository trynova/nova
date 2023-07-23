use std::fmt::Debug;

/// Small UTF-8 string of up to 7 bytes in length
///
/// The string is terminated after either the last non-null
/// byte or at the end of the byte slice. In essence
/// this is a mix between a CStr and a str.
#[derive(Copy, Clone)]
pub struct StackString {
    bytes: [u8; 7],
}

impl StackString {
    // TODO: Need to get the length, and UTF-16 length, of the string.

    pub fn byte_len(&self) -> usize {
        // Find the last non-null character and add one to its index to get length.
        self.bytes
            .as_slice()
            .iter()
            .rev()
            .position(|&x| x != 0)
            .map_or(0, |i| 7 - i)
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: Guaranteed to be ASCII, which is a subset of UTF-8.
        unsafe { &std::str::from_utf8_unchecked(self.as_slice()) }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.data().as_slice().split_at(self.byte_len()).0
    }

    #[inline(always)]
    pub fn data(&self) -> &[u8; 7] {
        return &self.bytes;
    }

    // TODO: try_from_X should return Result. Option is smaller and we
    // do not care about the reason why conversion failed so we prefer
    // that but the method name should be changed.
    pub(crate) fn try_from_str(string: &str) -> Option<Self> {
        let string_bytes = string.as_bytes();
        if string_bytes.len() > 7 || string_bytes.last() == Some(&0) {
            // We have only 7 bytes to work with, and we cannot tell apart
            // UTF-8 strings that end with a null byte from our null
            // terminator so we must fail to convert on those.
            return None;
        };
        let mut this = StackString {
            bytes: [0, 0, 0, 0, 0, 0, 0],
        };
        this.bytes
            .as_mut_slice()
            .split_at_mut(string_bytes.len())
            .0
            .copy_from_slice(string_bytes);
        Some(this)
    }
}

impl Debug for StackString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

#[test]
fn valid_stack_strings() {
    assert!(StackString::try_from_str("").is_some());
    assert_eq!(StackString::try_from_str("").unwrap().byte_len(), 0);
    assert!(StackString::try_from_str("asd").is_some());
    assert_eq!(StackString::try_from_str("asd").unwrap().byte_len(), 3);
    assert!(StackString::try_from_str("asdasd").is_some());
    assert_eq!(StackString::try_from_str("asdasd").unwrap().byte_len(), 6);
    assert!(StackString::try_from_str("asdasda").is_some());
    assert_eq!(StackString::try_from_str("asdasda").unwrap().byte_len(), 7);
    assert!(StackString::try_from_str("asd76fd").is_some());
    assert_eq!(StackString::try_from_str("asd76fd").unwrap().byte_len(), 7);
    assert!(StackString::try_from_str("💩").is_some());
    assert_eq!(StackString::try_from_str("💩").unwrap().byte_len(), 4);
    assert!(StackString::try_from_str("asd\0foo").is_some());
    assert_eq!(StackString::try_from_str("asd\0foo").unwrap().byte_len(), 7);
}

#[test]
fn not_valid_stack_strings() {
    assert!(StackString::try_from_str("asd asd r 547 gdfg").is_none());
    assert!(StackString::try_from_str("asdfoo\0").is_none());
}
