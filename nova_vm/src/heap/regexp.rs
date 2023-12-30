use super::{heap_constants::WellKnownSymbolIndexes, indexes::ObjectIndex, object::ObjectEntry};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult},
        types::{BuiltinFunctionHeapData, Object, PropertyKey, Value},
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
            BuiltinObjectIndexes::RegExpPrototype.into(),
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
        BuiltinObjectIndexes::RegExpConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::RegExpConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::RegExpConstructor.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::RegExpConstructor,
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
        BuiltinObjectIndexes::RegExpPrototype,
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
