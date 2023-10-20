use super::{
    heap_constants::WellKnownSymbolIndexes,
    indexes::{FunctionIndex, ObjectIndex},
    object::ObjectEntry,
};
use crate::{
    execution::JsResult,
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, PropertyDescriptor,
    },
    types::{Object, PropertyKey, Value},
};

#[derive(Debug, Clone)]
pub struct FunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub initial_name: Value,
    // pub behaviour: Behaviour,
    // TODO: Should we create a `BoundFunctionHeapData` for an exotic object
    //       that allows setting fields and other deoptimizations?
    // pub(super) uses_arguments: bool,
    // pub(super) bound: Option<Box<[Value]>>,
    // pub(super) visible: Option<Vec<Value>>,
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
        Some(Object::Function(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::FunctionConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::FunctionConstructorIndex.into()),
            length: 1,
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            initial_name: Value::Null,
        });
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "apply", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "bind", 1, true),
        ObjectEntry::new_prototype_function_entry(heap, "call", 1, true),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::FunctionConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "hasInstance",
            WellKnownSymbolIndexes::HasInstance.into(),
            1,
            false,
        ),
    ];
    // NOTE: According to ECMAScript spec https://tc39.es/ecma262/#sec-properties-of-the-function-prototype-object
    // the %Function.prototype% object should itself be a function that always returns undefined. This is not
    // upheld here and we probably do not care. It's seemingly the only prototype that is a function.
    heap.insert_builtin_object(
        BuiltinObjectIndexes::FunctionPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
        entries,
    );
}

fn function_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(FunctionIndex::from_index(0)))
}

fn function_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!()
}
