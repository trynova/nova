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
pub use heap_constants::BuiltinObjectIndexes;
pub use number::NumberHeapData;
pub use object::{ObjectEntry, ObjectHeapData};
pub use string::StringHeapData;
pub use symbol::SymbolHeapData;

use self::heap_trace::HeapTrace;
use crate::types::{Function, Number, Object, String, Value};
use std::{cell::Cell, marker::PhantomData, num::NonZeroU32};
use wtf8::{Wtf8, Wtf8Buf};

/// A handle to GC-managed memory.
#[derive(Clone)]
pub struct Handle<T: 'static> {
    id: NonZeroU32,
    _marker: &'static PhantomData<T>,
}

impl<T: 'static + Clone> Copy for Handle<T> {}

impl<T: 'static> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> Handle<T> {
    /// Id must not be 0.
    pub const fn new(id: u32) -> Self {
        debug_assert!(id != 0);
        Self {
            id: unsafe { NonZeroU32::new_unchecked(id) },
            // SAFETY: We hopefully will make sure handles are safe.
            _marker: unsafe {
                std::mem::transmute::<&PhantomData<T>, &'static PhantomData<T>>(&PhantomData)
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

pub trait GetHeapData<'a, T, F> {
    fn get(&'a self, handle: Handle<T>) -> &'a F;
    fn get_mut(&'a mut self, handle: Handle<T>) -> &'a mut F;
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

macro_rules! impl_heap_data {
    ($table: ident, $in: ty, $out: ty) => {
        impl<'a> GetHeapData<'a, $in, $out> for Heap {
            fn get(&'a self, handle: Handle<$in>) -> &'a $out {
                self.$table
                    .get(handle.id.get() as usize)
                    .unwrap()
                    .as_ref()
                    .unwrap()
            }

            fn get_mut(&'a mut self, handle: Handle<$in>) -> &'a mut $out {
                self.$table
                    .get_mut(handle.id.get() as usize)
                    .unwrap()
                    .as_mut()
                    .unwrap()
            }
        }
    };
    ($table: ident, $in: ty, $out: ty, $accessor: ident) => {
        impl<'a> GetHeapData<'a, $in, $out> for Heap {
            fn get(&'a self, handle: Handle<$in>) -> &'a $out {
                &self
                    .$table
                    .get(handle.id.get() as usize)
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .$accessor
            }

            fn get_mut(&'a mut self, handle: Handle<$in>) -> &'a mut $out {
                &mut self
                    .$table
                    .get_mut(handle.id.get() as usize)
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .$accessor
            }
        }
    };
}

impl_heap_data!(numbers, NumberHeapData, f64, data);
impl_heap_data!(objects, ObjectHeapData, ObjectHeapData);
impl_heap_data!(strings, StringHeapData, Wtf8Buf, data);
impl_heap_data!(functions, FunctionHeapData, FunctionHeapData);
impl_heap_data!(arrays, ArrayHeapData, ArrayHeapData);

impl CreateHeapData<&str, String> for Heap {
    fn create(&mut self, data: &str) -> String {
        if let Ok(value) = String::try_from(data) {
            value
        } else {
            let id = self.alloc_string(data);
            String::from(Handle::new(id))
        }
    }
}

impl CreateHeapData<FunctionHeapData, Function> for Heap {
    fn create(&mut self, data: FunctionHeapData) -> Function {
        let id = self.functions.len();
        self.functions.push(Some(data));
        Function(Handle::new(id as u32))
    }
}

impl CreateHeapData<ObjectHeapData, Object> for Heap {
    fn create(&mut self, data: ObjectHeapData) -> Object {
        let id: usize = self.functions.len();
        self.objects.push(Some(data));
        Object::Object(Handle::new(id as u32))
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

        heap.strings.push(Some(StringHeapData::dummy()));
        heap.symbols.push(Some(SymbolHeapData::dummy()));
        heap.numbers.push(Some(NumberHeapData::new(0.0)));
        heap.bigints.push(Some(BigIntHeapData::dummy()));
        heap.objects.push(Some(ObjectHeapData::dummy()));
        heap.arrays.push(Some(ArrayHeapData::dummy()));
        heap.functions.push(Some(FunctionHeapData::dummy()));

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
        for object in self.objects.iter().skip(1) {
            let Some(data) = object else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for string in self.strings.iter().skip(1) {
            let Some(data) = string else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for symbol in self.symbols.iter().skip(1) {
            let Some(data) = symbol else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for number in self.numbers.iter().skip(1) {
            let Some(data) = number else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        for bigint in self.bigints.iter().skip(1) {
            let Some(data) = bigint else {
                continue;
            };
            data.bits.marked.set(false);
            data.bits.dirty.set(false);
        }
        stop_the_world();
        // Trace from dirty objects and symbols.
        for object in self.objects.iter_mut().skip(1) {
            let Some(data) = object else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = object.take();
            }
        }
        for string in self.strings.iter_mut().skip(1) {
            let Some(data) = string else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = string.take();
            }
        }
        for symbol in self.symbols.iter_mut().skip(1) {
            let Some(data) = symbol else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = symbol.take();
            }
        }
        for number in self.numbers.iter_mut().skip(1) {
            let Some(data) = number else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = number.take();
            }
        }
        for bigint in self.bigints.iter_mut().skip(1) {
            let Some(data) = bigint else {
                continue;
            };
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = bigint.take();
            }
        }
        for object in self.objects.iter_mut().skip(1) {
            let Some(data) = object else {
				continue;
			};
            let marked = data.bits.marked.replace(true);
            if !marked {
                let _ = object.take();
            }
        }
        for function in self.functions.iter_mut().skip(1) {
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
            &Value::String(handle) => heap.strings[handle.id.get() as usize].trace(heap),
            &Value::Symbol(handle) => heap.symbols[handle.id.get() as usize].trace(heap),
            &Value::Number(handle) => heap.numbers[handle.id.get() as usize].trace(heap),
            &Value::BigInt(handle) => heap.bigints[handle.id.get() as usize].trace(heap),
            &Value::Object(handle) => heap.objects[handle.id.get() as usize].trace(heap),
            &Value::ArrayObject(handle) => heap.arrays[handle.id.get() as usize].trace(heap),
            &Value::Function(handle) => heap.functions[handle.id.get() as usize].trace(heap),
            _ => {}
        }
    }

    fn root(&self, heap: &Heap) {
        match self {
            &Value::String(handle) => heap.strings[handle.id.get() as usize].root(heap),
            &Value::Symbol(handle) => heap.symbols[handle.id.get() as usize].root(heap),
            &Value::Number(handle) => heap.numbers[handle.id.get() as usize].root(heap),
            &Value::BigInt(handle) => heap.bigints[handle.id.get() as usize].root(heap),
            &Value::Object(handle) => heap.objects[handle.id.get() as usize].root(heap),
            &Value::ArrayObject(handle) => heap.arrays[handle.id.get() as usize].root(heap),
            &Value::Function(handle) => heap.functions[handle.id.get() as usize].root(heap),
            _ => {}
        }
    }

    fn unroot(&self, heap: &Heap) {
        match self {
            &Value::String(handle) => heap.strings[handle.id.get() as usize].unroot(heap),
            &Value::Symbol(handle) => heap.symbols[handle.id.get() as usize].unroot(heap),
            &Value::Number(handle) => heap.numbers[handle.id.get() as usize].unroot(heap),
            &Value::BigInt(handle) => heap.bigints[handle.id.get() as usize].unroot(heap),
            &Value::Object(handle) => heap.objects[handle.id.get() as usize].unroot(heap),
            &Value::ArrayObject(handle) => heap.arrays[handle.id.get() as usize].unroot(heap),
            &Value::Function(handle) => heap.functions[handle.id.get() as usize].unroot(heap),
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
