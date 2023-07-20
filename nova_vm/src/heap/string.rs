use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        heap_trace::HeapTrace,
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};
use wtf8::Wtf8Buf;

pub(crate) struct StringHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) data: Wtf8Buf,
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

pub fn initialize_string_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::StringConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            // TODO: Methods and properties
            Vec::with_capacity(0),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::StringConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::StringConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: string_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::StringPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        // TODO: Methods and properties
        Vec::with_capacity(0),
    ));
}

fn string_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::EmptyString
}
