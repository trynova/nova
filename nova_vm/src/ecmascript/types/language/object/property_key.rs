// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::{Hash, Hasher};

use ahash::AHasher;
use wtf8::{Wtf8, Wtf8Buf};

use crate::{
    ecmascript::{
    SmallInteger, SmallString,
        Agent, HeapString, INTEGER_DISCRIMINANT, Primitive, PrivateName, SMALL_STRING_DISCRIMINANT,
        STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT, String, Symbol, Value,
        parse_string_to_integer_property_key,
    },
    engine::{
        Scoped,
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{ArenaAccess, CompactionLists, HeapMarkAndSweep, PropertyKeyHeapAccess, WorkQueues},
};

const PRIVATE_NAME_DISCRIMINANT: u8 = SYMBOL_DISCRIMINANT + 0b1000_0000;

/// # [Property key](https://tc39.es/ecma262/#property-key)
///
/// The properties of an object are uniquely identified using property keys. A
/// _property key_ is either a String or a Symbol. All Strings and Symbols,
/// including the empty String, are valid as property keys. A _property name_
/// is a property key that is a String.
///
/// An _integer index_ is a property name `n` such that
/// `CanonicalNumericIndexString(n)` returns an
/// [integral Number](crate::ecmascript::types::Number::Integer) in the
/// inclusive interval from `+0𝔽` to `𝔽(2**53 - 1)`. An _array index_ is an
/// integer index `n` such that `CanonicalNumericIndexString(n)` returns an
/// [integral Number](crate::ecmascript::types::Number::Integer) in the
/// inclusive interval from `+0𝔽` to `𝔽(2**32 - 2)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PropertyKey<'a> {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    String(HeapString<'a>) = STRING_DISCRIMINANT,
    Symbol(Symbol<'a>) = SYMBOL_DISCRIMINANT,
    PrivateName(PrivateName) = PRIVATE_NAME_DISCRIMINANT,
}

impl<'a> PropertyKey<'a> {
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
    pub fn convert_to_value<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> Primitive<'gc> {
        match self {
            PropertyKey::Integer(small_integer) => {
                String::from_string(agent, small_integer.into_i64().to_string(), gc).into()
            }
            PropertyKey::SmallString(small_string) => Primitive::SmallString(small_string),
            PropertyKey::String(heap_string) => Primitive::String(heap_string.unbind()),
            PropertyKey::Symbol(symbol) => Primitive::Symbol(symbol.unbind()),
            PropertyKey::PrivateName(_) => unreachable!(),
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
    #[inline(always)]
    pub(crate) unsafe fn into_value_unchecked(self) -> Value<'a> {
        match self {
            PropertyKey::Integer(small_integer) => Value::Integer(small_integer),
            PropertyKey::SmallString(small_string) => Value::SmallString(small_string),
            PropertyKey::String(heap_string) => Value::String(heap_string),
            PropertyKey::Symbol(symbol) => Value::Symbol(symbol),
            PropertyKey::PrivateName(_) => unreachable!(),
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
    /// Caller must ensure this is value is a valid PropertyKey.
    ///
    /// If the source Value is an integer key string, then using the resulting
    /// PropertyKey may not match the ECMAScript specification or user's
    /// expectations.
    ///
    /// ## Panics
    ///
    /// If the passed-in Value is not a string, integer, or symbol, the method
    /// will panic.
    #[inline(always)]
    pub(crate) unsafe fn from_value_unchecked(value: Value<'a>) -> Self {
        match value {
            Value::Integer(small_integer) => PropertyKey::Integer(small_integer),
            Value::SmallString(small_string) => PropertyKey::SmallString(small_string),
            Value::String(heap_string) => PropertyKey::String(heap_string),
            Value::Symbol(symbol) => PropertyKey::Symbol(symbol),
            _ => unreachable!(),
        }
    }

    pub fn is_array_index(self) -> bool {
        matches!(self, PropertyKey::Integer(_))
    }

    /// Extract a JavaScript Array index (an integer in the inclusive range of
    /// `0..=(2^32 - 2)`) from a PropertyKey if it matches the range.
    ///
    /// Returns None if the PropertyKey does not match the range.
    pub fn into_u32(self) -> Option<u32> {
        let PropertyKey::Integer(int) = self else {
            return None;
        };
        let int = int.into_i64();
        if !(0..=(2i64.pow(32) - 2)).contains(&int) {
            return None;
        }
        Some(int as u32)
    }

    pub(self) fn is_str_eq_num(s: &Wtf8, n: i64) -> bool {
        // TODO: Come up with some advanced algorithm.
        s == Wtf8Buf::from_string(n.to_string())
    }

    pub fn equals(self, agent: &Agent, y: Self) -> bool {
        let x = self;

        match (x, y) {
            // Assumes the interner is working correctly.
            (PropertyKey::String(s1), PropertyKey::String(s2)) => s1 == s2,
            (PropertyKey::SmallString(s1), PropertyKey::SmallString(s2)) => {
                s1.as_wtf8() == s2.as_wtf8()
            }
            (PropertyKey::String(s), PropertyKey::Integer(n)) => {
                let s = s.unbind().get(agent).as_wtf8();

                Self::is_str_eq_num(s, n.into_i64())
            }
            (PropertyKey::SmallString(s), PropertyKey::Integer(n)) => {
                Self::is_str_eq_num(s.as_wtf8(), n.into_i64())
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

    /// Returns true if the PropertyKey is a PrivateName.
    pub fn is_private_name(&self) -> bool {
        matches!(self, PropertyKey::PrivateName(_))
    }

    pub(crate) fn heap_hash(self, agent: &impl PropertyKeyHeapAccess) -> u64 {
        let mut hasher = AHasher::default();
        match self {
            PropertyKey::Symbol(sym) => {
                core::mem::discriminant(&self).hash(&mut hasher);
                sym.hash(&mut hasher);
            }
            PropertyKey::String(s) => {
                // Skip discriminant hashing in strings
                s.get(agent).data.hash(&mut hasher);
            }
            PropertyKey::SmallString(s) => {
                s.as_wtf8().hash(&mut hasher);
            }
            PropertyKey::Integer(n) => n.into_i64().hash(&mut hasher),
            PropertyKey::PrivateName(p) => {
                core::mem::discriminant(&self).hash(&mut hasher);
                p.hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

bindable_handle!(PropertyKey);

pub(crate) struct DisplayablePropertyKey<'a, 'b, 'c> {
    key: &'b PropertyKey<'a>,
    agent: &'c Agent,
}

impl core::fmt::Display for DisplayablePropertyKey<'_, '_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.key {
            PropertyKey::Integer(data) => data.into_i64().fmt(f),
            PropertyKey::SmallString(data) => data.to_string_lossy().fmt(f),
            PropertyKey::String(data) => data.to_string_lossy(self.agent).fmt(f),
            PropertyKey::Symbol(data) => {
                if let Some(descriptor) = data.get(self.agent).descriptor {
                    let descriptor = descriptor.to_string_lossy_(self.agent);
                    f.debug_tuple("Symbol").field(&descriptor).finish()
                } else {
                    "Symbol()".fmt(f)
                }
            }
            PropertyKey::PrivateName(data) => {
                write!(f, "##{}", data.into_u32())
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
            String::SmallString(x) => {
                // NOTE: Makes property keys slightly more correct by converting
                // small strings to integers when possible.
                if let Ok(n) = x.to_string_lossy().parse::<i64>() {
                    return PropertyKey::Integer(SmallInteger::try_from(n).unwrap());
                }

                PropertyKey::SmallString(x)
            }
        }
    }
}

impl<'a> From<PropertyKey<'a>> for Value<'a> {
    /// Note: You should not be using this conversion without thinking. Integer
    /// keys don't actually become proper strings here, so converting a
    /// PropertyKey into a Value using this and then comparing that with an
    /// actual Value is unsound.
    fn from(value: PropertyKey<'a>) -> Self {
        // SAFETY: Don't be silly!
        unsafe { value.into_value_unchecked() }
    }
}

impl TryFrom<u64> for PropertyKey<'static> {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, ()> {
        Ok(PropertyKey::Integer(SmallInteger::try_from(value)?))
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
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::PrivateName(_) => {
            }
            PropertyKey::String(string) => string.mark_values(queues),
            PropertyKey::Symbol(symbol) => symbol.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::PrivateName(_) => {
            }
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
    PrivateName(PrivateName) = PRIVATE_NAME_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl Rootable for PropertyKey<'_> {
    type RootRepr = PropertyKeyRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            PropertyKey::Integer(small_integer) => Ok(Self::RootRepr::Integer(small_integer)),
            PropertyKey::SmallString(small_string) => Ok(Self::RootRepr::SmallString(small_string)),
            PropertyKey::String(heap_string) => Err(HeapRootData::String(heap_string.unbind())),
            PropertyKey::Symbol(symbol) => Err(HeapRootData::Symbol(symbol.unbind())),
            PropertyKey::PrivateName(p) => Ok(Self::RootRepr::PrivateName(p)),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            PropertyKeyRootRepr::Integer(small_integer) => Ok(Self::Integer(small_integer)),
            PropertyKeyRootRepr::SmallString(small_string) => Ok(Self::SmallString(small_string)),
            PropertyKeyRootRepr::PrivateName(p) => Ok(Self::PrivateName(p)),
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
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("23"), 23));
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("-23"), -23));
    assert!(PropertyKey::is_str_eq_num(
        Wtf8::from_str("-120543809"),
        -120543809
    ));
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("985493"), 985493));
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("0"), 0));
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("5"), 5));
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("-5"), -5));
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("9302"), 9302));
    assert!(PropertyKey::is_str_eq_num(Wtf8::from_str("19"), 19));

    assert!(!PropertyKey::is_str_eq_num(Wtf8::from_str("19"), 91));
    assert!(!PropertyKey::is_str_eq_num(Wtf8::from_str("-19"), 19));
}
