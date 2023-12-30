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
use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
pub struct DateHeapData {
    pub(super) object_index: ObjectIndex,
    pub(super) _date: SystemTime,
}

pub fn initialize_date_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "now", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "parse", 1, false),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::DatePrototype.into(),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "UTC", 7, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::DateConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::DateConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::DateConstructor.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::DateConstructor,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "getDate", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getDay", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getFullYear", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getHours", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getMilliseconds", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getMinutes", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getMonth", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getSeconds", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getTime", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getTimezoneOffset", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCDate", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCDay", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCFullYear", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCHours", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCMilliseconds", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCMinutes", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCMonth", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "getUTCSeconds", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "setDate", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "setFullYear", 3, false),
        ObjectEntry::new_prototype_function_entry(heap, "setHours", 4, false),
        ObjectEntry::new_prototype_function_entry(heap, "setMilliseconds", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "setMinutes", 3, false),
        ObjectEntry::new_prototype_function_entry(heap, "setMonth", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "setSeconds", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "setTime", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "setUTCDate", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "setUTCFullYear", 3, false),
        ObjectEntry::new_prototype_function_entry(heap, "setUTCHours", 4, false),
        ObjectEntry::new_prototype_function_entry(heap, "setUTCMilliseconds", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "setUTCMinutes", 3, false),
        ObjectEntry::new_prototype_function_entry(heap, "setUTCMonth", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "setUTCSeconds", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "toDateString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toJSON", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleDateString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleTimeString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toTimeString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toUTCString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.toPrimitive]",
            WellKnownSymbolIndexes::ToPrimitive.into(),
            1,
            false,
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::DatePrototype,
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
