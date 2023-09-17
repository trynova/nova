use std::collections::HashMap;

use crate::{
    execution::JsResult,
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap,
    },
    types::Value,
    SmallString,
};
use std::{fmt::Debug, vec};

use super::{
    element_array::ElementsVector,
    indexes::{FunctionIndex, ObjectIndex, StringIndex, SymbolIndex},
};

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
            PropertyKey::SmallString(data) => Value::SmallString(data.clone()),
            PropertyKey::Smi(_) => unreachable!("No prototype functions should have SMI names"),
            PropertyKey::String(idx) => Value::String(idx),
            PropertyKey::Symbol(idx) => Value::Symbol(idx),
        };
        let func_index = heap.create_function(name, length, uses_arguments);
        let value = PropertyDescriptor::rwxh(Value::Function(func_index));
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
        let value = PropertyDescriptor::roxh(Value::Function(func_index));
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
pub enum PropertyKey {
    SmallString(SmallString),
    Smi(i32),
    String(StringIndex),
    Symbol(SymbolIndex),
}

impl PropertyKey {
    pub fn from_str(heap: &mut Heap, str: &str) -> Self {
        if let Ok(ascii_string) = SmallString::try_from(str) {
            PropertyKey::SmallString(ascii_string)
        } else {
            PropertyKey::String(heap.alloc_string(str))
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
        get: FunctionIndex,
        enumerable: bool,
        configurable: bool,
    },
    WriteOnly {
        set: FunctionIndex,
        enumerable: bool,
        configurable: bool,
    },
    ReadWrite {
        get: FunctionIndex,
        set: FunctionIndex,
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

#[derive(Debug, Clone, Copy)]
pub struct ObjectHeapData {
    pub extensible: bool,
    // TODO: It's probably not necessary to have a whole Value here.
    // A prototype can only be set to be null or an object, meaning that most of the
    // possible Value options are impossible.
    // We could possibly do with just a `Option<ObjectIndex>` but it would cause issues
    // with functions and possible other special object cases we want to track with partially
    // separate heap fields later down the line.
    pub prototype: Value,
    pub keys: ElementsVector,
    pub values: ElementsVector,
}

impl ObjectHeapData {
    pub fn new(
        extensible: bool,
        prototype: Value,
        keys: ElementsVector,
        values: ElementsVector,
    ) -> Self {
        Self {
            extensible,
            // TODO: Number, Boolean, etc. objects exist. These can all be
            // modeled with their own heap vector or alternatively by adding
            // a [[PrimitiveValue]] field to objects: Normally this field is None
            // to signal that the object is its own primitive value. For
            // Number objects etc the field is Some(Value).
            // TODO: Move prototype and key vector into shapes
            prototype,
            // TODO: Consider using SmallVec<[Option<Value>; 3]> or such?
            keys,
            values,
        }
    }

    pub fn has(&self, heap: &Heap, key: Value) -> bool {
        debug_assert!(key.is_string() || key.is_number() || key.is_symbol());
        heap.elements.has(self.keys, key)
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
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "seal", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "setPrototypeOf", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "values", 1, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ObjectConstructorIndex,
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex.into()),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::ObjectConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::ObjectConstructorIndex.into()),
            length: 1,
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            initial_name: Value::Null,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::ObjectConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "hasOwnProperty", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isPrototypeOf", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "propertyIsEnumerable", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ObjectConstructorIndex,
        true,
        Value::Null,
        entries,
    );
}

fn object_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Object(ObjectIndex::from_index(0)))
}

fn object_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!()
}
