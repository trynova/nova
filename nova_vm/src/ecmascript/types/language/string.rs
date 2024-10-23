// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

include!(concat!(env!("OUT_DIR"), "/builtin_strings.rs"));
mod data;

use std::ops::{Index, IndexMut};

use super::{
    IntoPrimitive, IntoValue, Primitive, PropertyKey, Value, SMALL_STRING_DISCRIMINANT,
    STRING_DISCRIMINANT,
};
use crate::{
    ecmascript::{execution::Agent, types::PropertyDescriptor},
    engine::rootable::{HeapRootData, HeapRootRef, Rootable},
    heap::{
        indexes::StringIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        PrimitiveHeap, WorkQueues,
    },
    SmallInteger, SmallString,
};

pub use data::StringHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HeapString(pub(crate) StringIndex);

impl HeapString {
    pub fn len(self, agent: &Agent) -> usize {
        agent[self].len()
    }

    pub(crate) const fn _def() -> Self {
        HeapString(StringIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn as_str(self, agent: &Agent) -> &str {
        agent[self].as_str()
    }
}

impl Index<HeapString> for PrimitiveHeap<'_> {
    type Output = StringHeapData;

    fn index(&self, index: HeapString) -> &Self::Output {
        &self.strings[index]
    }
}

impl Index<HeapString> for Agent {
    type Output = StringHeapData;

    fn index(&self, index: HeapString) -> &Self::Output {
        &self.heap.strings[index]
    }
}

impl IndexMut<HeapString> for Agent {
    fn index_mut(&mut self, index: HeapString) -> &mut Self::Output {
        &mut self.heap.strings[index]
    }
}

impl Index<HeapString> for Vec<Option<StringHeapData>> {
    type Output = StringHeapData;

