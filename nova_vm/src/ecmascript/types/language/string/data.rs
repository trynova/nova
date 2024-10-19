// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cell::OnceCell, num::NonZeroUsize};

use wtf8::{Wtf8, Wtf8Buf};

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone)]
pub struct StringHeapData {
    pub(crate) data: StringBuffer,
    pub(crate) mapping: OnceCell<IndexMapping>,
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
        mapping: Box<[Option<NonZeroUsize>]>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum StringBuffer {
    Owned(Wtf8Buf),
    Static(&'static Wtf8),
}

impl StringHeapData {
    /// The maximum UTf-16 length of a JS string, according to the spec (2^53 - 1).
    const MAX_UTF16_LENGTH: usize = (1 << 53) - 1;

    /// The maximum UTF-8 length of a JS string.
    const MAX_UTF8_LENGTH: usize = 3 * Self::MAX_UTF16_LENGTH;

    pub fn len(&self) -> usize {
        match &self.data {
            StringBuffer::Owned(buf) => buf.len(),
            StringBuffer::Static(buf) => buf.len(),
        }
    }

    fn index_mapping(&self) -> &IndexMapping {
        self.mapping.get_or_init(|| {
            let mut iter = self.as_str().char_indices();

            let Some((idx, ch)) = iter.find(|(_, ch)| !ch.is_ascii()) else {
                return IndexMapping::Ascii;
            };

            // All indices less than `idx` map to ASCII bytes, so all UTF-16
            // indices less *or equal* than `idx` map to that same UTF-8 index
            let mut mapping: Vec<Option<NonZeroUsize>> = (0..=idx).map(NonZeroUsize::new).collect();

            if ch.len_utf16() != 1 {
                debug_assert_eq!(ch.len_utf16(), 2);
                mapping.push(None);
            }

            for (idx, ch) in iter {
                assert_ne!(idx, 0);
                mapping.push(NonZeroUsize::new(idx));
                if ch.len_utf16() != 1 {
                    debug_assert_eq!(ch.len_utf16(), 2);
                    mapping.push(None);
                }
            }

            assert!(
                mapping.len() <= Self::MAX_UTF16_LENGTH,
                "String is too long."
            );

            IndexMapping::NonAscii {
                mapping: mapping.into_boxed_slice(),
            }
        })
    }

    pub fn utf16_len(&self) -> usize {
        match self.index_mapping() {
            IndexMapping::Ascii => self.len(),
            IndexMapping::NonAscii { mapping } => mapping.len(),
        }
    }

    // TODO: This should return a wtf8::CodePoint.
    pub fn utf16_char(&self, idx: usize) -> char {
        let utf8_idx = if idx != 0 {
            match self.index_mapping() {
                IndexMapping::Ascii => idx,
                IndexMapping::NonAscii { mapping } => {
                    // TODO: Deal with surrogates.
                    mapping[idx].unwrap().get()
                }
            }
        } else {
            0
        };
        let ch = self.as_str()[utf8_idx..].chars().next().unwrap();
        // TODO: Deal with surrogates.
        assert_eq!(ch.len_utf16(), 1);
        ch
    }

    pub fn utf8_index(&self, utf16_idx: usize) -> Option<usize> {
        if utf16_idx == 0 {
            Some(0)
        } else {
            match self.index_mapping() {
                IndexMapping::Ascii => {
                    assert!(utf16_idx <= self.len());
                    Some(utf16_idx)
                }
                IndexMapping::NonAscii { mapping } => {
                    if utf16_idx == mapping.len() {
                        Some(self.len())
                    } else {
                        mapping[utf16_idx].map(NonZeroUsize::get)
                    }
                }
            }
        }
    }

    pub fn utf16_index(&self, utf8_idx: usize) -> usize {
        if utf8_idx == 0 {
            0
        } else {
            assert!(utf8_idx <= self.len());
            match self.index_mapping() {
                IndexMapping::Ascii => utf8_idx,
                IndexMapping::NonAscii { mapping } => {
                    if utf8_idx == self.len() {
                        return mapping.len();
                    }

                    // Binary search `mapping`. We start at `utf8_idx` though,
                    // if it's in range.
                    let mut range = if utf8_idx >= mapping.len() {
                        0..mapping.len()
                    } else {
                        let mut pivot = utf8_idx;
                        if mapping[pivot].is_none() {
                            pivot -= 1;
                        }
                        match mapping[pivot].unwrap().get().cmp(&utf8_idx) {
                            std::cmp::Ordering::Less => pivot..mapping.len(),
                            std::cmp::Ordering::Equal => return pivot,
                            std::cmp::Ordering::Greater => 0..pivot,
                        }
                    };

                    loop {
                        let mut pivot = (range.start + range.end) / 2;
                        if mapping[pivot].is_none() {
                            pivot -= 1;
                        }
                        debug_assert!(range.contains(&pivot));

                        // Since we're adjusting the pivot due to None elements
                        // (i.e. surrogates), we might get stuck in an infinite
                        // loop if pivot is the start of the range. In this
                        // case, we walk through the range we know is valid.
                        if pivot == range.start {
                            for i in range {
                                if mapping[i].is_some() && mapping[i].unwrap().get() == utf8_idx {
                                    return i;
                                }
                            }
                            unreachable!();
                        }

                        let new_range = match mapping[pivot].unwrap().get().cmp(&utf8_idx) {
                            std::cmp::Ordering::Less => pivot..range.end,
                            std::cmp::Ordering::Equal => return pivot,
                            std::cmp::Ordering::Greater => range.start..pivot,
                        };
                        assert_ne!(range, new_range);

                        range = new_range;
                    }
                }
            }
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.data {
            StringBuffer::Owned(buf) => buf.as_str().unwrap(),
            StringBuffer::Static(buf) => buf.as_str().unwrap(),
        }
    }

    pub fn from_str(str: &str) -> Self {
        debug_assert!(str.len() > 7);
        assert!(str.len() <= Self::MAX_UTF8_LENGTH, "String is too long.");
        StringHeapData {
            data: StringBuffer::Owned(Wtf8Buf::from_str(str)),
            mapping: OnceCell::new(),
        }
    }

    pub fn from_static_str(str: &'static str) -> Self {
        debug_assert!(str.len() > 7);
        assert!(str.len() <= Self::MAX_UTF8_LENGTH, "String is too long.");
        StringHeapData {
            data: StringBuffer::Static(Wtf8::from_str(str)),
            mapping: OnceCell::new(),
        }
    }

    pub fn from_string(str: String) -> Self {
        debug_assert!(str.len() > 7);
        assert!(str.len() <= Self::MAX_UTF8_LENGTH, "String is too long.");
        StringHeapData {
            data: StringBuffer::Owned(Wtf8Buf::from_string(str)),
            mapping: OnceCell::new(),
        }
    }
}

impl HeapMarkAndSweep for StringHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues) {
        let Self {
            data: _,
            mapping: _,
        } = self;
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        let Self {
            data: _,
            mapping: _,
        } = self;
    }
}
