include!(concat!(env!("OUT_DIR"), "/builtin_strings.rs"));
mod data;

use super::{IntoPrimitive, IntoValue, Primitive, PropertyKey, Value};
use crate::{
    ecmascript::execution::Agent,
    heap::{indexes::StringIndex, CreateHeapData},
    SmallString,
};

pub use data::StringHeapData;

/// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum String {
    String(StringIndex),
    SmallString(SmallString),
}

impl IntoValue for StringIndex {
    fn into_value(self) -> Value {
        Value::String(self)
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

impl From<StringIndex> for String {
    fn from(value: StringIndex) -> Self {
        String::String(value)
    }
}

impl From<StringIndex> for Primitive {
    fn from(value: StringIndex) -> Self {
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
            ExistingString(StringIndex),
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
                            // TODO: This is reachable if `string` ends with a
                            // null byte.
                            todo!()
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
    pub fn len(self, agent: &Agent) -> usize {
        match self {
            String::String(s) => agent[s].len(),
            String::SmallString(s) => s.len(),
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
    pub fn eq(agent: &mut Agent, x: String, y: String) -> bool {
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

    /// ### [6.1.4.1 StringIndexOf ( string, searchValue, fromIndex )](https://tc39.es/ecma262/#sec-stringindexof)
    pub fn index_of(self, agent: &mut Agent, search_value: Self, from_index: i64) -> i64 {
        // TODO: Figure out what we should do for invalid cases.
        let string = self.as_str(agent);
        let search_value = search_value.as_str(agent);

        // 1. Let len be the length of string.
        let len = string.len() as i64;

        // 2. If searchValue is the empty String and fromIndex ≤ len, return fromIndex.
        if len == 0 && from_index <= len {
            return from_index;
        }

        // 3. Let searchLen be the length of searchValue.
        let search_len = search_value.len() as i64;

        // 4. For each integer i such that fromIndex ≤ i ≤ len - searchLen, in ascending order, do
        for i in from_index..=(len - search_len) {
            // a. Let candidate be the substring of string from i to i + searchLen.
            let candidate = &string[i as usize..(i + search_len) as usize];

            // b. If candidate is searchValue, return i.
            if candidate == search_value {
                return i;
            }
        }

        // 5. Return -1.
        -1
    }
}
