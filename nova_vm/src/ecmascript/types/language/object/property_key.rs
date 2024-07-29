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

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PropertyKey<'gen> {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    String(HeapString<'gen>) = STRING_DISCRIMINANT,
    Symbol(Symbol<'gen>) = SYMBOL_DISCRIMINANT,
    // TODO: PrivateKey
}

impl<'gen> PropertyKey<'gen> {
    // FIXME: This API is not necessarily in the right place.
    pub fn from_str<'gen>(agent: &mut Agent<'gen>, str: &str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_str(agent, str).into())
    }

    pub fn from_static_str<'gen>(agent: &mut Agent<'gen>, str: &'static str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_static_str(agent, str).into())
    }

    pub fn from_string<'gen>(agent: &mut Agent<'gen>, string: std::string::String) -> Self {
        parse_string_to_integer_property_key(&string)
            .unwrap_or_else(|| String::from_string(agent, string).into())
    }

    pub fn into_value(self) -> Value<'gen> {
        self.into()
    }

    pub fn is_array_index(self) -> bool {
        // TODO: string check
        matches!(self.into_value(), Value::Integer(_))
    }

    pub(self) fn is_str_eq_num(s: &str, n: i64) -> bool {
        // TODO: Come up with some advanced algorithm.
        s == n.to_string()
    }

    pub fn equals(self, agent: &mut Agent<'gen>, y: Self) -> bool {
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

    pub(crate) fn as_display<'a, 'b>(&'a self, agent: &'b Agent<'gen>) -> DisplayablePropertyKey<'gen, 'a, 'b> {
        DisplayablePropertyKey { key: self, agent }
    }
}

pub(crate) struct DisplayablePropertyKey<'gen, 'a, 'b> {
    key: &'a PropertyKey<'gen>,
    agent: &'b Agent<'gen>,
}

impl<'a, 'b> core::fmt::Display for DisplayablePropertyKey<'_, 'a, 'b> {
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

impl From<u32> for PropertyKey<'static> {
    fn from(value: u32) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<u16> for PropertyKey<'static> {
    fn from(value: u16) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<u8> for PropertyKey<'static> {
    fn from(value: u8) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i32> for PropertyKey<'static> {
    fn from(value: i32) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i16> for PropertyKey<'static> {
    fn from(value: i16) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i8> for PropertyKey<'static> {
    fn from(value: i8) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<SmallInteger> for PropertyKey<'static> {
    fn from(value: SmallInteger) -> Self {
        PropertyKey::Integer(value)
    }
}

impl From<SmallString> for PropertyKey<'static> {
    fn from(value: SmallString) -> Self {
        PropertyKey::SmallString(value)
    }
}

impl<'gen> From<HeapString<'gen>> for PropertyKey<'gen> {
    fn from(value: HeapString<'gen>) -> Self {
        PropertyKey::String(value)
    }
}

impl<'gen> From<Symbol<'gen>> for PropertyKey<'gen> {
    fn from(value: Symbol<'gen>) -> Self {
        PropertyKey::Symbol(value)
    }
}

impl<'gen> From<String<'gen>> for PropertyKey<'gen> {
    fn from(value: String<'gen>) -> Self {
        match value {
            String::String(x) => PropertyKey::String(x),
            String::SmallString(x) => PropertyKey::SmallString(x),
        }
    }
}

impl<'gen> From<PropertyKey<'gen>> for Value<'gen> {
    fn from(value: PropertyKey<'gen>) -> Self {
        match value {
            PropertyKey::Integer(x) => Value::Integer(x),
            PropertyKey::SmallString(x) => Value::SmallString(x),
            PropertyKey::String(x) => Value::String(x),
            PropertyKey::Symbol(x) => Value::Symbol(x),
        }
    }
}

impl<'gen> TryFrom<Value<'gen>> for PropertyKey<'gen> {
    type Error = ();

    #[inline(always)]
    fn try_from(value: Value<'gen>) -> Result<Self, Self::Error> {
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

impl TryFrom<i64> for PropertyKey<'static> {
    type Error = ();

    fn try_from(value: i64) -> Result<Self, ()> {
        Ok(PropertyKey::Integer(SmallInteger::try_from(value)?))
    }
}

impl TryFrom<usize> for PropertyKey<'static> {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, ()> {
        if let Ok(i64) = i64::try_from(value) {
            Self::try_from(i64)
        } else {
            Err(())
        }
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for PropertyKey<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
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
