use wtf8::{Wtf8, Wtf8Buf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringHeapData {
    // pub utf16_len: Option<u32>,
    pub(crate) data: StringBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StringBuffer {
    Owned(Wtf8Buf),
    Static(&'static Wtf8),
}

impl StringHeapData {
    pub fn len(&self) -> usize {
        match &self.data {
            StringBuffer::Owned(buf) => buf.len(),
            StringBuffer::Static(buf) => buf.len(),
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.data {
            StringBuffer::Owned(buf) => buf.as_str().unwrap(),
            StringBuffer::Static(buf) => buf.as_str().unwrap(),
        }
    }

    pub fn from_str(str: &str) -> Self {
        StringHeapData {
            data: StringBuffer::Owned(Wtf8Buf::from_str(str)),
        }
    }

    pub fn from_static_str(str: &'static str) -> Self {
        debug_assert!(str.len() > 7);
        StringHeapData {
            data: StringBuffer::Static(Wtf8::from_str(str)),
        }
    }

    pub fn from_string(str: String) -> Self {
        StringHeapData {
            data: StringBuffer::Owned(Wtf8Buf::from_string(str)),
        }
    }
}
