use super::{
    function::FunctionHeapData,
    heap_constants::WellKnownSymbolIndexes,
    indexes::{FunctionIndex, ObjectIndex},
    object::ObjectEntry,
};
use crate::{
    ecmascript::{
        execution::JsResult,
        types::{Object, PropertyKey, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, PropertyDescriptor,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct RegExpHeapData {
    pub(super) object_index: ObjectIndex,
    // pub(super) _regex: RegExp,
}

pub fn initialize_regexp_heap(heap: &mut Heap) {
    let species_function_name = Value::from_str(heap, "get [Symbol.species]");
    let entries = vec![
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::RegExpPrototypeIndex.into(),
        ),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::Species.into()),
            PropertyDescriptor::ReadOnly {
                get: heap.create_function(species_function_name, 0, false),
                enumerable: false,
                configurable: true,
            },
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::RegExpConstructorIndex,
        true,
        Some(Object::Function(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::RegExpConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::RegExpConstructorIndex.into()),
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
                BuiltinObjectIndexes::RegExpConstructorIndex,
            ))),
        ),
        // TODO: Write out all the getters
        ObjectEntry::new_prototype_function_entry(heap, "exec", 1, false),
        // TODO: These symbol function properties are actually rwxh, this helper generates roxh instead.
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.match]",
            WellKnownSymbolIndexes::Match.into(),
            1,
            false,
        ),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.matchAll]",
            WellKnownSymbolIndexes::MatchAll.into(),
            1,
            false,
        ),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.replace]",
            WellKnownSymbolIndexes::Replace.into(),
            2,
            false,
        ),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.search]",
            WellKnownSymbolIndexes::Search.into(),
            1,
            false,
        ),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.split]",
            WellKnownSymbolIndexes::Split.into(),
            2,
            false,
        ),
        ObjectEntry::new_prototype_function_entry(heap, "test", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::RegExpPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
        entries,
    );
}

fn regexp_constructor_binding(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(FunctionIndex::from_index(0)))
}

fn regexp_species(_heap: &mut Heap, this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(this)
}

fn regexp_todo(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    todo!()
}
