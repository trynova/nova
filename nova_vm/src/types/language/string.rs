use super::Value;
use crate::{execution::Agent, heap::GetHeapData, SmallString};

/// 6.1.4 The String Type
/// https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type
#[derive(Debug)]
pub struct String(Value);

impl TryFrom<&str> for String {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        SmallString::try_from(value).map(|s| String::new(Value::SmallString(s)))
    }
}

impl String {
    pub(crate) fn new(value: Value) -> Self {
        matches!(value, Value::String(_) | Value::SmallString(_));
        Self(value)
    }

    pub fn into_value(self) -> Value {
        self.0
    }

    /// Byte length of the string.
    pub fn len(self, agent: &Agent) -> usize {
        let s = self.into_value();

        match s {
            Value::String(s) => agent.heap.get(s).len(),
            Value::SmallString(s) => s.len(),
            _ => unreachable!(),
        }
    }

    pub fn as_str<'a>(&'a self, agent: &'a Agent) -> Option<&'a str> {
        match &self.0 {
            Value::String(s) => agent.heap.get(*s).as_str(),
            Value::SmallString(s) => Some(s.as_str()),
            _ => unreachable!(),
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
            return from_index as i64;
        }

        // 3. Let searchLen be the length of searchValue.
        let search_len = search_value.len() as i64;

        // 4. For each integer i such that fromIndex ≤ i ≤ len - searchLen, in ascending order, do
        for i in from_index..=(len - search_len) as i64 {
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
