use super::indexes::ObjectIndex;
use crate::{
    ecmascript::{
        builtins::ArgumentsList,
        execution::{Agent, JsResult},
        types::{Function, Object, PropertyDescriptor, PropertyKey, Value},
    },
    heap::Heap,
};
use std::fmt::Debug;

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

impl ObjectEntryPropertyDescriptor {
    #[inline(always)]
    pub const fn prototype_slot(idx: ObjectIndex) -> Self {
        Self::Data {
            value: Value::Object(idx),
            writable: false,
            enumerable: false,
            configurable: false,
        }
    }

    #[inline(always)]
    /// Read-only, unconfigurable, enumerable data descriptor
    pub const fn ro(value: Value) -> Self {
        Self::Data {
            value,
            writable: false,
            enumerable: true,
            configurable: false,
        }
    }
    #[inline(always)]
    /// Read-only, unconfigurable, unenumerable data descriptor
    pub const fn roh(value: Value) -> Self {
        Self::Data {
            value,
            writable: false,
            enumerable: false,
            configurable: false,
        }
    }

    #[inline(always)]
    /// Read-only, configurable, enumerable data descriptor
    pub const fn rox(value: Value) -> Self {
        Self::Data {
            value,
            writable: false,
            enumerable: true,
            configurable: true,
        }
    }
    #[inline(always)]
    /// Read-only, configurable, unenumerable data descriptor
    pub const fn roxh(value: Value) -> Self {
        Self::Data {
            value,
            writable: false,
            enumerable: false,
            configurable: true,
        }
    }

    #[inline(always)]
    /// Writable, unconfigurable, enumerable data descriptor
    pub const fn rw(value: Value) -> Self {
        Self::Data {
            value,
            writable: true,
            enumerable: false,
            configurable: false,
        }
    }
    #[inline(always)]
    /// Writable, unconfigurable, unenumerable data descriptor
    pub const fn rwh(value: Value) -> Self {
        Self::Data {
            value,
            writable: true,
            enumerable: false,
            configurable: false,
        }
    }

    #[inline(always)]
    /// Writable, configurable, enumerable data descriptor
    pub const fn rwx(value: Value) -> Self {
        Self::Data {
            value,
            writable: true,
            enumerable: false,
            configurable: true,
        }
    }
    #[inline(always)]
    /// Writable, configurable, unenumerable data descriptor
    pub const fn rwxh(value: Value) -> Self {
        Self::Data {
            value,
            writable: true,
            enumerable: false,
            configurable: true,
        }
    }
}

pub fn initialize_object_heap(_heap: &mut Heap) {
    // let entries = vec![
    //     ObjectEntry::new_prototype_function_entry(heap, "assign", 1, true),
    //     ObjectEntry::new_prototype_function_entry(heap, "create", 2, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "defineProperties", 2, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "defineProperty", 3, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "entries", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "freeze", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "fromEntries", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertyDescriptor", 2, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertyDescriptors", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertyNames", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertySymbols", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "getPrototypeOf", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "hasOwn", 2, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "is", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "isExtensible", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "isFrozen", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "isSealed", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "keys", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "preventExtensions", 1, false),
    //     ObjectEntry::new_constructor_prototype_entry(
    //         heap,
    //         IntrinsicObjectIndexes::ObjectPrototype.into(),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "seal", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "setPrototypeOf", 2, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "values", 1, false),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::ObjectConstructor,
    //     true,
    //     Some(Object::BuiltinFunction(
    //         IntrinsicObjectIndexes::FunctionPrototype.into(),
    //     )),
    //     entries,
    // );
    // heap.builtin_functions
    //     [get_constructor_index(IntrinsicObjectIndexes::ObjectConstructor).into_index()] =
    //     Some(BuiltinFunctionHeapData {
    //         object_index: Some(IntrinsicObjectIndexes::ObjectConstructor.into()),
    //         length: 1,
    //         initial_name: None,
    //         behaviour: Behaviour::Constructor(object_constructor_binding),
    //         realm: RealmIdentifier::from_index(0),
    //     });
    // let entries = vec![
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "constructor"),
    //         ObjectEntryPropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
    //             IntrinsicObjectIndexes::ObjectConstructor,
    //         ))),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "hasOwnProperty", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "isPrototypeOf", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "propertyIsEnumerable", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::ObjectConstructor,
    //     true,
    //     None,
    //     entries,
    // );
}

fn object_constructor_binding(
    _heap: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    Ok(Value::Object(ObjectIndex::from_index(0)))
}
