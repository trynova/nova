#[derive(Clone, Copy)]
pub struct SmallString {
    data: [u8; 7],
}

impl std::fmt::Debug for SmallString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

impl SmallString {
    pub fn len(&self) -> usize {
        self.data.iter().position(|byte| *byte == 0).unwrap_or(7)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data[0..self.len()]
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: Guaranteed to be valid UTF-8.
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    pub(crate) fn from_str_unchecked(value: &str) -> Self {
        let len = value.len();
        assert!(len < 8);
        let mut data: [u8; 7] = [0, 0, 0, 0, 0, 0, 0];
        let data_slice = &mut data.as_mut_slice()[0..len];
        data_slice.copy_from_slice(value.as_bytes());
        Self { data }
    }
}

impl TryFrom<&str> for SmallString {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() < 8 {
            Ok(Self::from_str_unchecked(value))
        } else {
            Err(())
        }
    }
}
