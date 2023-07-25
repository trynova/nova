mod array;
mod bigint;
mod function;
mod heap_constants;
mod heap_trace;
mod number;
mod object;
mod string;
mod symbol;

pub use array::ArrayHeapData;
pub use bigint::BigIntHeapData;
pub use function::FunctionHeapData;
pub use number::NumberHeapData;
pub use object::ObjectHeapData;
pub use string::StringHeapData;
pub use symbol::SymbolHeapData;

use self::heap_trace::HeapTrace;
use crate::types::{Function, Number, Object, String, Value};
use std::{cell::Cell, marker::PhantomData};
use wtf8::{Wtf8, Wtf8Buf};

/// A handle to GC-managed memory.
#[derive(Clone)]
pub struct Handle<T: 'static> {
    id: u32,
    _marker: &'static PhantomData<T>,
}

impl<T: 'static + Clone> Copy for Handle<T> {}

impl<T: 'static> Handle<T> {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            // SAFETY: We hopefully will make sure handles ar esafe.
            _marker: unsafe {
                std::mem::transmute::<&PhantomData<T>, &'static PhantomData<T>>(
                    &PhantomData::default(),
                )
            },
        }
    }
}

macro_rules! impl_handle_debug {
    ($name: ty) => {
        impl std::fmt::Debug for Handle<$name> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "$name(0x{:x})", self.id)
            }
        }
    };
}

impl_handle_debug!(StringHeapData);
impl_handle_debug!(SymbolHeapData);
impl_handle_debug!(NumberHeapData);
impl_handle_debug!(BigIntHeapData);
impl_handle_debug!(ObjectHeapData);
impl_handle_debug!(ArrayHeapData);
impl_handle_debug!(FunctionHeapData);

#[derive(Debug)]
pub struct Heap {
    pub(crate) strings: Vec<Option<StringHeapData>>,
    pub(crate) symbols: Vec<Option<SymbolHeapData>>,
    pub(crate) numbers: Vec<Option<NumberHeapData>>,
    pub(crate) bigints: Vec<Option<BigIntHeapData>>,
    pub(crate) objects: Vec<Option<ObjectHeapData>>,
    pub(crate) arrays: Vec<Option<ArrayHeapData>>,
    pub(crate) functions: Vec<Option<FunctionHeapData>>,
}

fn stop_the_world() {}
fn start_the_world() {}

pub trait CreateHeapData<T, F> {
    /// Creates a [`Value`] from the given data. Allocating the data is **not**
    /// guaranteed.
    fn create(&mut self, data: T) -> F;
}

