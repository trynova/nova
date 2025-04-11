// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

include!(concat!(env!("OUT_DIR"), "/builtin_strings.rs"));
mod data;

use core::{
    hash::Hash,
    ops::{Index, IndexMut},
};

use super::{
    IntoPrimitive, IntoValue, Primitive, PropertyKey, SMALL_STRING_DISCRIMINANT,
    STRING_DISCRIMINANT, Value,
};
use crate::{
    SmallInteger, SmallString,
    ecmascript::{execution::Agent, types::PropertyDescriptor},
    engine::{
        Scoped,
        context::{Bindable, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, PrimitiveHeap, WorkQueues,
        indexes::{GetBaseIndexMut, IntoBaseIndex, StringIndex},
    },
};

pub use data::StringHeapData;
use wtf8::Wtf8Buf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HeapString<'a>(pub(crate) StringIndex<'a>);

impl HeapString<'_> {
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for HeapString<'_> {
    type Of<'a> = HeapString<'a>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl Index<HeapString<'_>> for PrimitiveHeap<'_> {
    type Output = StringHeapData;

    fn index(&self, index: HeapString<'_>) -> &Self::Output {
        &self.strings[index]
    }
}

impl Index<HeapString<'_>> for Agent {
    type Output = StringHeapData;

    fn index(&self, index: HeapString<'_>) -> &Self::Output {
        &self.heap.strings[index]
    }
}

impl IndexMut<HeapString<'_>> for Agent {
    fn index_mut(&mut self, index: HeapString<'_>) -> &mut Self::Output {
        &mut self.heap.strings[index]
    }
}

impl Index<HeapString<'_>> for Vec<Option<StringHeapData>> {
    type Output = StringHeapData;

    fn index(&self, index: HeapString<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("HeapString out of bounds")
            .as_ref()
            .expect("HeapString slot empty")
    }
}

impl IndexMut<HeapString<'_>> for Vec<Option<StringHeapData>> {
    fn index_mut(&mut self, index: HeapString<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("HeapString out of bounds")
            .as_mut()
            .expect("HeapString slot empty")
    }
}

impl<'a> IntoBaseIndex<'a, StringHeapData> for HeapString<'a> {
    fn into_base_index(self) -> StringIndex<'a> {
        self.0
    }
}

impl<'a> GetBaseIndexMut<'a, StringHeapData> for HeapString<'a> {
    fn get_base_index_mut(&mut self) -> &mut StringIndex<'a> {
        &mut self.0
    }
}

/// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum String<'a> {
    String(HeapString<'a>) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum StringRootRepr {
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl<'a> IntoValue<'a> for HeapString<'a> {
    fn into_value(self) -> Value<'a> {
        Value::String(self.unbind())
    }
}

impl<'a> TryFrom<Value<'a>> for HeapString<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        if let Value::String(x) = value {
            Ok(x)
        } else {
            Err(())
        }
    }
}

impl<'a> IntoValue<'a> for String<'a> {
    fn into_value(self) -> Value<'a> {
        match self {
            String::String(idx) => Value::String(idx),
            String::SmallString(data) => Value::SmallString(data),
        }
    }
}

impl<'a> IntoPrimitive<'a> for String<'a> {
    fn into_primitive(self) -> Primitive<'a> {
        match self {
            String::String(idx) => Primitive::String(idx),
            String::SmallString(data) => Primitive::SmallString(data),
        }
    }
}

impl<'a> From<HeapString<'a>> for String<'a> {
    fn from(value: HeapString<'a>) -> Self {
        String::String(value)
    }
}

impl<'a> From<HeapString<'a>> for Primitive<'a> {
    fn from(value: HeapString<'a>) -> Self {
        Self::String(value.unbind())
    }
}

impl TryFrom<&str> for String<'static> {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        SmallString::try_from(value).map(String::SmallString)
    }
}

impl<'a> TryFrom<Value<'a>> for String<'a> {
    type Error = ();
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::String(x) => Ok(String::String(x)),
            Value::SmallString(x) => Ok(String::SmallString(x)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Primitive<'a>> for String<'a> {
    type Error = ();
    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        match value {
            Primitive::String(x) => Ok(String::String(x)),
            Primitive::SmallString(x) => Ok(String::SmallString(x)),
            _ => Err(()),
        }
    }
}

