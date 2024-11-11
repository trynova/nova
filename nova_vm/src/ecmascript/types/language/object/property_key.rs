// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::parse_string_to_integer_property_key,
        execution::Agent,
        types::{
            language::{
                string::HeapString,
                value::{
                    INTEGER_DISCRIMINANT, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT,
                    SYMBOL_DISCRIMINANT,
                },
            },
            String, Symbol, Value,
        },
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
    SmallInteger, SmallString,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PropertyKey {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    String(HeapString) = STRING_DISCRIMINANT,
    Symbol(Symbol) = SYMBOL_DISCRIMINANT,
    // TODO: PrivateKey
}

impl PropertyKey {
    // FIXME: This API is not necessarily in the right place.
    pub fn from_str(agent: &mut Agent, str: &str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_str(agent, str).into())
    }

    pub fn from_static_str(agent: &mut Agent, str: &'static str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_static_str(agent, str).into())
    }

    pub fn from_string(agent: &mut Agent, string: std::string::String) -> Self {
        parse_string_to_integer_property_key(&string)
            .unwrap_or_else(|| String::from_string(agent, string).into())
    }

    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn from_value(agent: &Agent, value: Value) -> Option<Self> {
        if let Ok(string) = String::try_from(value) {
            if let Some(pk) = parse_string_to_integer_property_key(string.as_str(agent)) {
                return Some(pk);
            }
        }
        Self::try_from(value).ok()
    }

    pub fn is_array_index(self) -> bool {
        // TODO: string check
        matches!(self.into_value(), Value::Integer(_))
    }

    pub(self) fn is_str_eq_num(s: &str, n: i64) -> bool {
        // TODO: Come up with some advanced algorithm.
        s == n.to_string()
    }

    pub fn equals(self, agent: &Agent, y: Self) -> bool {
        let x = self;

        match (x, y) {
            // Assumes the interner is working correctly.
            (PropertyKey::String(s1), PropertyKey::String(s2)) => s1 == s2,
            (PropertyKey::SmallString(s1), PropertyKey::SmallString(s2)) => {
                s1.as_str() == s2.as_str()
            }
            (PropertyKey::String(s), PropertyKey::Integer(n)) => {
                let s = agent[s].as_str();

                Self::is_str_eq_num(s, n.into_i64())
            }
            (PropertyKey::SmallString(s), PropertyKey::Integer(n)) => {
                Self::is_str_eq_num(s.as_str(), n.into_i64())
            }
            (PropertyKey::Integer(n1), PropertyKey::Integer(n2)) => n1.into_i64() == n2.into_i64(),
            (PropertyKey::Integer(_), _) => y.equals(agent, self),
            _ => false,
        }
    }

    pub(crate) fn as_display<'a, 'b>(&'a self, agent: &'b Agent) -> DisplayablePropertyKey<'a, 'b> {
        DisplayablePropertyKey { key: self, agent }
    }
}

pub(crate) struct DisplayablePropertyKey<'a, 'b> {
    key: &'a PropertyKey,
    agent: &'b Agent,
}

impl<'a, 'b> core::fmt::Display for DisplayablePropertyKey<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.key {
            PropertyKey::Integer(data) => data.into_i64().fmt(f),
            PropertyKey::SmallString(data) => data.as_str().fmt(f),
            PropertyKey::String(data) => data.as_str(self.agent).fmt(f),
            PropertyKey::Symbol(data) => {
                if let Some(descriptor) = self.agent[*data].descriptor {
                    let descriptor = descriptor.as_str(self.agent);
                    f.debug_tuple("Symbol").field(&descriptor).finish()
                } else {
                    "Symbol()".fmt(f)
                }
            }
        }
    }
}

