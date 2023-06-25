use crate::value::{self, JsResult, ObjectIndex, StringIndex, SymbolIndex, Value};
use gc::{unsafe_empty_trace, Finalize, Gc, Trace};
use std::fmt::Debug;
use wtf8::Wtf8Buf;

/// The Trace trait, which needs to be implemented on garbage-collected objects.
pub unsafe trait HeapTrace: Finalize + Trace {
    /// Marks all contained `Gc`s.
    unsafe fn heap_trace(&self, _heap: &Heap) {
        self.trace();
    }

    /// Increments the root-count of all contained `Gc`s.
    unsafe fn heap_root(&self, _heap: &Heap) {
        self.root();
    }

    /// Decrements the root-count of all contained `Gc`s.
    unsafe fn heap_unroot(&self, _heap: &Heap) {
        self.unroot();
    }

    /// Runs Finalize::finalize() on this object and all
    /// contained subobjects
    fn heap_finalize_glue(&self, _heap: &Heap) {
        self.finalize_glue();
    }
}

pub struct Heap {
    pub bigints: Vec<Gc<BigIntHeapData>>,
    pub globals: Vec<Value>,
    pub numbers: Vec<Gc<NumberHeapData>>,
    pub objects: Vec<Gc<dyn ObjectHeapData>>,
    pub strings: Vec<Gc<StringHeapData>>,
    pub symbols: Vec<Gc<SymbolHeapData>>,
}

impl Heap {
    pub fn new() -> Heap {
        Heap {
            bigints: Vec::with_capacity(1024),
            globals: Vec::with_capacity(1024),
            numbers: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            strings: Vec::with_capacity(1024),
            symbols: Vec::with_capacity(1024),
        }
    }

    fn trace_value(&self, value: Value) {
        match value {
            Value::String(idx) => unsafe { self.strings[idx as usize].heap_trace(self) },
            Value::Symbol(idx) => unsafe { self.symbols[idx as usize].heap_trace(self) },
            Value::Number(idx) => unsafe { self.numbers[idx as usize].heap_trace(self) },
            Value::BigInt(idx) => unsafe { self.bigints[idx as usize].heap_trace(self) },
            Value::Object(idx) => unsafe { self.objects[idx as usize].heap_trace(self) },
            _ => {}
        }
    }
}

pub trait ObjectHeapData: Trace + Debug {
    fn get_prototype_of(&self, heap: &mut Heap) -> JsResult<Option<ObjectIndex>>;
    fn set_prototype_of(&self, heap: &mut Heap, prototype: Option<ObjectIndex>) -> JsResult<bool>;
    fn is_extensible(&self, heap: &mut Heap) -> JsResult<bool>;
    fn prevent_extensions(&self, heap: &mut Heap) -> JsResult<bool>;
    fn get_own_property(
        &self,
        heap: &mut Heap,
        key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>>;
    fn define_own_property(
        &self,
        heap: &mut Heap,
        key: PropertyKey,
        descriptor: PropertyDescriptor,
    ) -> JsResult<bool>;
    fn has_property(&self, heap: &mut Heap, key: PropertyKey) -> JsResult<bool>;
    fn get(&self, heap: &mut Heap, key: PropertyKey, receiver: &Value) -> JsResult<Value>;
    fn set(
        &self,
        heap: &mut Heap,
        key: PropertyKey,
        value: Value,
        receiver: &Value,
    ) -> JsResult<bool>;
    fn delete(&self, heap: &mut Heap, key: PropertyKey) -> JsResult<bool>;
    fn own_property_keys(&self, heap: &mut Heap) -> JsResult<Vec<PropertyKey>>;

    // Tracing helpers
    fn get_strong_references(&self) -> Vec<Value>;
    fn get_weak_references(&self) -> Vec<Value>;
}
unsafe impl HeapTrace for Gc<dyn ObjectHeapData> {
    unsafe fn heap_trace(&self, heap: &Heap) {
        for reference in self.get_strong_references().iter() {
            heap.trace_value(reference.clone());
        }
    }

    unsafe fn heap_root(&self, _heap: &Heap) {
        self.root();
    }

    unsafe fn heap_unroot(&self, _heap: &Heap) {
        self.unroot();
    }

    fn heap_finalize_glue(&self, _heap: &Heap) {
        self.finalize_glue();
    }
}

pub trait FunctionHeapData: ObjectHeapData {
    fn call(&self, this: &Value, args: &[Value]) -> JsResult<Value>;
}

pub trait ConstructorHeapData: FunctionHeapData {
    fn construct(&self, args: &[Value], target: ObjectIndex) -> JsResult<Value>;
}

pub enum PropertyKey {
    String(StringIndex),
    Symbol(SymbolIndex),
}

// TODO(andreubotella): This name isn't great.
pub enum PropertyDescriptor {
    Data {
        value: Value,
        writable: bool,
        enumerable: bool,
        configurable: bool,
    },
    Readable {
        get: ObjectIndex,
        enumerable: bool,
        configurable: bool,
    },
    Writable {
        set: ObjectIndex,
        enumerable: bool,
        configurable: bool,
    },
    ReadableWritable {
        get: ObjectIndex,
        set: ObjectIndex,
        enumerable: bool,
        configurable: bool,
    },
}

#[derive(Clone)]
pub struct StringHeapData {
    pub data: Wtf8Buf,
}

impl Finalize for StringHeapData {
    fn finalize(&self) {}
}
unsafe impl Trace for StringHeapData {
    unsafe_empty_trace!();
}
unsafe impl HeapTrace for Gc<StringHeapData> {}

#[derive(Trace, Finalize)]
pub struct SymbolHeapData {
    descriptor: Option<StringIndex>,
}

unsafe impl HeapTrace for Gc<SymbolHeapData> {
    unsafe fn heap_trace(&self, heap: &Heap) {
        if let Some(idx) = self.descriptor {
            unsafe {
                heap.strings[idx as usize].trace();
            }
        }
    }

    unsafe fn heap_root(&self, heap: &Heap) {
        if let Some(idx) = self.descriptor {
            unsafe {
                heap.strings[idx as usize].root();
            }
        }
    }

    unsafe fn heap_unroot(&self, heap: &Heap) {
        if let Some(idx) = self.descriptor {
            unsafe {
                heap.strings[idx as usize].unroot();
            }
        }
    }

    fn heap_finalize_glue(&self, heap: &Heap) {
        if let Some(idx) = self.descriptor {
            heap.strings[idx as usize].finalize_glue();
        }
    }
}

#[derive(Clone)]
pub struct NumberHeapData {
    pub data: f64,
}
impl Finalize for NumberHeapData {
    fn finalize(&self) {}
}
unsafe impl Trace for NumberHeapData {
    unsafe_empty_trace!();
}

impl NumberHeapData {
    pub fn new(data: f64) -> NumberHeapData {
        NumberHeapData { data }
    }
}
unsafe impl HeapTrace for Gc<NumberHeapData> {}

#[derive(Clone)]
pub struct BigIntHeapData {
    pub sign: bool,
    pub len: u32,
    pub parts: Box<[u64]>,
}
impl Finalize for BigIntHeapData {
    fn finalize(&self) {}
}
unsafe impl Trace for BigIntHeapData {
    unsafe_empty_trace!();
}
unsafe impl HeapTrace for Gc<BigIntHeapData> {}