impl<'a> From<String<'a>> for Value<'a> {
    fn from(value: String<'a>) -> Self {
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

impl<'a> String<'a> {
    pub const EMPTY_STRING: String<'static> = String::from_small_string("");

    /// Scope a stack-only String. Stack-only Strings do not need to store any
    /// data on the heap, hence scoping them is effectively a no-op. These
    /// Strings are also not concerned with the garbage collector.
    ///
    /// ## Panics
    ///
    /// If the String is not stack-only, this method will panic.
    pub const fn scope_static(self) -> Scoped<'static, String<'static>> {
        let key_root_repr = match self {
            String::SmallString(small_string) => StringRootRepr::SmallString(small_string),
            _ => panic!("String required rooting"),
        };
        Scoped::from_root_repr(key_root_repr)
    }

    pub fn is_empty_string(self) -> bool {
        self == Self::EMPTY_STRING
    }

    pub const fn to_property_key(self) -> PropertyKey<'a> {
        match self {
            String::String(data) => PropertyKey::String(data),
            String::SmallString(data) => PropertyKey::SmallString(data),
        }
    }

    pub const fn from_small_string(message: &'static str) -> String<'static> {
        assert!(
            message.len() < 8 && (message.is_empty() || message.as_bytes()[message.len() - 1] != 0)
        );
        String::SmallString(SmallString::from_str_unchecked(message))
    }

    pub fn concat<'gc>(
        agent: &mut Agent,
        strings: impl AsRef<[Self]>,
        gc: NoGcScope<'gc, '_>,
    ) -> String<'gc> {
        // TODO: This function will need heavy changes once we support creating
        // WTF-8 strings, since WTF-8 concatenation isn't byte concatenation.

        // We use this status enum so we can reuse one of the heap string inputs
        // if the output would be identical, and so we don't allocate at all
        // until it's clear we need a new heap string.
        enum Status<'a> {
            Empty,
            ExistingString(HeapString<'a>),
            SmallString { data: [u8; 7], len: usize },
            String(Wtf8Buf),
        }
        let strings = strings.as_ref();
        let mut status = if strings.len() > 1 {
            let len = strings.iter().fold(0usize, |a, s| a + s.len(agent));
            if len > 7 {
                Status::String(Wtf8Buf::with_capacity(len))
            } else {
                Status::Empty
            }
        } else {
            Status::Empty
        };

        fn push_string_to_wtf8(agent: &Agent, buf: &mut Wtf8Buf, string: String) {
            match string {
                String::String(heap_string) => {
                    buf.push_wtf8(agent[heap_string].as_wtf8());
                }
                String::SmallString(small_string) => {
                    buf.push_str(small_string.as_str());
                }
            }
        }

        for string in strings {
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
                Status::ExistingString(heap_string) => {
                    let heap_string = *heap_string;
                    let mut result =
                        Wtf8Buf::with_capacity(agent[heap_string].len() + string.len(agent));
                    result.push_wtf8(agent[heap_string].as_wtf8());
                    push_string_to_wtf8(agent, &mut result, *string);
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
                        let mut result = Wtf8Buf::with_capacity(*len + string_len);
                        // SAFETY: Since SmallStrings are guaranteed UTF-8, `&data[..len]` is the result
                        // of concatenating UTF-8 strings, which is always valid UTF-8.
                        result.push_str(unsafe { core::str::from_utf8_unchecked(&data[..*len]) });
                        push_string_to_wtf8(agent, &mut result, *string);
                        status = Status::String(result);
                    }
                }
                Status::String(buffer) => push_string_to_wtf8(agent, buffer, *string),
            }
        }

        match status {
            Status::Empty => String::EMPTY_STRING,
            Status::ExistingString(idx) => String::String(idx.bind(gc)),
            Status::SmallString { data, len } => {
                // SAFETY: Since SmallStrings are guaranteed UTF-8, `&data[..len]` is the result of
                // concatenating UTF-8 strings, which is always valid UTF-8.
                let str_slice = unsafe { core::str::from_utf8_unchecked(&data[..len]) };
                SmallString::from_str_unchecked(str_slice).into()
            }
            Status::String(string) => agent.heap.create(string.into_string().unwrap()).bind(gc),
        }
    }

    /// Byte length of the string.
    pub fn len(self, agent: &impl Index<HeapString<'static>, Output = StringHeapData>) -> usize {
        match self {
            String::String(s) => agent[s.unbind()].len(),
            String::SmallString(s) => s.len(),
        }
    }

    /// UTF-16 length of the string.
    pub fn utf16_len(
        self,
        agent: &impl Index<HeapString<'static>, Output = StringHeapData>,
    ) -> usize {
        match self {
            String::String(s) => agent[s.unbind()].utf16_len(),
            String::SmallString(s) => s.utf16_len(),
        }
    }

    // TODO: This should return a wtf8::CodePoint.
    pub fn utf16_char(
        self,
        agent: &impl Index<HeapString<'static>, Output = StringHeapData>,
        idx: usize,
    ) -> char {
        match self {
            String::String(s) => agent[s.unbind()].utf16_char(idx),
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
        agent: &impl Index<HeapString<'static>, Output = StringHeapData>,
        utf16_idx: usize,
    ) -> Option<usize> {
        match self {
            String::String(s) => agent[s.unbind()].utf8_index(utf16_idx),
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
        agent: &impl Index<HeapString<'static>, Output = StringHeapData>,
        utf8_idx: usize,
    ) -> usize {
        match self {
            String::String(s) => agent[s.unbind()].utf16_index(utf8_idx),
            String::SmallString(s) => s.utf16_index(utf8_idx),
        }
    }

    pub fn as_str<'string, 'agent: 'string>(
        &'string self,
        agent: &'agent impl Index<HeapString<'static>, Output = StringHeapData>,
    ) -> &'string str {
        match self {
            String::String(s) => agent[s.unbind()].as_str(),
            String::SmallString(s) => s.as_str(),
        }
    }

    /// If x and y have the same length and the same code units in the same
    /// positions, return true; otherwise, return false.
    pub fn eq(
        agent: &impl Index<HeapString<'static>, Output = StringHeapData>,
        x: Self,
        y: Self,
    ) -> bool {
        match (x, y) {
            (Self::String(x), Self::String(y)) => {
                let x = &agent[x.unbind()];
                let y = &agent[y.unbind()];
                x == y
            }
            (Self::SmallString(x), Self::SmallString(y)) => x == y,
            // The string heap guarantees that small strings must never equal
            // heap strings.
            _ => false,
        }
    }

    pub(crate) fn get_property_descriptor(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> Option<PropertyDescriptor<'static>> {
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

    pub(crate) fn get_property_value(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> Option<Value<'a>> {
        if property_key == BUILTIN_STRING_MEMORY.length.into() {
            let smi = SmallInteger::try_from(self.utf16_len(agent) as u64)
                .expect("String length is over MAX_SAFE_INTEGER");
            Some(super::Number::from(smi).into_value())
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if index >= 0 && (index as usize) < self.utf16_len(agent) {
                let ch = self.utf16_char(agent, index as usize);
                Some(SmallString::from_code_point(ch).into_value())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<'gc> String<'gc> {
    pub fn from_str(agent: &mut Agent, str: &str, _gc: NoGcScope<'gc, '_>) -> Self {
        agent.heap.create(str)
    }

    pub fn from_string(
        agent: &mut Agent,
        string: std::string::String,
        gc: NoGcScope<'gc, '_>,
    ) -> Self {
        agent.heap.create(string).bind(gc)
    }

    pub fn from_static_str(agent: &mut Agent, str: &'static str, _gc: NoGcScope<'gc, '_>) -> Self {
        if let Ok(value) = String::try_from(str) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { agent.heap.alloc_static_str(str) }
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for String<'_> {
    type Of<'a> = String<'a>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl Scoped<'_, String<'static>> {
    pub fn as_str<'string, 'agent: 'string>(&'string self, agent: &'agent Agent) -> &'string str {
        match &self.inner {
            StringRootRepr::SmallString(small_string) => small_string.as_str(),
            StringRootRepr::HeapRef(_) => {
                let String::String(string) = self.get(agent) else {
                    unreachable!();
                };
                string.as_str(agent)
            }
        }
    }
}

impl<'a> CreateHeapData<(StringHeapData, u64), String<'a>> for Heap {
    fn create(&mut self, (data, hash): (StringHeapData, u64)) -> String<'a> {
        self.strings.push(Some(data));
        let index = StringIndex::last(&self.strings);
        let heap_string = HeapString(index);
        self.string_lookup_table
            .insert_unique(hash, heap_string, |_| hash);
        String::String(heap_string)
    }
}

impl HeapMarkAndSweep for String<'static> {
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

impl HeapMarkAndSweep for HeapString<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.strings.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.strings.shift_index(&mut self.0);
    }
}

impl Rootable for String<'_> {
    type RootRepr = StringRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::String(heap_string) => Err(HeapRootData::String(heap_string.unbind())),
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
