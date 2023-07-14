use crate::{
    heap::{
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
    // BigInt constructor properties
    let bigint_as_int_n_entry =
        ObjectEntry::new_prototype_function(heap, "asIntN", 2, bigint_as_int_n);
    let bigint_as_uint_n_entry =
        ObjectEntry::new_prototype_function(heap, "asUintN", 2, bigint_as_uint_n);

    // BigInt prototype properties
    let bigint_prototype_to_locale_string = ObjectEntry::new_prototype_function(
        heap,
        "toLocaleString",
        0,
        bigint_prototype_to_locale_string,
    );
    let bigint_prototype_to_string =
        ObjectEntry::new_prototype_function(heap, "toString", 0, bigint_prototype_to_string);
    let bigint_prototype_value_of =
        ObjectEntry::new_prototype_function(heap, "valueOf", 0, bigint_prototype_value_of);
    // let bigint_prototype_to_string_tag = ObjectEntry { key: PropertyKey::Symbol(), PropertyDescriptor };

    let bigint_constructor_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::Data {
            // TODO: Get %Function.prototype%
            value: Value::Object(1),
            writable: false,
            enumerable: false,
            configurable: false,
        },
        vec![
            bigint_as_int_n_entry,
            bigint_as_uint_n_entry,
            ObjectEntry::new_prototype(heap, heap.objects.len() as u32 + 2),
        ],
    );
    heap.objects.push(Some(bigint_constructor_object));
    let bigint_constructor_object_idx = heap.objects.len() as u32;
    let bigint_prototype_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::Data {
            value: Value::Object(0),
            writable: false,
            enumerable: false,
            configurable: false,
        },
        vec![
            ObjectEntry::new(
                PropertyKey::from_str(heap, "constructor"),
                PropertyDescriptor::Data {
                    value: Value::Object(bigint_constructor_object_idx),
                    writable: true,
                    enumerable: true,
                    configurable: true,
                },
            ),
            bigint_prototype_to_locale_string,
            bigint_prototype_to_string,
            bigint_prototype_value_of,
            // bigint_prototype_to_string_tag,
        ],
    );
    heap.objects.push(Some(bigint_prototype_object));
    heap.functions.push(Some(FunctionHeapData {
        bits: HeapBits::new(),
        object_index: heap.objects.len() as u32,
        length: 1,
        uses_arguments: false,
        bound: None,
        visible: None,
        binding: bigint_constructor,
    }))
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
