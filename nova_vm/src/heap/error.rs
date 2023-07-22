use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

use super::{
    function::FunctionHeapData,
    heap_trace::HeapTrace,
    object::{ObjectEntry, PropertyKey},
};

#[derive(Debug)]
pub(crate) struct ErrorHeapData {
    pub(super) bits: HeapBits,
    pub(super) object_index: u32,
    // TODO: stack? name?
}

impl HeapTrace for Option<ErrorHeapData> {
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

pub fn initialize_error_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::ErrorConstructorIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
        vec![ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::ErrorPrototypeIndex as u32,
        )],
    ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::ErrorConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::ErrorConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: error_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::ErrorPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        vec![
            ObjectEntry::new(
                PropertyKey::from_str(heap, "constructor"),
                PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                    BuiltinObjectIndexes::ErrorConstructorIndex,
                ))),
            ),
            ObjectEntry::new(
                PropertyKey::from_str(heap, "name"),
                PropertyDescriptor::rwx(Value::EmptyString),
            ),
            ObjectEntry::new(
                PropertyKey::from_str(heap, "name"),
                PropertyDescriptor::rwx(Value::new_string(heap, "Error")),
            ),
            ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, error_todo),
        ],
    ));
}

fn error_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(0))
}

fn error_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!()
}
