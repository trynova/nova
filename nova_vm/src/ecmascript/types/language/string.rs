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
    engine::{
        context::NoGcScope,
        rootable::{HeapRootData, HeapRootRef, Rootable},
        Scoped,
    },
    heap::{
        indexes::StringIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        PrimitiveHeap, WorkQueues,
    },
    SmallInteger, SmallString,
};

pub use data::StringHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HeapString<'a>(pub(crate) StringIndex<'a>);

impl HeapString<'_> {
    /// Unbind this HeapString from its current lifetime. This is necessary to
    /// use the HeapString as a parameter in a call that can perform garbage
    /// collection.
    pub const fn unbind(self) -> HeapString<'static> {
        unsafe { std::mem::transmute::<HeapString<'_>, HeapString<'static>>(self) }
    }

    // Bind this HeapString to the garbage collection lifetime. This enables
    // Rust's borrow checker to verify that your HeapStrings cannot not be
    // invalidated by garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let heap_string = heap_string.bind(&gc);
    // ```
    // to make sure that the unbound HeapString cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> HeapString<'gc> {
        unsafe { std::mem::transmute::<HeapString<'_>, HeapString<'gc>>(self) }
    }

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

impl IntoValue for HeapString<'_> {
    fn into_value(self) -> Value {
        Value::String(self.unbind())
    }
}

impl TryFrom<Value> for HeapString<'_> {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::String(x) = value {
            Ok(x)
        } else {
            Err(())
        }
    }
}

impl IntoValue for String<'_> {
    fn into_value(self) -> Value {
        match self {
            String::String(idx) => Value::String(idx.unbind()),
            String::SmallString(data) => Value::SmallString(data),
        }
    }
}

impl<'a> IntoPrimitive<'a> for String<'a> {
    fn into_primitive(self) -> Primitive<'a> {
        match self {
            String::String(idx) => Primitive::String(idx.unbind()),
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

impl TryFrom<Value> for String<'_> {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
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

impl From<String<'_>> for Value {
    fn from(value: String) -> Self {
        match value {
            String::String(x) => Value::String(x.unbind()),
            String::SmallString(x) => Value::SmallString(x),
        }
    }
}

impl From<SmallString> for Value {
    fn from(value: SmallString) -> Self {
        Value::SmallString(value)
    }
}

impl From<SmallString> for String<'static> {
    fn from(value: SmallString) -> Self {
        Self::SmallString(value)
    }
}

impl IntoValue for SmallString {
    fn into_value(self) -> Value {
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

impl String<'_> {
    pub const EMPTY_STRING: String<'static> = String::from_small_string("");

    /// Unbind this String from its current lifetime. This is necessary to use
    /// the String as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> String<'static> {
        unsafe { std::mem::transmute::<String<'_>, String<'static>>(self) }
    }

    // Bind this String to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your Strings cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let string = string.bind(&gc);
    // ```
    // to make sure that the unbound String cannot be used after binding.
    pub fn bind<'gc>(self, _gc: NoGcScope<'gc, '_>) -> String<'gc> {
        unsafe { std::mem::transmute::<String<'_>, String<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, String<'static>> {
        Scoped::new(agent, gc, self.unbind())
    }

    pub fn is_empty_string(self) -> bool {
        self == Self::EMPTY_STRING
    }

    pub const fn to_property_key(self) -> PropertyKey {
        match self {
            String::String(data) => PropertyKey::String(data.unbind()),
            String::SmallString(data) => PropertyKey::SmallString(data),
        }
    }

    pub const fn from_small_string(message: &'static str) -> Self {
        assert!(
            message.len() < 8
                && (message.is_empty() || message.as_bytes()[message.as_bytes().len() - 1] != 0)
        );
        String::SmallString(SmallString::from_str_unchecked(message))
    }

    pub fn concat<'gc>(
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
        strings: impl AsRef<[Self]>,
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
            Status::ExistingString(idx) => String::String(idx.bind(gc)),
            Status::SmallString { data, len } => {
                // SAFETY: Since SmallStrings are guaranteed UTF-8, `&data[..len]` is the result of
                // concatenating UTF-8 strings, which is always valid UTF-8.
                let str_slice = unsafe { std::str::from_utf8_unchecked(&data[..len]) };
                SmallString::from_str_unchecked(str_slice).into()
            }
            Status::String(string) => agent.heap.create(string).bind(gc),
        }
    }

    pub fn into_value(self) -> Value {
        self.into()
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

impl<'gc> String<'gc> {
    pub fn from_str(agent: &mut Agent, _gc: NoGcScope<'gc, '_>, str: &str) -> Self {
        agent.heap.create(str)
    }

    pub fn from_string(
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
        string: std::string::String,
    ) -> Self {
        agent.heap.create(string).bind(gc)
    }

    pub fn from_static_str(agent: &mut Agent, _gc: NoGcScope<'gc, '_>, str: &'static str) -> Self {
        if let Ok(value) = String::try_from(str) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { agent.heap.alloc_static_str(str) }
        }
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

impl CreateHeapData<StringHeapData, String<'static>> for Heap {
    fn create(&mut self, data: StringHeapData) -> String<'static> {
        self.strings.push(Some(data));
        String::String(HeapString(StringIndex::last(&self.strings)))
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

impl Rootable for String<'static> {
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
