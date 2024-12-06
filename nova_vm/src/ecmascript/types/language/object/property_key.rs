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
    engine::{
        context::NoGcScope,
        rootable::{HeapRootData, HeapRootRef, Rootable},
        Scoped,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
    SmallInteger, SmallString,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PropertyKey<'a> {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    String(HeapString<'a>) = STRING_DISCRIMINANT,
    Symbol(Symbol<'a>) = SYMBOL_DISCRIMINANT,
    // TODO: PrivateKey
}

impl<'a> PropertyKey<'a> {
    /// Unbind this PropertyKey from its current lifetime. This is necessary to
    /// use the PropertyKey as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> PropertyKey<'static> {
        unsafe { std::mem::transmute::<Self, PropertyKey<'static>>(self) }
    }

    // Bind this PropertyKey to the garbage collection lifetime. This enables
    // Rust's borrow checker to verify that your PropertyKeys cannot not be
    // invalidated by garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let primitive = primitive.bind(&gc);
    // ```
    // to make sure that the unbound PropertyKey cannot be used after binding.
    pub const fn bind(self, _: NoGcScope<'a, '_>) -> Self {
        unsafe { std::mem::transmute::<PropertyKey<'_>, Self>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, PropertyKey<'static>> {
        Scoped::new(agent, gc, self.unbind())
    }

    // FIXME: This API is not necessarily in the right place.
    pub fn from_str(agent: &mut Agent, gc: NoGcScope<'a, '_>, str: &str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_str(agent, gc, str).into())
    }

    pub fn from_static_str(agent: &mut Agent, gc: NoGcScope<'a, '_>, str: &'static str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_static_str(agent, gc, str).into())
    }

    pub fn from_string(
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
        string: std::string::String,
    ) -> Self {
        parse_string_to_integer_property_key(&string)
            .unwrap_or_else(|| String::from_string(agent, gc, string).into())
    }

    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn from_value(agent: &Agent, gc: NoGcScope<'a, '_>, value: Value) -> Option<Self> {
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
                let s = agent[s.unbind()].as_str();

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

    pub(crate) fn as_display<'b, 'c>(
        &'b self,
        agent: &'c Agent,
    ) -> DisplayablePropertyKey<'a, 'b, 'c> {
        DisplayablePropertyKey { key: self, agent }
    }
}

#[inline(always)]
pub fn unbind_property_keys<'a>(vec: Vec<PropertyKey<'a>>) -> Vec<PropertyKey<'static>> {
    unsafe { std::mem::transmute::<Vec<PropertyKey<'a>>, Vec<PropertyKey<'static>>>(vec) }
}

#[inline(always)]
pub fn bind_property_keys<'a>(
    vec: Vec<PropertyKey<'static>>,
    gc: NoGcScope<'a, '_>,
) -> Vec<PropertyKey<'a>> {
    unsafe { std::mem::transmute::<Vec<PropertyKey<'static>>, Vec<PropertyKey<'a>>>(vec) }
}

pub(crate) struct DisplayablePropertyKey<'a, 'b, 'c> {
    key: &'b PropertyKey<'a>,
    agent: &'c Agent,
}

impl<'a, 'b, 'c> core::fmt::Display for DisplayablePropertyKey<'a, 'b, 'c> {
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
        parse_string_to_integer_property_key(value.as_str())
            .unwrap_or(PropertyKey::SmallString(value))
    }
}

impl<'a> From<Symbol<'a>> for PropertyKey<'a> {
    fn from(value: Symbol<'a>) -> Self {
        PropertyKey::Symbol(value)
    }
}

impl<'a> From<String<'a>> for PropertyKey<'a> {
    fn from(value: String<'a>) -> Self {
        match value {
            String::String(x) => PropertyKey::String(x),
            String::SmallString(x) => PropertyKey::SmallString(x),
        }
    }
}

impl From<PropertyKey<'_>> for Value {
    fn from(value: PropertyKey) -> Self {
        match value {
            PropertyKey::Integer(x) => Value::Integer(x),
            PropertyKey::SmallString(x) => Value::SmallString(x),
            PropertyKey::String(x) => Value::String(x.unbind()),
            PropertyKey::Symbol(x) => Value::Symbol(x.unbind()),
        }
    }
}

impl TryFrom<Value> for PropertyKey<'_> {
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

impl HeapMarkAndSweep for PropertyKey<'static> {
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum PropertyKeyRootRepr {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl Rootable for PropertyKey<'static> {
    type RootRepr = PropertyKeyRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            PropertyKey::Integer(small_integer) => Ok(Self::RootRepr::Integer(small_integer)),
            PropertyKey::SmallString(small_string) => Ok(Self::RootRepr::SmallString(small_string)),
            PropertyKey::String(heap_string) => Err(HeapRootData::String(heap_string)),
            PropertyKey::Symbol(symbol) => Err(HeapRootData::Symbol(symbol)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            PropertyKeyRootRepr::Integer(small_integer) => Ok(Self::Integer(small_integer)),
            PropertyKeyRootRepr::SmallString(small_string) => Ok(Self::SmallString(small_string)),
            PropertyKeyRootRepr::HeapRef(heap_root_ref) => Err(heap_root_ref),
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
            HeapRootData::Symbol(symbol) => Some(Self::Symbol(symbol)),
            _ => None,
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
