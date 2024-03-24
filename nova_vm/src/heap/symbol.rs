use super::{indexes::BuiltinFunctionIndex, object::ObjectEntry, CreateHeapData};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{Function, Object, PropertyKey, String, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes, WellKnownSymbolIndexes},
        BuiltinFunctionHeapData, Heap, ObjectEntryPropertyDescriptor,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData {
    pub(crate) descriptor: Option<String>,
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
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::AsyncIterator.into(),
            )),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "for", 1, false),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "hasInstance"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::HasInstance.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "isConcatSpreadable"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::IsConcatSpreadable.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "iterator"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::Iterator.into(),
            )),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "keyFor", 1, false),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Match"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Match.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MatchAll"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::MatchAll.into(),
            )),
        ),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::SymbolPrototype.into(),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Replace"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::Replace.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Search"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::Search.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Species"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::Species.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Split"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(WellKnownSymbolIndexes::Split.into())),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "ToPrimitive"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::ToPrimitive.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "ToStringTag"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::ToStringTag.into(),
            )),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "Unscopables"),
            ObjectEntryPropertyDescriptor::roh(Value::Symbol(
                WellKnownSymbolIndexes::Unscopables.into(),
            )),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::SymbolConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::SymbolConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::SymbolConstructor.into()),
            length: 1,
            initial_name: None,
            behaviour: Behaviour::Constructor(constructor_binding),
            realm: RealmIdentifier::from_index(0),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            ObjectEntryPropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::SymbolConstructor,
            ))),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "description"),
            // TODO: create description getter function
            ObjectEntryPropertyDescriptor::ReadOnly {
                get: Function::BuiltinFunction(BuiltinFunctionIndex::from_index(0)),
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
            ObjectEntryPropertyDescriptor::roxh(Value::from_str(heap, "Symbol")),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::SymbolPrototype,
        true,
        Some(Object::Object(BuiltinObjectIndexes::ObjectPrototype.into())),
        entries,
    );
}

fn constructor_binding(
    _agent: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    todo!()
}
