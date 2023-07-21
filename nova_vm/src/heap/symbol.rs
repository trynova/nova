use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes, WellKnownSymbolIndexes},
        heap_trace::HeapTrace,
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::{StringIndex, Value},
};

use super::object::{ObjectEntry, PropertyKey};

pub(crate) struct SymbolHeapData {
    pub(super) bits: HeapBits,
    pub(super) descriptor: Option<StringIndex>,
}

impl HeapTrace for Option<SymbolHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        if let Some(idx) = self.as_ref().unwrap().descriptor {
            heap.strings[idx as usize].trace(heap);
        }
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

pub fn initialize_symbol_heap(heap: &mut Heap) {
    // AsyncIterator
    heap.symbols[WellKnownSymbolIndexes::AsyncIterator as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.asyncIterator")),
    });
    // HasInstance
    heap.symbols[WellKnownSymbolIndexes::HasInstance as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.hasInstance")),
    });
    // IsConcatSpreadable
    heap.symbols[WellKnownSymbolIndexes::IsConcatSpreadable as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.isConcatSpreadable")),
    });
    // Iterator
    heap.symbols[WellKnownSymbolIndexes::Iterator as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.iterator")),
    });
    // Match
    heap.symbols[WellKnownSymbolIndexes::Match as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.match")),
    });
    // MatchAll
    heap.symbols[WellKnownSymbolIndexes::MatchAll as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.matchAll")),
    });
    // Replace
    heap.symbols[WellKnownSymbolIndexes::Replace as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.replace")),
    });
    // Search
    heap.symbols[WellKnownSymbolIndexes::Search as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.search")),
    });
    // Species
    heap.symbols[WellKnownSymbolIndexes::Species as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.species")),
    });
    // Split
    heap.symbols[WellKnownSymbolIndexes::Split as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.split")),
    });
    // ToPrimitive
    heap.symbols[WellKnownSymbolIndexes::ToPrimitive as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.toPrimitive")),
    });
    // ToStringTag
    heap.symbols[WellKnownSymbolIndexes::ToStringTag as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.toStringTag")),
    });
    // Unscopables
    heap.symbols[WellKnownSymbolIndexes::Unscopables as usize] = Some(SymbolHeapData {
        bits: HeapBits::new(),
        descriptor: Some(heap.alloc_string("Symbol.unscopables")),
    });

    heap.objects[BuiltinObjectIndexes::SymbolConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            vec![
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "asyncIterator"),
                    PropertyDescriptor::roh(Value::Symbol(
                        WellKnownSymbolIndexes::AsyncIterator as u32,
                    )),
                ),
                ObjectEntry::new_prototype_function(heap, "for", 1, symbol_todo),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "hasInstance"),
                    PropertyDescriptor::roh(Value::Symbol(
                        WellKnownSymbolIndexes::HasInstance as u32,
                    )),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "isConcatSpreadable"),
                    PropertyDescriptor::roh(Value::Symbol(
                        WellKnownSymbolIndexes::IsConcatSpreadable as u32,
                    )),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "iterator"),
                    PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Iterator as u32)),
                ),
                ObjectEntry::new_prototype_function(heap, "keyFor", 1, symbol_todo),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "Match"),
                    PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Match as u32)),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "MatchAll"),
                    PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::MatchAll as u32)),
                ),
                ObjectEntry::new_prototype(heap, BuiltinObjectIndexes::SymbolPrototypeIndex as u32),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "Replace"),
                    PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Replace as u32)),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "Search"),
                    PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Search as u32)),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "Species"),
                    PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Species as u32)),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "Split"),
                    PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Split as u32)),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "ToPrimitive"),
                    PropertyDescriptor::roh(Value::Symbol(
                        WellKnownSymbolIndexes::ToPrimitive as u32,
                    )),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "ToStringTag"),
                    PropertyDescriptor::roh(Value::Symbol(
                        WellKnownSymbolIndexes::ToStringTag as u32,
                    )),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "Unscopables"),
                    PropertyDescriptor::roh(Value::Symbol(
                        WellKnownSymbolIndexes::Unscopables as u32,
                    )),
                ),
            ],
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
        vec![
            ObjectEntry::new(
                PropertyKey::from_str(heap, "constructor"),
                PropertyDescriptor::rwx(Value::Object(
                    BuiltinObjectIndexes::SymbolConstructorIndex as u32,
                )),
            ),
            ObjectEntry::new(
                PropertyKey::from_str(heap, "description"),
                // TODO: create description getter function
                PropertyDescriptor::ReadOnly {
                    get: 0,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new_prototype_function(heap, "toString", 0, symbol_todo),
            ObjectEntry::new_prototype_function(heap, "valueOf", 0, symbol_todo),
            ObjectEntry::new_prototype_symbol_function(
                heap,
                "[Symbol.toPrimitive]",
                WellKnownSymbolIndexes::ToPrimitive as u32,
                1,
                symbol_todo,
            ),
            ObjectEntry::new(
                PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag as u32),
                PropertyDescriptor::roxh(Value::new_string(heap, "Symbol")),
            ),
        ],
    ));
}

fn symbol_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Symbol(0)
}

fn symbol_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    todo!();
}
