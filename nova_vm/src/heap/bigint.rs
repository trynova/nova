use super::indexes::ObjectIndex;
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult},
        types::{Object, PropertyKey, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        BuiltinFunctionHeapData, Heap, ObjectEntry, PropertyDescriptor,
    },
};

pub fn initialize_bigint_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "asIntN", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "asUintN", 2, false),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::BigintPrototype.into(),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BigintConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::BigintConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(ObjectIndex::last(&heap.objects)),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::BigintConstructor,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
        // @@ToStringTag
        // ObjectEntry { key: PropertyKey::Symbol(), PropertyDescriptor }
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BigintPrototype,
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
