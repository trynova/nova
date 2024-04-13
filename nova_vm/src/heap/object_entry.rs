use crate::ecmascript::types::{Function, PropertyDescriptor, PropertyKey, Value};

#[derive(Debug)]
pub(crate) struct ObjectEntry {
    pub key: PropertyKey,
    pub value: ObjectEntryPropertyDescriptor,
}

impl From<PropertyDescriptor> for ObjectEntryPropertyDescriptor {
    fn from(value: PropertyDescriptor) -> Self {
        let configurable = value.configurable.unwrap_or(true);
        let enumerable = value.enumerable.unwrap_or(true);
        if value.get.is_some() && value.set.is_some() {
            ObjectEntryPropertyDescriptor::ReadWrite {
                get: value.get.unwrap(),
                set: value.set.unwrap(),
                enumerable,
                configurable,
            }
        } else if value.get.is_some() {
            ObjectEntryPropertyDescriptor::ReadOnly {
                get: value.get.unwrap(),
                enumerable,
                configurable,
            }
        } else if value.set.is_some() {
            ObjectEntryPropertyDescriptor::WriteOnly {
                set: value.set.unwrap(),
                enumerable,
                configurable,
            }
        } else if value.value.is_some() {
            ObjectEntryPropertyDescriptor::Data {
                value: value.value.unwrap(),
                writable: value.writable.unwrap_or(true),
                enumerable,
                configurable,
            }
        } else if value.writable == Some(false) {
            ObjectEntryPropertyDescriptor::Blocked {
                enumerable,
                configurable,
            }
        } else {
            todo!()
        }
    }
}

#[derive(Debug)]
pub(crate) enum ObjectEntryPropertyDescriptor {
    Data {
        value: Value,
        writable: bool,
        enumerable: bool,
        configurable: bool,
    },
    Blocked {
        enumerable: bool,
        configurable: bool,
    },
    ReadOnly {
        get: Function,
        enumerable: bool,
        configurable: bool,
    },
    WriteOnly {
        set: Function,
        enumerable: bool,
        configurable: bool,
    },
    ReadWrite {
        get: Function,
        set: Function,
        enumerable: bool,
        configurable: bool,
    },
}
