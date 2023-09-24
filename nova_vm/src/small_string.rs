#[derive(Clone, Copy)]
pub struct SmallString {
    bytes: [u8; 7],
}

impl std::fmt::Debug for SmallString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

impl SmallString {
    pub fn len(&self) -> usize {
        // Find the last non-null character and add one to its index to get length.
        self.bytes
            .as_slice()
            .iter()
            .rev()
            .position(|&x| x != 0)
            .map_or(0, |i| 7 - i)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: Guaranteed to be UTF-8.
        unsafe { &std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes.as_slice().split_at(self.len()).0
    }

    #[inline]
    pub fn data(&self) -> &[u8; 7] {
        return &self.bytes;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self.bytes, [0, 0, 0, 0, 0, 0, 0])
    }

    pub(crate) fn from_str_unchecked(string: &str) -> Self {
        let string_bytes = string.as_bytes();

        // We have only 7 bytes to work with, and we cannot tell apart
        // UTF-8 strings that end with a null byte from our null
        // terminator so we must fail to convert on those.
        debug_assert!(string_bytes.len() < 8 && string_bytes.last() != Some(&0));

        let mut bytes = [0, 0, 0, 0, 0, 0, 0];
        bytes
            .as_mut_slice()
            .split_at_mut(string_bytes.len())
            .0
            .copy_from_slice(string_bytes);

        Self { bytes }
    }
}

impl TryFrom<&str> for SmallString {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // We have only 7 bytes to work with, and we cannot tell apart
        // UTF-8 strings that end with a null byte from our null
        // terminator so we must fail to convert on those.
        if value.len() < 8 && value.as_bytes().last() != Some(&0) {
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
}

#[test]
fn not_valid_stack_strings() {
    assert!(SmallString::try_from("asd asd r 547 gdfg").is_err());
    assert!(SmallString::try_from("asdfoo\0").is_err());
}
