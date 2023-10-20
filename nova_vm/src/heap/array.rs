use super::{
    element_array::ElementsVector,
    function::FunctionHeapData,
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

#[derive(Debug, Clone, Copy)]
pub struct ArrayHeapData {
    pub object_index: Option<ObjectIndex>,
    // TODO: Use SmallVec<[Value; 4]>
    pub elements: ElementsVector,
}

pub fn initialize_array_heap(heap: &mut Heap) {
    let species_function_name = Value::from_str(heap, "get [Symbol.species]");
    let at_key = PropertyKey::from_str(heap, "at");
    let copy_within_key = PropertyKey::from_str(heap, "copyWithin");
    let entries_key = PropertyKey::from_str(heap, "entries");
    let fill_key = PropertyKey::from_str(heap, "fill");
    let find_key = PropertyKey::from_str(heap, "find");
    let find_index_key = PropertyKey::from_str(heap, "findIndex");
    let find_last_key = PropertyKey::from_str(heap, "findLast");
    let find_last_index_key = PropertyKey::from_str(heap, "findLastIndex");
    let flat_key = PropertyKey::from_str(heap, "flat");
    let flat_map_key = PropertyKey::from_str(heap, "flatMap");
    let includes_key = PropertyKey::from_str(heap, "includes");
    let keys_key = PropertyKey::from_str(heap, "keys");
    let to_reversed_key = PropertyKey::from_str(heap, "toReversed");
    let to_sorted_key = PropertyKey::from_str(heap, "toSorted");
    let to_spliced_key = PropertyKey::from_str(heap, "toSpliced");
    let values_key = PropertyKey::from_str(heap, "values");
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "from", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "isArray", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "of", 0, true),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::ArrayPrototypeIndex.into(),
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
        BuiltinObjectIndexes::ArrayConstructorIndex,
        true,
        Some(Object::Function(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::ArrayConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::ArrayConstructorIndex.into()),
            length: 1,
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            initial_name: Value::Null,
        });
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "at", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "concat", 1, true),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::ArrayConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "copyWithin", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "entries", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "every", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "fill", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "filter", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "find", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "findIndex", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "findLast", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "findLastIndex", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "flat", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "flatMap", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "forEach", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "includes", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "indexOf", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "join", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "keys", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "lastIndexOf", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "map", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "pop", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "push", 1, true),
        ObjectEntry::new_prototype_function_entry(heap, "reduce", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "reduceRight", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "reverse", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "shift", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "slice", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "some", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "sort", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "splice", 2, true),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toReversed", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toSorted", 1, false),
        ObjectEntry::new_prototype_function_entry(heap, "toSpliced", 2, true),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "unshift", 1, true),
        ObjectEntry::new_prototype_function_entry(heap, "values", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "with", 2, false),
        // TODO: These symbol function properties are actually rwxh, this helper generates roxh instead.
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "[Symbol.iterator]",
            WellKnownSymbolIndexes::Iterator.into(),
            0,
            false,
        ),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::Unscopables.into()),
            PropertyDescriptor::roxh(Value::Object(heap.create_object(vec![
                ObjectEntry::new(at_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(
                    copy_within_key,
                    PropertyDescriptor::rwx(Value::Boolean(true)),
                ),
                ObjectEntry::new(entries_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(fill_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(find_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(
                    find_index_key,
                    PropertyDescriptor::rwx(Value::Boolean(true)),
                ),
                ObjectEntry::new(find_last_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(
                    find_last_index_key,
                    PropertyDescriptor::rwx(Value::Boolean(true)),
                ),
                ObjectEntry::new(flat_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(flat_map_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(includes_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(keys_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(
                    to_reversed_key,
                    PropertyDescriptor::rwx(Value::Boolean(true)),
                ),
                ObjectEntry::new(to_sorted_key, PropertyDescriptor::rwx(Value::Boolean(true))),
                ObjectEntry::new(
                    to_spliced_key,
                    PropertyDescriptor::rwx(Value::Boolean(true)),
                ),
                ObjectEntry::new(values_key, PropertyDescriptor::rwx(Value::Boolean(true))),
            ]))),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ArrayPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
        entries,
    );
}

fn array_constructor_binding(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(FunctionIndex::from_index(0)))
}

fn array_species(_heap: &mut Heap, this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(this)
}

fn array_todo(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    todo!()
}
