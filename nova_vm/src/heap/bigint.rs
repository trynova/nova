use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, HeapBits, ObjectEntry, ObjectHeapData, PropertyDescriptor,
        PropertyKey,
    },
    value::Value,
};

fn bigint_constructor(heap: &mut Heap, this: Value, args: &[Value]) -> Value {
    if !this.is_undefined() {
        // TODO: Throw TypeError
        return Value::Undefined;
    } else {
        return Value::SmallBigInt(3);
    }
}

pub fn initialize_bigint_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::BigintConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            vec![
                ObjectEntry::new_prototype_function(heap, "asIntN", 2, bigint_as_int_n),
                ObjectEntry::new_prototype_function(heap, "asUintN", 2, bigint_as_uint_n),
                ObjectEntry::new_prototype(heap, heap.objects.len() as u32 + 2),
            ],
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::BigintConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: heap.objects.len() as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: bigint_constructor,
        });
    heap.objects[BuiltinObjectIndexes::BigintPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        vec![
            ObjectEntry::new(
                PropertyKey::from_str(heap, "constructor"),
                PropertyDescriptor::rwx(Value::Object(
                    BuiltinObjectIndexes::BigintConstructorIndex as u32,
                )),
            ),
            ObjectEntry::new_prototype_function(
                heap,
                "toLocaleString",
                0,
                bigint_prototype_to_locale_string,
            ),
            ObjectEntry::new_prototype_function(heap, "toString", 0, bigint_prototype_to_string),
            ObjectEntry::new_prototype_function(heap, "valueOf", 0, bigint_prototype_value_of),
            // @@ToStringTag
            // ObjectEntry { key: PropertyKey::Symbol(), PropertyDescriptor }
        ],
    ));
}

fn bigint_as_int_n(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::SmallBigInt(3)
}

fn bigint_as_uint_n(heap: &mut Heap, this: Value, args: &[Value]) -> Value {
    Value::SmallBigIntU(3)
}

fn bigint_prototype_to_locale_string(heap: &mut Heap, this: Value, args: &[Value]) -> Value {
    Value::new_string(heap, "BigInt(3n)")
}

fn bigint_prototype_to_string(heap: &mut Heap, this: Value, args: &[Value]) -> Value {
    Value::new_string(heap, "BigInt(3n)")
}

fn bigint_prototype_value_of(heap: &mut Heap, this: Value, args: &[Value]) -> Value {
    Value::new_string(heap, "BigInt(3n)")
}
