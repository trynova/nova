mod data;

use super::Value;
use crate::{
    ecmascript::execution::Agent,
    heap::{indexes::StringIndex, CreateHeapData, GetHeapData},
    SmallString,
};

pub use data::StringHeapData;

/// 6.1.4 The String Type
/// https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum String {
    String(StringIndex),
    SmallString(SmallString),
}

impl From<StringIndex> for String {
    fn from(value: StringIndex) -> Self {
        String::String(value)
    }
}

impl From<SmallString> for String {
    fn from(value: SmallString) -> Self {
        String::SmallString(value)
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

impl From<String> for Value {
    fn from(value: String) -> Self {
        match value {
            String::String(x) => Value::String(x),
            String::SmallString(x) => Value::SmallString(x),
        }
    }
}

impl String {
    pub fn from_str(agent: &mut Agent, message: &str) -> String {
        agent.heap.create(message)
    }

    pub fn from_small_string(message: &'static str) -> String {
        assert!(message.len() < 8 && !message.ends_with('\0'));
        String::SmallString(SmallString::from_str_unchecked(message))
    }

    pub fn into_value(self) -> Value {
        self.into()
    }

    /// Byte length of the string.
    pub fn len(self, agent: &Agent) -> usize {
        match self {
            String::String(s) => agent.heap.get(s).len(),
            String::SmallString(s) => s.len(),
        }
    }

    pub fn as_str<'a>(&'a self, agent: &mut Agent) -> Option<&'a str> {
        match self {
            // SAFETY: The mutable reference to the Agent ensures no mutable
            //         access to the realm.
            String::String(s) => unsafe { std::mem::transmute(agent.heap.get(*s).as_str()) },
            String::SmallString(s) => Some(s.as_str()),
        }
    }

    /// 6.1.4.1 StringIndexOf ( string, searchValue, fromIndex )
    /// https://tc39.es/ecma262/#sec-stringindexof
    pub fn index_of(self, agent: &mut Agent, search_value: Self, from_index: i64) -> i64 {
        // TODO: Figure out what we should do for invalid cases.
        let string = self.as_str(agent).unwrap();
        let search_value = search_value.as_str(agent).unwrap();

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
