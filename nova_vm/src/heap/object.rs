use crate::{
    heap::{
        function::JsBindingFunction,
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        heap_trace::HeapTrace,
        FunctionHeapData, Heap, HeapBits,
    },
    value::{FunctionIndex, StringIndex, SymbolIndex, Value},
};

pub struct ObjectEntry {
    key: PropertyKey,
    value: PropertyDescriptor,
}

impl ObjectEntry {
    pub(crate) fn new(key: PropertyKey, value: PropertyDescriptor) -> Self {
        ObjectEntry { key, value }
    }

    pub(crate) fn new_prototype_function(
        heap: &mut Heap,
        name: &str,
        length: u8,
        uses_arguments: bool,
        binding: JsBindingFunction,
    ) -> Self {
        let key = PropertyKey::from_str(heap, name);
        let name = match key {
            PropertyKey::SmallAsciiString(data) => Value::SmallAsciiString(data.clone()),
            PropertyKey::Smi(_) => unreachable!("No prototype functions should have SMI names"),
            PropertyKey::String(idx) => Value::HeapString(idx),
            PropertyKey::Symbol(idx) => Value::Symbol(idx),
        };
        let func_index = heap.create_function(name, length, uses_arguments, binding);
        let value = PropertyDescriptor::rwxh(Value::Function(func_index));
        ObjectEntry { key, value }
    }

    pub(crate) fn new_prototype_symbol_function(
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

    pub(crate) fn new_prototype(heap: &mut Heap, idx: u32) -> Self {
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
}

pub enum PropertyKey {
    SmallAsciiString([i8; 7]),
    Smi(i32),
    String(StringIndex),
    Symbol(SymbolIndex),
}

impl PropertyKey {
    pub fn from_str(heap: &mut Heap, str: &str) -> Self {
        if str.len() <= 7 && str.is_ascii() {
            let mut bytes: [i8; 7] = [0, 0, 0, 0, 0, 0, 0];
            let str_ascii_bytes = str.as_bytes();
            for (idx, byte) in str_ascii_bytes.iter().enumerate() {
                bytes[idx] = *byte as i8;
            }
            PropertyKey::SmallAsciiString(bytes)
        } else {
            PropertyKey::String(heap.alloc_string(str))
        }
    }
}

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

pub(crate) struct ObjectHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) _extensible: bool,
    // TODO: It's probably not necessary to have a whole data descriptor here.
    // A prototype can only be set to be null or an object, meaning that most of the
    // possible Value options are impossible.
    // We could possibly do with just a `Option<ObjectIndex>` but it would cause issues
    // with functions and possible other special object cases we want to track with partially
    // separate heap fields later down the line.
    pub(crate) prototype: PropertyDescriptor,
    pub(crate) entries: Vec<ObjectEntry>,
}

impl ObjectHeapData {
    pub fn new(extensible: bool, prototype: PropertyDescriptor, entries: Vec<ObjectEntry>) -> Self {
        Self {
            bits: HeapBits::new(),
            _extensible: extensible,
            // TODO: Number, Boolean, etc. objects exist. These can all be
            // modeled with their own heap vector or alternatively by adding
            // a [[PrimitiveValue]] field to objects: Normally this field is None
            // to signal that the object is its own primitive value. For
            // Number objects etc the field is Some(Value).
            // TODO: Move prototype and key vector into shapes
            prototype,
            // TODO: Separate entries into key and value vectors
            // TODO: Use SmallVec<[T; 4]>
            entries,
        }
    }
}

