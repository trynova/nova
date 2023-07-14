use crate::{
    bigint::create_bigint_prototype,
    function::create_function_prototype,
    heap_trace::HeapTrace,
    number::create_number_prototype,
    object::create_object_prototype,
    string::create_string_prototype,
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
        let object_prototype = create_object_prototype(&mut heap);
        heap.objects.push(Some(object_prototype));
        heap.globals.push(Value::Object(0));
        let function_prototype = create_function_prototype(&mut heap);
        heap.objects.push(Some(function_prototype));
        heap.globals.push(Value::Object(1));
        heap.objects.push(Some(create_string_prototype()));
        heap.globals.push(Value::Object(2));
        heap.objects.push(Some(create_number_prototype()));
        heap.globals.push(Value::Object(3));
        heap.objects.push(Some(create_bigint_prototype()));
        heap.globals.push(Value::Object(4));
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
            &Value::BigInt(idx) => heap.bigints[idx as usize].trace(heap),
            &Value::Function(idx) => heap.functions[idx as usize].trace(heap),
            &Value::Number(idx) => heap.numbers[idx as usize].trace(heap),
            &Value::Object(idx) => heap.objects[idx as usize].trace(heap),
            &Value::String(idx) => heap.strings[idx as usize].trace(heap),
            &Value::Symbol(idx) => heap.symbols[idx as usize].trace(heap),
            _ => {}
        }
    }

    fn root(&self, heap: &Heap) {
        match self {
            &Value::BigInt(idx) => heap.bigints[idx as usize].root(heap),
            &Value::Function(idx) => heap.functions[idx as usize].root(heap),
            &Value::Number(idx) => heap.numbers[idx as usize].root(heap),
            &Value::Object(idx) => heap.objects[idx as usize].root(heap),
            &Value::String(idx) => heap.strings[idx as usize].root(heap),
            &Value::Symbol(idx) => heap.symbols[idx as usize].root(heap),
            _ => {}
        }
    }

    fn unroot(&self, heap: &Heap) {
        match self {
            &Value::BigInt(idx) => heap.bigints[idx as usize].unroot(heap),
            &Value::Function(idx) => heap.functions[idx as usize].unroot(heap),
            &Value::Number(idx) => heap.numbers[idx as usize].unroot(heap),
            &Value::Object(idx) => heap.objects[idx as usize].unroot(heap),
            &Value::String(idx) => heap.strings[idx as usize].unroot(heap),
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
}

pub struct ObjectHeapData {
    bits: HeapBits,
    _extensible: bool,
    prototype: PropertyDescriptor,
    entries: Vec<ObjectEntry>,
}

impl ObjectHeapData {
    pub fn new(
        bits: HeapBits,
        extensible: bool,
        prototype: PropertyDescriptor,
        entries: Vec<ObjectEntry>,
    ) -> Self {
        Self {
            bits,
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
                PropertyKey::String(idx) => heap.strings[idx as usize].trace(heap),
                PropertyKey::Symbol(idx) => heap.symbols[idx as usize].trace(heap),
                PropertyKey::Smi(_) => {}
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
    String(StringIndex),
    Symbol(SymbolIndex),
    Smi(i32),
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

pub struct FunctionHeapData {
    bits: HeapBits,
    object_index: u32,
    length: u8,
    uses_arguments: bool,
    bound: Vec<Value>,
    visible: Vec<Value>,
    binding: fn(this: Value, args: Vec<Value>) -> Value,
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
