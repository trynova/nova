// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

include!(concat!(env!("OUT_DIR"), "/builtin_strings.rs"));
mod data;

use std::ops::{Index, IndexMut};

use super::{IntoPrimitive, IntoValue, Primitive, PropertyKey, Value};
use crate::{
    ecmascript::{execution::Agent, types::PropertyDescriptor},
    heap::{
        indexes::StringIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
    SmallInteger, SmallString,
};

pub use data::StringHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HeapString<'gen>(pub(crate) StringIndex<'gen>);

impl<'gen> HeapString<'gen> {
    pub fn len(self, agent: &Agent<'gen>) -> usize {
        agent[self].len()
    }

    pub(crate) const fn _def() -> Self {
        HeapString(StringIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn as_str(self, agent: &Agent<'gen>) -> &str {
        agent[self].as_str()
    }
}

impl<'gen> Index<HeapString<'gen>> for Agent<'gen> {
    type Output = StringHeapData;

    fn index(&self, index: HeapString) -> &Self::Output {
        &self.heap.strings[index]
    }
}

impl<'gen> IndexMut<HeapString<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: HeapString) -> &mut Self::Output {
        &mut self.heap.strings[index]
    }
}

impl<'gen> Index<HeapString<'gen>> for Vec<Option<StringHeapData>> {
    type Output = StringHeapData;

    fn index(&self, index: HeapString) -> &Self::Output {
        self.get(index.get_index())
            .expect("HeapString out of bounds")
            .as_ref()
            .expect("HeapString slot empty")
    }
}

impl<'gen> IndexMut<HeapString<'gen>> for Vec<Option<StringHeapData>> {
    fn index_mut(&mut self, index: HeapString) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("HeapString out of bounds")
            .as_mut()
            .expect("HeapString slot empty")
    }
}

/// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum String<'gen> {
    String(HeapString<'gen>),
    SmallString(SmallString),
}

impl<'gen> IntoValue<'gen> for HeapString<'gen> {
    fn into_value(self) -> Value<'gen> {
        Value::String(self)
    }
}

impl<'gen> IntoValue<'gen> for String<'gen> {
    fn into_value(self) -> Value<'gen> {
        match self {
            String::String(idx) => Value::String(idx),
            String::SmallString(data) => Value::SmallString(data),
        }
    }
}

impl<'gen> IntoPrimitive<'gen> for String<'gen> {
    fn into_primitive(self) -> Primitive<'gen> {
        match self {
            String::String(idx) => Primitive::String(idx),
            String::SmallString(data) => Primitive::SmallString(data),
        }
    }
}

impl<'gen> From<HeapString<'gen>> for String<'gen> {
    fn from(value: HeapString<'gen>) -> Self {
        String::String(value)
    }
}

impl<'gen> From<HeapString<'gen>> for Primitive<'gen> {
    fn from(value: HeapString<'gen>) -> Self {
        Self::String(value)
    }
}

impl<'gen> TryFrom<&str> for String<'gen> {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        SmallString::try_from(value).map(String::SmallString)
    }
}

impl<'gen> TryFrom<Value<'gen>> for String<'gen> {
    type Error = ();
    fn try_from(value: Value<'gen>) -> Result<Self, Self::Error> {
        match value {
            Value::String(x) => Ok(String::String(x)),
            Value::SmallString(x) => Ok(String::SmallString(x)),
            _ => Err(()),
        }
    }
}

impl<'gen> TryFrom<Primitive<'gen>> for String<'gen> {
    type Error = ();
    fn try_from(value: Primitive<'gen>) -> Result<Self, Self::Error> {
        match value {
            Primitive::String(x) => Ok(String::String(x)),
            Primitive::SmallString(x) => Ok(String::SmallString(x)),
            _ => Err(()),
        }
    }
}

impl<'gen> From<String<'gen>> for Value<'gen> {
    fn from(value: String<'gen>) -> Self {
        match value {
            String::String(x) => Value::String(x),
            String::SmallString(x) => Value::SmallString(x),
        }
    }
}

impl From<SmallString> for Value<'static> {
    fn from(value: SmallString) -> Self {
        Value::SmallString(value)
    }
}

impl From<SmallString> for String<'static> {
    fn from(value: SmallString) -> Self {
        Self::SmallString(value)
    }
}

