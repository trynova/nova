use crate::{
    execution::JsResult,
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, PropertyDescriptor,
    },
    types::Value,
};

use super::{
    heap_constants::WellKnownSymbolIndexes,
    indexes::{FunctionIndex, ObjectIndex},
    object::{ObjectEntry, PropertyKey},
};

pub type JsBindingFunction = fn(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value>;

#[derive(Debug, Clone)]
pub(crate) struct FunctionHeapData {
    pub(super) object_index: ObjectIndex,
    pub(super) length: u8,
    pub(super) uses_arguments: bool,
    pub(super) bound: Option<Box<[Value]>>,
    pub(super) visible: Option<Vec<Value>>,
    pub(super) binding: JsBindingFunction,
    // TODO: Should name be here as an "internal slot" of sorts?
}

pub fn initialize_function_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
    )];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::FunctionConstructorIndex,
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex.into()),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::FunctionConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::FunctionConstructorIndex.into(),
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: function_constructor_binding,
        });
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "apply", 2, false, function_todo),
        ObjectEntry::new_prototype_function_entry(heap, "bind", 1, true, function_todo),
        ObjectEntry::new_prototype_function_entry(heap, "call", 1, true, function_todo),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::FunctionConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, function_todo),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "hasInstance",
            WellKnownSymbolIndexes::HasInstance.into(),
            1,
            false,
            function_todo,
        ),
    ];
    // NOTE: According to ECMAScript spec https://tc39.es/ecma262/#sec-properties-of-the-function-prototype-object
    // the %Function.prototype% object should itself be a function that always returns undefined. This is not
    // upheld here and we probably do not care. It's seemingly the only prototype that is a function.
    heap.insert_builtin_object(
        BuiltinObjectIndexes::FunctionPrototypeIndex,
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex.into()),
        entries,
    );
}

fn function_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(FunctionIndex::from_index(0)))
}

fn function_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!()
}