impl HeapTrace for Option<ObjectHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        let data = self.as_ref().unwrap();
        let dirty = data.bits.dirty.replace(false);
        let marked = data.bits.marked.replace(true);
        if marked && !dirty {
            // Do not keep recursing into already-marked heap values.
            return;
        }
        match &data.prototype {
            PropertyDescriptor::Data { value, .. } => value.trace(heap),
            PropertyDescriptor::Blocked { .. } => {}
            PropertyDescriptor::ReadOnly { get, .. } => {
                heap.objects[*get as usize].trace(heap);
            }
            PropertyDescriptor::WriteOnly { set, .. } => {
                heap.objects[*set as usize].trace(heap);
            }
            PropertyDescriptor::ReadWrite { get, set, .. } => {
                heap.objects[*get as usize].trace(heap);
                heap.objects[*set as usize].trace(heap);
            }
        }
        for reference in data.entries.iter() {
            match reference.key {
                PropertyKey::SmallAsciiString(_) | PropertyKey::Smi(_) => {}
                PropertyKey::String(idx) => heap.strings[idx as usize].trace(heap),
                PropertyKey::Symbol(idx) => heap.symbols[idx as usize].trace(heap),
            }
            match &reference.value {
                PropertyDescriptor::Data { value, .. } => value.trace(heap),
                PropertyDescriptor::Blocked { .. } => {}
                PropertyDescriptor::ReadOnly { get, .. } => {
                    heap.objects[*get as usize].trace(heap);
                }
                PropertyDescriptor::WriteOnly { set, .. } => {
                    heap.objects[*set as usize].trace(heap);
                }
                PropertyDescriptor::ReadWrite { get, set, .. } => {
                    heap.objects[*get as usize].trace(heap);
                    heap.objects[*set as usize].trace(heap);
                }
            }
        }
    }

    fn root(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.root();
    }

    fn unroot(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.unroot();
    }

    fn finalize(&mut self, _heap: &Heap) {
        self.take();
    }
}

pub fn initialize_object_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::ObjectConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            vec![
                ObjectEntry::new_prototype_function(heap, "assign", 1, true, object_todo),
                ObjectEntry::new_prototype_function(heap, "create", 2, false, object_todo),
                ObjectEntry::new_prototype_function(
                    heap,
                    "defineProperties",
                    2,
                    false,
                    object_todo,
                ),
                ObjectEntry::new_prototype_function(heap, "defineProperty", 3, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "entries", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "freeze", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "fromEntries", 1, false, object_todo),
                ObjectEntry::new_prototype_function(
                    heap,
                    "getOwnPropertyDescriptor",
                    2,
                    false,
                    object_todo,
                ),
                ObjectEntry::new_prototype_function(
                    heap,
                    "getOwnPropertyDescriptors",
                    1,
                    false,
                    object_todo,
                ),
                ObjectEntry::new_prototype_function(
                    heap,
                    "getOwnPropertyNames",
                    1,
                    false,
                    object_todo,
                ),
                ObjectEntry::new_prototype_function(
                    heap,
                    "getOwnPropertySymbols",
                    1,
                    false,
                    object_todo,
                ),
                ObjectEntry::new_prototype_function(heap, "getPrototypeOf", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "hasOwn", 2, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "is", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "isExtensible", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "isFrozen", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "isSealed", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "keys", 1, false, object_todo),
                ObjectEntry::new_prototype_function(
                    heap,
                    "preventExtensions",
                    1,
                    false,
                    object_todo,
                ),
                ObjectEntry::new_prototype(heap, BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
                ObjectEntry::new_prototype_function(heap, "seal", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "setPrototypeOf", 2, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "values", 1, false, object_todo),
            ],
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::ObjectConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::ObjectConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: object_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::ObjectConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::roh(Value::Null),
            vec![
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "constructor"),
                    PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                        BuiltinObjectIndexes::ObjectConstructorIndex,
                    ))),
                ),
                ObjectEntry::new_prototype_function(heap, "hasOwnProperty", 1, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "isPrototypeOf", 1, false, object_todo),
                ObjectEntry::new_prototype_function(
                    heap,
                    "propertyIsEnumerable",
                    1,
                    false,
                    object_todo,
                ),
                ObjectEntry::new_prototype_function(heap, "toLocaleString", 0, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "toString", 0, false, object_todo),
                ObjectEntry::new_prototype_function(heap, "valueOf", 0, false, object_todo),
            ],
        ));
}

fn object_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Object(0)
}

fn object_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    todo!()
}
