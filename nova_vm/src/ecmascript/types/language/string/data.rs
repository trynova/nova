use wtf8::{Wtf8, Wtf8Buf};

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone)]
pub struct StringHeapData {
    pub(crate) data: StringBuffer,
    pub(crate) mapping: IndexMapping,
}

impl PartialEq for StringHeapData {
    fn eq(&self, other: &Self) -> bool {
        // If both strings are static, we can compare their pointers directly.
        if let (&StringBuffer::Static(self_str), &StringBuffer::Static(other_str)) =
            (&self.data, &other.data)
        {
            if std::ptr::eq(self_str, other_str) {
                return true;
            }
        }
        self.as_str() == other.as_str()
    }
}
impl Eq for StringHeapData {}

#[derive(Debug, Clone)]
pub(crate) enum IndexMapping {
    Ascii,
    NonAscii {
        /// Mapping from UTF-16 indices to indices in the UTF-8 representation.
        /// When the UTF-16 character would be the second character in a
        /// surrogate pair, it maps to None because there is no corresponding
        /// UTF-8 index.
        mapping: Box<[Option<usize>]>,
    },
}

#[derive(Debug, Clone)]
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

    pub fn utf16_len(&self) -> usize {
        match &self.mapping {
            IndexMapping::Ascii => self.len(),
            IndexMapping::NonAscii { mapping } => mapping.len(),
        }
    }

    // TODO: This should return a wtf8::CodePoint.
    pub fn utf16_char(&self, idx: usize) -> char {
        let utf8_idx = match &self.mapping {
            IndexMapping::Ascii => idx,
            IndexMapping::NonAscii { mapping } => {
                // TODO: Deal with surrogates.
                mapping[idx].unwrap()
            }
        };
        let ch = self.as_str()[utf8_idx..].chars().next().unwrap();
        // TODO: Deal with surrogates.
        assert_eq!(ch.len_utf16(), 1);
        ch
    }

    pub fn as_str(&self) -> &str {
        match &self.data {
            StringBuffer::Owned(buf) => buf.as_str().unwrap(),
            StringBuffer::Static(buf) => buf.as_str().unwrap(),
        }
    }

    pub fn from_str(str: &str) -> Self {
        debug_assert!(str.len() > 7);
        StringHeapData {
            data: StringBuffer::Owned(Wtf8Buf::from_str(str)),
            mapping: build_mapping(str),
        }
    }

    pub fn from_static_str(str: &'static str) -> Self {
        debug_assert!(str.len() > 7);
        StringHeapData {
            data: StringBuffer::Static(Wtf8::from_str(str)),
            mapping: build_mapping(str),
        }
    }

    pub fn from_string(str: String) -> Self {
        debug_assert!(str.len() > 7);
        let mapping = build_mapping(&str);
        StringHeapData {
            data: StringBuffer::Owned(Wtf8Buf::from_string(str)),
            mapping,
        }
    }
}

fn build_mapping(str: &str) -> IndexMapping {
    let mut iter = str.char_indices();

    let Some((idx, ch)) = iter.find(|(_, ch)| !ch.is_ascii()) else {
        return IndexMapping::Ascii;
    };

    // All indices less than `idx` map to ASCII bytes, so all UTF-16
    // indices less *or equal* than `idx` map to that same UTF-8 index
    let mut mapping: Vec<Option<usize>> = (0..=idx).map(Some).collect();

    if ch.len_utf16() != 1 {
        debug_assert_eq!(ch.len_utf16(), 2);
        mapping.push(None);
    }

    for (idx, ch) in iter {
        mapping.push(Some(idx));
        if ch.len_utf16() != 1 {
            debug_assert_eq!(ch.len_utf16(), 2);
            mapping.push(None);
        }
    }

    IndexMapping::NonAscii {
        mapping: mapping.into_boxed_slice(),
    }
}

impl HeapMarkAndSweep for StringHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues) {}

    fn sweep_values(&mut self, _compactions: &CompactionLists) {}
}
