use super::indexes::{BuiltinFunctionIndex, ObjectIndex, SymbolIndex};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult},
        types::{Object, PropertyKey, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        BuiltinFunctionHeapData, Heap,
    },
};
use std::{fmt::Debug, vec};

#[derive(Debug)]
pub struct ObjectEntry {
    pub key: PropertyKey,
    pub value: PropertyDescriptor,
}

impl ObjectEntry {
    pub fn new(key: PropertyKey, value: PropertyDescriptor) -> Self {
        ObjectEntry { key, value }
    }

    pub fn new_prototype_function_entry(
        heap: &mut Heap,
        name: &str,
        length: u8,
        uses_arguments: bool,
        // behaviour: Behaviour,
    ) -> Self {
        let key = PropertyKey::from_str(heap, name);
        let name = match key {
            PropertyKey::SmallString(data) => Value::SmallString(data),
            PropertyKey::Integer(_) => unreachable!("No prototype functions should have SMI names"),
            PropertyKey::String(idx) => Value::String(idx),
            PropertyKey::Symbol(idx) => Value::Symbol(idx),
        };
        let func_index = heap.create_function(name, length, uses_arguments);
        let value = PropertyDescriptor::rwxh(Value::BuiltinFunction(func_index));
        ObjectEntry { key, value }
    }

    pub fn new_prototype_symbol_function_entry(
        heap: &mut Heap,
        name: &str,
        symbol_index: SymbolIndex,
        length: u8,
        uses_arguments: bool,
        // behaviour: Behaviour,
    ) -> Self {
        let name = Value::from_str(heap, name);
        let key = PropertyKey::Symbol(symbol_index);
        let func_index = heap.create_function(name, length, uses_arguments);
        let value = PropertyDescriptor::roxh(Value::BuiltinFunction(func_index));
        ObjectEntry { key, value }
    }

    pub fn new_constructor_prototype_entry(heap: &mut Heap, idx: ObjectIndex) -> Self {
        ObjectEntry {
            key: PropertyKey::from_str(heap, "prototype"),
            value: PropertyDescriptor::Data {
                value: Value::Object(idx),
                writable: false,
                enumerable: false,
                configurable: false,
            },
        }
    }

    pub fn new_frozen_entry(heap: &mut Heap, key: &str, value: Value) -> Self {
        ObjectEntry {
            key: PropertyKey::from_str(heap, key),
            value: PropertyDescriptor::roh(value),
        }
    }
}

#[derive(Debug)]
pub enum PropertyDescriptor {
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
        get: BuiltinFunctionIndex,
        enumerable: bool,
        configurable: bool,
    },
    WriteOnly {
        set: BuiltinFunctionIndex,
        enumerable: bool,
        configurable: bool,
    },
    ReadWrite {
        get: BuiltinFunctionIndex,
        set: BuiltinFunctionIndex,
        enumerable: bool,
        configurable: bool,
    },
}

impl PropertyDescriptor {
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

pub fn initialize_object_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "assign", 1, true),
        ObjectEntry::new_prototype_function_entry(heap, "create", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "defineProperties", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "defineProperty", 3, false),
        ObjectEntry::new_prototype_function_entry(heap, "entries", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "freeze", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "fromEntries", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertyDescriptor", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertyDescriptors", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertyNames", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "getOwnPropertySymbols", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "getPrototypeOf", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "hasOwn", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "is", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isExtensible", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isFrozen", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isSealed", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "keys", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "preventExtensions", 1, false),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::ObjectPrototype.into(),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "seal", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "setPrototypeOf", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "values", 1, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ObjectConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::ObjectConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::ObjectConstructor.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(object_constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::ObjectConstructor,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "hasOwnProperty", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isPrototypeOf", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "propertyIsEnumerable", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
    ];
    heap.insert_builtin_object(BuiltinObjectIndexes::ObjectConstructor, true, None, entries);
}

fn object_constructor_binding(
    _heap: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    Ok(Value::Object(ObjectIndex::from_index(0)))
}
