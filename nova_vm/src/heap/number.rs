use super::{object::ObjectEntry, CreateHeapData, Heap};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult},
        types::{Number, Object, PropertyKey, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        BuiltinFunctionHeapData, PropertyDescriptor,
    },
    SmallInteger,
};

pub fn initialize_number_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "EPSILON"),
            PropertyDescriptor::roh(heap.create(f64::EPSILON).into()),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "isFinite", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isInteger", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isNan", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isSafeInteger", 1, false),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MAX_SAFE_INTEGER"),
            PropertyDescriptor::roh(Number::try_from(SmallInteger::MAX_NUMBER).unwrap().into()),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MAX_VALUE"),
            PropertyDescriptor::roh(heap.create(f64::MAX).into()),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MIN_SAFE_INTEGER"),
            PropertyDescriptor::roh(Number::try_from(SmallInteger::MIN_NUMBER).unwrap().into()),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MIN_VALUE"),
            PropertyDescriptor::roh(heap.create(f64::MIN).into()),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "NaN"),
            PropertyDescriptor::roh(Value::from(f32::NAN)),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "NEGATIVE_INFINITY"),
            PropertyDescriptor::roh(Value::from(f32::NEG_INFINITY)),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "parseFloat", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "parseInt", 2, false),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "POSITIVE_INFINITY"),
            PropertyDescriptor::roh(Value::from(f32::INFINITY)),
        ),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::NumberPrototype.into(),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::NumberConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::NumberConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::NumberConstructor.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::NumberConstructor,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toExponential", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toExponential", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toPrecision", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::NumberPrototype,
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
