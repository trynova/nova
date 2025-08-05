// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::types::{Function, PropertyDescriptor, PropertyKey, Value};

#[derive(Debug, Clone, Copy)]
pub struct ObjectEntry<'a> {
    pub key: PropertyKey<'a>,
    pub value: ObjectEntryPropertyDescriptor<'a>,
}

impl<'a> ObjectEntry<'a> {
    pub(crate) fn new_data_entry(key: PropertyKey<'a>, value: Value<'a>) -> Self {
        Self {
            key,
            value: ObjectEntryPropertyDescriptor::Data {
                value,
                writable: true,
                enumerable: true,
                configurable: true,
            },
        }
    }

    /// Returns true if the entry is a data entry with WEC bits all true.
    pub(crate) fn is_trivial(&self) -> bool {
        matches!(
            self.value,
            ObjectEntryPropertyDescriptor::Data {
                writable: true,
                enumerable: true,
                configurable: true,
                ..
            }
        )
    }
}

impl<'a> From<PropertyDescriptor<'a>> for ObjectEntryPropertyDescriptor<'a> {
    fn from(desc: PropertyDescriptor<'a>) -> Self {
        let configurable = desc.configurable.unwrap_or(true);
        let enumerable = desc.enumerable.unwrap_or(true);
        let get = desc.get.flatten();
        let set = desc.set.flatten();
        if let (Some(get), Some(set)) = (get, set) {
            ObjectEntryPropertyDescriptor::ReadWrite {
                get,
                set,
                enumerable,
                configurable,
            }
        } else if let Some(get) = get {
            ObjectEntryPropertyDescriptor::ReadOnly {
                get,
                enumerable,
                configurable,
            }
        } else if let Some(set) = set {
            ObjectEntryPropertyDescriptor::WriteOnly {
                set,
                enumerable,
                configurable,
            }
        } else if let Some(value) = desc.value {
            ObjectEntryPropertyDescriptor::Data {
                value,
                writable: desc.writable.unwrap_or(true),
                enumerable,
                configurable,
            }
        } else if desc.writable == Some(false) {
            ObjectEntryPropertyDescriptor::Blocked {
                enumerable,
                configurable,
            }
        } else {
            todo!()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ObjectEntryPropertyDescriptor<'a> {
    Data {
        value: Value<'a>,
        writable: bool,
        enumerable: bool,
        configurable: bool,
    },
    Blocked {
        enumerable: bool,
        configurable: bool,
    },
    ReadOnly {
        get: Function<'a>,
        enumerable: bool,
        configurable: bool,
    },
    WriteOnly {
        set: Function<'a>,
        enumerable: bool,
        configurable: bool,
    },
    ReadWrite {
        get: Function<'a>,
        set: Function<'a>,
        enumerable: bool,
        configurable: bool,
    },
}
