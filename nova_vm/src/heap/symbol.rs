use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

pub fn initialize_symbol_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::SymbolConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            Vec::with_capacity(24),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::SymbolConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::SymbolConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: symbol_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::SymbolPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        Vec::with_capacity(7),
    ));
}

fn symbol_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Symbol(0)
}
