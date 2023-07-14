use crate::{
    bigint::initialize_bigint_heap,
    function::create_function_prototype,
    heap_trace::HeapTrace,
    number::create_number_prototype,
    object::initialize_object_heap,
    string::initiate_string_heap,
    value::{FunctionIndex, StringIndex, SymbolIndex, Value},
};
use std::cell::Cell;
use wtf8::{Wtf8, Wtf8Buf};

pub struct Heap {
    pub bigints: Vec<Option<BigIntHeapData>>,
    pub functions: Vec<Option<FunctionHeapData>>,
    pub globals: Vec<Value>,
    pub numbers: Vec<Option<NumberHeapData>>,
    pub objects: Vec<Option<ObjectHeapData>>,
    pub strings: Vec<Option<StringHeapData>>,
    pub symbols: Vec<Option<SymbolHeapData>>,
}

fn stop_the_world() {}
fn start_the_world() {}

impl Heap {
    pub fn new() -> Heap {
        let mut heap = Heap {
            bigints: Vec::with_capacity(1024),
            functions: Vec::with_capacity(1024),
            globals: Vec::with_capacity(1024),
            numbers: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            strings: Vec::with_capacity(1024),
            symbols: Vec::with_capacity(1024),
        };
        let object_prototype = initialize_object_heap(&mut heap);
        heap.objects.push(Some(object_prototype));
        heap.globals.push(Value::Object(0));
        let function_prototype = create_function_prototype(&mut heap);
        heap.objects.push(Some(function_prototype));
        heap.globals.push(Value::Object(1));
        heap.objects.push(Some(initiate_string_heap()));
        heap.globals.push(Value::Object(2));
        heap.objects.push(Some(create_number_prototype()));
        heap.globals.push(Value::Object(3));
        initialize_bigint_heap(&mut heap);
        heap
    }

    pub(crate) fn alloc_string(&mut self, message: &str) -> u32 {
        let found = self.strings.iter().position(|opt| {
            opt.as_ref()
                .map_or(false, |data| data.data == Wtf8::from_str(message))
        });
        if let Some(idx) = found {
            return idx as u32;
        }
        let data = StringHeapData::from_str(message);
        let found = self.strings.iter().position(|opt| opt.is_none());
        if let Some(idx) = found {
            self.strings[idx].replace(data);
            idx as u32
        } else {
            self.strings.push(Some(data));
            self.strings.len() as u32
        }
    }

    pub(crate) fn create_function(
        &mut self,
        name: Value,
        length: u8,
        binding: JsBindingFunction,
    ) -> u32 {
        let func_object_data = ObjectHeapData {
            _extensible: true,
            bits: HeapBits::new(),
            entries: vec![
                ObjectEntry::new(
                    PropertyKey::from_str(self, "length"),
                    PropertyDescriptor::Data {
                        value: Value::SmiU(length as u32),
                        writable: false,
                        enumerable: false,
                        configurable: true,
                    },
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(self, "name"),
                    PropertyDescriptor::Data {
                        value: name,
                        writable: false,
                        enumerable: false,
                        configurable: true,
                    },
                ),
            ],
            prototype: PropertyDescriptor::Data {
                // TODO: Get %Function.prototype%
                value: Value::Object(1),
                writable: false,
                enumerable: false,
                configurable: false,
            },
        };
        self.objects.push(Some(func_object_data));
        let func_data = FunctionHeapData {
            binding,
            bits: HeapBits::new(),
            bound: None,
            length,
            object_index: self.objects.len() as u32,
            uses_arguments: false,
            visible: None,
        };
        self.functions.push(Some(func_data));
        self.functions.len() as u32
    }