    fn index(&self, index: HeapString) -> &Self::Output {
        self.get(index.get_index())
            .expect("HeapString out of bounds")
            .as_ref()
            .expect("HeapString slot empty")
    }
}

impl IndexMut<HeapString> for Vec<Option<StringHeapData>> {
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
pub enum String {
    String(HeapString) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum StringRootRepr {
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl IntoValue for HeapString {
    fn into_value(self) -> Value {
        Value::String(self)
    }
}

impl TryFrom<Value> for HeapString {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::String(x) = value {
            Ok(x)
        } else {
            Err(())
        }
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        match self {
            String::String(idx) => Value::String(idx),
            String::SmallString(data) => Value::SmallString(data),
        }
    }
}

impl IntoPrimitive for String {
    fn into_primitive(self) -> Primitive {
        match self {
            String::String(idx) => Primitive::String(idx),
            String::SmallString(data) => Primitive::SmallString(data),
        }
    }
}

impl From<HeapString> for String {
    fn from(value: HeapString) -> Self {
        String::String(value)
    }
}

impl From<HeapString> for Primitive {
    fn from(value: HeapString) -> Self {
        Self::String(value)
    }
}

impl TryFrom<&str> for String {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        SmallString::try_from(value).map(String::SmallString)
    }
}

impl TryFrom<Value> for String {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(x) => Ok(String::String(x)),
            Value::SmallString(x) => Ok(String::SmallString(x)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Primitive> for String {
    type Error = ();
    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        match value {
            Primitive::String(x) => Ok(String::String(x)),
            Primitive::SmallString(x) => Ok(String::SmallString(x)),
            _ => Err(()),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        match value {
            String::String(x) => Value::String(x),
            String::SmallString(x) => Value::SmallString(x),
        }
    }
}

impl From<SmallString> for Value {
    fn from(value: SmallString) -> Self {
        Value::SmallString(value)
    }
}

impl From<SmallString> for String {
    fn from(value: SmallString) -> Self {
        Self::SmallString(value)
    }
}

impl IntoValue for SmallString {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl From<SmallString> for Primitive {
    fn from(value: SmallString) -> Self {
        Self::SmallString(value)
    }
}

impl IntoPrimitive for SmallString {
    fn into_primitive(self) -> Primitive {
        self.into()
    }
}

impl String {
    pub const EMPTY_STRING: String = String::from_small_string("");

    pub fn is_empty_string(self) -> bool {
        self == Self::EMPTY_STRING
    }

    pub fn from_str(agent: &mut Agent, str: &str) -> String {
        agent.heap.create(str)
    }

    pub fn from_string(agent: &mut Agent, string: std::string::String) -> String {
        agent.heap.create(string)
    }

    pub const fn to_property_key(self) -> PropertyKey {
        match self {
            String::String(data) => PropertyKey::String(data),
            String::SmallString(data) => PropertyKey::SmallString(data),
        }
    }

    pub fn from_static_str(agent: &mut Agent, str: &'static str) -> Self {
        if let Ok(value) = String::try_from(str) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { agent.heap.alloc_static_str(str) }
        }
    }

    pub const fn from_small_string(message: &'static str) -> String {
        assert!(
            message.len() < 8
                && (message.is_empty() || message.as_bytes()[message.as_bytes().len() - 1] != 0)
        );
        String::SmallString(SmallString::from_str_unchecked(message))
    }

    pub fn concat(agent: &mut Agent, strings: impl AsRef<[String]>) -> String {
        // TODO: This function will need heavy changes once we support creating
        // WTF-8 strings, since WTF-8 concatenation isn't byte concatenation.

        // We use this status enum so we can reuse one of the heap string inputs
        // if the output would be identical, and so we don't allocate at all
        // until it's clear we need a new heap string.
        enum Status {
            Empty,
            ExistingString(HeapString),
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

    pub fn into_value(self) -> Value {
        self.into()
    }

    /// Byte length of the string.
    pub fn len(self, agent: &impl Index<HeapString, Output = StringHeapData>) -> usize {
        match self {
            String::String(s) => agent[s].len(),
            String::SmallString(s) => s.len(),
        }
    }

    /// UTF-16 length of the string.
    pub fn utf16_len(self, agent: &impl Index<HeapString, Output = StringHeapData>) -> usize {
        match self {
            String::String(s) => agent[s].utf16_len(),
            String::SmallString(s) => s.utf16_len(),
        }
    }

    // TODO: This should return a wtf8::CodePoint.
    pub fn utf16_char(
        self,
        agent: &impl Index<HeapString, Output = StringHeapData>,
        idx: usize,
    ) -> char {
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
    pub fn utf8_index(
        self,
        agent: &impl Index<HeapString, Output = StringHeapData>,
        utf16_idx: usize,
    ) -> Option<usize> {
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
    pub fn utf16_index(
        self,
        agent: &impl Index<HeapString, Output = StringHeapData>,
        utf8_idx: usize,
    ) -> usize {
        match self {
            String::String(s) => agent[s].utf16_index(utf8_idx),
            String::SmallString(s) => s.utf16_index(utf8_idx),
        }
    }

    pub fn as_str<'string, 'agent: 'string>(
        &'string self,
        agent: &'agent impl Index<HeapString, Output = StringHeapData>,
    ) -> &'string str {
        match self {
            String::String(s) => agent[*s].as_str(),
            String::SmallString(s) => s.as_str(),
        }
    }

    /// If x and y have the same length and the same code units in the same
    /// positions, return true; otherwise, return false.
    pub fn eq(
        agent: &impl Index<HeapString, Output = StringHeapData>,
        x: String,
        y: String,
    ) -> bool {
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
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> Option<PropertyDescriptor> {
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

impl CreateHeapData<StringHeapData, String> for Heap {
    fn create(&mut self, data: StringHeapData) -> String {
        self.strings.push(Some(data));
        String::String(HeapString(StringIndex::last(&self.strings)))
    }
}

impl HeapMarkAndSweep for String {
    #[inline(always)]
    fn mark_values(&self, queues: &mut WorkQueues) {
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

impl HeapMarkAndSweep for HeapString {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.strings.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.strings.shift_index(&mut self.0);
    }
}

impl Rootable for String {
    type RootRepr = StringRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::String(heap_string) => Err(HeapRootData::String(heap_string)),
            Self::SmallString(small_string) => Ok(Self::RootRepr::SmallString(small_string)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::SmallString(small_string) => Ok(Self::SmallString(small_string)),
            Self::RootRepr::HeapRef(heap_root_ref) => Err(heap_root_ref),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::String(heap_string) => Some(Self::String(heap_string)),
            _ => None,
        }
    }
}
