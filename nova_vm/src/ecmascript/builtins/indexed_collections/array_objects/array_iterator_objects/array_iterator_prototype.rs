// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ecmascript_atomics::Ordering;

#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::indexed_collections::typed_array_objects::abstract_operations::make_typed_array_with_buffer_witness_record;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::create_iter_result_object,
            operations_on_objects::{create_array_from_list, get, length_of_array_like},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, array::ARRAY_INDEX_RANGE,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            typed_array::AnyTypedArray,
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, InternalSlots, IntoValue, Object, String, Value},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct ArrayIteratorPrototype;

struct ArrayIteratorPrototypeNext;
impl Builtin for ArrayIteratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayIteratorPrototype::next);
}

impl ArrayIteratorPrototype {
    fn next<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        // 27.5.3.2 GeneratorValidate ( generator, generatorBrand )
        // 3. If generator.[[GeneratorBrand]] is not generatorBrand, throw a TypeError exception.
        let Value::ArrayIterator(iterator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "ArrayIterator expected",
                gc.into_nogc(),
            ));
        };
        let mut iterator = iterator.bind(gc.nogc());

        // 23.1.5.1 CreateArrayIterator ( array, kind ), step 1. b
        // NOTE: We set `array` to None when the generator in the spec text has returned.
        let Some(array) = agent[iterator].array else {
            return Ok(create_iter_result_object(agent, Value::Undefined, true).into_value());
        };
        let mut array = array.bind(gc.nogc());

        let len: i64 = match array {
            // ii. Else,
            //     1. Let len be ? LengthOfArrayLike(array).
            Object::Array(array) => array.len(agent).into(),
            _ => {
                // i. If array has a [[TypedArrayName]] internal slot, then
                #[cfg(feature = "array-buffer")]
                if let Ok(array) = AnyTypedArray::try_from(array) {
                    // a. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(array, seq-cst).
                    let ta_record =
                        make_typed_array_with_buffer_witness_record(agent, array, Ordering::SeqCst);
                    // b. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                    if ta_record.is_typed_array_out_of_bounds(agent) {
                        return Err(agent.throw_exception_with_static_message(
                            ExceptionType::TypeError,
                            "TypedArray out of bounds",
                            gc.into_nogc(),
                        ));
                    }
                    // c. Let len be TypedArrayLength(taRecord).
                    i64::try_from(ta_record.typed_array_length(agent)).unwrap()
                } else {
                    let scoped_iterator = iterator.scope(agent, gc.nogc());
                    let scoped_array = array.scope(agent, gc.nogc());
                    let res =
                        length_of_array_like(agent, array.unbind(), gc.reborrow()).unbind()?;
                    array = unsafe { scoped_array.take(agent) }.bind(gc.nogc());
                    iterator = unsafe { scoped_iterator.take(agent) }.bind(gc.nogc());
                    res
                }
            }
        };

        // iii. If index ≥ len, return NormalCompletion(undefined).
        if agent[iterator].next_index >= len {
            agent[iterator].array = None;
            return Ok(create_iter_result_object(agent, Value::Undefined, true).into_value());
        }

        // iv. Let indexNumber be 𝔽(index).
        let index = agent[iterator].next_index;
        // viii. Set index to index + 1.
        agent[iterator].next_index += 1;

        let result = match agent[iterator].kind {
            // v. If kind is key, then
            CollectionIteratorKind::Key => {
                // 1. Let result be indexNumber.
                Value::Integer(index.try_into().unwrap())
            }
            // 3. If kind is value, then
            CollectionIteratorKind::Value => {
                // 1. Let elementKey be ! ToString(indexNumber).
                // 2. Let elementValue be ? Get(array, elementKey).
                // a. Let result be elementValue.
                let fast_path_result = match array {
                    Object::Array(array) => {
                        assert!(ARRAY_INDEX_RANGE.contains(&index));
                        let idx = usize::try_from(index).unwrap();
                        array.as_slice(agent)[idx]
                    }
                    _ => None,
                };
                match fast_path_result {
                    Some(result) => result,
                    None => get(
                        agent,
                        array.unbind(),
                        index.try_into().unwrap(),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc()),
                }
            }
            // 4. Else,
            CollectionIteratorKind::KeyAndValue => {
                // 1. Let elementKey be ! ToString(indexNumber).
                // 2. Let elementValue be ? Get(array, elementKey).
                let fast_path_result = match array {
                    Object::Array(array) if array.get_backing_object(agent).is_none() => {
                        assert!(ARRAY_INDEX_RANGE.contains(&index));
                        let idx = usize::try_from(index).unwrap();
                        array.as_slice(agent)[idx]
                    }
                    _ => None,
                };
                let value = match fast_path_result {
                    Some(result) => result,
                    None => get(
                        agent,
                        array.unbind(),
                        index.try_into().unwrap(),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc()),
                };
                // a. Assert: kind is key+value.
                // b. Let result be CreateArrayFromList(« indexNumber, elementValue »).
                create_array_from_list(
                    agent,
                    &[index.try_into().unwrap(), value.unbind()],
                    gc.nogc(),
                )
                .into_value()
            }
        };

        // vii. Perform ? GeneratorYield(CreateIteratorResultObject(result, false)).
        Ok(create_iter_result_object(agent, result.unbind(), false).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.array_iterator_prototype();
        let iterator_prototype = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(iterator_prototype)
            .with_builtin_function_property::<ArrayIteratorPrototypeNext>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Array_Iterator.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