impl From<u32> for PropertyKey {
    fn from(value: u32) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<u16> for PropertyKey {
    fn from(value: u16) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<u8> for PropertyKey {
    fn from(value: u8) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i32> for PropertyKey {
    fn from(value: i32) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i16> for PropertyKey {
    fn from(value: i16) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i8> for PropertyKey {
    fn from(value: i8) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<SmallInteger> for PropertyKey {
    fn from(value: SmallInteger) -> Self {
        PropertyKey::Integer(value)
    }
}

impl From<SmallString> for PropertyKey {
    fn from(value: SmallString) -> Self {
        parse_string_to_integer_property_key(value.as_str())
            .unwrap_or(PropertyKey::SmallString(value))
    }
}

impl From<Symbol> for PropertyKey {
    fn from(value: Symbol) -> Self {
        PropertyKey::Symbol(value)
    }
}

impl From<String> for PropertyKey {
    fn from(value: String) -> Self {
        match value {
            String::String(x) => PropertyKey::String(x),
            String::SmallString(x) => PropertyKey::SmallString(x),
        }
    }
}

impl From<PropertyKey> for Value {
    fn from(value: PropertyKey) -> Self {
        match value {
            PropertyKey::Integer(x) => Value::Integer(x),
            PropertyKey::SmallString(x) => Value::SmallString(x),
            PropertyKey::String(x) => Value::String(x),
            PropertyKey::Symbol(x) => Value::Symbol(x),
        }
    }
}

impl TryFrom<Value> for PropertyKey {
    type Error = ();

    #[inline(always)]
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(x) => Ok(PropertyKey::Integer(x)),
            Value::SmallF64(x) => {
                let x = x.into_f64();
                if x == -0.0 {
                    Ok(PropertyKey::Integer(0.into()))
                } else if x.fract() == 0.0
                    && (SmallInteger::MIN_NUMBER..=SmallInteger::MAX_NUMBER).contains(&(x as i64))
                {
                    unreachable!("Value::Float should not contain safe integers");
                } else {
                    Err(())
                }
            }
            Value::SmallString(x) => Ok(PropertyKey::SmallString(x)),
            Value::String(x) => Ok(PropertyKey::String(x)),
            Value::Symbol(x) => Ok(PropertyKey::Symbol(x)),
            Value::SmallBigInt(x)
                if (SmallInteger::MIN_NUMBER..=SmallInteger::MAX_NUMBER)
                    .contains(&x.into_i64()) =>
            {
                Ok(PropertyKey::Integer(x.into_inner()))
            }
            _ => Err(()),
        }
    }
}

impl TryFrom<i64> for PropertyKey {
    type Error = ();

    fn try_from(value: i64) -> Result<Self, ()> {
        Ok(PropertyKey::Integer(SmallInteger::try_from(value)?))
    }
}

impl TryFrom<usize> for PropertyKey {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, ()> {
        if let Ok(i64) = i64::try_from(value) {
            Self::try_from(i64)
        } else {
            Err(())
        }
    }
}

impl HeapMarkAndSweep for PropertyKey {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            PropertyKey::Integer(_) => {}
            PropertyKey::SmallString(_) => {}
            PropertyKey::String(string) => string.mark_values(queues),
            PropertyKey::Symbol(symbol) => symbol.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            PropertyKey::Integer(_) => {}
            PropertyKey::SmallString(_) => {}
            PropertyKey::String(string) => string.sweep_values(compactions),
            PropertyKey::Symbol(symbol) => symbol.sweep_values(compactions),
        }
    }
}

#[test]
fn compare_num_str() {
    assert!(PropertyKey::is_str_eq_num("23", 23));
    assert!(PropertyKey::is_str_eq_num("-23", -23));
    assert!(PropertyKey::is_str_eq_num("-120543809", -120543809));
    assert!(PropertyKey::is_str_eq_num("985493", 985493));
    assert!(PropertyKey::is_str_eq_num("0", 0));
    assert!(PropertyKey::is_str_eq_num("5", 5));
    assert!(PropertyKey::is_str_eq_num("-5", -5));
    assert!(PropertyKey::is_str_eq_num("9302", 9302));
    assert!(PropertyKey::is_str_eq_num("19", 19));

    assert!(!PropertyKey::is_str_eq_num("19", 91));
    assert!(!PropertyKey::is_str_eq_num("-19", 19));
}
