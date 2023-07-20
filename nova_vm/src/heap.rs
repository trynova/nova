mod bigint;
mod boolean;
mod function;
mod heap_constants;
mod heap_trace;
mod number;
mod object;
mod string;
mod symbol;

use self::{
    bigint::initialize_bigint_heap,
    boolean::initialize_boolean_heap,
    function::initialize_function_heap,
    heap_constants::{BuiltinObjectIndexes, FIRST_CONSTRUCTOR_INDEX, LAST_BUILTIN_OBJECT_INDEX},
    heap_trace::HeapTrace,
    number::initialize_number_heap,
    object::{
        initialize_object_heap, ObjectEntry, ObjectHeapData, PropertyDescriptor, PropertyKey,
    },
    string::{initialize_string_heap, StringHeapData},
    symbol::initialize_symbol_heap,
};
use crate::value::{StringIndex, Value};
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
        for i in 0..LAST_BUILTIN_OBJECT_INDEX {
            // Initialize all static slots in heap objects.
            heap.objects.push(None);
            if i >= FIRST_CONSTRUCTOR_INDEX {
                heap.functions.push(None);
            }
        }
        initialize_object_heap(&mut heap);
        initialize_function_heap(&mut heap);
        initialize_boolean_heap(&mut heap);
        initialize_symbol_heap(&mut heap);
        // initialize_error_heap(&mut heap);
        initialize_number_heap(&mut heap);
        initialize_bigint_heap(&mut heap);
        // initialize_math_object(&mut heap);
        // initialize_date_object(&mut heap);
        initialize_string_heap(&mut heap);
        // initialize_regexp_object(&mut heap);
        // initialize_typedarray_object(&mut heap);
        // initialize_map_object(&mut heap);
        // initialize_set_object(&mut heap);
        // initialize_weak_map_object(&mut heap);
        // initialize_weak_set_object(&mut heap);
        // initialize_array_buffer_object(&mut heap);
        // initialize_shared_array_buffer_object(&mut heap);
        // initialize_data_view_object(&mut heap);
        // initialize_json_object(&mut heap);
        // initialize_atomics_object(&mut heap);
        // initialize_weak_ref_object(&mut heap);
        // initialize_finalization_registry_object(&mut heap);
        // initialize_iterator_object(&mut heap);
        // initialize_async_iterator_object(&mut heap);
        // initialize_promise_object(&mut heap);
        // initialize_generator_function_object(&mut heap);
        // initialize_async_generator_function_object(&mut heap);
        // initialize_generator_object(&mut heap);
        // initialize_async_generator_object(&mut heap);
        // initialize_async_function_object(&mut heap);
        // initialize_reflect_object(&mut heap);
        // initialize_proxy_object(&mut heap);
        // initialize_module_object(&mut heap);

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
                    PropertyDescriptor::roxh(Value::SmiU(length as u32)),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(self, "name"),
                    PropertyDescriptor::roxh(name),
                ),
            ],
            prototype: PropertyDescriptor::roh(Value::Object(
                BuiltinObjectIndexes::FunctionPrototypeIndex as u32,
            )),
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
