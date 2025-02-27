// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "array-buffer")]
use crate::ecmascript::{
    builtins::{
        indexed_collections::typed_array_objects::abstract_operations::{
            is_typed_array_out_of_bounds, make_typed_array_with_buffer_witness_record,
            typed_array_length,
        },
        typed_array::TypedArray,
    },
    types::U8Clamped,
};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::create_iter_result_object,
            operations_on_objects::{create_array_from_list, get, length_of_array_like},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            array::ARRAY_INDEX_RANGE,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            ArgumentsList, Behaviour, Builtin,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::context::{Bindable, GcScope},
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
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        // 27.5.3.2 GeneratorValidate ( generator, generatorBrand )
        // 3. If generator.[[GeneratorBrand]] is not generatorBrand, throw a TypeError exception.
        let Value::ArrayIterator(iterator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "ArrayIterator expected",
                nogc,
            ));
        };
        let mut iterator = iterator.bind(nogc);

        // 23.1.5.1 CreateArrayIterator ( array, kind ), step 1. b
        // NOTE: We set `array` to None when the generator in the spec text has returned.
        let Some(array) = agent[iterator].array else {
            return Ok(
                create_iter_result_object(agent, Value::Undefined, true, gc.into_nogc())
                    .into_value(),
            );
        };

        #[cfg(feature = "array-buffer")]
        macro_rules! handle_typed_array {
            ($array:expr) => {{
                let array = $array;
                // 1. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(array, seq-cst).
                let ta_record = make_typed_array_with_buffer_witness_record(
                    agent,
                    array,
                    crate::ecmascript::builtins::array_buffer::Ordering::SeqCst,
                    nogc,
                );
                // 2. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if match array {
                    TypedArray::Int8Array(_) => {
                        is_typed_array_out_of_bounds::<i8>(agent, &ta_record, nogc)
                    }
                    TypedArray::Uint8Array(_) => {
                        is_typed_array_out_of_bounds::<u8>(agent, &ta_record, nogc)
                    }
                    TypedArray::Uint8ClampedArray(_) => {
                        is_typed_array_out_of_bounds::<U8Clamped>(agent, &ta_record, nogc)
                    }
                    TypedArray::Int16Array(_) => {
                        is_typed_array_out_of_bounds::<i16>(agent, &ta_record, nogc)
                    }
                    TypedArray::Uint16Array(_) => {
                        is_typed_array_out_of_bounds::<u16>(agent, &ta_record, nogc)
                    }
                    TypedArray::Int32Array(_) => {
                        is_typed_array_out_of_bounds::<i32>(agent, &ta_record, nogc)
                    }
                    TypedArray::Uint32Array(_) => {
                        is_typed_array_out_of_bounds::<u32>(agent, &ta_record, nogc)
                    }
                    TypedArray::BigInt64Array(_) => {
                        is_typed_array_out_of_bounds::<i64>(agent, &ta_record, nogc)
                    }
                    TypedArray::BigUint64Array(_) => {
                        is_typed_array_out_of_bounds::<u64>(agent, &ta_record, nogc)
                    }
                    #[cfg(feature = "proposal-float16array")]
                    TypedArray::Float16Array(_) => {
                        is_typed_array_out_of_bounds::<f16>(agent, &ta_record, nogc)
                    }
                    TypedArray::Float32Array(_) => {
                        is_typed_array_out_of_bounds::<f32>(agent, &ta_record, nogc)
                    }
                    TypedArray::Float64Array(_) => {
                        is_typed_array_out_of_bounds::<f64>(agent, &ta_record, nogc)
                    }
                } {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        nogc,
                    ));
                }

                // 3. Let len be TypedArrayLength(taRecord).
                (match array {
                    TypedArray::Int8Array(_) => typed_array_length::<i8>(agent, &ta_record, nogc),
                    TypedArray::Uint8Array(_) => typed_array_length::<u8>(agent, &ta_record, nogc),
                    TypedArray::Uint8ClampedArray(_) => {
                        typed_array_length::<U8Clamped>(agent, &ta_record, nogc)
                    }
                    TypedArray::Int16Array(_) => typed_array_length::<i16>(agent, &ta_record, nogc),
                    TypedArray::Uint16Array(_) => {
                        typed_array_length::<u16>(agent, &ta_record, nogc)
                    }
                    TypedArray::Int32Array(_) => typed_array_length::<i32>(agent, &ta_record, nogc),
                    TypedArray::Uint32Array(_) => {
                        typed_array_length::<u32>(agent, &ta_record, nogc)
                    }
                    TypedArray::BigInt64Array(_) => {
                        typed_array_length::<i64>(agent, &ta_record, nogc)
                    }
                    TypedArray::BigUint64Array(_) => {
                        typed_array_length::<u64>(agent, &ta_record, nogc)
                    }
                    #[cfg(feature = "proposal-float16array")]
                    TypedArray::Float16Array(_) => {
                        typed_array_length::<f16>(agent, &ta_record, nogc)
                    }
                    TypedArray::Float32Array(_) => {
                        typed_array_length::<f32>(agent, &ta_record, nogc)
                    }
                    TypedArray::Float64Array(_) => {
                        typed_array_length::<f64>(agent, &ta_record, nogc)
                    }
                }) as i64
            }};
        }

        let len: i64 = match array {
            // i. If array has a [[TypedArrayName]] internal slot, then
            #[cfg(feature = "array-buffer")]
            Object::Int8Array(array)
            | Object::Uint8Array(array)
            | Object::Uint8ClampedArray(array)
            | Object::Int16Array(array)
            | Object::Uint16Array(array)
            | Object::Int32Array(array)
            | Object::Uint32Array(array)
            | Object::BigInt64Array(array)
            | Object::BigUint64Array(array)
            | Object::Float32Array(array)
            | Object::Float64Array(array) => handle_typed_array!(array.into()),
            #[cfg(feature = "proposal-float16array")]
            Object::Float16Array(array) => handle_typed_array!(array.into()),
            // ii. Else,
            //     1. Let len be ? LengthOfArrayLike(array).
            Object::Array(array) => array.len(agent).into(),
            _ => {
                let scoped_iterator = iterator.scope(agent, nogc);
                let res = length_of_array_like(agent, array, gc.reborrow())?;
                iterator = scoped_iterator.get(agent).bind(gc.nogc());
                res
            }
        };

        // iii. If index ≥ len, return NormalCompletion(undefined).
        if agent[iterator].next_index >= len {
            agent[iterator].array = None;
            return Ok(
                create_iter_result_object(agent, Value::Undefined, true, gc.into_nogc())
                    .into_value(),
            );
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
                    None => get(agent, array, index.try_into().unwrap(), gc.reborrow())?,
                }
            }
            // 4. Else,
            CollectionIteratorKind::KeyAndValue => {
                // 1. Let elementKey be ! ToString(indexNumber).
                // 2. Let elementValue be ? Get(array, elementKey).
                let fast_path_result = match array {
                    Object::Array(array) if agent[array].object_index.is_none() => {
                        assert!(ARRAY_INDEX_RANGE.contains(&index));
                        let idx = usize::try_from(index).unwrap();
                        array.as_slice(agent)[idx]
                    }
                    _ => None,
                };
                let value = match fast_path_result {
                    Some(result) => result,
                    None => get(agent, array, index.try_into().unwrap(), gc.reborrow())?,
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
        Ok(create_iter_result_object(agent, result.unbind(), false, gc.into_nogc()).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
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
