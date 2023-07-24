use crate::{
    execution::JsResult,
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes, WellKnownSymbolIndexes},
        FunctionHeapData, Heap, PropertyDescriptor,
    },
    types::Value,
};

use super::{
    indexes::{FunctionIndex, StringIndex, SymbolIndex},
    object::{ObjectEntry, PropertyKey},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SymbolHeapData {
    pub(super) descriptor: Option<StringIndex>,
}

pub fn initialize_symbol_heap(heap: &mut Heap) {
    // AsyncIterator
    heap.symbols[WellKnownSymbolIndexes::AsyncIterator as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.asyncIterator")),
    });
    // HasInstance
    heap.symbols[WellKnownSymbolIndexes::HasInstance as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.hasInstance")),
    });
    // IsConcatSpreadable
    heap.symbols[WellKnownSymbolIndexes::IsConcatSpreadable as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.isConcatSpreadable")),
    });
    // Iterator
    heap.symbols[WellKnownSymbolIndexes::Iterator as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.iterator")),
    });
    // Match
    heap.symbols[WellKnownSymbolIndexes::Match as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.match")),
    });
    // MatchAll
    heap.symbols[WellKnownSymbolIndexes::MatchAll as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.matchAll")),
    });
    // Replace
    heap.symbols[WellKnownSymbolIndexes::Replace as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.replace")),
    });
    // Search
    heap.symbols[WellKnownSymbolIndexes::Search as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.search")),
    });
    // Species
    heap.symbols[WellKnownSymbolIndexes::Species as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.species")),
    });
    // Split
    heap.symbols[WellKnownSymbolIndexes::Split as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.split")),
    });
    // ToPrimitive
    heap.symbols[WellKnownSymbolIndexes::ToPrimitive as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.toPrimitive")),
    });
    // ToStringTag
    heap.symbols[WellKnownSymbolIndexes::ToStringTag as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.toStringTag")),
    });
    // Unscopables
    heap.symbols[WellKnownSymbolIndexes::Unscopables as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.alloc_string("Symbol.unscopables")),
    });

    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "asyncIterator"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::AsyncIterator.into())),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "for", 1, false, symbol_todo),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "hasInstance"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::HasInstance.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "isConcatSpreadable"),
            PropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::IsConcatSpreadable.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "iterator"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Iterator.into())),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "keyFor", 1, false, symbol_todo),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Match"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Match.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MatchAll"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::MatchAll.into())),
        ),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::SymbolPrototypeIndex.into(),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Replace"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Replace.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Search"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Search.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Species"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Species.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Split"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Split.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "ToPrimitive"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::ToPrimitive.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "ToStringTag"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::ToStringTag.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Unscopables"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Unscopables.into())),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::SymbolConstructorIndex,
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex.into()),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::SymbolConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::SymbolConstructorIndex.into(),
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: symbol_constructor_binding,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::SymbolConstructorIndex,
            ))),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "description"),
            // TODO: create description getter function
            PropertyDescriptor::ReadOnly {
                get: FunctionIndex::from_index(0),
                enumerable: false,
                configurable: true,
            },
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, symbol_todo),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false, symbol_todo),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.toPrimitive]",
            WellKnownSymbolIndexes::ToPrimitive.into(),
            1,
            false,
            symbol_todo,
        ),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag.into()),
            PropertyDescriptor::roxh(Value::from_str(heap, "Symbol")),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::SymbolPrototypeIndex,
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex.into()),
        entries,
    );
}

fn symbol_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Symbol(SymbolIndex::from_index(0)))
}

fn symbol_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!();
}
