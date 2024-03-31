use super::Heap;
use crate::ecmascript::{
    builtins::ArgumentsList,
    execution::{Agent, JsResult},
    types::{Object, Value},
};

pub fn initialize_number_heap(_heap: &mut Heap) {
    // let entries = vec![
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "EPSILON"),
    //         ObjectEntryPropertyDescriptor::roh(heap.create(f64::EPSILON).into()),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "isFinite", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "isInteger", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "isNan", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "isSafeInteger", 1, false),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "MAX_SAFE_INTEGER"),
    //         ObjectEntryPropertyDescriptor::roh(Number::from(SmallInteger::MAX_NUMBER).into()),
    //     ),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "MAX_VALUE"),
    //         ObjectEntryPropertyDescriptor::roh(heap.create(f64::MAX).into()),
    //     ),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "MIN_SAFE_INTEGER"),
    //         ObjectEntryPropertyDescriptor::roh(Number::from(SmallInteger::MIN_NUMBER).into()),
    //     ),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "MIN_VALUE"),
    //         ObjectEntryPropertyDescriptor::roh(heap.create(f64::MIN).into()),
    //     ),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "NaN"),
    //         ObjectEntryPropertyDescriptor::roh(Value::from(f32::NAN)),
    //     ),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "NEGATIVE_INFINITY"),
    //         ObjectEntryPropertyDescriptor::roh(Value::from(f32::NEG_INFINITY)),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "parseFloat", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "parseInt", 2, false),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "POSITIVE_INFINITY"),
    //         ObjectEntryPropertyDescriptor::roh(Value::from(f32::INFINITY)),
    //     ),
    //     ObjectEntry::new_constructor_prototype_entry(
    //         heap,
    //         IntrinsicObjectIndexes::NumberPrototype.into(),
    //     ),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::NumberConstructor,
    //     true,
    //     Some(Object::BuiltinFunction(
    //         IntrinsicObjectIndexes::FunctionPrototype.into(),
    //     )),
    //     entries,
    // );
    // heap.builtin_functions
    //     [get_constructor_index(IntrinsicObjectIndexes::NumberConstructor).into_index()] =
    //     Some(BuiltinFunctionHeapData {
    //         object_index: Some(IntrinsicObjectIndexes::NumberConstructor.into()),
    //         length: 1,
    //         initial_name: None,
    //         behaviour: Behaviour::Constructor(constructor_binding),
    //         realm: RealmIdentifier::from_index(0),
    //     });
    // let entries = vec![
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "constructor"),
    //         ObjectEntryPropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
    //             IntrinsicObjectIndexes::NumberConstructor,
    //         ))),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "toExponential", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "toExponential", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "toPrecision", 1, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::NumberPrototype,
    //     true,
    //     Some(Object::Object(
    //         IntrinsicObjectIndexes::ObjectPrototype.into(),
    //     )),
    //     entries,
    // );
}

fn constructor_binding(
    _agent: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    todo!()
}