impl IntoValue<'static> for SmallString {
    fn into_value(self) -> Value<'static> {
        self.into()
    }
}

impl From<SmallString> for Primitive<'static> {
    fn from(value: SmallString) -> Self {
        Self::SmallString(value)
    }
}

impl IntoPrimitive<'static> for SmallString {
    fn into_primitive(self) -> Primitive<'static> {
        self.into()
    }
}

impl<'gen> String<'gen> {
    pub const EMPTY_STRING: String<'static> = String::from_small_string("");

    pub fn is_empty_string(self) -> bool {
        self == Self::EMPTY_STRING
    }

    

    pub const fn to_property_key(self) -> PropertyKey<'gen> {
        match self {
            String::String(data) => PropertyKey::String(data),
            String::SmallString(data) => PropertyKey::SmallString(data),
        }
    }

    pub fn from_static_str<'gen>(agent: &mut Agent<'gen>, str: &'static str) -> Self {
        if let Ok(value) = String::try_from(str) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { agent.heap.alloc_static_str(str) }
        }
    }

    pub const fn from_small_string(message: &'static str) -> String<'static> {
        assert!(
            message.len() < 8
                && (message.is_empty() || message.as_bytes()[message.as_bytes().len() - 1] != 0)
        );
        String::SmallString(SmallString::from_str_unchecked(message))
    }

    /// Byte length of the string.
    pub fn len(self, agent: &Agent<'gen>) -> usize {
        match self {
            String::String(s) => agent[s].len(),
            String::SmallString(s) => s.len(),
        }
    }

    /// UTF-16 length of the string.
    pub fn utf16_len(self, agent: &Agent<'gen>) -> usize {
        match self {
            String::String(s) => agent[s].utf16_len(),
            String::SmallString(s) => s.utf16_len(),
        }
    }

    // TODO: This should return a wtf8::CodePoint.
    pub fn utf16_char(self, agent: &Agent<'gen>, idx: usize) -> char {
        match self {
            String::String(s) => agent[s].utf16_char(idx),
            String::SmallString(s) => s.utf16_char(idx),
        }
    }

    /// Returns the corresponding UTF-8 index for a UTF-16 index into the
    /// string, or `None` if the UTF-16 index is the second code unit in a
    /// surrogate pair.
    ///
    /// # Panics
    ///
    /// This function panics if `utf16_idx` is greater (but not equal) than the
    /// UTF-16 string length.
    pub fn utf8_index(self, agent: &Agent<'gen>, utf16_idx: usize) -> Option<usize> {
        match self {
            String::String(s) => agent[s].utf8_index(utf16_idx),
            String::SmallString(s) => s.utf8_index(utf16_idx),
        }
    }

    /// Returns the corresponding UTF-16 index for a UTF-8 index into the
    /// string.
    ///
    /// # Panics
    ///
    /// This function panics if `utf8_idx` isn't at a UTF-8 code point boundary,
    /// or if it is past the end (but not *at* the end) of the UTF-8 string.
    pub fn utf16_index(self, agent: &Agent<'gen>, utf8_idx: usize) -> usize {
        match self {
            String::String(s) => agent[s].utf16_index(utf8_idx),
            String::SmallString(s) => s.utf16_index(utf8_idx),
        }
    }

    pub fn as_str<'string, 'agent: 'string>(&'string self, agent: &'agent Agent) -> &'string str {
        match self {
            String::String(s) => agent[*s].as_str(),
            String::SmallString(s) => s.as_str(),
        }
    }

    /// If x and y have the same length and the same code units in the same
    /// positions, return true; otherwise, return false.
    pub fn eq(agent: &Agent<'gen>, x: String<'gen>, y: String<'gen>) -> bool {
        match (x, y) {
            (String::String(x), String::String(y)) => {
                let x = &agent[x];
                let y = &agent[y];
                x == y
            }
            (String::SmallString(x), String::SmallString(y)) => x == y,
            // The string heap guarantees that small strings must never equal
            // heap strings.
            _ => false,
        }
    }

    pub(crate) fn get_property_descriptor(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
    ) -> Option<PropertyDescriptor<'gen>> {
        if property_key == BUILTIN_STRING_MEMORY.length.into() {
            let smi = SmallInteger::try_from(self.utf16_len(agent) as u64)
                .expect("String length is over MAX_SAFE_INTEGER");
            Some(PropertyDescriptor {
                value: Some(super::Number::from(smi).into_value()),
                writable: Some(false),
                get: None,
                set: None,
                enumerable: Some(false),
                configurable: Some(false),
            })
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if index >= 0 && (index as usize) < self.utf16_len(agent) {
                let ch = self.utf16_char(agent, index as usize);
                Some(PropertyDescriptor {
                    value: Some(SmallString::from_code_point(ch).into_value()),
                    writable: Some(false),
                    get: None,
                    set: None,
                    enumerable: Some(true),
                    configurable: Some(false),
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<'gen> String<'gen> {
        pub fn from_str<'gen>(agent: &mut Agent<'gen>, str: &str) -> String<'gen> {
        agent.heap.create(str)
    }

    pub fn from_string<'gen>(agent: &mut Agent<'gen>, string: std::string::String) -> String<'gen> {
        agent.heap.create(string)
    }

    pub fn concat<'gen>(agent: &mut Agent<'gen>, strings: impl AsRef<[String<'gen>]>) -> String<'gen> {
        // TODO: This function will need heavy changes once we support creating
        // WTF-8 strings, since WTF-8 concatenation isn't byte concatenation.

        // We use this status enum so we can reuse one of the heap string inputs
        // if the output would be identical, and so we don't allocate at all
        // until it's clear we need a new heap string.
        enum Status<'gen> {
            Empty,
            ExistingString(HeapString<'gen>),
            SmallString { data: [u8; 7], len: usize },
            String(std::string::String),
        }
        let mut status = Status::Empty;

        for string in strings.as_ref() {
            if string.is_empty_string() {
                continue;
            }

            match &mut status {
                Status::Empty => {
                    status = match string {
                        String::SmallString(smstr) => Status::SmallString {
                            data: *smstr.data(),
                            len: smstr.len(),
                        },
                        String::String(idx) => Status::ExistingString(*idx),
                    };
                }
                Status::ExistingString(idx) => {
                    let mut result =
                        std::string::String::with_capacity(agent[*idx].len() + string.len(agent));
                    result.push_str(agent[*idx].as_str());
                    result.push_str(string.as_str(agent));
                    status = Status::String(result)
                }
                Status::SmallString { data, len } => {
                    let string_len = string.len(agent);
                    if *len + string_len <= 7 {
                        let String::SmallString(smstr) = string else {
                            unreachable!()
                        };
                        data[*len..(*len + string_len)]
                            .copy_from_slice(&smstr.data()[..string_len]);
                        *len += string_len;
                    } else {
                        let mut result = std::string::String::with_capacity(*len + string_len);
                        // SAFETY: Since SmallStrings are guaranteed UTF-8, `&data[..len]` is the result
                        // of concatenating UTF-8 strings, which is always valid UTF-8.
                        result.push_str(unsafe { std::str::from_utf8_unchecked(&data[..*len]) });
                        result.push_str(string.as_str(agent));
                        status = Status::String(result);
                    }
                }
                Status::String(buffer) => buffer.push_str(string.as_str(agent)),
            }
        }

        match status {
            Status::Empty => String::EMPTY_STRING,
            Status::ExistingString(idx) => String::String(idx),
            Status::SmallString { data, len } => {
                // SAFETY: Since SmallStrings are guaranteed UTF-8, `&data[..len]` is the result of
                // concatenating UTF-8 strings, which is always valid UTF-8.
                let str_slice = unsafe { std::str::from_utf8_unchecked(&data[..len]) };
                SmallString::from_str_unchecked(str_slice).into()
            }
            Status::String(string) => agent.heap.create(string),
        }
    }
}

impl<'gen> CreateHeapData<StringHeapData, String<'gen>> for Heap<'gen> {
    fn create(&mut self, data: StringHeapData) -> String {
        self.strings.push(Some(data));
        String::String(HeapString(StringIndex::last(&self.strings)))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for String<'_> {
    #[inline(always)]
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        if let Self::String(idx) = self {
            idx.mark_values(queues);
        }
    }

    #[inline(always)]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        if let Self::String(idx) = self {
            idx.sweep_values(compactions);
        }
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for HeapString<'_> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.strings.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.strings.shift_index(&mut self.0);
    }
}
