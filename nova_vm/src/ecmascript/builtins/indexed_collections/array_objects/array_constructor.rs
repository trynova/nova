use crate::ecmascript::abstract_operations::operations_on_iterator_objects::get_iterator_from_method;
use crate::ecmascript::abstract_operations::operations_on_iterator_objects::if_abrupt_close_iterator;
use crate::ecmascript::abstract_operations::operations_on_iterator_objects::iterator_close;
use crate::ecmascript::abstract_operations::operations_on_iterator_objects::iterator_step_value;
use crate::ecmascript::abstract_operations::operations_on_objects::call;
use crate::ecmascript::abstract_operations::operations_on_objects::construct;
use crate::ecmascript::abstract_operations::operations_on_objects::create_data_property_or_throw;
use crate::ecmascript::abstract_operations::operations_on_objects::get;
use crate::ecmascript::abstract_operations::operations_on_objects::get_method;
use crate::ecmascript::abstract_operations::operations_on_objects::length_of_array_like;
use crate::ecmascript::abstract_operations::operations_on_objects::set;
use crate::ecmascript::abstract_operations::testing_and_comparison::is_array;

use crate::ecmascript::abstract_operations::testing_and_comparison::is_callable;
use crate::ecmascript::abstract_operations::testing_and_comparison::is_constructor;
use crate::ecmascript::abstract_operations::testing_and_comparison::same_value_zero;
use crate::ecmascript::abstract_operations::type_conversion::to_object;
use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;

use crate::ecmascript::builtins::array_create;
use crate::ecmascript::builtins::ordinary::get_prototype_from_constructor;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinGetter;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;

use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;

use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::PropertyKey;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::heap::IntrinsicConstructorIndexes;
use crate::heap::WellKnownSymbolIndexes;

pub struct ArrayConstructor;

impl Builtin for ArrayConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.String;
}
impl BuiltinIntrinsicConstructor for ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Array;
}

struct ArrayFrom;
impl Builtin for ArrayFrom {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::from);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.from;
}
struct ArrayIsArray;
impl Builtin for ArrayIsArray {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::is_array);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.isArray;
}
struct ArrayOf;
impl Builtin for ArrayOf {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::of);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.fromCodePoint;
}
struct ArrayGetSpecies;
impl Builtin for ArrayGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for ArrayGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}

/// ### [23.1.1 The Array Constructor](https://tc39.es/ecma262/#sec-array-constructor)
impl ArrayConstructor {
    /// ### [23.1.1.1 Array ( ...values )](https://tc39.es/ecma262/#sec-array)
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        // 1. If NewTarget is undefined, let newTarget be the active function object; else let newTarget be NewTarget.
        let new_target = new_target.map_or_else(
            || agent.running_execution_context().function.unwrap(),
            |new_target| Function::try_from(new_target).unwrap(),
        );

        // 2. Let proto be ? GetPrototypeFromConstructor(newTarget, "%Array.prototype%").
        let proto = get_prototype_from_constructor(agent, new_target, ProtoIntrinsics::Array)?;

        // 3. Let numberOfArgs be the number of elements in values.
        let number_of_args = arguments.len();

        // 4. If numberOfArgs = 0, then
        if number_of_args == 0 {
            // a. Return ! ArrayCreate(0, proto).
            return Ok(array_create(agent, 0, 0, Some(proto)).unwrap().into_value());
        }

        // 5. Else if numberOfArgs = 1, then
        if number_of_args == 1 {
            // a. Let len be values[0].
            let len = arguments.get(0);

            // b. Let array be ! ArrayCreate(0, proto).
            let array = array_create(agent, 0, 0, Some(proto)).unwrap();

            // c. If len is not a Number, then
            let int_len = if !len.is_number() {
                // i. Perform ! CreateDataPropertyOrThrow(array, "0", len).
                create_data_property_or_throw(
                    agent,
                    array.into(),
                    PropertyKey::from(BUILTIN_STRING_MEMORY.r#_0),
                    len,
                )
                .unwrap();
                // ii. Let intLen be 1𝔽.
                1
            } else {
                // d. Else,
                // i. Let intLen be ! ToUint32(len).
                let int_len = len.to_uint32(agent).unwrap();
                // ii. If SameValueZero(intLen, len) is false, throw a RangeError exception.
                if !same_value_zero(agent, int_len, len) {
                    return Err(
                        agent.throw_exception(ExceptionType::RangeError, "Invalid array length")
                    );
                }
                int_len
            };

            // e. Perform ! Set(array, "length", intLen, true).
            set(
                agent,
                array.into(),
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                int_len.into(),
                true,
            )
            .unwrap();

            // f. Return array.
            return Ok(array.into_value());
        }

        // 6. Else,
        // a. Assert: numberOfArgs ≥ 2.
        debug_assert!(number_of_args >= 2);

        // b. Let array be ? ArrayCreate(numberOfArgs, proto).
        let array = array_create(agent, number_of_args, 0, Some(proto))?;

        // c. Let k be 0.
        let mut k = 0;

        // d. Repeat, while k < numberOfArgs,
        while k < number_of_args {
            // i. Let Pk be ! ToString(𝔽(k)).
            let pk = to_string(agent, Value::try_from(k as f64).unwrap())
                .unwrap()
                .to_property_key();

            // ii. Let itemK be values[k].
            let item_k = arguments.get(k);

            // iii. Perform ! CreateDataPropertyOrThrow(array, Pk, itemK).
            create_data_property_or_throw(agent, array.into(), pk, item_k).unwrap();

            // iv. Set k to k + 1.
            k += 1;
        }

        // e. Assert: The mathematical value of array's "length" property is numberOfArgs.
        debug_assert_eq!(array.len(agent), number_of_args as u32);

        // f. Return array.
        Ok(array.into_value())
    }

