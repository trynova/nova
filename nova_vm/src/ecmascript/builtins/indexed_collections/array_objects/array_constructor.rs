// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::get_iterator_from_method;
use crate::ecmascript::abstract_operations::operations_on_iterator_objects::if_abrupt_close_iterator;
use crate::ecmascript::abstract_operations::operations_on_iterator_objects::iterator_close;
use crate::ecmascript::abstract_operations::operations_on_iterator_objects::iterator_step_value;
use crate::ecmascript::abstract_operations::operations_on_objects::call_function;
use crate::ecmascript::abstract_operations::operations_on_objects::construct;
use crate::ecmascript::abstract_operations::operations_on_objects::create_data_property_or_throw;
use crate::ecmascript::abstract_operations::operations_on_objects::get;
use crate::ecmascript::abstract_operations::operations_on_objects::get_method;
use crate::ecmascript::abstract_operations::operations_on_objects::length_of_array_like;
use crate::ecmascript::abstract_operations::operations_on_objects::set;
use crate::ecmascript::abstract_operations::operations_on_objects::try_create_data_property_or_throw;
use crate::ecmascript::abstract_operations::testing_and_comparison::is_array;

use crate::ecmascript::abstract_operations::testing_and_comparison::is_callable;
use crate::ecmascript::abstract_operations::testing_and_comparison::is_constructor;
use crate::ecmascript::abstract_operations::testing_and_comparison::same_value_zero;
use crate::ecmascript::abstract_operations::type_conversion::to_object;
use crate::ecmascript::abstract_operations::type_conversion::to_uint32_number;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;

use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinGetter;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::builtins::array_create;
use crate::ecmascript::builtins::ordinary::get_prototype_from_constructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::agent::ExceptionType;

use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;

use crate::SmallInteger;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::InternalMethods;
use crate::ecmascript::types::IntoFunction;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Number;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::PropertyDescriptor;
use crate::ecmascript::types::PropertyKey;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::engine::Scoped;
use crate::engine::context::Bindable;
use crate::engine::context::GcScope;
use crate::engine::rootable::Scopable;
use crate::engine::unwrap_try;
use crate::heap::IntrinsicConstructorIndexes;
use crate::heap::WellKnownSymbolIndexes;

pub struct ArrayConstructor;

impl Builtin for ArrayConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Array;
}
impl BuiltinIntrinsicConstructor for ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Array;
}

struct ArrayFrom;
impl Builtin for ArrayFrom {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::from);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.from;
}
struct ArrayIsArray;
impl Builtin for ArrayIsArray {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::is_array);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isArray;
}
struct ArrayOf;
impl Builtin for ArrayOf {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::of);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.of;
}
struct ArrayGetSpecies;
impl Builtin for ArrayGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());
}
impl BuiltinGetter for ArrayGetSpecies {}

