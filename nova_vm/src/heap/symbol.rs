use super::{
    indexes::{FunctionIndex, SymbolIndex},
    object::ObjectEntry,
    CreateHeapData,
};
use crate::{
    ecmascript::{
        execution::JsResult,
        types::{Object, PropertyKey, String, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes, WellKnownSymbolIndexes},
        FunctionHeapData, Heap, PropertyDescriptor,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData {
    pub(super) descriptor: Option<String>,
}

pub fn initialize_symbol_heap(heap: &mut Heap) {
    // AsyncIterator
    heap.symbols[WellKnownSymbolIndexes::AsyncIterator as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.asyncIterator")),
    });
    // HasInstance
    heap.symbols[WellKnownSymbolIndexes::HasInstance as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.hasInstance")),
    });
    // IsConcatSpreadable
    heap.symbols[WellKnownSymbolIndexes::IsConcatSpreadable as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.isConcatSpreadable")),
    });
    // Iterator
    heap.symbols[WellKnownSymbolIndexes::Iterator as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.iterator")),
    });
    // Match
    heap.symbols[WellKnownSymbolIndexes::Match as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.match")),
    });
    // MatchAll
    heap.symbols[WellKnownSymbolIndexes::MatchAll as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.matchAll")),
    });
    // Replace
    heap.symbols[WellKnownSymbolIndexes::Replace as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.replace")),
    });
    // Search
    heap.symbols[WellKnownSymbolIndexes::Search as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.search")),
    });
    // Species
    heap.symbols[WellKnownSymbolIndexes::Species as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.species")),
    });
    // Split
    heap.symbols[WellKnownSymbolIndexes::Split as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.split")),
    });
    // ToPrimitive
    heap.symbols[WellKnownSymbolIndexes::ToPrimitive as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.toPrimitive")),
    });
    // ToStringTag
    heap.symbols[WellKnownSymbolIndexes::ToStringTag as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.toStringTag")),
    });
    // Unscopables
    heap.symbols[WellKnownSymbolIndexes::Unscopables as usize] = Some(SymbolHeapData {
        descriptor: Some(heap.create("Symbol.unscopables")),
    });

    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "asyncIterator"),
            PropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::AsyncIterator.into())),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "for", 1, false),
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
        ObjectEntry::new_prototype_function_entry(heap, "keyFor", 1, false),
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
        Some(Object::Function(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::SymbolConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::SymbolConstructorIndex.into()),
            length: 1,
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            initial_name: Value::Null,
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
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.toPrimitive]",
            WellKnownSymbolIndexes::ToPrimitive.into(),
            1,
            false,
        ),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag.into()),
            PropertyDescriptor::roxh(Value::from_str(heap, "Symbol")),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::SymbolPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
        entries,
    );
}

fn symbol_constructor_binding(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::Symbol(SymbolIndex::from_index(0)))
}

fn symbol_todo(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    todo!();
}
