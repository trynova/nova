use std::time::SystemTime;

use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

use super::{
    function::FunctionHeapData,
    heap_constants::WellKnownSymbolIndexes,
    object::{ObjectEntry, PropertyKey},
};

#[derive(Debug)]
pub(crate) struct DateHeapData {
    pub(super) object_index: u32,
    pub(super) _date: SystemTime,
}

pub fn initialize_date_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::DateConstructorIndex as usize] = Some(ObjectHeapData::new(
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
        vec![
            ObjectEntry::new_prototype_function_entry(heap, "now", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "parse", 1, false, date_todo),
            ObjectEntry::new_constructor_prototype_entry(
                heap,
                BuiltinObjectIndexes::DatePrototypeIndex as u32,
            ),
            ObjectEntry::new_prototype_function_entry(heap, "UTC", 7, false, date_todo),
        ],
    ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::DateConstructorIndex) as usize] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::DateConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: date_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::DatePrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        vec![
            ObjectEntry::new(
                PropertyKey::from_str(heap, "constructor"),
                PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                    BuiltinObjectIndexes::DateConstructorIndex,
                ))),
            ),
            ObjectEntry::new_prototype_function_entry(heap, "getDate", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getDay", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getFullYear", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getHours", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getMilliseconds", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getMinutes", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getMonth", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getSeconds", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getTime", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(
                heap,
                "getTimezoneOffset",
                0,
                false,
                date_todo,
            ),
            ObjectEntry::new_prototype_function_entry(heap, "getUTCDate", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getUTCDay", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getUTCFullYear", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getUTCHours", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(
                heap,
                "getUTCMilliseconds",
                0,
                false,
                date_todo,
            ),
            ObjectEntry::new_prototype_function_entry(heap, "getUTCMinutes", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getUTCMonth", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "getUTCSeconds", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setDate", 1, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setFullYear", 3, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setHours", 4, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setMilliseconds", 1, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setMinutes", 3, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setMonth", 2, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setSeconds", 2, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setTime", 1, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setUTCDate", 1, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setUTCFullYear", 3, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setUTCHours", 4, false, date_todo),
            ObjectEntry::new_prototype_function_entry(
                heap,
                "setUTCMilliseconds",
                1,
                false,
                date_todo,
            ),
            ObjectEntry::new_prototype_function_entry(heap, "setUTCMinutes", 3, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setUTCMonth", 2, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "setUTCSeconds", 2, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "toDateString", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "toJSON", 1, false, date_todo),
            ObjectEntry::new_prototype_function_entry(
                heap,
                "toLocaleDateString",
                0,
                false,
                date_todo,
            ),
            ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(
                heap,
                "toLocaleTimeString",
                0,
                false,
                date_todo,
            ),
            ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "toTimeString", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "toUTCString", 0, false, date_todo),
            ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false, date_todo),
            ObjectEntry::new_prototype_symbol_function_entry(
                heap,
                "[Symbol.toPrimitive]",
                WellKnownSymbolIndexes::ToPrimitive as u32,
                1,
                false,
                date_todo,
            ),
        ],
    ));
}

fn date_constructor_binding(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(0))
}

fn date_todo(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    todo!()
}