/// ### [23.1.1 The Array Constructor](https://tc39.es/ecma262/#sec-array-constructor)
impl ArrayConstructor {
    /// ### [23.1.1.1 Array ( ...values )](https://tc39.es/ecma262/#sec-array)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        enum BoundArgs<'a> {
            Length(Scoped<'a, Value<'static>>),
            Items(Vec<Scoped<'a, Value<'static>>>),
        }
        let new_target = new_target.bind(gc.nogc());
        // 1. If NewTarget is undefined, let newTarget be the active function object; else let newTarget be NewTarget.
        let new_target = new_target.map_or_else(
            || agent.running_execution_context().function.unwrap(),
            |new_target| Function::try_from(new_target).unwrap(),
        );

        let arguments = if arguments.len() == 0 {
            None
        } else if arguments.len() == 1 {
            Some(BoundArgs::Length(arguments.get(0).scope(agent, gc.nogc())))
        } else {
            Some(BoundArgs::Items(
                arguments
                    .iter()
                    .map(|v| v.scope(agent, gc.nogc()))
                    .collect(),
            ))
        };

        // 2. Let proto be ? GetPrototypeFromConstructor(newTarget, "%Array.prototype%").
        let proto = get_prototype_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::Array,
            gc.reborrow(),
        )?
        .unbind();
        let gc = gc.into_nogc();
        let proto = proto.bind(gc);

        // 3. Let numberOfArgs be the number of elements in values.

        // 4. If numberOfArgs = 0, then
        let Some(arguments) = arguments else {
            // a. Return ! ArrayCreate(0, proto).
            return Ok(array_create(agent, 0, 0, proto, gc).unwrap().into_value());
        };

        // 5. Else if numberOfArgs = 1, then
        match arguments {
            BoundArgs::Length(len) => {
                // a. Let len be values[0].
                let len = len.get(agent).bind(gc);

                // c. If len is not a Number, then
                let array = if let Ok(len) = Number::try_from(len) {
                    // d. Else,
                    // i. Let intLen be ! ToUint32(len).
                    let proto = proto.map(|p| p.scope(agent, gc));
                    let int_len = to_uint32_number(agent, len);
                    // ii. If SameValueZero(intLen, len) is false, throw a RangeError exception.
                    if !same_value_zero(agent, int_len, len) {
                        return Err(agent.throw_exception_with_static_message(
                            ExceptionType::RangeError,
                            "Invalid array length",
                            gc,
                        ));
                    }
                    let array = array_create(
                        agent,
                        int_len as usize,
                        int_len as usize,
                        proto.map(|p| p.get(agent)),
                        gc,
                    )
                    .unwrap();
                    // e. Perform ! Set(array, "length", intLen, true).
                    debug_assert_eq!(agent[array].elements.len(), int_len);
                    array
                } else {
                    // b. Let array be ! ArrayCreate(0, proto).
                    let array = array_create(agent, 1, 1, proto, gc).unwrap();
                    // i. Perform ! CreateDataPropertyOrThrow(array, "0", len).
                    unwrap_try(try_create_data_property_or_throw(
                        agent,
                        array,
                        PropertyKey::from(SmallInteger::zero()),
                        len,
                        gc,
                    ))
                    .unwrap();
                    // ii. Let intLen be 1𝔽.
                    // e. Perform ! Set(array, "length", intLen, true).
                    debug_assert_eq!(agent[array].elements.len(), 1);
                    array
                };

                // f. Return array.
                Ok(array.into_value())
            }
            BoundArgs::Items(args) => {
                // 6. Else,
                // a. Assert: numberOfArgs ≥ 2.
                let number_of_args = args.len();
                debug_assert!(number_of_args >= 2);

                // b. Let array be ? ArrayCreate(numberOfArgs, proto).
                let array = array_create(agent, number_of_args, number_of_args, proto, gc)?;

                // c. Let k be 0.
                // d. Repeat, while k < numberOfArgs,
                for (k, item_k) in args.into_iter().enumerate() {
                    // NOTE: `array_create` guarantees that it is less than `u32::MAX`
                    let k = k as u32;
                    // NOTE: We slightly deviate from the exact spec wording
                    // here, see [@aapoalas comment on #180](https://github.com/trynova/nova/pull/180#discussion_r1600382492)
                    // i. Let Pk be ! ToString(𝔽(k)).
                    let pk = PropertyKey::from(SmallInteger::from(k));

                    // ii. Let itemK be values[k].

                    // iii. Perform ! CreateDataPropertyOrThrow(array, Pk, itemK).
                    unwrap_try(try_create_data_property_or_throw(
                        agent,
                        array,
                        pk,
                        item_k.get(agent),
                        gc,
                    ))
                    .unwrap();

                    // iv. Set k to k + 1.
                }

                // e. Assert: The mathematical value of array's "length" property is numberOfArgs.
                debug_assert_eq!(array.len(agent) as usize, number_of_args);

                // f. Return array.
                Ok(array.into_value())
            }
        }
    }

    /// ### [23.1.2.1 Array.from ( items \[ , mapfn \[ , thisArg \] \] )](https://tc39.es/ecma262/#sec-array.from)
    fn from<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let items = arguments.get(0).bind(gc.nogc());
        let mapfn = arguments.get(1).bind(gc.nogc());
        let this_arg = arguments.get(2).bind(gc.nogc());

        // 1. Let C be the this value.
        // 2. If mapfn is undefined, then
        let mapping = if mapfn.is_undefined() {
            // a. Let mapping be false.
            None
        } else {
            // 3. Else,
            // a. If IsCallable(mapfn) is false, throw a TypeError exception.
            let Some(mapfn) = is_callable(mapfn, gc.nogc()) else {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "The map function of Array.from is not callable",
                    gc.nogc(),
                ));
            };

            // b. Let mapping be true.
            Some(mapfn.scope(agent, gc.nogc()))
        };
        let scoped_this_value = this_value.scope(agent, gc.nogc());
        let scoped_items = items.scope(agent, gc.nogc());
        let scoped_this_arg = this_arg.scope(agent, gc.nogc());

        // 4. Let usingIterator be ? GetMethod(items, @@iterator).
        let using_iterator = get_method(
            agent,
            items.unbind(),
            WellKnownSymbolIndexes::Iterator.into(),
            gc.reborrow(),
        )?;

        // 5. If usingIterator is not undefined, then
        if let Some(using_iterator) = using_iterator {
            let mut using_iterator = using_iterator.unbind().bind(gc.nogc());
            // a. If IsConstructor(C) is true, then
            let a = if let Some(c) = is_constructor(agent, scoped_this_value.get(agent)) {
                let scoped_using_iterator = using_iterator.scope(agent, gc.nogc());
                // i. Let A be ? Construct(C).
                let a = construct(agent, c.unbind(), None, None, gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                using_iterator = scoped_using_iterator.get(agent).bind(gc.nogc());
                a
            } else {
                // b. Else,
                // i. Let A be ! ArrayCreate(0).
                array_create(agent, 0, 0, None, gc.nogc())
                    .unwrap()
                    .into_object()
            };

            let a = a.scope(agent, gc.nogc());

            // c. Let iteratorRecord be ? GetIteratorFromMethod(items, usingIterator).
            let mut iterator_record = get_iterator_from_method(
                agent,
                scoped_items.get(agent),
                using_iterator.unbind(),
                gc.reborrow(),
            )?;

            // d. Let k be 0.
            let mut k = 0;

            // e. Repeat,
            loop {
                // NOTE: The actual max size of an array is u32::MAX
                // i. If k ≥ 2**53 - 1, then
                if k >= u32::MAX as usize {
                    // 1. Let error be ThrowCompletion(a newly created TypeError object).
                    let error = agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Maximum array size of 2**53-1 exceeded",
                        gc.nogc(),
                    );
                    // 2. Return ? IteratorClose(iteratorRecord, error).
                    return iterator_close(agent, &iterator_record, Err(error), gc.reborrow());
                }

                let sk = SmallInteger::from(k as u32);
                // 𝔽(k)
                let fk = Number::from(sk).into_value();

                // ii. Let Pk be ! ToString(𝔽(k)).
                let pk = PropertyKey::from(sk);

                // iii. Let next be ? IteratorStepValue(iteratorRecord).
                let Some(next) = iterator_step_value(agent, &mut iterator_record, gc.reborrow())?
                else {
                    // iv. If next is done, then
                    // 1. Perform ? Set(A, "length", 𝔽(k), true).
                    set(
                        agent,
                        a.get(agent),
                        PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                        fk,
                        true,
                        gc.reborrow(),
                    )?;

                    // 2. Return A.
                    return Ok(a.get(agent).into_value());
                };

                // v. If mapping is true, then
                let mapped_value = if let Some(mapping) = &mapping {
                    // 1. Let mappedValue be Completion(Call(mapfn, thisArg, « next, 𝔽(k) »)).
                    let mapped_value = call_function(
                        agent,
                        mapping.get(agent),
                        scoped_this_arg.get(agent),
                        Some(ArgumentsList(&[next.unbind(), fk])),
                        gc.reborrow(),
                    );

                    // 2. IfAbruptCloseIterator(mappedValue, iteratorRecord).
                    if_abrupt_close_iterator!(agent, mapped_value, iterator_record, gc)
                } else {
                    // vi. Else,
                    // 1. Let mappedValue be next.
                    next
                };

                // vii. Let defineStatus be Completion(CreateDataPropertyOrThrow(A, Pk, mappedValue)).
                let define_status = create_data_property_or_throw(
                    agent,
                    a.get(agent),
                    pk,
                    mapped_value.unbind(),
                    gc.reborrow(),
                );

                // viii. IfAbruptCloseIterator(defineStatus, iteratorRecord).
                if_abrupt_close_iterator!(agent, define_status, iterator_record, gc);

                // ix. Set k to k + 1.
                k += 1;
            }
        }

        // 6. NOTE: items is not an Iterable so assume it is an array-like object.
        // 7. Let arrayLike be ! ToObject(items).
        let array_like = to_object(agent, scoped_items.get(agent), gc.nogc())
            .unwrap()
            .scope(agent, gc.nogc());

        // 8. Let len be ? LengthOfArrayLike(arrayLike).
        let len = length_of_array_like(agent, array_like.get(agent), gc.reborrow())?;
        let len_value = Value::try_from(len).unwrap();

        // 9. If IsConstructor(C) is true, then
        let a = if let Some(c) = is_constructor(agent, scoped_this_value.get(agent)) {
            // a. Let A be ? Construct(C, « 𝔽(len) »).
            construct(
                agent,
                c,
                Some(ArgumentsList(&[len_value])),
                None,
                gc.reborrow(),
            )?
            .unbind()
            .bind(gc.nogc())
        } else {
            // 10. Else,
            // a. Let A be ? ArrayCreate(len).
            array_create(agent, len as usize, len as usize, None, gc.nogc())?.into_object()
        };

        let a = a.scope(agent, gc.nogc());

        // 11. Let k be 0.
        let mut k = 0;

        // 12. Repeat, while k < len,
        while k < len {
            let sk = SmallInteger::from(k as u32);
            // 𝔽(k)
            let fk = Number::from(sk).into_value();

            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::from(sk);

            // b. Let kValue be ? Get(arrayLike, Pk).
            let k_value = get(agent, array_like.get(agent), pk, gc.reborrow())?;

            // c. If mapping is true, then
            let mapped_value = if let Some(mapping) = &mapping {
                // i. Let mappedValue be ? Call(mapfn, thisArg, « kValue, 𝔽(k) »).
                call_function(
                    agent,
                    mapping.get(agent),
                    scoped_this_arg.get(agent),
                    Some(ArgumentsList(&[k_value.unbind(), fk])),
                    gc.reborrow(),
                )?
            } else {
                // d. Else,
                // i. Let mappedValue be kValue.
                k_value
            };

            // e. Perform ? CreateDataPropertyOrThrow(A, Pk, mappedValue).
            create_data_property_or_throw(
                agent,
                a.get(agent),
                pk,
                mapped_value.unbind(),
                gc.reborrow(),
            )?;

            // f. Set k to k + 1.
            k += 1;
        }

        // 13. Perform ? Set(A, "length", 𝔽(len), true).
        set(
            agent,
            a.get(agent),
            PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            Value::try_from(len).unwrap(),
            true,
            gc.reborrow(),
        )?;

        // 14. Return A.
        Ok(a.get(agent).into_value())
    }

    /// ### [23.1.2.2 Array.isArray ( arg )](https://tc39.es/ecma262/#sec-array.isarray)
    fn is_array<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        is_array(agent, arguments.get(0), gc.into_nogc()).map(Value::Boolean)
    }

    /// ### [23.1.2.3 Array.of ( ...items )](https://tc39.es/ecma262/#sec-array.of)
    fn of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());

        // 3. Let C be the this value.
        // 4. If IsConstructor(C) is true, then
        if let Some(c) = is_constructor(agent, this_value) {
            // a. Let A be ? Construct(C, « lenNumber »).
            if c != agent.current_realm().intrinsics().array().into_function() {
                let arguments = arguments
                    .iter()
                    .map(|v| v.scope(agent, gc.nogc()))
                    .collect();
                return array_of_generic(agent, c.unbind(), arguments, gc);
            }
            // We're constructring an array with the default constructor.
        }

        // 1. Let len be the number of elements in items.
        // 2. Let lenNumber be 𝔽(len).
        let len = arguments.len();

        // 5. Else,
        // a. Let A be ? ArrayCreate(len).
        let gc = gc.into_nogc();
        let a = array_create(agent, len, len, None, gc)?;

        // 6. Let k be 0.
        // 7. Repeat, while k < len,
        for (k, &k_value) in arguments.iter().enumerate() {
            // a. Let kValue be items[k].

            // NOTE: `array_create` guarantees that `len` and by extension `k` is less than `u32::MAX`
            // b. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::from(SmallInteger::from(k as u32));

            // c. Perform ? CreateDataPropertyOrThrow(A, Pk, kValue).
            assert!(unwrap_try(a.try_define_own_property(
                agent,
                pk,
                PropertyDescriptor::new_data_descriptor(k_value),
                gc
            )));

            // d. Set k to k + 1.
        }

        // 8. Perform ? Set(A, "length", lenNumber, true).
        // Note: Array's own length setting cannot be observed.

        // 9. Return A.
        Ok(a.into_value())
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        Ok(this_value.bind(gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let function_prototype = intrinsics.function_prototype().into_object();
        let array_prototype = intrinsics.array_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ArrayConstructor>(agent, realm)
            .with_property_capacity(5)
            .with_prototype(function_prototype)
            .with_builtin_function_property::<ArrayFrom>()
            .with_builtin_function_property::<ArrayIsArray>()
            .with_builtin_function_property::<ArrayOf>()
            .with_prototype_property(array_prototype.into_object())
            .with_builtin_function_getter_property::<ArrayGetSpecies>()
            .build();
    }
}