    /// ### [23.1.2.1 Array.from ( items \[ , mapfn \[ , thisArg \] \] )](https://tc39.es/ecma262/#sec-array.from)
    fn from(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let items = arguments.get(0);
        let mapfn = arguments.get(1);
        let this_arg = arguments.get(2);

        // 1. Let C be the this value.
        // 2. If mapfn is undefined, then
        let mapping = if mapfn.is_undefined() {
            // a. Let mapping be false.
            false
        } else {
            // 3. Else,
            // a. If IsCallable(mapfn) is false, throw a TypeError exception.
            if !is_callable(mapfn) {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    "The map function of Array.from is not callable",
                ));
            }

            // b. Let mapping be true.
            true
        };

        // 4. Let usingIterator be ? GetMethod(items, @@iterator).
        let using_iterator = get_method(agent, items, WellKnownSymbolIndexes::Iterator.into())?;

        // 5. If usingIterator is not undefined, then
        if let Some(using_iterator) = using_iterator {
            // a. If IsConstructor(C) is true, then
            let a = if is_constructor(agent, this_value) {
                // SAFETY: Only Functions can be constructors
                let constructor = unsafe { Function::try_from(this_value).unwrap_unchecked() };

                // i. Let A be ? Construct(C).
                construct(agent, constructor, None, None)?
            } else {
                // b. Else,
                // i. Let A be ! ArrayCreate(0).
                array_create(agent, 0, 0, None).unwrap().into_object()
            };

            // c. Let iteratorRecord be ? GetIteratorFromMethod(items, usingIterator).
            let iterator_record = get_iterator_from_method(agent, items, using_iterator)?;

            // d. Let k be 0.
            let mut k = 0;

            // e. Repeat,
            loop {
                // i. If k ≥ 2**53 - 1, then
                if k >= 2usize.pow(53) - 1 {
                    // 1. Let error be ThrowCompletion(a newly created TypeError object).
                    let error = agent.throw_exception(
                        ExceptionType::TypeError,
                        "Maximum array size of 2**53-1 exceeded",
                    );
                    // 2. Return ? IteratorClose(iteratorRecord, error).
                    return iterator_close(agent, &iterator_record, Err(error));
                }

                // 𝔽(k)
                let fk: Value = Value::try_from(k as f64).unwrap();
                // ii. Let Pk be ! ToString(𝔽(k)).
                let pk = to_string(agent, fk).unwrap().to_property_key();

                // iii. Let next be ? IteratorStepValue(iteratorRecord).
                let Some(next) = iterator_step_value(agent, &iterator_record)? else {
                    // iv. If next is done, then
                    // 1. Perform ? Set(A, "length", 𝔽(k), true).
                    set(
                        agent,
                        a,
                        PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                        fk,
                        true,
                    )?;

                    // 2. Return A.
                    return Ok(a.into_value());
                };

                // v. If mapping is true, then
                let mapped_value = if mapping {
                    // 1. Let mappedValue be Completion(Call(mapfn, thisArg, « next, 𝔽(k) »)).
                    let mapped_value =
                        call(agent, mapfn, this_arg, Some(ArgumentsList(&[next, fk])));

                    // 2. IfAbruptCloseIterator(mappedValue, iteratorRecord).
                    let _ = if_abrupt_close_iterator(agent, mapped_value, &iterator_record);

                    mapped_value.unwrap()
                } else {
                    // vi. Else,
                    // 1. Let mappedValue be next.
                    next
                };

                // vii. Let defineStatus be Completion(CreateDataPropertyOrThrow(A, Pk, mappedValue)).
                let define_status = create_data_property_or_throw(agent, a, pk, mapped_value);

                // viii. IfAbruptCloseIterator(defineStatus, iteratorRecord).
                let _ = if_abrupt_close_iterator(
                    agent,
                    define_status.map(|_| Value::Undefined),
                    &iterator_record,
                );

                // ix. Set k to k + 1.
                k += 1;
            }
        }

        // 6. NOTE: items is not an Iterable so assume it is an array-like object.
        // 7. Let arrayLike be ! ToObject(items).
        let array_like = to_object(agent, items).unwrap();

        // 8. Let len be ? LengthOfArrayLike(arrayLike).
        let len = length_of_array_like(agent, array_like)? as usize;
        let len_value = Value::try_from(len as f64).unwrap();

        // 9. If IsConstructor(C) is true, then
        let a = if is_constructor(agent, this_value) {
            // SAFETY: Only Functions can be constructors
            let constructor = unsafe { Function::try_from(this_value).unwrap_unchecked() };

            // a. Let A be ? Construct(C, « 𝔽(len) »).
            construct(agent, constructor, Some(ArgumentsList(&[len_value])), None)?
        } else {
            // 10. Else,
            // a. Let A be ? ArrayCreate(len).
            array_create(agent, len, len, None)?.into_object()
        };

        // 11. Let k be 0.
        let mut k = 0;

        // 12. Repeat, while k < len,
        while k < len {
            // 𝔽(k)
            let fk = Value::try_from(k as f64).unwrap();

            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = to_string(agent, fk).unwrap().to_property_key();

            // b. Let kValue be ? Get(arrayLike, Pk).
            let k_value = get(agent, array_like, pk)?;

            // c. If mapping is true, then
            let mapped_value = if mapping {
                // i. Let mappedValue be ? Call(mapfn, thisArg, « kValue, 𝔽(k) »).
                call(agent, mapfn, this_arg, Some(ArgumentsList(&[k_value, fk])))?
            } else {
                // d. Else,
                // i. Let mappedValue be kValue.
                k_value
            };

            // e. Perform ? CreateDataPropertyOrThrow(A, Pk, mappedValue).
            create_data_property_or_throw(agent, a, pk, mapped_value)?;

            // f. Set k to k + 1.
            k += 1;
        }

        // 13. Perform ? Set(A, "length", 𝔽(len), true).
        set(
            agent,
            a,
            PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            Value::try_from(len as f64).unwrap(),
            true,
        )?;

        // 14. Return A.
        Ok(a.into_value())
    }

    /// ### [23.1.2.2 Array.isArray ( arg )](https://tc39.es/ecma262/#sec-array.isarray)
    fn is_array(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        is_array(agent, arguments.get(0)).map(Value::Boolean)
    }

    /// ### [23.1.2.3 Array.of ( ...items )](https://tc39.es/ecma262/#sec-array.of)
    fn of(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let len be the number of elements in items.
        let len = arguments.len();

        // 2. Let lenNumber be 𝔽(len).
        let len_number = Value::try_from(len as f64).unwrap();

        // 3. Let C be the this value.
        // 4. If IsConstructor(C) is true, then
        let a = if is_constructor(agent, this_value) {
            // SAFETY: Only Functions can be constructors
            let constructor = unsafe { Function::try_from(this_value).unwrap_unchecked() };

            // a. Let A be ? Construct(C, « lenNumber »).
            construct(agent, constructor, Some(ArgumentsList(&[len_number])), None)?
        } else {
            // 5. Else,
            // a. Let A be ? ArrayCreate(len).
            array_create(agent, len, len, None)?.into_object()
        };

        // 6. Let k be 0.
        let mut k = 0;

        // 7. Repeat, while k < len,
        while k < len {
            // a. Let kValue be items[k].
            let k_value = arguments.get(k);

            // b. Let Pk be ! ToString(𝔽(k)).
            let pk = to_string(agent, Value::try_from(k as f64).unwrap())
                .unwrap()
                .to_property_key();

            // c. Perform ? CreateDataPropertyOrThrow(A, Pk, kValue).
            create_data_property_or_throw(agent, a, pk, k_value)?;

            // d. Set k to k + 1.
            k += 1;
        }

        // 8. Perform ? Set(A, "length", lenNumber, true).
        set(
            agent,
            a,
            PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            len_number,
            true,
        )?;

        // 9. Return A.
        Ok(a.into_value())
    }

    fn get_species(
        _agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let array_prototype = intrinsics.array_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ArrayConstructor>(agent, realm)
            .with_property_capacity(5)
            .with_builtin_function_property::<ArrayFrom>()
            .with_builtin_function_property::<ArrayIsArray>()
            .with_builtin_function_property::<ArrayOf>()
            .with_prototype_property(array_prototype.into_object())
            .with_builtin_function_getter_property::<ArrayGetSpecies>()
            .build();
    }
}