    fn partial_trace(&mut self) -> () {
        // TODO: Consider roots count
        for global in self.globals.iter() {
            global.trace(self);
        }
        for object in self.objects.iter() {
            let Some(data) = object else {
                continue;
            };
            let marked = data.bits.marked.take();
            data.bits.marked.set(marked);
            if !marked {
                continue;
            }
            let dirty = data.bits.dirty.take();
            data.bits.dirty.set(dirty);
            if dirty {
                object.trace(self);
            }
        }
        for symbol in self.symbols.iter() {
            let Some(data) = symbol else {
                continue;
            };
            let marked = data.bits.marked.take();
            data.bits.marked.set(marked);
            if !marked {
                continue;
            }
            let dirty = data.bits.dirty.take();
            data.bits.dirty.set(dirty);
            if dirty {
                symbol.trace(self);
            }
        }
        stop_the_world();
        // Repeat above tracing to check for mutations that happened while we were tracing.
        for object in self.objects.iter_mut() {
            let Some(data) = object else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = object.take();
            }
        }
        for string in self.strings.iter_mut() {
            let Some(data) = string else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = string.take();
            }
        }
        for symbol in self.symbols.iter_mut() {
            let Some(data) = symbol else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = symbol.take();
            }
        }
        for number in self.numbers.iter_mut() {
            let Some(data) = number else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = number.take();
            }
        }
        for bigint in self.bigints.iter_mut() {
            let Some(data) = bigint else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = bigint.take();
            }
        }
        while self.objects.last().is_none() {
            self.objects.pop();
        }
        while self.strings.last().is_none() {
            self.strings.pop();
        }
        while self.symbols.last().is_none() {
            self.symbols.pop();
        }
        while self.numbers.last().is_none() {
            self.numbers.pop();
        }
        while self.bigints.last().is_none() {
            self.bigints.pop();
        }
        start_the_world();
    }

    fn complete_trace(&mut self) -> () {
        // TODO: Consider roots count
        for object in self.objects.iter() {
            let Some(data) = object else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for string in self.strings.iter() {
            let Some(data) = string else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for symbol in self.symbols.iter() {
            let Some(data) = symbol else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for number in self.numbers.iter() {
            let Some(data) = number else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for bigint in self.bigints.iter() {
            let Some(data) = bigint else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for global in self.globals.iter() {
            global.trace(self);
        }
        stop_the_world();
        // Trace from dirty objects and symbols.
        for object in self.objects.iter_mut() {
            let Some(data) = object else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = object.take();
            }
        }
        for string in self.strings.iter_mut() {
            let Some(data) = string else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = string.take();
            }
        }
        for symbol in self.symbols.iter_mut() {
            let Some(data) = symbol else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = symbol.take();
            }
        }
        for number in self.numbers.iter_mut() {
            let Some(data) = number else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = number.take();
            }
        }
        for bigint in self.bigints.iter_mut() {
            let Some(data) = bigint else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = bigint.take();
            }
        }
        while self.objects.last().is_none() {
            self.objects.pop();
        }
        while self.strings.last().is_none() {
            self.strings.pop();
        }
        while self.symbols.last().is_none() {
            self.symbols.pop();
        }
        while self.numbers.last().is_none() {
            self.numbers.pop();
        }
        while self.bigints.last().is_none() {
            self.bigints.pop();
        }
        start_the_world();
    }
}

impl HeapTrace for Value {
    fn trace(&self, heap: &Heap) {
        match self {
            &Value::HeapBigInt(idx) => heap.bigints[idx as usize].trace(heap),
            &Value::Function(idx) => heap.functions[idx as usize].trace(heap),
            &Value::HeapNumber(idx) => heap.numbers[idx as usize].trace(heap),
            &Value::Object(idx) => heap.objects[idx as usize].trace(heap),
            &Value::HeapString(idx) => heap.strings[idx as usize].trace(heap),
            &Value::Symbol(idx) => heap.symbols[idx as usize].trace(heap),
            _ => {}
        }
    }

    fn root(&self, heap: &Heap) {
        match self {
            &Value::HeapBigInt(idx) => heap.bigints[idx as usize].root(heap),
            &Value::Function(idx) => heap.functions[idx as usize].root(heap),
            &Value::HeapNumber(idx) => heap.numbers[idx as usize].root(heap),
            &Value::Object(idx) => heap.objects[idx as usize].root(heap),
            &Value::HeapString(idx) => heap.strings[idx as usize].root(heap),
            &Value::Symbol(idx) => heap.symbols[idx as usize].root(heap),
            _ => {}
        }
    }

    fn unroot(&self, heap: &Heap) {
        match self {
            &Value::HeapBigInt(idx) => heap.bigints[idx as usize].unroot(heap),
            &Value::Function(idx) => heap.functions[idx as usize].unroot(heap),
            &Value::HeapNumber(idx) => heap.numbers[idx as usize].unroot(heap),
            &Value::Object(idx) => heap.objects[idx as usize].unroot(heap),
            &Value::HeapString(idx) => heap.strings[idx as usize].unroot(heap),
            &Value::Symbol(idx) => heap.symbols[idx as usize].unroot(heap),
            _ => {}
        }
    }

