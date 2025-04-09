// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::RegExpFlags;

use crate::{
    ecmascript::types::{OrdinaryObject, PropertyDescriptor, String, Value},
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

/// ## Optimistic storage for the RegExp "lastIndex" property
///
/// The property can take any JavaScript Value, but under any reasonable use it
/// is an index into a JavaScript String which then means it is a 32-bit
/// unsigned integer since strings of 4 GiB length or larger are rare.
///
/// As such, the optimistic storage is a 32-bit unsigned integer with the
/// `u32::MAX` value used as a sentinel to signify that the backing object
/// should be asked for the actual value. If the backing object doesn't exist,
/// then the property value is `undefined`.
///
/// ### Writability of the "lastIndex" property
///
/// The `lastIndex` property can be made read-only using
/// `Object.defineProperty` or `Object.freeze`. These are rare operations and
/// as such, the writable bit of the "lastIndex" property is not stored in the
/// RegExp data at all. This means that if the backing object does not exists,
/// then the writable bit is guaranteed to be `true`. If the backing object
/// does exist, the writable bit must be asked from the backing object in all
/// cases.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct RegExpLastIndex(u32);

impl Default for RegExpLastIndex {
    fn default() -> Self {
        Self(u32::MAX)
    }
}

impl RegExpLastIndex {
    pub(crate) const ZERO: Self = Self(0);
    const INVALID: Self = Self(u32::MAX);

    /// Create a RegExpLastIndex from a JavaScript Value.
    pub(super) fn from_value(value: Value) -> Self {
        let Value::Integer(value) = value else {
            return Self::INVALID;
        };
        let value = value.into_i64();
        if let Ok(value) = u32::try_from(value) {
            if value == u32::MAX {
                Self::INVALID
            } else {
                Self(value)
            }
        } else {
            Self::INVALID
        }
    }

    /// Check if the RegExpLastIndex is an accurate JavaScript value.
    ///
    /// Note: If backing object does not exist, `undefined` is also accurately
    /// represented by an invalid RegExpLastIndex.
    #[inline]
    pub(super) fn is_valid(self) -> bool {
        self.0 != u32::MAX
    }

    /// Get the current lastIndex property index value.
    ///
    /// If the current value is not a 32-bit unsigned integer, then None is
    /// returned.
    pub(super) fn get_value(self) -> Option<u32> {
        if self.0 == u32::MAX {
            None
        } else {
            Some(self.0)
        }
    }

    /// Get the lastIndex property descriptor value.
    ///
    /// This property descriptor is only valid if the backing object does not
    /// exist. If it does exist, then this both the value and the writable bit
    /// of this descriptor may differ from the real values.
    pub(super) fn into_property_descriptor(self) -> PropertyDescriptor {
        PropertyDescriptor {
            value: Some(self.get_value().map_or(Value::Undefined, |i| i.into())),
            writable: Some(true),
            get: None,
            set: None,
            enumerable: Some(false),
            configurable: Some(false),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RegExpHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    // _regex: RegExp,
    pub(crate) original_source: String<'a>,
    pub(crate) original_flags: RegExpFlags,
    pub(crate) last_index: RegExpLastIndex,
}

impl Default for RegExpHeapData<'_> {
    fn default() -> Self {
        Self {
            object_index: Default::default(),
            original_source: String::EMPTY_STRING,
            original_flags: RegExpFlags::empty(),
            last_index: Default::default(),
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for RegExpHeapData<'_> {
    type Of<'a> = RegExpHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for RegExpHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            original_source,
            original_flags: _,
            last_index: _,
        } = self;
        object_index.mark_values(queues);
        original_source.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            original_source,
            original_flags: _,
            last_index: _,
        } = self;
        object_index.sweep_values(compactions);
        original_source.sweep_values(compactions);
    }
}
