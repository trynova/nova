// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{cell::OnceCell, hash::Hash, num::NonZeroUsize};
use std::borrow::Cow;

use wtf8::{CodePoint, Wtf8, Wtf8Buf};

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone)]
pub(crate) struct StringRecord {
    pub(crate) data: StringBuffer,
    pub(crate) mapping: OnceCell<IndexMapping>,
}

impl PartialEq for StringRecord {
    fn eq(&self, other: &Self) -> bool {
        // If both strings are static, we can compare their pointers directly.
        if let (&StringBuffer::Static(self_str), &StringBuffer::Static(other_str)) =
            (&self.data, &other.data)
            && core::ptr::eq(self_str, other_str)
        {
            return true;
        }
        match (&self.data, &other.data) {
            (StringBuffer::Owned(a), StringBuffer::Owned(b)) => a == b,
            (StringBuffer::Owned(a), StringBuffer::Static(b)) => a == b,
            (StringBuffer::Static(a), StringBuffer::Owned(b)) => a == b,
            (StringBuffer::Static(a), StringBuffer::Static(b)) => a == b,
        }
    }
}
impl Eq for StringRecord {}

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

impl Hash for StringBuffer {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            StringBuffer::Owned(wtf8_buf) => wtf8_buf.hash(state),
            StringBuffer::Static(wtf8) => wtf8.hash(state),
        }
    }
}

impl StringRecord {
    /// The maximum UTf-16 length of a JS string, according to the spec (2^53 - 1).
    const MAX_UTF16_LENGTH: usize = (1 << 53) - 1;

    /// The maximum UTF-8 length of a JS string.
    const MAX_UTF8_LENGTH: usize = 3 * Self::MAX_UTF16_LENGTH;

    /// Get the byte length of the string.
    pub fn len(&self) -> usize {
        match &self.data {
            StringBuffer::Owned(buf) => buf.len(),
            StringBuffer::Static(buf) => buf.len(),
        }
    }

    fn index_mapping(&self) -> &IndexMapping {
        self.mapping.get_or_init(|| {
            fn is_surrogate_pair(cp: CodePoint) -> bool {
                let code = cp.to_u32();
                (code & !0xFFFF) > 0
            }

            let Some((mut idx, _)) = self
                .as_bytes()
                .iter()
                .enumerate()
                .find(|(_, ch)| !ch.is_ascii())
            else {
                return IndexMapping::Ascii;
            };

            // All indices less than `idx` map to ASCII bytes, so all UTF-16
            // indices less *or equal* than `idx` map to that same UTF-8 index
            let mut mapping: Vec<Option<NonZeroUsize>> = (0..=idx).map(NonZeroUsize::new).collect();

            let mut buf = [0u8; 4];
            let mut iter = self.as_wtf8().slice_from(idx).code_points();

            // SAFETY: We
            let ch = unsafe { iter.next().unwrap_unchecked() };

            if is_surrogate_pair(ch) {
                mapping.push(None);
            }

            if let Some(ch) = ch.to_char() {
                idx += ch.encode_utf8(&mut buf).len();
            } else {
                // Lone surrogate; these always take 3 bytes in WTF-8.
                idx += 3;
            }

            assert_ne!(idx, 0);
            for ch in iter {
                mapping.push(NonZeroUsize::new(idx));
                if is_surrogate_pair(ch) {
                    mapping.push(None);
                }
                if let Some(ch) = ch.to_char() {
                    idx += ch.encode_utf8(&mut buf).len();
                } else {
                    // Lone surrogate; these always take 3 bytes in WTF-8.
                    idx += 3;
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

    /// Get the WTF-16 length of the string.
    pub fn utf16_len(&self) -> usize {
        match self.index_mapping() {
            IndexMapping::Ascii => self.len(),
            IndexMapping::NonAscii { mapping } => mapping.len(),
        }
    }

    pub fn char_code_at(&self, idx: usize) -> CodePoint {
        let (utf8_idx, take_latter_half): (usize, bool) = if idx != 0 {
            match self.index_mapping() {
                IndexMapping::Ascii => {
                    // SAFETY: ASCII is always valid CodePoints.
                    return unsafe { CodePoint::from_u32_unchecked(self.as_bytes()[idx] as u32) };
                }
                IndexMapping::NonAscii { mapping } => {
                    match mapping[idx] {
                        Some(idx) => (idx.into(), false),
                        None => {
                            // We got a None; that means we're looking at a latter
                            // surrogate here.
                            // SAFETY: idx is not 0.
                            let idx = mapping[unsafe { idx.unchecked_sub(1) }].unwrap();
                            (idx.into(), true)
                        }
                    }
                }
            }
        } else {
            (0, false)
        };
        let char = self
            .as_wtf8()
            .slice_from(utf8_idx)
            .code_points()
            .next()
            .unwrap();
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
                            core::cmp::Ordering::Less => pivot..mapping.len(),
                            core::cmp::Ordering::Equal => return pivot,
                            core::cmp::Ordering::Greater => 0..pivot,
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
                            core::cmp::Ordering::Less => pivot..range.end,
                            core::cmp::Ordering::Equal => return pivot,
                            core::cmp::Ordering::Greater => range.start..pivot,
                        };
                        assert_ne!(range, new_range);

                        range = new_range;
                    }
                }
            }
        }
    }