    fn finalize(&mut self, _heap: &Heap) {
        unreachable!("Finalize should never be called on a Value in stack");
    }
}

pub struct HeapBits {
    marked: Cell<bool>,
    _weak_marked: Cell<bool>,
    dirty: Cell<bool>,
    roots: Cell<u8>,
}

impl HeapBits {
    pub fn new() -> Self {
        HeapBits {
            marked: Cell::new(false),
            _weak_marked: Cell::new(false),
            dirty: Cell::new(false),
            roots: Cell::new(0),
        }
    }

    fn root(&self) {
        let roots = self.roots.replace(1);
        assert!(roots != u8::MAX);
        self.roots.replace(roots + 1);
    }

    fn unroot(&self) {
        let roots = self.roots.replace(1);
        assert!(roots != 0);
        self.roots.replace(roots - 1);
    }
}

unsafe impl Sync for HeapBits {}

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
        binding: JsBindingFunction,
    ) -> Self {
        let key = PropertyKey::from_str(heap, name);
        let name = match key {
            PropertyKey::SmallAsciiString(data) => Value::SmallAsciiString(data.clone()),
            PropertyKey::Smi(_) => unreachable!("No prototype functions should have SMI names"),
            PropertyKey::String(idx) => Value::HeapString(idx),
            PropertyKey::Symbol(idx) => Value::Symbol(idx),
        };
        let func_index = heap.create_function(name, length, binding);
        let value = PropertyDescriptor::Data {
            value: Value::Function(func_index),
            writable: true,
            enumerable: false,
            configurable: true,
        };
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

pub struct ObjectHeapData {
    bits: HeapBits,
    _extensible: bool,
    prototype: PropertyDescriptor,
    entries: Vec<ObjectEntry>,
}

impl ObjectHeapData {
    pub fn new(extensible: bool, prototype: PropertyDescriptor, entries: Vec<ObjectEntry>) -> Self {
        Self {
            bits: HeapBits::new(),
            _extensible: extensible,
            prototype,
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
    pub fn prototype_slot(idx: u32) -> Self {
        Self::Data {
            value: Value::Object(idx),
            writable: false,
            enumerable: false,
            configurable: false,
        }
    }
}

pub struct StringHeapData {
    bits: HeapBits,
    pub data: Wtf8Buf,
}

impl StringHeapData {
    pub fn from_str(str: &str) -> Self {
        StringHeapData {
            bits: HeapBits::new(),
            data: Wtf8Buf::from_str(str),
        }
    }

    pub fn len(&self) -> usize {
        // TODO: We should return the UTF-16 length.
        self.data.len()
    }
}

impl HeapTrace for Option<StringHeapData> {
    fn trace(&self, _heap: &Heap) {}

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

pub struct SymbolHeapData {
    bits: HeapBits,
    descriptor: Option<StringIndex>,
}

impl HeapTrace for Option<SymbolHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        if let Some(idx) = self.as_ref().unwrap().descriptor {
            heap.strings[idx as usize].trace(heap);
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

pub struct NumberHeapData {
    bits: HeapBits,
    pub data: f64,
}

impl NumberHeapData {
    pub fn new(data: f64) -> NumberHeapData {
        NumberHeapData {
            bits: HeapBits::new(),
            data,
        }
    }
}
impl HeapTrace for Option<NumberHeapData> {
    fn trace(&self, _heap: &Heap) {}

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

pub struct BigIntHeapData {
    bits: HeapBits,
    pub sign: bool,
    pub len: u32,
    pub parts: Box<[u64]>,
}
impl HeapTrace for Option<BigIntHeapData> {
    fn trace(&self, _heap: &Heap) {}

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

pub type JsBindingFunction = fn(heap: &mut Heap, this: Value, args: &[Value]) -> Value;

pub struct FunctionHeapData {
    bits: HeapBits,
    object_index: u32,
    length: u8,
    uses_arguments: bool,
    bound: Option<Box<[Value]>>,
    visible: Option<Vec<Value>>,
    binding: JsBindingFunction,
}
impl HeapTrace for Option<FunctionHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        heap.objects[self.as_ref().unwrap().object_index as usize].trace(heap);
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