fn array_of_generic<'gc>(
    agent: &mut Agent,
    c: Function,
    args: Vec<Scoped<'_, Value<'static>>>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Value<'gc>> {
    let c = c.bind(gc.nogc());
    // a. Let A be ? Construct(C, « lenNumber »).
    let len_number = Number::try_from(args.len()).unwrap();
    let a = construct(
        agent,
        c.unbind(),
        Some(ArgumentsList(&[len_number.into_value()])),
        None,
        gc.reborrow(),
    )?
    .unbind();

    let a = a.unbind().scope(agent, gc.nogc());

    // 6. Let k be 0.
    // 7. Repeat, while k < len,
    for (k, k_value) in args.into_iter().enumerate() {
        // a. Let kValue be items[k].

        let pk = PropertyKey::try_from(k).unwrap();

        // c. Perform ? CreateDataPropertyOrThrow(A, Pk, kValue).
        create_data_property_or_throw(agent, a.get(agent), pk, k_value.get(agent), gc.reborrow())?;

        // d. Set k to k + 1.
    }

    // 8. Perform ? Set(A, "length", lenNumber, true).
    set(
        agent,
        a.get(agent),
        PropertyKey::from(BUILTIN_STRING_MEMORY.length),
        len_number.into_value(),
        true,
        gc.reborrow(),
    )?;

    Ok(a.get(agent).bind(gc.into_nogc()).into_value())
}
