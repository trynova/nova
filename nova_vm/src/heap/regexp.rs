use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

use super::{
    function::FunctionHeapData,
    heap_constants::WellKnownSymbolIndexes,
    heap_trace::HeapTrace,
    object::{ObjectEntry, PropertyKey},
};

#[derive(Debug)]
pub(crate) struct RegExpHeapData {
    pub(super) bits: HeapBits,
    pub(super) object_index: u32,
    // pub(super) _regex: RegExp,
}

impl HeapTrace for Option<RegExpHeapData> {
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

pub fn initialize_regexp_heap(heap: &mut Heap) {
    let species_function_name = Value::new_string(heap, "get [Symbol.species]");
    heap.objects[BuiltinObjectIndexes::RegExpConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            vec![
                ObjectEntry::new_constructor_prototype_entry(
                    heap,
                    BuiltinObjectIndexes::RegExpPrototypeIndex as u32,
                ),
                ObjectEntry::new(
                    PropertyKey::Symbol(WellKnownSymbolIndexes::Species as u32),
                    PropertyDescriptor::ReadOnly {
                        get: heap.create_function(species_function_name, 0, false, regexp_species),
                        enumerable: false,
                        configurable: true,
                    },
                ),
            ],
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::RegExpConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::RegExpConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: regexp_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::RegExpPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        vec![
            ObjectEntry::new(
                PropertyKey::from_str(heap, "constructor"),
                PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                    BuiltinObjectIndexes::RegExpConstructorIndex,
                ))),
            ),
            // TODO: Write out all the getters
            ObjectEntry::new_prototype_function_entry(heap, "exec", 1, false, regexp_todo),
            // TODO: These symbol function properties are actually rwxh, this helper generates roxh instead.
            ObjectEntry::new_prototype_symbol_function_entry(
                heap,
                "[Symbol.match]",
                WellKnownSymbolIndexes::Match as u32,
                1,
                false,
                regexp_todo,
            ),
            ObjectEntry::new_prototype_symbol_function_entry(
                heap,
                "[Symbol.matchAll]",
                WellKnownSymbolIndexes::MatchAll as u32,
                1,
                false,
                regexp_todo,
            ),
            ObjectEntry::new_prototype_symbol_function_entry(
                heap,
                "[Symbol.replace]",
                WellKnownSymbolIndexes::Replace as u32,
                2,
                false,
                regexp_todo,
            ),
            ObjectEntry::new_prototype_symbol_function_entry(
                heap,
                "[Symbol.search]",
                WellKnownSymbolIndexes::Search as u32,
                1,
                false,
                regexp_todo,
            ),
            ObjectEntry::new_prototype_symbol_function_entry(
                heap,
                "[Symbol.split]",
                WellKnownSymbolIndexes::Split as u32,
                2,
                false,
                regexp_todo,
            ),
            ObjectEntry::new_prototype_function_entry(heap, "test", 1, false, regexp_todo),
            ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, regexp_todo),
        ],
    ));
}

fn regexp_constructor_binding(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(0))
}

fn regexp_species(_heap: &mut Heap, this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(this)
}

fn regexp_todo(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    todo!()
}
