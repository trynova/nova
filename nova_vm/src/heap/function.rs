use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

use super::{
    heap_constants::WellKnownSymbolIndexes,
    heap_trace::HeapTrace,
    object::{ObjectEntry, PropertyKey},
};

pub type JsBindingFunction = fn(heap: &mut Heap, this: Value, args: &[Value]) -> Value;

pub(crate) struct FunctionHeapData {
    pub(super) bits: HeapBits,
    pub(super) object_index: u32,
    pub(super) length: u8,
    pub(super) uses_arguments: bool,
    pub(super) bound: Option<Box<[Value]>>,
    pub(super) visible: Option<Vec<Value>>,
    pub(super) binding: JsBindingFunction,
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

pub fn initialize_function_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::FunctionConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            vec![ObjectEntry::new_prototype(
                heap,
                BuiltinObjectIndexes::FunctionPrototypeIndex as u32,
            )],
        ));
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::FunctionConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::FunctionConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: function_constructor_binding,
        });
    // NOTE: According to ECMAScript spec https://tc39.es/ecma262/#sec-properties-of-the-function-prototype-object
    // the %Function.prototype% object should itself be a function that always returns undefined. This is not
    // upheld here and we probably do not care. It's seemingly the only prototype that is a function.
    heap.objects[BuiltinObjectIndexes::FunctionPrototypeIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
            vec![
                ObjectEntry::new_prototype_function(heap, "apply", 2, false, function_todo),
                ObjectEntry::new_prototype_function(heap, "bind", 1, true, function_todo),
                ObjectEntry::new_prototype_function(heap, "call", 1, true, function_todo),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "constructor"),
                    PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                        BuiltinObjectIndexes::FunctionConstructorIndex,
                    ))),
                ),
                ObjectEntry::new_prototype_function(heap, "toString", 0, false, function_todo),
                ObjectEntry::new_prototype_symbol_function(
                    heap,
                    "hasInstance",
                    WellKnownSymbolIndexes::HasInstance as u32,
                    1,
                    false,
                    function_todo,
                ),
            ],
        ));
}

fn function_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Function(0)
}

fn function_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    todo!()
}
