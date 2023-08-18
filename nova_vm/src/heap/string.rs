use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};
use wtf8::Wtf8Buf;

#[derive(Debug)]
pub(crate) struct StringHeapData {
    pub(crate) data: Wtf8Buf,
}

impl StringHeapData {
    pub fn from_str(str: &str) -> Self {
        StringHeapData {
            data: Wtf8Buf::from_str(str),
        }
    }

    pub fn len(&self) -> usize {
        // TODO: We should return the UTF-16 length.
        self.data.len()
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

fn string_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::EmptyString)
}
