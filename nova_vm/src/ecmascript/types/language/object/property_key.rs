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
        unsafe { core::mem::transmute::<Self, PropertyKey<'static>>(self) }
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
        unsafe { core::mem::transmute::<PropertyKey, Self>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, PropertyKey<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub const fn scope_static(self) -> Scoped<'static, PropertyKey<'static>> {
        let key_root_repr = match self {
            PropertyKey::Integer(small_integer) => PropertyKeyRootRepr::Integer(small_integer),
            PropertyKey::SmallString(small_string) => {
                PropertyKeyRootRepr::SmallString(small_string)
            }
            _ => panic!("PropertyKey required rooting"),
        };
        Scoped::from_root_repr(key_root_repr)
    }

    // FIXME: This API is not necessarily in the right place.
    pub fn from_str(agent: &mut Agent, str: &str, gc: NoGcScope<'a, '_>) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_str(agent, str, gc).into())
    }

    pub fn from_static_str(agent: &mut Agent, str: &'static str, gc: NoGcScope<'a, '_>) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_static_str(agent, str, gc).into())
    }

    pub fn from_string(
        agent: &mut Agent,
        string: std::string::String,
        gc: NoGcScope<'a, '_>,
    ) -> Self {
        parse_string_to_integer_property_key(&string)
            .unwrap_or_else(|| String::from_string(agent, string, gc).into())
    }

    /// Convert a PropertyKey into a Value.
    ///
    /// This converts any integer keys into strings. This matches what the
    /// ECMAScript specification expects.
    pub fn convert_to_value(self, agent: &mut Agent, gc: NoGcScope) -> Value {
        match self {
            PropertyKey::Integer(small_integer) => {
                Value::from_string(agent, format!("{}", small_integer.into_i64()), gc)
            }
            PropertyKey::SmallString(small_string) => Value::SmallString(small_string),
            PropertyKey::String(heap_string) => Value::String(heap_string.unbind()),
            PropertyKey::Symbol(symbol) => Value::Symbol(symbol.unbind()),
        }
    }

    /// Convert a PropertyKey into a Value directly.
    ///
    /// This does not convert integer keys into strings. This is not correct
    /// from the specification point of view and should only be done when
    /// used with other directly converted PropertyKeys.
    ///
    /// ## Safety
    ///
    /// If the resulting PropertyKey is mixed with normal JavaScript values or
    /// passed to user code, the resulting JavaScript will not necessarily
    /// correctly match the ECMAScript specification or user's expectations.
    pub(crate) unsafe fn into_value_unchecked(self) -> Value {
        match self {
            PropertyKey::Integer(small_integer) => Value::Integer(small_integer),
            PropertyKey::SmallString(small_string) => Value::SmallString(small_string),
            PropertyKey::String(heap_string) => Value::String(heap_string.unbind()),
            PropertyKey::Symbol(symbol) => Value::Symbol(symbol.unbind()),
        }
    }

    /// Reinterpret a Value as a PropertyKey directly.
    ///
    /// This does not check strings for being integer-like. This is problematic
    /// from the engine point of view if an integer-string like Value gets
    /// reinterpreted as a PropertyKey without the conversion.
    ///
    /// ## Safety
    ///
    /// If the source Value is an integer key string, then using the resulting
    /// PropertyKey may not match the ECMAScript specification or user's
    /// expectations.
    ///
    /// ## Panics
    ///
    /// If the passed-in Value is not a string, integer, or symbol, the method
    /// will panic.
    pub(crate) unsafe fn from_value_unchecked(value: Value) -> Self {
        match value {
            Value::Integer(small_integer) => PropertyKey::Integer(small_integer),
            Value::SmallString(small_string) => PropertyKey::SmallString(small_string),
            Value::String(heap_string) => PropertyKey::String(heap_string.unbind()),
            Value::Symbol(symbol) => PropertyKey::Symbol(symbol.unbind()),
            _ => unreachable!(),
        }
    }

    pub fn is_array_index(self) -> bool {
        matches!(self, PropertyKey::Integer(_))
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

    /// Returns true if the PropertyKey is a Symbol.
    pub fn is_symbol(&self) -> bool {
        matches!(self, PropertyKey::Symbol(_))
    }

    /// Returns true if the PropertyKey is a String according to the ECMAScript
    /// specification.
    ///
    /// > Note: This returns true for Integer property keys as well.
    pub fn is_string(&self) -> bool {
        matches!(
            self,
            PropertyKey::String(_) | PropertyKey::SmallString(_) | PropertyKey::Integer(_)
        )
    }
}

#[inline(always)]
pub fn unbind_property_keys<'a>(vec: Vec<PropertyKey<'a>>) -> Vec<PropertyKey<'static>> {
    unsafe { core::mem::transmute::<Vec<PropertyKey<'a>>, Vec<PropertyKey<'static>>>(vec) }
}

#[inline(always)]
pub fn bind_property_keys<'a>(vec: Vec<PropertyKey>, _: NoGcScope<'a, '_>) -> Vec<PropertyKey<'a>> {
    unsafe { core::mem::transmute::<Vec<PropertyKey>, Vec<PropertyKey<'a>>>(vec) }
}

#[inline]
pub fn scope_property_keys<'a>(
    agent: &mut Agent,
    keys: Vec<PropertyKey>,
    gc: NoGcScope<'_, 'a>,
) -> Vec<Scoped<'a, PropertyKey<'static>>> {
    keys.into_iter()
        .map(|k| k.scope(agent, gc))
        .collect::<Vec<_>>()
}

pub(crate) struct DisplayablePropertyKey<'a, 'b, 'c> {
    key: &'b PropertyKey<'a>,
    agent: &'c Agent,
}

impl core::fmt::Display for DisplayablePropertyKey<'_, '_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
    /// Note: You should not be using this conversion without thinking. Integer
    /// keys don't actually become proper strings here, so converting a
    /// PropertyKey into a Value using this and then comparing that with an
    /// actual Value is unsound.
    fn from(value: PropertyKey) -> Self {
        // SAFETY: Don't be silly!
        unsafe { value.into_value_unchecked() }
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