pub trait GetHeapData<'a, T, F: 'a> {
    fn get(&'a self, handle: Handle<T>) -> F;
}

impl CreateHeapData<f64, Number> for Heap {
    fn create(&mut self, data: f64) -> Number {
        if let Ok(value) = Value::try_from(data) {
            Number::new(value)
        } else if data as f32 as f64 == data {
            Number::new(Value::FloatNumber(data as f32))
        } else {
            let id = self.alloc_number(data);
            Number::new(Value::Number(Handle::new(id)))
        }
    }
}

impl<'a> GetHeapData<'a, NumberHeapData, f64> for Heap {
    fn get(&'a self, handle: Handle<NumberHeapData>) -> f64 {
        self.numbers
            .get(handle.id as usize)
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .data
    }
}

impl CreateHeapData<&str, String> for Heap {
    fn create(&mut self, data: &str) -> String {
        if let Ok(value) = String::try_from(data) {
            value
        } else {
            let id = self.alloc_string(data);
            String::new(Value::String(Handle::new(id)))
        }
    }
}

impl<'a> GetHeapData<'a, StringHeapData, &'a Wtf8> for Heap {
    fn get(&'a self, handle: Handle<StringHeapData>) -> &'a Wtf8 {
        let data = self
            .strings
            .get(handle.id as usize)
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap();
        &data.data.slice(0, data.data.len())
    }
}

impl CreateHeapData<FunctionHeapData, Function> for Heap {
    fn create(&mut self, data: FunctionHeapData) -> Function {
        let id = self.functions.len();
        self.functions.push(Some(data));
        Function::new(Value::Function(Handle::new(id as u32)))
    }
}

impl<'a> GetHeapData<'a, FunctionHeapData, &'a FunctionHeapData> for Heap {
    fn get(&'a self, handle: Handle<FunctionHeapData>) -> &'a FunctionHeapData {
        self.functions
            .get(handle.id as usize)
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
    }
}

impl Heap {
    pub fn new() -> Heap {
        let mut heap = Heap {
            strings: Vec::with_capacity(1024),
            symbols: Vec::with_capacity(1024),
            numbers: Vec::with_capacity(1024),
            bigints: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            arrays: Vec::with_capacity(1024),
            functions: Vec::with_capacity(1024),
        };

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

    pub(crate) fn alloc_number(&mut self, number: f64) -> u32 {
        self.numbers.push(Some(NumberHeapData::new(number)));
        self.numbers.len() as u32
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
        for object in self.objects.iter_mut() {
            let Some(data) = object else {
				continue;
			};
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = object.take();
            }
        }
        for function in self.functions.iter_mut() {
            let Some(data) = function else {
				continue;
			};
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = function.take();
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
        while self.objects.last().is_none() {
            self.objects.pop();
        }
        while self.functions.last().is_none() {
            self.functions.pop();
        }
        start_the_world();
    }
}

impl HeapTrace for Value {
    fn trace(&self, heap: &Heap) {
        match self {
            &Value::String(handle) => heap.strings[handle.id as usize].trace(heap),
            &Value::Symbol(handle) => heap.symbols[handle.id as usize].trace(heap),
            &Value::Number(handle) => heap.numbers[handle.id as usize].trace(heap),
            &Value::BigInt(handle) => heap.bigints[handle.id as usize].trace(heap),
            &Value::Object(handle) => heap.objects[handle.id as usize].trace(heap),
            &Value::ArrayObject(handle) => heap.arrays[handle.id as usize].trace(heap),
            &Value::Function(handle) => heap.functions[handle.id as usize].trace(heap),
            _ => {}
        }
    }

    fn root(&self, heap: &Heap) {
        match self {
            &Value::String(handle) => heap.strings[handle.id as usize].root(heap),
            &Value::Symbol(handle) => heap.symbols[handle.id as usize].root(heap),
            &Value::Number(handle) => heap.numbers[handle.id as usize].root(heap),
            &Value::BigInt(handle) => heap.bigints[handle.id as usize].root(heap),
            &Value::Object(handle) => heap.objects[handle.id as usize].root(heap),
            &Value::ArrayObject(handle) => heap.arrays[handle.id as usize].root(heap),
            &Value::Function(handle) => heap.functions[handle.id as usize].root(heap),
            _ => {}
        }
    }

    fn unroot(&self, heap: &Heap) {
        match self {
            &Value::String(handle) => heap.strings[handle.id as usize].unroot(heap),
            &Value::Symbol(handle) => heap.symbols[handle.id as usize].unroot(heap),
            &Value::Number(handle) => heap.numbers[handle.id as usize].unroot(heap),
            &Value::BigInt(handle) => heap.bigints[handle.id as usize].unroot(heap),
            &Value::Object(handle) => heap.objects[handle.id as usize].unroot(heap),
            &Value::ArrayObject(handle) => heap.arrays[handle.id as usize].unroot(heap),
            &Value::Function(handle) => heap.functions[handle.id as usize].unroot(heap),
            _ => {}
        }
    }

    fn finalize(&mut self, _heap: &Heap) {
        unreachable!("Finalize should never be called on a Value in stack");
    }
}

// TODO: Change to using vectors of u8 bitfields for mark and dirty bits.
#[derive(Debug, Clone)]
pub struct HeapBits {
    marked: Cell<bool>,
    _weak_marked: Cell<bool>,
    dirty: Cell<bool>,
    // TODO: Consider removing roots entirely and only using globals.
    // Roots are useful for stack allocated Values, as they can just
    // mark their holding of the value. But they're not particularly great
    // from a GC standpoint, probably.
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
