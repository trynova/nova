use crate::{
    heap::{
        function::JsBindingFunction,
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap,
    },
    stack_string::StackString,
    value::{FunctionIndex, JsResult, StringIndex, SymbolIndex, Value},
};
use std::{fmt::Debug, vec};

use super::{ElementArrayKey, ElementsVector, EntriesVector};

#[derive(Debug)]
pub struct ObjectEntry {
    key: PropertyKey,
    value: PropertyDescriptor,
}

impl ObjectEntry {
    pub(crate) fn new(key: PropertyKey, value: PropertyDescriptor) -> Self {
        ObjectEntry { key, value }
    }

    pub(crate) fn new_prototype_function_entry(
        heap: &mut Heap,
        name: &str,
        length: u8,
        uses_arguments: bool,
        binding: JsBindingFunction,
    ) -> Self {
        let key = PropertyKey::from_str(heap, name);
        let name = match key {
            PropertyKey::SmallAsciiString(data) => Value::StackString(data.clone()),
            PropertyKey::Smi(_) => unreachable!("No prototype functions should have SMI names"),
            PropertyKey::String(idx) => Value::HeapString(idx),
            PropertyKey::Symbol(idx) => Value::Symbol(idx),
        };
        let func_index = heap.create_function(name, length, uses_arguments, binding);
        let value = PropertyDescriptor::rwxh(Value::Function(func_index));
        ObjectEntry { key, value }
    }

    pub(crate) fn new_prototype_symbol_function_entry(
        heap: &mut Heap,
        name: &str,
        symbol_index: u32,
        length: u8,
        uses_arguments: bool,
        binding: JsBindingFunction,
    ) -> Self {
        let name = Value::new_string(heap, name);
        let key = PropertyKey::Symbol(symbol_index);
        let func_index = heap.create_function(name, length, uses_arguments, binding);
        let value = PropertyDescriptor::roxh(Value::Function(func_index));
        ObjectEntry { key, value }
    }

    pub(crate) fn new_constructor_prototype_entry(heap: &mut Heap, idx: u32) -> Self {
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

    pub(crate) fn new_frozen_entry(heap: &mut Heap, key: &str, value: Value) -> Self {
        ObjectEntry {
            key: PropertyKey::from_str(heap, key),
            value: PropertyDescriptor::roh(value),
        }
    }
}

#[derive(Debug)]
pub enum PropertyKey {
    SmallAsciiString(StackString),
    Smi(i32),
    String(StringIndex),
    Symbol(SymbolIndex),
}

impl PropertyKey {
    pub fn from_str(heap: &mut Heap, str: &str) -> Self {
        if let Some(ascii_string) = StackString::try_from_str(str) {
            PropertyKey::SmallAsciiString(ascii_string)
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
    pub const fn prototype_slot(idx: u32) -> Self {
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

#[derive(Debug)]
pub(crate) struct ObjectHeapData {
    pub(crate) _extensible: bool,
    // TODO: It's probably not necessary to have a whole data descriptor here.
    // A prototype can only be set to be null or an object, meaning that most of the
    // possible Value options are impossible.
    // We could possibly do with just a `Option<ObjectIndex>` but it would cause issues
    // with functions and possible other special object cases we want to track with partially
    // separate heap fields later down the line.
    pub(crate) prototype: Value,
    pub(crate) keys: ElementsVector,
    pub(crate) values: ElementsVector,
}

impl ObjectHeapData {
    pub fn new(
        extensible: bool,
        prototype: Value,
        keys: ElementsVector,
        values: ElementsVector,
    ) -> Self {
        Self {
            _extensible: extensible,
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
}

pub fn initialize_object_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "assign", 1, true, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "create", 2, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "defineProperties", 2, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "defineProperty", 3, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "entries", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "freeze", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "fromEntries", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "getOwnPropertyDescriptor",
            2,
            false,
            object_todo,
        ),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "getOwnPropertyDescriptors",
            1,
            false,
            object_todo,
        ),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "getOwnPropertyNames",
            1,
            false,
            object_todo,
        ),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "getOwnPropertySymbols",
            1,
            false,
            object_todo,
        ),
        ObjectEntry::new_prototype_function_entry(heap, "getPrototypeOf", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "hasOwn", 2, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "is", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "isExtensible", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "isFrozen", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "isSealed", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "keys", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "preventExtensions", 1, false, object_todo),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::ObjectPrototypeIndex as u32,
        ),
        ObjectEntry::new_prototype_function_entry(heap, "seal", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "setPrototypeOf", 2, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "values", 1, false, object_todo),
    ];
    heap.objects[BuiltinObjectIndexes::ObjectConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::ObjectConstructorIndex) as usize] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::ObjectConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: object_constructor_binding,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::ObjectConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "hasOwnProperty", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "isPrototypeOf", 1, false, object_todo),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "propertyIsEnumerable",
            1,
            false,
            object_todo,
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, object_todo),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false, object_todo),
    ];
    heap.objects[BuiltinObjectIndexes::ObjectConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            Value::Null,
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ));
}

fn object_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Object(0))
}

fn object_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!()
}