    pub fn code_point_at(&self, utf16_idx: usize) -> CodePoint {
        assert!(utf16_idx <= self.utf16_len());
        let mapping = match self.index_mapping() {
            // SAFETY: ASCII is all valid CodePoints.
            IndexMapping::Ascii => {
                return unsafe { CodePoint::from_u32_unchecked(self.as_bytes()[utf16_idx] as u32) };
            }
            IndexMapping::NonAscii { mapping } => mapping,
        };
        if utf16_idx == 0 {
            let char = self.as_wtf8().code_points().next().unwrap();
            return char;
        }
        match mapping[utf16_idx] {
            Some(wtf8_idx) => {
                let wtf8_index: usize = wtf8_idx.into();
                self.as_wtf8()
                    .slice_from(wtf8_index)
                    .code_points()
                    .next()
                    .unwrap()
            }
            None => {
                // Matched None; this is the second character in a surrogate pair.
                let wtf8_index: usize = mapping[utf16_idx - 1].unwrap().into();
                let char = self
                    .as_wtf8()
                    .slice_from(wtf8_index)
                    .code_points()
                    .next()
                    .unwrap();
                let char = char
                    .to_char()
                    .expect("Surrogate pair did not map to a char");
                let encoded = &mut [0; 2];
                let encoded = char.encode_utf16(encoded);
                // Note: since this is a surrogate pair, it should always
                // encode into two u16s.
                debug_assert_eq!(encoded.len(), 2);
                let surrogate = encoded[1];
                // SAFETY: 0..0xFFFF is always less than 0x10FFFF.
                unsafe { CodePoint::from_u32_unchecked(surrogate as u32) }
            }
        }
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

    pub fn as_bytes(&self) -> &[u8] {
        let buf = self.as_wtf8();
        // SAFETY: converting to backing store data.
        unsafe { core::mem::transmute::<&Wtf8, &[u8]>(buf) }
    }

    pub fn as_wtf8(&self) -> &Wtf8 {
        match &self.data {
            StringBuffer::Owned(buf) => buf,
            StringBuffer::Static(buf) => buf,
        }
    }

    pub fn from_str(str: &str) -> Self {
        debug_assert!(str.len() > 7);
        assert!(str.len() <= Self::MAX_UTF8_LENGTH, "String is too long.");
        StringRecord {
            data: StringBuffer::Owned(Wtf8Buf::from_str(str)),
            mapping: OnceCell::new(),
        }
    }

    pub fn from_static_str(str: &'static str) -> Self {
        debug_assert!(str.len() > 7);
        assert!(str.len() <= Self::MAX_UTF8_LENGTH, "String is too long.");
        StringRecord {
            data: StringBuffer::Static(Wtf8::from_str(str)),
            mapping: OnceCell::new(),
        }
    }

    pub fn from_string(str: String) -> Self {
        debug_assert!(str.len() > 7);
        assert!(str.len() <= Self::MAX_UTF8_LENGTH, "String is too long.");
        StringRecord {
            data: StringBuffer::Owned(Wtf8Buf::from_string(str)),
            mapping: OnceCell::new(),
        }
    }

    pub fn from_wtf8_buf(str: Wtf8Buf) -> Self {
        debug_assert!(str.len() > 7);
        assert!(str.len() <= Self::MAX_UTF8_LENGTH, "String is too long.");
        StringRecord {
            data: StringBuffer::Owned(str),
            mapping: OnceCell::new(),
        }
    }
}

impl HeapMarkAndSweep for StringRecord {
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
