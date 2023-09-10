use crate::{
    execution::JsResult,
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap,
    },
    types::Value,
};
use wtf8::Wtf8Buf;

#[derive(Debug, Clone)]
pub(crate) struct StringHeapData {
    pub(crate) data: Wtf8Buf,
}

impl StringHeapData {
    pub fn dummy() -> Self {
        Self {
            data: Wtf8Buf::new(),
        }
    }

    pub fn from_str(str: &str) -> Self {
        StringHeapData {
            data: Wtf8Buf::from_str(str),
        }
    }
}

pub fn initialize_string_heap(heap: &mut Heap) {
    heap.insert_builtin_object(
        BuiltinObjectIndexes::StringConstructorIndex,
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex.into()),
        // TODO: Methods and properties
        Vec::with_capacity(0),
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::StringConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::StringConstructorIndex.into()),
            length: 1,
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            initial_name: Value::Null,
        });
    heap.insert_builtin_object(
        BuiltinObjectIndexes::StringPrototypeIndex,
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex.into()),
        // TODO: Methods and properties
        Vec::with_capacity(0),
    );
}

fn string_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Null)
}
