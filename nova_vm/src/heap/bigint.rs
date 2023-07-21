use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, HeapBits, ObjectEntry, ObjectHeapData, PropertyDescriptor,
        PropertyKey,
    },
    value::Value,
};

use super::heap_trace::HeapTrace;

pub(crate) struct BigIntHeapData {
    pub(super) bits: HeapBits,
    pub(super) sign: bool,
    pub(super) len: u32,
    pub(super) parts: Box<[u64]>,
}

impl BigIntHeapData {
    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn try_into_f64(&self) -> Option<f64> {
        if self.len == 1 {
            Some(self.parts[0] as f64)
        } else {
            None
        }
    }
}

impl HeapTrace for Option<BigIntHeapData> {
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
                ObjectEntry::new_prototype_function(heap, "asIntN", 2, false, bigint_as_int_n),
                ObjectEntry::new_prototype_function(heap, "asUintN", 2, false, bigint_as_uint_n),
                ObjectEntry::new_prototype(heap, BuiltinObjectIndexes::BigintPrototypeIndex),
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
                PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                    BuiltinObjectIndexes::BigintConstructorIndex,
                ))),
            ),
            ObjectEntry::new_prototype_function(
                heap,
                "toLocaleString",
                0,
                false,
                bigint_prototype_to_locale_string,
            ),
            ObjectEntry::new_prototype_function(
                heap,
                "toString",
                0,
                false,
                bigint_prototype_to_string,
            ),
            ObjectEntry::new_prototype_function(
                heap,
                "valueOf",
                0,
                false,
                bigint_prototype_value_of,
            ),
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
