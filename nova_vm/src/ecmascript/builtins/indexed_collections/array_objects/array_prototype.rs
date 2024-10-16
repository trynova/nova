// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::cmp::Ordering;

use small_string::SmallString;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_data_property_or_throw, delete_property_or_throw, get,
                has_property, length_of_array_like, set,
            },
            testing_and_comparison::{
                is_array, is_callable, is_less_than, is_strictly_equal, same_value_zero,
            },
            type_conversion::{
                to_boolean, to_integer_or_infinity, to_number, to_object, to_string,
            },
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            array_create, array_species_create, ArgumentsList, ArrayHeapData, Behaviour, Builtin,
            BuiltinIntrinsic,
        },
        execution::{
            agent::{ExceptionType, JsError},
            Agent, JsResult, RealmIdentifier,
        },
        types::{
            Function, IntoFunction, IntoObject, IntoValue, Number, Object, PropertyKey, String,
            Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{Heap, IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
    SmallInteger,
};

use super::array_iterator_objects::array_iterator::{ArrayIterator, CollectionIteratorKind};

pub(crate) struct ArrayPrototype;

struct ArrayPrototypeAt;
impl Builtin for ArrayPrototypeAt {
    const NAME: String = BUILTIN_STRING_MEMORY.at;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::at);
}
struct ArrayPrototypeConcat;
impl Builtin for ArrayPrototypeConcat {
    const NAME: String = BUILTIN_STRING_MEMORY.concat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::concat);
}
struct ArrayPrototypeCopyWithin;
impl Builtin for ArrayPrototypeCopyWithin {
    const NAME: String = BUILTIN_STRING_MEMORY.copyWithin;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::copy_within);
}
struct ArrayPrototypeEntries;
impl Builtin for ArrayPrototypeEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::entries);
}
struct ArrayPrototypeEvery;
impl Builtin for ArrayPrototypeEvery {
    const NAME: String = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::every);
}
struct ArrayPrototypeFill;
impl Builtin for ArrayPrototypeFill {
    const NAME: String = BUILTIN_STRING_MEMORY.fill;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::fill);
}
struct ArrayPrototypeFilter;
impl Builtin for ArrayPrototypeFilter {
    const NAME: String = BUILTIN_STRING_MEMORY.filter;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::filter);
}
struct ArrayPrototypeFind;
impl Builtin for ArrayPrototypeFind {
    const NAME: String = BUILTIN_STRING_MEMORY.find;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find);
}
struct ArrayPrototypeFindIndex;
impl Builtin for ArrayPrototypeFindIndex {
    const NAME: String = BUILTIN_STRING_MEMORY.findIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_index);
}
struct ArrayPrototypeFindLast;
impl Builtin for ArrayPrototypeFindLast {
    const NAME: String = BUILTIN_STRING_MEMORY.findLast;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_last);
}
struct ArrayPrototypeFindLastIndex;
impl Builtin for ArrayPrototypeFindLastIndex {
    const NAME: String = BUILTIN_STRING_MEMORY.findLastIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_last_index);
}
struct ArrayPrototypeFlat;
impl Builtin for ArrayPrototypeFlat {
    const NAME: String = BUILTIN_STRING_MEMORY.flat;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::flat);
}
struct ArrayPrototypeFlatMap;
impl Builtin for ArrayPrototypeFlatMap {
    const NAME: String = BUILTIN_STRING_MEMORY.flatMap;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::flat_map);
}
struct ArrayPrototypeForEach;
impl Builtin for ArrayPrototypeForEach {
    const NAME: String = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::for_each);
}
struct ArrayPrototypeIncludes;
impl Builtin for ArrayPrototypeIncludes {
    const NAME: String = BUILTIN_STRING_MEMORY.includes;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::includes);
}
struct ArrayPrototypeIndexOf;
impl Builtin for ArrayPrototypeIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.indexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::index_of);
}
struct ArrayPrototypeJoin;
impl Builtin for ArrayPrototypeJoin {
    const NAME: String = BUILTIN_STRING_MEMORY.join;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::join);
}
struct ArrayPrototypeKeys;
impl Builtin for ArrayPrototypeKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::keys);
}
struct ArrayPrototypeLastIndexOf;
impl Builtin for ArrayPrototypeLastIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::last_index_of);
}
struct ArrayPrototypeMap;
impl Builtin for ArrayPrototypeMap {
    const NAME: String = BUILTIN_STRING_MEMORY.map;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::map);
}
struct ArrayPrototypePop;
impl Builtin for ArrayPrototypePop {
    const NAME: String = BUILTIN_STRING_MEMORY.pop;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::pop);
}
struct ArrayPrototypePush;
impl Builtin for ArrayPrototypePush {
    const NAME: String = BUILTIN_STRING_MEMORY.push;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::push);
}
struct ArrayPrototypeReduce;
impl Builtin for ArrayPrototypeReduce {
    const NAME: String = BUILTIN_STRING_MEMORY.reduce;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reduce);
}
struct ArrayPrototypeReduceRight;
impl Builtin for ArrayPrototypeReduceRight {
    const NAME: String = BUILTIN_STRING_MEMORY.reduceRight;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reduce_right);
}
struct ArrayPrototypeReverse;
impl Builtin for ArrayPrototypeReverse {
    const NAME: String = BUILTIN_STRING_MEMORY.reverse;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reverse);
}
struct ArrayPrototypeShift;
impl Builtin for ArrayPrototypeShift {
    const NAME: String = BUILTIN_STRING_MEMORY.shift;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::shift);
}
struct ArrayPrototypeSlice;
impl Builtin for ArrayPrototypeSlice {
    const NAME: String = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::slice);
}
struct ArrayPrototypeSome;
impl Builtin for ArrayPrototypeSome {
    const NAME: String = BUILTIN_STRING_MEMORY.some;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::some);
}
struct ArrayPrototypeSort;
impl Builtin for ArrayPrototypeSort {
    const NAME: String = BUILTIN_STRING_MEMORY.sort;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::sort);
}
impl BuiltinIntrinsic for ArrayPrototypeSort {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeSort;
}
struct ArrayPrototypeSplice;
impl Builtin for ArrayPrototypeSplice {
    const NAME: String = BUILTIN_STRING_MEMORY.splice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::splice);
}
struct ArrayPrototypeToLocaleString;
impl Builtin for ArrayPrototypeToLocaleString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_locale_string);
}
struct ArrayPrototypeToReversed;
impl Builtin for ArrayPrototypeToReversed {
    const NAME: String = BUILTIN_STRING_MEMORY.toReversed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_reversed);
}
struct ArrayPrototypeToSorted;
impl Builtin for ArrayPrototypeToSorted {
    const NAME: String = BUILTIN_STRING_MEMORY.toSorted;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_sorted);
}
struct ArrayPrototypeToSpliced;
impl Builtin for ArrayPrototypeToSpliced {
    const NAME: String = BUILTIN_STRING_MEMORY.toSpliced;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_spliced);
}
struct ArrayPrototypeToString;
impl Builtin for ArrayPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_string);
}
impl BuiltinIntrinsic for ArrayPrototypeToString {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeToString;
}
struct ArrayPrototypeUnshift;
impl Builtin for ArrayPrototypeUnshift {
    const NAME: String = BUILTIN_STRING_MEMORY.unshift;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::unshift);
}
struct ArrayPrototypeValues;
impl Builtin for ArrayPrototypeValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::values);
}
impl BuiltinIntrinsic for ArrayPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeValues;
}
struct ArrayPrototypeWith;
impl Builtin for ArrayPrototypeWith {
    const NAME: String = BUILTIN_STRING_MEMORY.with;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::with);
}

impl ArrayPrototype {
    /// ### [23.1.3.1 Array.prototype.at ( index )](https://tc39.es/ecma262/#sec-array.prototype.at)
    fn at(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        let index = arguments.get(0);
        // 3. Let relativeIndex be ? ToIntegerOrInfinity(index).
        let relative_index = to_integer_or_infinity(agent, index)?;
        let relative_index = match relative_index {
            Number::SmallF64(_) | Number::Number(_) => {
                // Heap number or f32 here means that the value is over the
                // safe integer limit, which is necessarily >= len
                return Ok(Value::Undefined);
            }
            Number::Integer(int) => int.into_i64(),
        };
        // 4. If relativeIndex ≥ 0, then
        let k = if relative_index >= 0 {
            // a. Let k be relativeIndex.
            relative_index
        } else {
            // 5. Else,
            // a. Let k be len + relativeIndex.
            len + relative_index
        };
        // 6. If k < 0 or k ≥ len, return undefined.
        if k < 0 || k >= len {
            Ok(Value::Undefined)
        } else {
            // 7. Return ? Get(O, ! ToString(𝔽(k))).
            get(agent, o, PropertyKey::Integer(k.try_into().unwrap()))
        }
    }

    /// ### [23.1.3.2 Array.prototype.concat ( ...items )](https://tc39.es/ecma262/#sec-array.prototype.concat)
    ///
    /// This method returns an array containing the array elements of the
    /// object followed by the array elements of each argument.
    ///
    /// > Note 1: The explicit setting of the "length" property in step 6 is
    /// > intended to ensure the length is correct when the final non-empty
    /// > element of items has trailing holes or when A is not a built-in
    /// > Array.
    ///
    /// > Note 2: This method is intentionally generic; it does not require
    /// > that its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    fn concat(agent: &mut Agent, this_value: Value, items: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o, 0)?;
        // 3. Let n be 0.
        let mut n = 0;
        // 4. Prepend O to items.
        let mut items = Vec::from(items.0);
        items.insert(0, o.into_value());
        // 5. For each element E of items, do
        for e in items {
            // a. Let spreadable be ? IsConcatSpreadable(E).
            let e_is_spreadable = is_concat_spreadable(agent, e)?;
            // b. If spreadable is true, then
            if let Some(e) = e_is_spreadable {
                // i. Let len be ? LengthOfArrayLike(E).
                let len = length_of_array_like(agent, e)?;
                // ii. If n + len > 2**53 - 1, throw a TypeError exception.
                if (n + len) > SmallInteger::MAX_NUMBER {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Array overflow",
                    ));
                }
                // iii. Let k be 0.
                let mut k = 0;
                // iv. Repeat, while k < len,
                while k < len {
                    // 1. Let Pk be ! ToString(𝔽(k)).
                    let pk = PropertyKey::Integer(k.try_into().unwrap());
                    // 2. Let exists be ? HasProperty(E, Pk).
                    let exists = has_property(agent, e, pk)?;
                    // 3. If exists is true, then
                    if exists {
                        // a. Let subElement be ? Get(E, Pk).
                        let sub_element = get(agent, e, pk)?;
                        // b. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), subElement).
                        create_data_property_or_throw(
                            agent,
                            a,
                            PropertyKey::Integer(n.try_into().unwrap()),
                            sub_element,
                        )?;
                    }
                    // 4. Set n to n + 1.
                    n += 1;
                    // 5. Set k to k + 1.
                    k += 1;
                }
            } else {
                // c. Else,
                // i. NOTE: E is added as a single item rather than spread.
                // ii. If n ≥ 2**53 - 1, throw a TypeError exception.
                if n >= SmallInteger::MAX_NUMBER {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Array overflow",
                    ));
                }
                // iii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), E).
                create_data_property_or_throw(
                    agent,
                    a,
                    PropertyKey::Integer(n.try_into().unwrap()),
                    e,
                )?;
                // iv. Set n to n + 1.
                n += 1;
            }
        }
        // 6. Perform ? Set(A, "length", 𝔽(n), true).
        set(
            agent,
            a,
            BUILTIN_STRING_MEMORY.length.into(),
            Value::try_from(n).unwrap(),
            true,
        )?;
        // 7. Return A.
        Ok(a.into_value())
    }

    /// ### [23.1.3.4 Array.prototype.copyWithin ( target, start \[ , end \] )](https://tc39.es/ecma262/#sec-array.prototype.copywithin)
    ///
    /// > Note 1
    /// >
    /// > The end argument is optional. If it is not provided, the length of
    /// > the this value is used.
    ///
    /// > Note 2
    /// >
    /// > If target is negative, it is treated as length + target where length
    /// > is the length of the array. If start is negative, it is treated as
    /// > length + start. If end is negative, it is treated as length + end.
    ///
    /// > Note 3
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn copy_within(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let target = arguments.get(0);
        let start = arguments.get(1);
        let end = if arguments.len() >= 3 {
            Some(arguments.get(2))
        } else {
            None
        };
        if let (
            Value::Array(array),
            Value::Integer(target),
            Value::Integer(start),
            None | Some(Value::Undefined) | Some(Value::Integer(_)),
        ) = (this_value, target, start, end)
        {
            // Fast path: Array with integer parameters, array is trivial
            // (no descriptors). Holes can exist, we'll just copy them
            // equivalently.
            if array.is_trivial(agent) {
                let len = array.len(agent) as i64;

                let relative_target = target.into_i64();
                let to = if relative_target < 0 {
                    (len + relative_target).max(0) as isize
                } else {
                    (relative_target as u64).min(len as u64) as isize
                };

                let relative_start = start.into_i64();
                let from = if relative_start < 0 {
                    (len + relative_start).max(0) as isize
                } else {
                    (relative_start as u64).min(len as u64) as isize
                };

                let final_end = if let Some(Value::Integer(end)) = end {
                    let relative_end = end.into_i64();
                    if relative_end < 0 {
                        (len + relative_end).max(0) as isize
                    } else {
                        (relative_end as u64).min(len as u64) as isize
                    }
                } else {
                    len as isize
                };

                let count = (final_end - from).min(len as isize - to);
                let data = array.as_mut_slice(agent);
                data.copy_within((from as usize)..((from + count) as usize), to as usize);

                return Ok(array.into_value());
            }
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len: i64 = length_of_array_like(agent, o)?;
        let len_f64 = len as f64;

        // 3. Let relativeTarget be ? ToIntegerOrInfinity(target).
        let relative_target = to_integer_or_infinity(agent, target)?;

        let to = if relative_target.is_neg_infinity(agent) {
            // 4. If relativeTarget = -∞, let to be 0.
            0
        } else if relative_target.is_sign_negative(agent) {
            // 5. Else if relativeTarget < 0, let to be max(len + relativeTarget, 0).
            (len_f64 + relative_target.to_real(agent)).max(0.0) as i64
        } else {
            // 6. Else, let to be min(relativeTarget, len).
            relative_target.to_real(agent).min(len_f64) as i64
        };

        // 7. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start = to_integer_or_infinity(agent, start)?;

        let from = if relative_start.is_neg_infinity(agent) {
            // 8. If relativeStart = -∞, let from be 0.
            0
        } else if relative_start.is_sign_negative(agent) {
            // 9. Else if relativeStart < 0, let from be max(len + relativeStart, 0).
            (len_f64 + relative_start.to_real(agent)).max(0.0) as i64
        } else {
            // 10. Else, let from be min(relativeStart, len).
            relative_start.to_real(agent).min(len_f64) as i64
        };

        // 11. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
        let relative_end = if end.is_none() || end.unwrap().is_undefined() {
            len_f64
        } else {
            to_integer_or_infinity(agent, end.unwrap())?.to_real(agent)
        };
        // 12. If relativeEnd = -∞, let final be 0.
        let final_end = if relative_end == f64::NEG_INFINITY {
            0
        } else if relative_end < 0.0 {
            // 13. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
            (len_f64 + relative_end).max(0.0) as i64
        } else {
            // 14. Else, let final be min(relativeEnd, len).
            relative_end.min(len_f64) as i64
        };

        // 15. Let count be min(final - from, len - to).
        let mut count = (final_end - from).min(len - to);
        // 16. If from < to and to < from + count, then
        let (direction, from, to) = if from < to && to < from + count {
            // a. Let direction be -1.
            // b. Set from to from + count - 1.
            // c. Set to to to + count - 1.
            (-1, from + count - 1, to + count - 1)
        } else {
            // 17. Else,
            // a. Let direction be 1.
            (1, from, to)
        };
        let mut from = from;
        let mut to = to;
        // 18. Repeat, while count > 0,
        while count > 0 {
            // a. Let fromKey be ! ToString(𝔽(from)).
            let from_key = PropertyKey::Integer(from.try_into().unwrap());
            // b. Let toKey be ! ToString(𝔽(to)).
            let to_key = PropertyKey::Integer(to.try_into().unwrap());
            // c. Let fromPresent be ? HasProperty(O, fromKey).
            let from_present = has_property(agent, o, from_key)?;
            // d. If fromPresent is true, then
            if from_present {
                // i. Let fromValue be ? Get(O, fromKey).
                let from_value = get(agent, o, from_key)?;
                // ii. Perform ? Set(O, toKey, fromValue, true).
                set(agent, o, to_key, from_value, true)?;
            } else {
                // e. Else,
                // i. Assert: fromPresent is false.
                // ii. Perform ? DeletePropertyOrThrow(O, toKey).
                delete_property_or_throw(agent, o, to_key)?;
            }
            // f. Set from to from + direction.
            from += direction;
            // g. Set to to to + direction.
            to += direction;
            // h. Set count to count - 1.
            count -= 1;
        }
        // 19. Return O.
        Ok(o.into_value())
    }

    fn entries(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected this to be an object",
            ));
        };
        // 2. Return CreateArrayIterator(O, key+value).
        Ok(ArrayIterator::from_object(agent, o, CollectionIteratorKind::KeyAndValue).into_value())
    }

    /// ### [23.1.3.6 Array.prototype.every ( callbackfn \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.every)
    ///
    /// > #### Note 1
    /// >
    /// > callbackfn should be a function that accepts three arguments and returns
    /// > a value that is coercible to a Boolean value. every calls callbackfn once
    /// > for each element present in the array, in ascending order, until it finds
    /// > one where callbackfn returns false. If such an element is found, every
    /// > immediately returns false. Otherwise, every returns true. callbackfn is
    /// > called only for elements of the array which actually exist; it is not
    /// > called for missing elements of the array.
    /// >
    /// > If a thisArg parameter is provided, it will be used as the this value for
    /// > each invocation of callbackfn. If it is not provided, undefined is used
    /// > instead.
    /// >
    /// > callbackfn is called with three arguments: the value of the element, the
    /// > index of the element, and the object being traversed.
    /// >
    /// > **every** does not directly mutate the object on which it is called but
    /// > the object may be mutated by the calls to callbackfn.
    /// >
    /// > The range of elements processed by every is set before the first call to
    /// > callbackfn. Elements which are appended to the array after the call to
    /// > every begins will not be visited by callbackfn. If existing elements of
    /// > the array are changed, their value as passed to callbackfn will be the
    /// > value at the time every visits them; elements that are deleted after the
    /// > call to every begins and before being visited are not visited. every acts
    /// > like the "for all" quantifier in mathematics. In particular, for an empty
    /// > array, it returns true.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its this
    /// > value be an Array. Therefore it can be transferred to other kinds of
    /// > objects for use as a method.
    fn every(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        let callback_fn = arguments.get(0);
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not a function",
            ));
        };
        let this_arg = arguments.get(1);
        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;
                // ii. Let testResult be ToBoolean(? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »)).
                let f_k = Number::try_from(k).unwrap().into_value();
                let test_result = call_function(
                    agent,
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[k_value, f_k])),
                )?;
                let test_result = to_boolean(agent, test_result);
                // iii. If testResult is false, return false.
                if !test_result {
                    return Ok(test_result.into());
                }
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 6. Return true.
        Ok(true.into())
    }

    /// ### [23.1.3.7 Array.prototype.fill ( value \[ , start \[ , end \] \] )](https://tc39.es/ecma262/#sec-array.prototype.fill)
    ///
    /// > #### Note 1
    /// >
    /// > The start argument is optional. If it is not provided, +0𝔽 is used.
    /// >
    /// > The end argument is optional. If it is not provided, the length of
    /// > the this value is used.
    ///
    /// > #### Note 2
    /// >
    /// > If start is negative, it is treated as length + start where length is
    /// > the length of the array. If end is negative, it is treated as
    /// > length + end.
    ///
    /// > #### Note 3
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn fill(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let value = arguments.get(0);
        let start = arguments.get(1);
        let end = arguments.get(2);
        if let (
            Value::Array(array),
            Value::Undefined | Value::Integer(_),
            Value::Undefined | Value::Integer(_),
        ) = (this_value, start, end)
        {
            // Fast path: If the array is simple (no descriptors) and dense (no
            // holes) then we can write directly into the backing memory.
            if array.is_simple(agent) && array.is_dense(agent) {
                let len = array.len(agent) as usize;

                let relative_start = if let Value::Integer(start) = start {
                    let start = start.into_i64();
                    if start < 0 {
                        (len as i64 + start).max(0) as usize
                    } else {
                        (start as usize).min(len)
                    }
                } else {
                    0
                };

                let k = relative_start.min(len);

                let final_end = if let Value::Integer(end) = end {
                    let relative_end = end.into_i64();
                    if relative_end < 0 {
                        (len as i64 + relative_end).max(0) as usize
                    } else {
                        (relative_end as usize).min(len)
                    }
                } else {
                    len
                };

                let data = array.as_mut_slice(agent);
                data[k..final_end].fill(Some(value));
                return Ok(value.into_value());
            }
        };
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start = to_integer_or_infinity(agent, start)?.to_real(agent);

        // 4. If relativeStart = -∞, let k be 0.
        let mut k = if relative_start == f64::NEG_INFINITY {
            0
        } else if relative_start < 0.0 {
            // 5. Else if relativeStart < 0, let k be max(len + relativeStart, 0).
            (len + relative_start as i64).max(0)
        } else {
            // 6. Else, let k be min(relativeStart, len).
            len.min(relative_start as i64)
        };

        // 7. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
        let final_end = if end.is_undefined() {
            len
        } else {
            let relative_end = to_integer_or_infinity(agent, end)?.to_real(agent);
            // 8. If relativeEnd = -∞, let final be 0.
            if relative_end == f64::NEG_INFINITY {
                0
            } else if relative_end < 0.0 {
                // 9. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
                (len + relative_end as i64).max(0)
            } else {
                // 10. Else, let final be min(relativeEnd, len).
                len.min(relative_end as i64)
            }
        };

        // 11. Repeat, while k < final,
        while k < final_end {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Perform ? Set(O, Pk, value, true).
            set(agent, o, pk, value, true)?;
            // c. Set k to k + 1.
            k += 1;
        }
        // 12. Return O.
        Ok(o.into_value())
    }

    /// ### [23.1.3.8 Array.prototype.filter ( callbackfn \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.filter)
    ///
    /// > #### Note 1
    /// > `callbackfn` should be a function that accepts three arguments and
    /// > returns a value that is coercible to a Boolean value. **filter**
    /// > calls `callbackfn` once for each element in the array, in ascending
    /// > order, and constructs a new array of all the values for which
    /// > `callbackfn` returns **true**. `callbackfn` is called only for
    /// > elements of the array which actually exist; it is not called for
    /// > missing elements of the array.
    /// >
    /// > If a `thisArg` parameter is provided, it will be used as the **this**
    /// > value for each invocation of `callbackfn`. If it is not provided,
    /// > **undefined** is used instead.
    /// >
    /// > `callbackfn` is called with three arguments: the value of the
    /// > element, the index of the element, and the object being traversed.
    /// >
    /// > **filter** does not directly mutate the object on which it is called
    /// > but the object may be mutated by the calls to `callbackfn`.
    /// >
    /// > The range of elements processed by **filter** is set before the first
    /// > call to `callbackfn`. Elements which are appended to the array after
    /// > the call to **filter** begins will not be visited by `callbackfn`. If
    /// > existing elements of the array are changed their value as passed to
    /// > `callbackfn` will be the value at the time **filter** visits them;
    /// > elements that are deleted after the call to **filter** begins and
    /// > before being visited are not visited.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > **this** value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn filter(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let callback_fn = arguments.get(0);
        let this_arg = arguments.get(1);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not callable",
            ));
        };
        // 4. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o, 0)?;
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Let to be 0.
        let mut to: u32 = 0;
        // 7. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::from(SmallInteger::try_from(k).unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;
                // ii. Let selected be ToBoolean(? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »)).
                let result = call_function(
                    agent,
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[
                        k_value,
                        k.try_into().unwrap(),
                        o.into_value(),
                    ])),
                )?;
                let selected = to_boolean(agent, result);
                // iii. If selected is true, then
                if selected {
                    // 1. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(to)), kValue).
                    create_data_property_or_throw(agent, a, to.into(), k_value)?;
                    // 2. Set to to to + 1.
                    to += 1;
                }
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 8. Return A.
        Ok(a.into_value())
    }

    /// ### [23.1.3.9 Array.prototype.find ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.find)
    ///
    /// > #### Note 1
    /// >
    /// > This method calls predicate once for each element of the array, in
    /// > ascending index order, until it finds one where predicate returns a
    /// > value that coerces to true. If such an element is found, find
    /// > immediately returns that element value. Otherwise, find returns
    /// > undefined.
    /// >
    /// > See FindViaPredicate for additional information.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn find(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        let predicate = arguments.get(0);
        let this_arg = arguments.get(1);
        // 3. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, true, predicate, this_arg)?;
        // 4. Return findRec.[[Value]].
        Ok(find_rec.1)
    }

    /// ### [23.1.3.10 Array.prototype.findIndex ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.findindex)
    ///
    /// > #### Note 1
    /// >
    /// > This method calls predicate once for each element of the array, in
    /// > ascending index order, until it finds one where predicate returns a
    /// > value that coerces to true. If such an element is found, findIndex
    /// > immediately returns the index of that element value. Otherwise,
    /// > findIndex returns -1.
    /// >
    /// > See FindViaPredicate for additional information.
    /// >
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn find_index(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        let predicate = arguments.get(0);
        let this_arg = arguments.get(1);
        // 3. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, true, predicate, this_arg)?;
        // 4. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into_value())
    }

    /// ### [23.1.3.11 Array.prototype.findLast ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.findlast)
    fn find_last(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        let predicate = arguments.get(0);
        let this_arg = arguments.get(1);
        // 3. Let findRec be ? FindViaPredicate(O, len, descending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg)?;
        // 4. Return findRec.[[Value]].
        Ok(find_rec.1)
    }

    /// ### [23.1.3.12 Array.prototype.findLastIndex ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.findlastindex)
    fn find_last_index(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        let predicate = arguments.get(0);
        let this_arg = arguments.get(1);
        // 3. Let findRec be ? FindViaPredicate(O, len, descending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg)?;
        // 4. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into_value())
    }

    /// ### [23.1.3.13 Array.prototype.flat ( \[ depth \] )]()
    fn flat(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let depth = arguments.get(0);
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let sourceLen be ? LengthOfArrayLike(O).
        let source_len = length_of_array_like(agent, o)? as usize;
        // 3. Let depthNum be 1.
        let mut depth_num = 1;
        // 4. If depth is not undefined, then
        if !depth.is_undefined() {
            // a. Set depthNum to ? ToIntegerOrInfinity(depth).
            depth_num = to_integer_or_infinity(agent, depth)?.into_i64(agent);
        }
        // b. If depthNum < 0, set depthNum to 0.
        if depth_num < 0 {
            depth_num = 0;
        }
        // 5. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o, 0)?;
        // 6. Perform ? FlattenIntoArray(A, O, sourceLen, 0, depthNum).
        flatten_into_array(
            agent,
            a,
            o,
            source_len,
            0,
            Some(depth_num as usize),
            None,
            None,
        )?;
        // 7. Return A.
        Ok(a.into_value())
    }

    /// ### [23.1.3.14 Array.prototype.flatMap ( mapperFunction \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.flatmap)
    fn flat_map(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let mapper_function = arguments.get(0);
        let this_arg = arguments.get(1);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let sourceLen be ? LengthOfArrayLike(O).
        let source_len = length_of_array_like(agent, o)? as usize;
        // 3. If IsCallable(mapperFunction) is false, throw a TypeError exception.
        let Some(mapper_function) = is_callable(mapper_function) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Mapper function is not callable",
            ));
        };
        // 4. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o, 0)?;
        // 5. Perform ? FlattenIntoArray(A, O, sourceLen, 0, 1, mapperFunction, thisArg).
        flatten_into_array(
            agent,
            a,
            o,
            source_len,
            0,
            Some(1),
            Some(mapper_function),
            Some(this_arg),
        )?;
        // 6. Return A.
        Ok(a.into_value())
    }

    /// ### [23.1.3.15 Array.prototype.forEach ( callbackfn \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.foreach)
    ///
    /// > #### Note 1
    /// >
    /// > callbackfn should be a function that accepts three arguments.
    /// > forEach calls callbackfn once for each element present in the
    /// > array, in ascending order. callbackfn is called only for elements
    /// > of the array which actually exist; it is not called for missing
    /// > elements of the array.
    /// >
    /// > If a thisArg parameter is provided, it will be used as the this
    /// > value for each invocation of callbackfn. If it is not provided,
    /// > undefined is used instead.
    /// >
    /// > callbackfn is called with three arguments: the value of the
    /// > element, the index of the element, and the object being
    /// > traversed.
    /// >
    /// > forEach does not directly mutate the object on which it is called
    /// > but the object may be mutated by the calls to callbackfn.
    /// >
    /// > The range of elements processed by forEach is set before the
    /// > first call to callbackfn. Elements which are appended to the
    /// > array after the call to forEach begins will not be visited by
    /// > callbackfn. If existing elements of the array are changed, their
    /// > value as passed to callbackfn will be the value at the time
    /// > forEach visits them; elements that are deleted after the call to
    /// > forEach begins and before being visited are not visited.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that
    /// > its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    fn for_each(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;

        let callback_fn = arguments.get(0);

        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
            ));
        };

        let this_arg = arguments.get(0);
        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;
                // ii. Perform ? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »).
                call_function(
                    agent,
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[
                        k_value,
                        k.try_into().unwrap(),
                        o.into_value(),
                    ])),
                )?;
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 6. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [23.1.3.16 Array.prototype.includes ( searchElement \[ , fromIndex \] )](https://tc39.es/ecma262/#sec-array.prototype.includes)
    ///
    /// > #### Note 1
    /// >
    /// > This method compares searchElement to the elements of the array,
    /// > in ascending order, using the SameValueZero algorithm, and if
    /// > found at any position, returns true; otherwise, it returns false.
    /// >
    /// > The optional second argument fromIndex defaults to +0𝔽 (i.e. the
    /// > whole array is searched). If it is greater than or equal to the
    /// > length of the array, false is returned, i.e. the array will not
    /// > be searched. If it is less than -0𝔽, it is used as the offset
    /// > from the end of the array to compute fromIndex. If the computed
    /// > index is less than or equal to +0𝔽, the whole array will be
    /// > searched.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that
    /// > its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    ///
    /// > #### Note 3
    /// >
    /// > This method intentionally differs from the similar indexOf method
    /// > in two ways. First, it uses the SameValueZero algorithm, instead
    /// > of IsStrictlyEqual, allowing it to detect NaN array elements.
    /// > Second, it does not skip missing array elements, instead treating
    /// > them as undefined.
    fn includes(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let search_element = arguments.get(0);
        let from_index = arguments.get(1);
        if let (Value::Array(array), Value::Undefined | Value::Integer(_)) =
            (this_value, from_index)
        {
            let len = array.len(agent);
            if len == 0 {
                return Ok(false.into());
            }
            let k = if let Value::Integer(n) = from_index {
                let n = n.into_i64();
                if n >= 0 {
                    n as usize
                } else {
                    let result = len as i64 + n;
                    if result < 0 {
                        0
                    } else {
                        result as usize
                    }
                }
            } else {
                0
            };
            let data = &array.as_slice(agent)[k..];
            let mut found_hole = false;
            for element_k in data {
                if let Some(element_k) = element_k {
                    if same_value_zero(agent, search_element, *element_k) {
                        return Ok(true.into());
                    }
                } else {
                    // A hole would require looking through the prototype
                    // chain. We're not going to do that.
                    found_hole = true;
                    break;
                }
            }
            if !found_hole {
                // No holes found so we can trust the result.
                return Ok(false.into());
            }
        };
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If len = 0, return false.
        if len == 0 {
            return Ok(false.into());
        }
        // 4. Let n be ? ToIntegerOrInfinity(fromIndex).
        let n = to_integer_or_infinity(agent, from_index)?;
        // 5. Assert: If fromIndex is undefined, then n is 0.
        assert_eq!(from_index.is_undefined(), n.is_pos_zero(agent));
        // 6. If n = +∞, return false.
        let n = if n.is_pos_infinity(agent) {
            return Ok(false.into());
        } else if n.is_neg_infinity(agent) {
            // 7. Else if n = -∞, set n to 0.
            0
        } else {
            n.into_i64(agent)
        };

        // 8. If n ≥ 0, then
        let mut k = if n >= 0 {
            // a. Let k be n.
            n
        } else {
            // 9. Else,
            // a. Let k be len + n.
            let k = len + n;
            // b. If k < 0, set k to 0.
            if k < 0 {
                0
            } else {
                k
            }
        };
        // 10. Repeat, while k < len,
        while k < len {
            // a. Let elementK be ? Get(O, ! ToString(𝔽(k))).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            let element_k = get(agent, o, pk)?;
            // b. If SameValueZero(searchElement, elementK) is true, return true.
            if same_value_zero(agent, search_element, element_k) {
                return Ok(true.into());
            }
            // c. Set k to k + 1.
            k += 1;
        }
        // 11. Return false.
        Ok(false.into())
    }

    /// ### [23.1.3.17 Array.prototype.indexOf ( searchElement \[ , fromIndex \] )](https://tc39.es/ecma262/#sec-array.prototype.indexof)
    ///
    /// This method compares searchElement to the elements of the array, in
    /// ascending order, using the IsStrictlyEqual algorithm, and if found
    /// at one or more indices, returns the smallest such index; otherwise,
    /// it returns -1𝔽.
    ///
    /// > #### Note 1
    /// >
    /// > The optional second argument fromIndex defaults to +0𝔽 (i.e. the
    /// > whole array is searched). If it is greater than or equal to the
    /// > length of the array, -1𝔽 is returned, i.e. the array will not be
    /// > searched. If it is less than -0𝔽, it is used as the offset from
    /// > the end of the array to compute fromIndex. If the computed index
    /// > is less than or equal to +0𝔽, the whole array will be searched.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that
    /// > its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    fn index_of(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let search_element = arguments.get(0);
        let from_index = arguments.get(1);
        if let (Value::Array(array), Value::Undefined | Value::Integer(_)) =
            (this_value, from_index)
        {
            let len = array.len(agent);
            if len == 0 {
                return Ok((-1).into());
            }
            let k = if let Value::Integer(n) = from_index {
                let n = n.into_i64();
                if n >= 0 {
                    n as usize
                } else {
                    let result = len as i64 + n;
                    if result < 0 {
                        0
                    } else {
                        result as usize
                    }
                }
            } else {
                0
            };
            let data = &array.as_slice(agent)[k..];
            let mut found_hole = false;
            for (index, element_k) in data.iter().enumerate() {
                if let Some(element_k) = element_k {
                    if is_strictly_equal(agent, search_element, *element_k) {
                        return Ok((k as u32 + index as u32).into());
                    }
                } else {
                    // A hole would require looking through the prototype
                    // chain. We're not going to do that.
                    found_hole = true;
                    break;
                }
            }
            if !found_hole {
                // No holes found so we can trust the result.
                return Ok((-1).into());
            }
        };
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If len = 0, return -1𝔽.
        if len == 0 {
            return Ok((-1).into());
        }
        // 4. Let n be ? ToIntegerOrInfinity(fromIndex).
        let n = to_integer_or_infinity(agent, from_index)?;
        // 5. Assert: If fromIndex is undefined, then n is 0.
        assert_eq!(from_index.is_undefined(), n.is_pos_zero(agent));
        // 6. If n = +∞, return -1𝔽.
        let n = if n.is_pos_infinity(agent) {
            return Ok((-1).into());
        } else if n.is_neg_infinity(agent) {
            // 7. Else if n = -∞, set n to 0.
            0
        } else {
            n.into_i64(agent)
        };

        // 8. If n ≥ 0, then
        let mut k = if n >= 0 {
            // a. Let k be n.
            n
        } else {
            // 9. Else,
            // a. Let k be len + n.
            let k = len + n;
            // b. If k < 0, set k to 0.
            if k < 0 {
                0
            } else {
                k
            }
        };
        // 10. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let elementK be ? Get(O, Pk).
                let element_k = get(agent, o, pk)?;
                // ii. If IsStrictlyEqual(searchElement, elementK) is true, return 𝔽(k).
                if is_strictly_equal(agent, search_element, element_k) {
                    return Ok(k.try_into().unwrap());
                }
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 11. Return -1𝔽.
        Ok((-1).into())
    }

    /// ### [23.1.3.18 Array.prototype.join ( separator )](https://tc39.es/ecma262/#sec-array.prototype.join)
    ///
    /// This method converts the elements of the array to Strings, and then
    /// concatenates these Strings, separated by occurrences of the separator.
    /// If no separator is provided, a single comma is used as the separator.
    fn join(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let separator = arguments.get(0);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        if len == 0 {
            return Ok(String::EMPTY_STRING.into_value());
        }
        let len = len as usize;
        // 3. If separator is undefined, let sep be ",".
        let separator = if separator.is_undefined() {
            SmallString::from_str_unchecked(",").into()
        } else {
            // 4. Else, let sep be ? ToString(separator).
            to_string(agent, separator)?
        };
        // 5. Let R be the empty String.
        let mut r = std::string::String::with_capacity(len * 10);
        // 6. Let k be 0.
        // 7. Repeat, while k < len,
        // b. Let element be ? Get(O, ! ToString(𝔽(k))).
        {
            let element = get(agent, o, 0.into())?;
            // c. If element is neither undefined nor null, then
            if !element.is_undefined() && !element.is_null() {
                // i. Let S be ? ToString(element).
                let s = to_string(agent, element)?;
                // ii. Set R to the string-concatenation of R and S.
                r.push_str(s.as_str(agent));
            }
        }
        for k in 1..len {
            // a. If k > 0, set R to the string-concatenation of R and sep.
            r.push_str(separator.as_str(agent));
            // b. Let element be ? Get(O, ! ToString(𝔽(k))).
            let element = get(agent, o, SmallInteger::try_from(k as u64).unwrap().into())?;
            // c. If element is neither undefined nor null, then
            if !element.is_undefined() && !element.is_null() {
                // i. Let S be ? ToString(element).
                let s = to_string(agent, element)?;
                // ii. Set R to the string-concatenation of R and S.
                r.push_str(s.as_str(agent));
            }
            // d. Set k to k + 1.
        }
        // 8. Return R.
        Ok(Value::from_string(agent, r).into_value())
    }

    fn keys(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected this to be an object",
            ));
        };
        // 2. Return CreateArrayIterator(O, key).
        Ok(ArrayIterator::from_object(agent, o, CollectionIteratorKind::Key).into_value())
    }

    /// ### [23.1.3.20 Array.prototype.lastIndexOf ( searchElement \[ , fromIndex \] )](https://tc39.es/ecma262/#sec-array.prototype.lastindexof)
    ///
    /// > Note 1
    /// >
    /// > This method compares searchElement to the elements of the array in
    /// > descending order using the IsStrictlyEqual algorithm, and if found at
    /// > one or more indices, returns the largest such index; otherwise, it
    /// > returns -1𝔽.
    /// >
    /// > The optional second argument fromIndex defaults to the array's length
    /// > minus one (i.e. the whole array is searched). If it is greater than
    /// > or equal to the length of the array, the whole array will be
    /// > searched. If it is less than -0𝔽, it is used as the offset from the
    /// > end of the array to compute fromIndex. If the computed index is less
    /// > than or equal to +0𝔽, -1𝔽 is returned.
    ///
    /// > Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn last_index_of(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let search_element = arguments.get(0);
        let from_index = if arguments.len() > 1 {
            Some(arguments.get(1))
        } else {
            None
        };
        if let (Value::Array(array), None | Some(Value::Undefined) | Some(Value::Integer(_))) =
            (this_value, from_index)
        {
            let len = array.len(agent);
            if len == 0 {
                return Ok((-1).into());
            }
            let last = (len - 1) as usize;
            let k = if let Some(Value::Integer(n)) = from_index {
                let n = n.into_i64();
                if n >= 0 {
                    (n as usize).min(last)
                } else {
                    let result = len as i64 + n;
                    if result < 0 {
                        0
                    } else {
                        result as usize
                    }
                }
            } else if from_index == Some(Value::Undefined) {
                0
            } else {
                last
            };
            let data = &array.as_slice(agent)[..=k];
            let mut found_hole = false;
            for (index, element_k) in data.iter().enumerate().rev() {
                if let Some(element_k) = element_k {
                    if is_strictly_equal(agent, search_element, *element_k) {
                        return Ok((index as u32).into());
                    }
                } else {
                    // A hole would require looking through the prototype
                    // chain. We're not going to do that.
                    found_hole = true;
                    break;
                }
            }
            if !found_hole {
                // No holes found so we can trust the result.
                return Ok((-1).into());
            }
        };
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If len = 0, return -1𝔽.
        if len == 0 {
            return Ok((-1).into());
        }
        // 4. If fromIndex is present, let n be ? ToIntegerOrInfinity(fromIndex); else let n be len - 1.
        let n = if let Some(from_index) = from_index {
            to_integer_or_infinity(agent, from_index)?.into_f64(agent)
        } else {
            (len - 1) as f64
        };

        // 5. If n = -∞, return -1𝔽.
        if n == f64::NEG_INFINITY {
            return Ok((-1).into());
        }
        // 6. If n ≥ 0, then
        let mut k = if n >= 0.0 {
            // a. Let k be min(n, len - 1).
            n.min(len as f64 - 1.0) as i64
        } else {
            // 7. Else,
            // a. Let k be len + n.
            (len as f64 + n) as i64
        };
        // 8. Repeat, while k ≥ 0,
        while k >= 0 {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let elementK be ? Get(O, Pk).
                let element_k = get(agent, o, pk)?;
                // ii. If IsStrictlyEqual(searchElement, elementK) is true, return 𝔽(k).
                if is_strictly_equal(agent, search_element, element_k) {
                    return Ok(k.try_into().unwrap());
                }
            }
            // d. Set k to k - 1.
            k -= 1;
        }
        // 9. Return -1𝔽.
        Ok((-1).into())
    }

    /// ### [23.1.3.21 Array.prototype.map ( callbackfn \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.map)
    ///
    /// > #### Note 1
    /// >
    /// > callbackfn should be a function that accepts three arguments. map
    /// > calls callbackfn once for each element in the array, in ascending
    /// > order, and constructs a new Array from the results. callbackfn is
    /// > called only for elements of the array which actually exist; it is
    /// > not called for missing elements of the array.
    /// >
    /// > If a thisArg parameter is provided, it will be used as the this value
    /// > for each invocation of callbackfn. If it is not provided, undefined
    /// > is used instead.
    /// >
    /// > callbackfn is called with three arguments: the value of the element,
    /// > the index of the element, and the object being traversed.
    /// >
    /// > map does not directly mutate the object on which it is called but the
    /// > object may be mutated by the calls to callbackfn.
    /// >
    /// > The range of elements processed by map is set before the first call
    /// > to callbackfn. Elements which are appended to the array after the
    /// > call to map begins will not be visited by callbackfn. If existing
    /// > elements of the array are changed, their value as passed to
    /// > callbackfn will be the value at the time map visits them; elements
    /// > that are deleted after the call to map begins and before being
    /// > visited are not visited.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn map(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let callback_fn = arguments.get(0);
        let this_arg = arguments.get(1);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
            ));
        };
        // 4. Let A be ? ArraySpeciesCreate(O, len).
        let a = array_species_create(agent, o, len as usize)?;
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;
                // ii. Let mappedValue be ? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »).
                let mapped_value = call_function(
                    agent,
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[
                        k_value,
                        k.try_into().unwrap(),
                        o.into_value(),
                    ])),
                )?;
                // iii. Perform ? CreateDataPropertyOrThrow(A, Pk, mappedValue).
                create_data_property_or_throw(agent, a, pk, mapped_value)?;
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 7. Return A.
        Ok(a.into_value())
    }

    /// ### [23.1.3.22 Array.prototype.pop ( )](https://tc39.es/ecma262/#sec-array.prototype.pop)
    ///
    /// > #### Note 1
    /// >
    /// > This method removes the last element of the array and returns it.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that
    /// > its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    fn pop(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        if let Value::Array(array) = this_value {
            // Fast path: Trivial (no descriptors) array means mutating
            // elements is direct.
            if array.is_trivial(agent) {
                let len = array.len(agent);
                let length_writable = agent[array].elements.len_writable;
                if len == 0 {
                    return if !length_writable {
                        Err(agent.throw_exception_with_static_message(
                            ExceptionType::TypeError,
                            "Could not set property.",
                        ))
                    } else {
                        Ok(Value::Undefined)
                    };
                }
                let element = array.as_mut_slice(agent).last_mut().unwrap();
                if let Some(last_element) = *element {
                    // Empty the last value.
                    *element = None;
                    if length_writable {
                        agent[array].elements.len -= 1;
                    } else {
                        return Err(agent.throw_exception_with_static_message(
                            ExceptionType::TypeError,
                            "Could not set property.",
                        ));
                    }
                    return Ok(last_element);
                }
                // Last element was a hole; this means we'd need to look into
                // the prototype chain. We're not going to do that.
            }
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If len = 0, then
        if len == 0 {
            // a. Perform ? Set(O, "length", +0𝔽, true).
            set(
                agent,
                o,
                BUILTIN_STRING_MEMORY.length.into(),
                0.into(),
                true,
            )?;
            // b. Return undefined.
            Ok(Value::Undefined)
        } else {
            // 4. Else,
            // a. Assert: len > 0.
            assert!(len > 0);
            // b. Let newLen be 𝔽(len - 1).
            let new_len = len - 1;
            // c. Let index be ! ToString(newLen).
            let index = PropertyKey::Integer(new_len.try_into().unwrap());
            // d. Let element be ? Get(O, index).
            let element = get(agent, o, index)?;
            // e. Perform ? DeletePropertyOrThrow(O, index).
            delete_property_or_throw(agent, o, index)?;
            // f. Perform ? Set(O, "length", newLen, true).
            set(
                agent,
                o,
                BUILTIN_STRING_MEMORY.length.into(),
                new_len.try_into().unwrap(),
                true,
            )?;
            // g. Return element.
            Ok(element)
        }
    }

    /// #### [23.1.3.23 Array.prototype.push ( ...items )](https://tc39.es/ecma262/#sec-array.prototype.push)
    ///
    /// > Note 1
    /// >
    /// > This method appends the arguments to the end of the array, in the
    /// > order in which they appear. It returns the new length of the
    /// > array.
    ///
    /// > Note 2
    /// >
    /// > This method is intentionally generic; it does not require that
    /// > its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    fn push(agent: &mut Agent, this_value: Value, items: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let mut len = length_of_array_like(agent, o)?;
        // 3. Let argCount be the number of elements in items.
        let arg_count = items.len();
        // 4. If len + argCount > 2**53 - 1, throw a TypeError exception.
        if (len + arg_count as i64) > SmallInteger::MAX_NUMBER {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length overflow",
            ));
        }
        if let Object::Array(array) = o {
            // Fast path: Reserve enough room in the array.
            let Heap {
                arrays, elements, ..
            } = &mut agent.heap;
            arrays[array]
                .elements
                .reserve(elements, len as u32 + arg_count as u32);
        }
        // 5. For each element E of items, do
        for e in items.iter() {
            // a. Perform ? Set(O, ! ToString(𝔽(len)), E, true).
            set(
                agent,
                o,
                PropertyKey::Integer(len.try_into().unwrap()),
                *e,
                true,
            )?;
            // b. Set len to len + 1.
            len += 1;
        }
        // 6. Perform ? Set(O, "length", 𝔽(len), true).
        let len: Value = len.try_into().unwrap();
        set(agent, o, BUILTIN_STRING_MEMORY.length.into(), len, true)?;

        // 7. Return 𝔽(len).
        Ok(len)
    }

    /// #### [23.1.3.24 Array.prototype.reduce ( callbackfn \[ , initialValue \] )](https://tc39.es/ecma262/indexed-collections.html#sec-array.prototype.reduce)
    ///
    /// > Note 1
    /// >
    /// > callbackfn should be a function that takes four arguments. reduce
    /// > calls the callback, as a function, once for each element after the
    /// > first element present in the array, in ascending order.
    /// >
    /// > callbackfn is called with four arguments: the previousValue (value
    /// > from the previous call to callbackfn), the currentValue (value of the
    /// > current element), the currentIndex, and the object being traversed.
    /// > The first time that callback is called, the previousValue and
    /// > currentValue can be one of two values. If an initialValue was
    /// > supplied in the call to reduce, then previousValue will be
    /// > initialValue and currentValue will be the first value in the array.
    /// > If no initialValue was supplied, then previousValue will be the first
    /// > value in the array and currentValue will be the second. It is a
    /// > TypeError if the array contains no elements and initialValue is not
    /// > provided.
    /// >
    /// > reduce does not directly mutate the object on which it is called but
    /// > the object may be mutated by the calls to callbackfn.
    /// >
    /// > The range of elements processed by reduce is set before the first
    /// > call to callbackfn. Elements that are appended to the array after the
    /// > call to reduce begins will not be visited by callbackfn. If existing
    /// > elements of the array are changed, their value as passed to callbackfn
    /// > will be the value at the time reduce visits them; elements that are
    /// > deleted after the call to reduce begins and before being visited are
    /// > not visited.
    ///
    /// > Note 2
    /// >
    /// > This method is intentionally generic; it does not require that
    /// > its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    fn reduce(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let callback_fn = arguments.get(0);
        let initial_value = if arguments.len() >= 2 {
            Some(arguments.get(1))
        } else {
            None
        };

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;

        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
            ));
        };

        // 4. If len = 0 and initialValue is not present, throw a TypeError exception.
        if len == 0 && initial_value.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length is 0 and no initial value provided",
            ));
        }

        // 5. Let k be 0.
        let mut k = 0;
        // 6. Let accumulator be undefined.
        // 7. If initialValue is present,
        // a. Set accumulator to initialValue.
        let mut accumulator = initial_value.unwrap_or(Value::Undefined);

        // 8. Else,
        if initial_value.is_none() {
            // a. Let kPresent be false.
            let mut k_present = false;

            // b. Repeat, while kPresent is false and k < len,
            while !k_present && k < len {
                // i. Let Pk be ! ToString(𝔽(k)).
                let pk = PropertyKey::Integer(k.try_into().unwrap());

                // ii. Set kPresent to ? HasProperty(O, Pk).
                k_present = has_property(agent, o, pk)?;

                // iii. If kPresent is true, then
                if k_present {
                    // 1. Set accumulator to ? Get(O, Pk).
                    accumulator = get(agent, o, pk)?;
                }

                // iv. Set k to k + 1.
                k += 1;
            }

            // c. If kPresent is false, throw a TypeError exception.
            if !k_present {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Array length is 0 and no initial value provided",
                ));
            }
        }

        // 9. Repeat, while k < len,
        while k < len {
            let k_int = k.try_into().unwrap();
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k_int);

            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;

            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;

                // ii. Set accumulator to ? Call(callbackfn, undefined, « accumulator, kValue, 𝔽(k), O »).
                accumulator = call_function(
                    agent,
                    callback_fn,
                    Value::Undefined,
                    Some(ArgumentsList(&[
                        accumulator,
                        k_value,
                        Number::from(k_int).into_value(),
                        o.into_value(),
                    ])),
                )?;
            }

            // d. Set k to k + 1.
            k += 1;
        }

        // 10. Return accumulator.
        Ok(accumulator)
    }

    /// ### [23.1.3.25 Array.prototype.reduceRight ( callbackfn \[ , initialValue \] )](https://tc39.es/ecma262/#sec-array.prototype.reduceright)
    ///
    /// > Note 1
    /// >
    /// > callbackfn should be a function that takes four arguments.
    /// > reduceRight calls the callback, as a function, once for each element
    /// > after the first element present in the array, in descending order.
    /// >
    /// > callbackfn is called with four arguments: the previousValue (value
    /// > from the previous call to callbackfn), the currentValue (value of the
    /// > current element), the currentIndex, and the object being traversed.
    /// > The first time the function is called, the previousValue and
    /// > currentValue can be one of two values. If an initialValue was
    /// > supplied in the call to reduceRight, then previousValue will be
    /// > initialValue and currentValue will be the last value in the array. If
    /// > no initialValue was supplied, then previousValue will be the last
    /// > value in the array and currentValue will be the second-to-last value.
    /// > It is a TypeError if the array contains no elements and initialValue
    /// > is not provided.
    /// >
    /// > reduceRight does not directly mutate the object on which it is called
    /// > but the object may be mutated by the calls to callbackfn.
    /// >
    /// > The range of elements processed by reduceRight is set before the
    /// > first call to callbackfn. Elements that are appended to the array
    /// > after the call to reduceRight begins will not be visited by
    /// > callbackfn. If existing elements of the array are changed by
    /// > callbackfn, their value as passed to callbackfn will be the value at
    /// > the time reduceRight visits them; elements that are deleted after the
    /// > call to reduceRight begins and before being visited are not visited.
    ///
    /// > Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn reduce_right(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let callback_fn = arguments.get(0);
        let initial_value = if arguments.len() >= 2 {
            Some(arguments.get(1))
        } else {
            None
        };

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;

        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;

        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
            ));
        };

        // 4. If len = 0 and initialValue is not present, throw a TypeError exception.
        if len == 0 && initial_value.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length is 0 and no initial value provided",
            ));
        }

        // 5. Let k be len - 1.
        let mut k = len - 1;
        // 6. Let accumulator be undefined.
        // 7. If initialValue is present, then
        // a. Set accumulator to initialValue.
        let mut accumulator = initial_value.unwrap_or(Value::Undefined);

        // 8. Else,
        if initial_value.is_none() {
            // a. Let kPresent be false.
            let mut k_present = false;

            // b. Repeat, while kPresent is false and k ≥ 0,
            while !k_present && k >= 0 {
                // i. Let Pk be ! ToString(𝔽(k)).
                let pk = PropertyKey::try_from(k).unwrap();

                // ii. Set kPresent to ? HasProperty(O, Pk).
                k_present = has_property(agent, o, pk)?;

                // iii. If kPresent is true, then
                if k_present {
                    // 1. Set accumulator to ? Get(O, Pk).
                    accumulator = get(agent, o, pk)?;
                }

                // iv. Set k to k - 1.
                k -= 1;
            }

            // c. If kPresent is false, throw a TypeError exception.
            if !k_present {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Array length is 0 and no initial value provided",
                ));
            }
        }

        // 9. Repeat, while k ≥ 0,
        while k >= 0 {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();

            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;

            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;

                // ii. Set accumulator to ? Call(callbackfn, undefined, « accumulator, kValue, 𝔽(k), O »).
                accumulator = call_function(
                    agent,
                    callback_fn,
                    Value::Undefined,
                    Some(ArgumentsList(&[
                        accumulator,
                        k_value,
                        Number::try_from(k).unwrap().into(),
                        o.into_value(),
                    ])),
                )?;
            }

            // d. Set k to k - 1.
            k -= 1;
        }

        // 10. Return accumulator.
        Ok(accumulator)
    }

    fn reverse(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        if let Value::Array(array) = this_value {
            // Fast path: Array is dense and contains no descriptors. No JS
            // functions can thus be called by shift.
            if array.is_trivial(agent) && array.is_dense(agent) {
                array.as_mut_slice(agent).reverse();
                return Ok(array.into_value());
            }
        }

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. Let middle be floor(len / 2).
        let middle = len / 2;
        // 4. Let lower be 0.
        let mut lower: i64 = 0;
        // 5. Repeat, while lower ≠ middle,
        while lower != middle {
            //    a. Let upper be len - lower - 1.
            let upper = len - lower - 1;
            //    b. Let upperP be ! ToString(𝔽(upper)).
            let upper_p = PropertyKey::Integer(upper.try_into().unwrap());
            //    c. Let lowerP be ! ToString(𝔽(lower)).
            let lower_p = PropertyKey::Integer(lower.try_into().unwrap());
            //    d. Let lowerExists be ? HasProperty(O, lowerP).
            //    e. If lowerExists is true, then
            //       i. Let lowerValue be ? Get(O, lowerP).
            let lower_exists = has_property(agent, o, lower_p)?;
            //    f. Let upperExists be ? HasProperty(O, upperP).
            //    g. If upperExists is true, then
            //       i. Let upperValue be ? Get(O, upperP).
            let upper_exists = has_property(agent, o, upper_p)?;

            //    h. If lowerExists is true and upperExists is true, then
            if lower_exists && upper_exists {
                //       i. Perform ? Set(O, lowerP, upperValue, true).
                //       ii. Perform ? Set(O, upperP, lowerValue, true).
                let lower_value = get(agent, o, lower_p)?;
                let upper_value = get(agent, o, upper_p)?;
                set(agent, o, lower_p, upper_value, true)?;
                set(agent, o, upper_p, lower_value, true)?;
            }
            //    i. Else if lowerExists is false and upperExists is true, then
            else if !lower_exists && upper_exists {
                //       i. Perform ? Set(O, lowerP, upperValue, true).
                //       ii. Perform ? DeletePropertyOrThrow(O, upperP).
                let upper_value = get(agent, o, upper_p)?;
                set(agent, o, lower_p, upper_value, true)?;
                delete_property_or_throw(agent, o, upper_p)?;
            }
            //    j. Else if lowerExists is true and upperExists is false, then
            else if lower_exists && !upper_exists {
                //       i. Perform ? DeletePropertyOrThrow(O, lowerP).
                //       ii. Perform ? Set(O, upperP, lowerValue, true).
                let lower_value = get(agent, o, lower_p)?;
                delete_property_or_throw(agent, o, lower_p)?;
                set(agent, o, upper_p, lower_value, true)?;
            }
            //    k. Else,
            else {
                //       i. Assert: lowerExists and upperExists are both false.
                //       ii. NOTE: No action is required.
                assert!(!(lower_exists && upper_exists));
            }
            //    l. Set lower to lower + 1.
            lower += 1;
        }
        // 6. Return O.
        Ok(o.into_value())
    }

    /// ### [23.1.3.27 Array.prototype.shift ( )](https://tc39.es/ecma262/#sec-array.prototype.shift)
    ///
    /// This method removes the first element of the array and returns it.
    ///
    /// > ### Note
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn shift(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        if let Value::Array(array) = this_value {
            if array.is_empty(agent) {
                if agent[array].elements.len_writable {
                    return Ok(Value::Undefined);
                } else {
                    // This will throw
                    set(
                        agent,
                        array.into_object(),
                        BUILTIN_STRING_MEMORY.length.into(),
                        0.into(),
                        true,
                    )?;
                    unreachable!();
                }
            }
            if array.is_trivial(agent) && array.is_dense(agent) {
                // Fast path: Array is dense and contains no descriptors. No JS
                // functions can thus be called by shift.
                let slice = array.as_mut_slice(agent);
                let first = slice[0].unwrap();
                slice.copy_within(1.., 0);
                *slice.last_mut().unwrap() = None;
                let array_data = &mut agent[array];
                if array_data.elements.len_writable {
                    array_data.elements.len -= 1;
                    return Ok(first);
                } else {
                    // This will throw
                    set(
                        agent,
                        array.into_object(),
                        BUILTIN_STRING_MEMORY.length.into(),
                        (array.len(agent) - 1).into(),
                        true,
                    )?;
                    unreachable!();
                }
            }
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If len = 0, then
        if len == 0 {
            // a. Perform ? Set(O, "length", +0𝔽, true).
            set(
                agent,
                o,
                BUILTIN_STRING_MEMORY.length.into(),
                0.into(),
                true,
            )?;
            // b. Return undefined.
            return Ok(Value::Undefined);
        }
        // 4. Let first be ? Get(O, "0").
        let first = get(agent, o, 0.into())?;
        // 5. Let k be 1.
        let mut k = 1;
        // 6. Repeat, while k < len,
        while k < len {
            // a. Let from be ! ToString(𝔽(k)).
            let from = k.try_into().unwrap();
            // b. Let to be ! ToString(𝔽(k - 1)).
            let to = (k - 1).try_into().unwrap();
            // c. Let fromPresent be ? HasProperty(O, from).
            let from_present = has_property(agent, o, from)?;
            // d. If fromPresent is true, then
            if from_present {
                // i. Let fromValue be ? Get(O, from).
                let from_value = get(agent, o, from)?;
                // ii. Perform ? Set(O, to, fromValue, true).
                set(agent, o, to, from_value, true)?;
            } else {
                // e. Else,
                // i. Assert: fromPresent is false.
                // ii. Perform ? DeletePropertyOrThrow(O, to).
                delete_property_or_throw(agent, o, to)?;
            }
            // f. Set k to k + 1.
            k += 1;
        }
        // 7. Perform ? DeletePropertyOrThrow(O, ! ToString(𝔽(len - 1))).
        delete_property_or_throw(agent, o, (len - 1).try_into().unwrap())?;
        // 8. Perform ? Set(O, "length", 𝔽(len - 1), true).
        set(
            agent,
            o,
            BUILTIN_STRING_MEMORY.length.into(),
            (len - 1).try_into().unwrap(),
            true,
        )?;
        // 9. Return first.
        Ok(first)
    }

    /// ### [23.1.3.28 Array.prototype.slice ( start, end )](https://tc39.es/ecma262/#sec-array.prototype.slice)
    ///
    /// This method returns an array containing the elements of the array from
    /// element start up to, but not including, element end (or through the end
    /// of the array if end is undefined). If start is negative, it is treated
    /// as length + start where length is the length of the array. If end is
    /// negative, it is treated as length + end where length is the length of
    /// the array.
    ///
    /// > #### Note 1
    /// > The explicit setting of the "length" property in step 15 is intended
    /// > to ensure the length is correct even when A is not a built-in Array.
    ///
    /// > #### Note 2
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn slice(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let start = arguments.get(0);
        let end = arguments.get(1);
        if let (
            Value::Array(array),
            Value::Undefined | Value::Integer(_),
            Value::Undefined | Value::Integer(_),
        ) = (this_value, start, end)
        {
            let len = array.len(agent) as usize;
            if array.is_trivial(agent) && array.is_dense(agent) {
                let start = if let Value::Integer(relative_start) = start {
                    let relative_start = relative_start.into_i64();
                    if relative_start < 0 {
                        (len as i64 + relative_start).max(0) as usize
                    } else {
                        relative_start as usize
                    }
                } else {
                    0
                };
                let end = if let Value::Integer(relative_end) = end {
                    let relative_end = relative_end.into_i64();
                    if relative_end < 0 {
                        (len as i64 + relative_end).max(0) as usize
                    } else {
                        (relative_end as usize).min(len)
                    }
                } else {
                    len
                };
                let count = end.saturating_sub(start);
                let a = array_species_create(agent, array.into_object(), count)?;
                if count == 0 {
                    set(
                        agent,
                        a,
                        BUILTIN_STRING_MEMORY.length.into(),
                        0.into(),
                        true,
                    )?;
                    return Ok(a.into_value());
                }
                if let Object::Array(a) = a {
                    if a.len(agent) as usize == count
                        && a.is_trivial(agent)
                        && a.as_slice(agent).iter().all(|el| el.is_none())
                    {
                        // Array full of holes
                        let source_data = array.as_slice(agent)[start..end].as_ptr();
                        let destination_data = a.as_mut_slice(agent).as_mut_ptr();
                        // SAFETY: Source and destination are properly aligned
                        // and valid for reads/writes. They do not overlap.
                        // From JS point of view, setting data properties to
                        // the destination would not call any JS code so this
                        // is spec-wise correct.
                        unsafe {
                            std::ptr::copy_nonoverlapping(source_data, destination_data, count)
                        };
                        set(
                            agent,
                            a.into_object(),
                            BUILTIN_STRING_MEMORY.length.into(),
                            Number::try_from(count).unwrap().into_value(),
                            true,
                        )?;
                        return Ok(a.into_value());
                    }
                }
                let mut k = start;
                let mut n = 0u32;
                while k < end {
                    // a. Let Pk be ! ToString(𝔽(k)).
                    // b. Let kPresent be ? HasProperty(O, Pk).
                    // Note: Array is dense, we do not need to check this.
                    // c. If kPresent is true, then
                    // i. Let kValue be ? Get(O, Pk).
                    let k_value = array.as_slice(agent)[k].unwrap();
                    // ii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), kValue).
                    create_data_property_or_throw(agent, a, n.into(), k_value)?;
                    // d. Set k to k + 1.
                    k += 1;
                    // e. Set n to n + 1.
                    n += 1;
                }
                // 15. Perform ? Set(A, "length", 𝔽(n), true).
                set(
                    agent,
                    a.into_object(),
                    BUILTIN_STRING_MEMORY.length.into(),
                    n.into(),
                    true,
                )?;
                // 16. Return A.
                return Ok(a.into_value());
            }
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)? as usize;
        // 3. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start = to_integer_or_infinity(agent, start)?;
        // 4. If relativeStart = -∞, let k be 0.
        let mut k = if relative_start.is_neg_infinity(agent) {
            0
        } else if relative_start.into_i64(agent) < 0 {
            // 5. Else if relativeStart < 0, let k be max(len + relativeStart, 0).
            (len as i64 + relative_start.into_i64(agent)).max(0) as usize
        } else {
            // 6. Else, let k be min(relativeStart, len).
            relative_start.into_usize(agent).min(len)
        };

        // 7. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
        let relative_end = if end.is_undefined() {
            len.try_into().unwrap()
        } else {
            to_integer_or_infinity(agent, end)?
        };
        // 8. If relativeEnd = -∞, let final be 0.
        let final_end = if relative_end.is_neg_infinity(agent) {
            0
        } else if relative_end.into_i64(agent) < 0 {
            // 9. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
            (len as i64 + relative_end.into_i64(agent)).max(0) as usize
        } else {
            // 10. Else, let final be min(relativeEnd, len).
            relative_end.into_usize(agent).min(len)
        };
        // 11. Let count be max(final - k, 0).
        let count = final_end.saturating_sub(k);
        // 12. Let A be ? ArraySpeciesCreate(O, count).
        let a = array_species_create(agent, o, count)?;
        // 13. Let n be 0.
        let mut n = 0u32;
        // 14. Repeat, while k < final,
        while k < final_end {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = k.try_into().unwrap();
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;
                // ii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), kValue).
                create_data_property_or_throw(agent, a, n.into(), k_value)?;
            }
            // d. Set k to k + 1.
            k += 1;
            // e. Set n to n + 1.
            n += 1;
        }
        // 15. Perform ? Set(A, "length", 𝔽(n), true).
        set(
            agent,
            a,
            BUILTIN_STRING_MEMORY.length.into(),
            n.into(),
            true,
        )?;
        // 16. Return A.
        Ok(a.into_value())
    }

    /// ### 23.1.3.29 Array.prototype.some ( callbackfn \[ , thisArg \] )(https://tc39.es/ecma262/#sec-array.prototype.some)
    ///
    /// > #### Note 1
    /// >
    /// > `callbackfn` should be a function that accepts three arguments and
    /// > returns a value that is coercible to a Boolean value. **some** calls
    /// > `callbackfn` once for each element present in the array, in ascending
    /// > order, until it finds one where `callbackfn` returns **true**. If
    /// > such an element is found, **some** immediately returns **true**.
    /// > Otherwise, **some** returns **false**. `callbackfn` is called only
    /// > for elements of the array which actually exist; it is not called for
    /// > missing elements of the array.
    /// >
    /// > If a `thisArg` parameter is provided, it will be used as the **this**
    /// > value for each invocation of `callbackfn`. If it is not provided,
    /// > **undefined** is used instead.
    /// >
    /// > `callbackfn` is called with three arguments: the value of the
    /// > element, the index of the element, and the object being traversed.
    /// >
    /// > **some** does not directly mutate the object on which it is called
    /// > but the object may be mutated by the calls to `callbackfn`.
    /// >
    /// > The range of elements processed by **some** is set before the first
    /// > call to `callbackfn`. Elements that are appended to the array after
    /// > the call to **some** begins will not be visited by `callbackfn`. If
    /// > existing elements of the array are changed, their value as passed to
    /// > `callbackfn` will be the value at the time that **some** visits them;
    /// > elements that are deleted after the call to **some** begins and
    /// > before being visited are not visited. **some** acts like the "exists"
    /// > quantifier in mathematics. In particular, for an empty array, it
    /// > returns false.
    ///
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn some(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let callback_fn = arguments.get(0);
        let this_arg = arguments.get(1);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not callable",
            ));
        };
        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = k.try_into().unwrap();
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o, pk)?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o, pk)?;
                // ii. Let testResult be ToBoolean(? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »)).
                let test_result = call_function(
                    agent,
                    callback_fn,
                    this_arg,
                    Some(ArgumentsList(&[
                        k_value,
                        k.try_into().unwrap(),
                        o.into_value(),
                    ])),
                )?;
                // iii. If testResult is true, return true.
                if test_result == Value::Boolean(true) || to_boolean(agent, test_result) {
                    return Ok(true.into());
                }
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 6. Return false.
        Ok(false.into())
    }

    /// ### [23.1.3.30 Array.prototype.sort ( comparator )](https://tc39.es/ecma262/#sec-array.prototype.sort)
    ///
    /// This method sorts the elements of this array. The sort must be stable
    /// (that is, elements that compare equal must remain in their original
    /// order). If comparator is not undefined, it should be a function that
    /// accepts two arguments x and y and returns a negative Number if x < y, a
    /// positive Number if x > y, or a zero otherwise.
    ///
    /// > #### Note 1
    /// > Because non-existent property values always compare greater than
    /// > undefined property values, and undefined always compares greater than
    /// > any other value (see CompareArrayElements), undefined property values
    /// > always sort to the end of the result, followed by non-existent
    /// > property values.
    ///
    /// > #### Note 2
    /// > Method calls performed by the ToString abstract operations in steps 5
    /// > and 6 have the potential to cause SortCompare to not behave as a
    /// > consistent comparator.
    ///
    /// > #### Note 3
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore, it can be transferred to other
    /// > kinds of objects for use as a method.
    fn sort(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        let comparator = args.get(0);
        // 1. If comparator is not undefined and IsCallable(comparator) is false, throw a TypeError exception.
        let comparator = if comparator.is_undefined() {
            None
        } else if let Some(comparator) = is_callable(comparator) {
            Some(comparator)
        } else {
            return Err(agent.throw_exception_with_static_message(ExceptionType::TypeError, ""));
        };
        // 2. Let obj be ? ToObject(this value).
        let obj = to_object(agent, this_value)?;
        // 3. Let len be ? LengthOfArrayLike(obj).
        let len = usize::try_from(length_of_array_like(agent, obj)?).unwrap();
        // 4. Let SortCompare be a new Abstract Closure with parameters (x, y)
        //     that captures comparator and performs the following steps when
        //     called:
        //       a. Return ? CompareArrayElements(x, y, comparator).
        // 5. Let sortedList be ? SortIndexedProperties(obj, len, SortCompare,
        //     skip-holes).
        let sorted_list: Vec<Value> =
            sort_indexed_properties::<true, false>(agent, obj, len, comparator)?;
        // 6. Let itemCount be the number of elements in sortedList.
        let item_count = sorted_list.len();
        // 7. Let j be 0.
        let mut j = 0;
        // 8. Repeat, while j < itemCount,
        while j < item_count {
            // a. Perform ? Set(obj, ! ToString(𝔽(j)), sortedList[j], true).
            set(agent, obj, j.try_into().unwrap(), sorted_list[j], true)?;
            // b. Set j to j + 1.
            j += 1;
        }
        // 9. NOTE: The call to SortIndexedProperties in step 5 uses
        // skip-holes. The remaining indices are deleted to preserve the number
        // of holes that were detected and excluded from the sort.

        // 10. Repeat, while j < len,
        while j < len {
            // a. Perform ? DeletePropertyOrThrow(obj, ! ToString(𝔽(j))).
            delete_property_or_throw(agent, obj, j.try_into().unwrap())?;
            // b. Set j to j + 1.
            j += 1;
        }
        // 11. Return obj.
        Ok(obj.into_value())
    }

    fn splice(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn to_locale_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn to_reversed(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. Let A be ? ArrayCreate(len).
        let a = array_create(agent, len as usize, len as usize, None)?;
        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            //    a. Let from be ! ToString(𝔽(len - k - 1)).
            let from = PropertyKey::Integer((len - k - 1).try_into().unwrap());
            //    b. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();
            //    c. Let fromValue be ? Get(O, from).
            let from_value = get(agent, o, from)?;
            //    d. Perform ! CreateDataPropertyOrThrow(A, Pk, fromValue).
            create_data_property_or_throw(agent, a, pk, from_value)?;
            //    e. Set k to k + 1.
            k += 1;
            eprintln!("k: {}", k);
        }
        // 6. Return A.
        Ok(a.into_value())
    }

    fn to_sorted(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn to_spliced(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    /// ### [23.1.3.36 Array.prototype.toString ( )](https://tc39.es/ecma262/#sec-array.prototype.tostring)
    fn to_string(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let array be ? ToObject(this value).
        let array = to_object(agent, this_value)?;
        // 2. Let func be ? Get(array, "join").
        let func = get(agent, array, BUILTIN_STRING_MEMORY.join.into())?;
        // 3. If IsCallable(func) is false, set func to the intrinsic function %Object.prototype.toString%.
        let func = is_callable(func).unwrap_or_else(|| {
            agent
                .current_realm()
                .intrinsics()
                .object_prototype_to_string()
                .into_function()
        });
        // 4. Return ? Call(func, array).
        call_function(agent, func, array.into_value(), None)
    }

    fn unshift(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn values(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected this to be an object",
            ));
        };
        // 2. Return CreateArrayIterator(O, value).
        Ok(ArrayIterator::from_object(agent, o, CollectionIteratorKind::Value).into_value())
    }

    fn with(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.array_prototype();
        let this_base_object = intrinsics.array_prototype_base_object();
        let array_constructor = intrinsics.array();
        let array_prototype_values = intrinsics.array_prototype_values();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this_base_object)
            .with_property_capacity(41)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<ArrayPrototypeAt>()
            .with_builtin_function_property::<ArrayPrototypeConcat>()
            .with_constructor_property(array_constructor)
            .with_builtin_function_property::<ArrayPrototypeCopyWithin>()
            .with_builtin_function_property::<ArrayPrototypeEntries>()
            .with_builtin_function_property::<ArrayPrototypeEvery>()
            .with_builtin_function_property::<ArrayPrototypeFill>()
            .with_builtin_function_property::<ArrayPrototypeFilter>()
            .with_builtin_function_property::<ArrayPrototypeFind>()
            .with_builtin_function_property::<ArrayPrototypeFindIndex>()
            .with_builtin_function_property::<ArrayPrototypeFindLast>()
            .with_builtin_function_property::<ArrayPrototypeFindLastIndex>()
            .with_builtin_function_property::<ArrayPrototypeFlat>()
            .with_builtin_function_property::<ArrayPrototypeFlatMap>()
            .with_builtin_function_property::<ArrayPrototypeForEach>()
            .with_builtin_function_property::<ArrayPrototypeIncludes>()
            .with_builtin_function_property::<ArrayPrototypeIndexOf>()
            .with_builtin_function_property::<ArrayPrototypeJoin>()
            .with_builtin_function_property::<ArrayPrototypeKeys>()
            .with_builtin_function_property::<ArrayPrototypeLastIndexOf>()
            .with_builtin_function_property::<ArrayPrototypeMap>()
            .with_builtin_function_property::<ArrayPrototypePop>()
            .with_builtin_function_property::<ArrayPrototypePush>()
            .with_builtin_function_property::<ArrayPrototypeReduce>()
            .with_builtin_function_property::<ArrayPrototypeReduceRight>()
            .with_builtin_function_property::<ArrayPrototypeReverse>()
            .with_builtin_function_property::<ArrayPrototypeShift>()
            .with_builtin_function_property::<ArrayPrototypeSlice>()
            .with_builtin_function_property::<ArrayPrototypeSome>()
            .with_builtin_intrinsic_function_property::<ArrayPrototypeSort>()
            .with_builtin_function_property::<ArrayPrototypeSplice>()
            .with_builtin_function_property::<ArrayPrototypeToLocaleString>()
            .with_builtin_function_property::<ArrayPrototypeToReversed>()
            .with_builtin_function_property::<ArrayPrototypeToSorted>()
            .with_builtin_function_property::<ArrayPrototypeToSpliced>()
            .with_builtin_intrinsic_function_property::<ArrayPrototypeToString>()
            .with_builtin_function_property::<ArrayPrototypeUnshift>()
            .with_builtin_intrinsic_function_property::<ArrayPrototypeValues>()
            .with_builtin_function_property::<ArrayPrototypeWith>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(array_prototype_values.into_value())
                    .with_enumerable(ArrayPrototypeValues::ENUMERABLE)
                    .with_configurable(ArrayPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Unscopables.into())
                    .with_value_creator_readonly(|agent| {
                        OrdinaryObjectBuilder::new(agent, realm)
                            .with_property_capacity(16)
                            .with_data_property(BUILTIN_STRING_MEMORY.at.into(), true.into())
                            .with_data_property(
                                BUILTIN_STRING_MEMORY.copyWithin.into(),
                                true.into(),
                            )
                            .with_data_property(BUILTIN_STRING_MEMORY.entries.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.fill.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.find.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.findIndex.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.findLast.into(), true.into())
                            .with_data_property(
                                BUILTIN_STRING_MEMORY.findLastIndex.into(),
                                true.into(),
                            )
                            .with_data_property(BUILTIN_STRING_MEMORY.flat.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.flatMap.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.includes.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.keys.into(), true.into())
                            .with_data_property(
                                BUILTIN_STRING_MEMORY.toReversed.into(),
                                true.into(),
                            )
                            .with_data_property(BUILTIN_STRING_MEMORY.toSorted.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.toSpliced.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.values.into(), true.into())
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .build();

        let slot = agent.heap.arrays.get_mut(this.get_index()).unwrap();
        assert!(slot.is_none());
        *slot = Some(ArrayHeapData {
            object_index: Some(this_base_object),
            // has a "length" property whose initial value is +0𝔽 and whose
            // attributes are { [[Writable]]: true, [[Enumerable]]: false,
            // [[Configurable]]: false }.
            elements: Default::default(),
        });
    }
}

/// ### [23.1.3.2.1 IsConcatSpreadable ( O )](https://tc39.es/ecma262/#sec-isconcatspreadable)
///
/// The abstract operation IsConcatSpreadable takes argument O (an ECMAScript
/// language value) and returns either a normal completion containing a Boolean
/// or a throw completion.
///
/// > Note: Instead of returning a bool, Nova returns an Option<Object>.

fn is_concat_spreadable(agent: &mut Agent, o: Value) -> JsResult<Option<Object>> {
    // 1. If O is not an Object, return false.
    if let Ok(o) = Object::try_from(o) {
        // 2. Let spreadable be ? Get(O, @@isConcatSpreadable).
        let spreadable = get(agent, o, WellKnownSymbolIndexes::IsConcatSpreadable.into())?;
        // 3. If spreadable is not undefined, return ToBoolean(spreadable).
        if !spreadable.is_undefined() {
            let spreadable = to_boolean(agent, spreadable);
            if spreadable {
                Ok(Some(o))
            } else {
                Ok(None)
            }
        } else {
            // 4. Return ? IsArray(O).
            let o_is_array = is_array(agent, o.into_value())?;
            if o_is_array {
                Ok(Some(o))
            } else {
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// ### [23.1.3.12.1 FindViaPredicate ( O, len, direction, predicate, thisArg )](https://tc39.es/ecma262/#sec-findviapredicate)
///
/// The abstract operation FindViaPredicate takes arguments O (an Object), len
/// (a non-negative integer), direction (ascending or descending), predicate
/// (an ECMAScript language value), and thisArg (an ECMAScript language value)
/// and returns either a normal completion containing a Record with fields
/// \[\[Index]] (an integral Number) and \[\[Value]] (an ECMAScript language
/// value) or a throw completion.
///
/// O should be an array-like object or a TypedArray. This operation calls
/// predicate once for each element of O, in either ascending index order or
/// descending index order (as indicated by direction), until it finds one
/// where predicate returns a value that coerces to true. At that point, this
/// operation returns a Record that gives the index and value of the element
/// found. If no such element is found, this operation returns a Record that
/// specifies -1𝔽 for the index and undefined for the value.
///
/// predicate should be a function. When called for an element of the array, it
/// is passed three arguments: the value of the element, the index of the
/// element, and the object being traversed. Its return value will be coerced
/// to a Boolean value.
///
/// thisArg will be used as the this value for each invocation of predicate.
///
/// This operation does not directly mutate the object on which it is called,
/// but the object may be mutated by the calls to predicate.
///
/// The range of elements processed is set before the first call to predicate,
/// just before the traversal begins. Elements that are appended to the array
/// after this will not be visited by predicate. If existing elements of the
/// array are changed, their value as passed to predicate will be the value at
/// the time that this operation visits them. Elements that are deleted after
/// traversal begins and before being visited are still visited and are either
/// looked up from the prototype or are undefined.
fn find_via_predicate(
    agent: &mut Agent,
    o: Object,
    len: i64,
    ascending: bool,
    predicate: Value,
    this_arg: Value,
) -> JsResult<(i64, Value)> {
    // 1. If IsCallable(predicate) is false, throw a TypeError exception.
    let Some(predicate) = is_callable(predicate) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Predicate is not a function",
        ));
    };
    // 4. For each integer k of indices, do
    let check = |agent: &mut Agent,
                 o: Object,
                 predicate: Function,
                 this_arg: Value,
                 k: i64|
     -> JsResult<Option<(i64, Value)>> {
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk = PropertyKey::Integer(k.try_into().unwrap());
        // b. NOTE: If O is a TypedArray, the following invocation of Get will return a normal completion.
        // c. Let kValue be ? Get(O, Pk).
        let k_value = get(agent, o, pk)?;
        // d. Let testResult be ? Call(predicate, thisArg, « kValue, 𝔽(k), O »).
        let test_result = call_function(
            agent,
            predicate,
            this_arg,
            Some(ArgumentsList(&[
                Number::try_from(k).unwrap().into_value(),
                o.into_value(),
            ])),
        )?;
        // e. If ToBoolean(testResult) is true, return the Record { [[Index]]: 𝔽(k), [[Value]]: kValue }.
        if to_boolean(agent, test_result) {
            Ok(Some((k, k_value)))
        } else {
            Ok(None)
        }
    };

    // 2. If direction is ascending, then
    if ascending {
        // a. Let indices be a List of the integers in the interval from 0 (inclusive) to len (exclusive), in ascending order.
        for k in 0..len {
            if let Some(result) = check(agent, o, predicate, this_arg, k)? {
                return Ok(result);
            }
        }
    } else {
        // 3. Else,
        // a. Let indices be a List of the integers in the interval from 0 (inclusive) to len (exclusive), in descending order.
        for k in (0..len).rev() {
            if let Some(result) = check(agent, o, predicate, this_arg, k)? {
                return Ok(result);
            }
        }
    };
    // 5. Return the Record { [[Index]]: -1𝔽, [[Value]]: undefined }.
    Ok((-1, Value::Undefined))
}

/// ### [23.1.3.13.1 FlattenIntoArray ( target, source, sourceLen, start, depth \[ , mapperFunction \[ , thisArg \] \] )](https://tc39.es/ecma262/#sec-flattenintoarray)
/// The abstract operation FlattenIntoArray takes arguments target (an Object),
/// source (an Object), sourceLen (a non-negative integer), start (a
/// non-negative integer), and depth (a non-negative integer or +∞) and
/// optional arguments mapperFunction (a function object) and thisArg (an
/// ECMAScript language value) and returns either a normal completion
/// containing a non-negative integer or a throw completion.
#[allow(clippy::too_many_arguments)]
fn flatten_into_array(
    agent: &mut Agent,
    target: Object,
    source: Object,
    source_len: usize,
    start: usize,
    depth: Option<usize>,
    mapper_function: Option<Function>,
    this_arg: Option<Value>,
) -> JsResult<usize> {
    // 1. Assert: If mapperFunction is present, then IsCallable(mapperFunction) is true, thisArg is present, and depth is 1.
    assert!(mapper_function.is_none() || this_arg.is_some() && depth == Some(1));
    // 2. Let targetIndex be start.
    let mut target_index = start;
    // 3. Let sourceIndex be +0𝔽.
    let mut source_index = 0;
    // 4. Repeat, while ℝ(sourceIndex) < sourceLen,
    while source_index < source_len {
        // a. Let P be ! ToString(sourceIndex).
        let source_index_number = Number::try_from(source_index).unwrap();
        let p = PropertyKey::try_from(source_index).unwrap();
        // b. Let exists be ? HasProperty(source, P).
        let exists = has_property(agent, source, p)?;
        // c. If exists is true, then
        if !exists {
            // d. Set sourceIndex to sourceIndex + 1𝔽.
            source_index += 1;
            continue;
        }
        // i. Let element be ? Get(source, P).
        let element = get(agent, source, p)?;
        // ii. If mapperFunction is present, then
        let element = if let Some(mapper_function) = mapper_function {
            // 1. Set element to ? Call(mapperFunction, thisArg, « element, sourceIndex, source »).
            call_function(
                agent,
                mapper_function,
                this_arg.unwrap(),
                Some(ArgumentsList(&[
                    element,
                    source_index_number.into_value(),
                    source.into_value(),
                ])),
            )?
        } else {
            element
        };
        // iii. Let shouldFlatten be false.
        let mut should_flatten = false;
        // iv. If depth > 0, then
        if depth.map_or(true, |depth| depth > 0) {
            // 1. Set shouldFlatten to ? IsArray(element).
            should_flatten = is_array(agent, element)?;
        }
        // v. If shouldFlatten is true, then
        if should_flatten {
            // Note: Element is necessary an Array.
            let element = Object::try_from(element).unwrap();
            let new_depth = depth.map(|depth| depth - 1);
            // 3. Let elementLen be ? LengthOfArrayLike(element).
            let element_len = length_of_array_like(agent, element)? as usize;
            // 4. Set targetIndex to ? FlattenIntoArray(target, element, elementLen, targetIndex, newDepth).
            target_index = flatten_into_array(
                agent,
                target,
                element,
                element_len,
                target_index,
                new_depth,
                None,
                None,
            )?;
        } else {
            // vi. Else,
            // 1. If targetIndex ≥ 2**53 - 1, throw a TypeError exception.
            if target_index >= SmallInteger::MAX_NUMBER as usize {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Target index overflowed",
                ));
            }
            // 2. Perform ? CreateDataPropertyOrThrow(target, ! ToString(𝔽(targetIndex)), element).
            create_data_property_or_throw(
                agent,
                target,
                target_index.try_into().unwrap(),
                element,
            )?;
            // 3. Set targetIndex to targetIndex + 1.
        }
        // d. Set sourceIndex to sourceIndex + 1𝔽.
        source_index += 1;
    }
    // 5. Return targetIndex.
    Ok(target_index)
}

/// ### [23.1.3.30.1 SortIndexedProperties ( obj, len, SortCompare, holes )](https://tc39.es/ecma262/#sec-sortindexedproperties)
///
/// The abstract operation SortIndexedProperties takes arguments obj (an
/// Object), len (a non-negative integer), SortCompare (an Abstract Closure
/// with two parameters), and holes (skip-holes or read-through-holes) and
/// returns either a normal completion containing a List of ECMAScript language
/// values or a throw completion.
///
/// The sort order is the ordering of items after completion of step 4 of the
/// algorithm. The sort order is implementation-defined if SortCompare is not a
/// consistent comparator for the elements of items. When SortIndexedProperties
/// is invoked by Array.prototype.sort, the sort order is also
/// implementation-defined if comparator is undefined, and all applications of
/// ToString, to any specific value passed as an argument to SortCompare, do
/// not produce the same result.
///
/// Unless the sort order is specified to be implementation-defined, it must
/// satisfy all of the following conditions:
/// * There must be some mathematical permutation π of the non-negative
///   integers less than itemCount, such that for every non-negative integer
///   `j` less than itemCount, the element `old[j]` is exactly the same as
///   `new[π(j)]`.
/// * Then for all non-negative integers `j` and `k`, each less than
///   itemCount, if `ℝ(SortCompare(old[j], old[k])) < 0`, then
///   `π(j) < π(k)`.
///
/// Here the notation `old[j]` is used to refer to `items[j]` before step 4 is
/// executed, and the notation `new[j]` to refer to `items[j]` after step 4 has
/// been executed.
///
/// An abstract closure or function comparator is a consistent comparator for a
/// set of values `S` if all of the requirements below are met for all values
/// `a`, `b`, and `c` (possibly the same value) in the set S: The notation
/// `a <C b` means `ℝ(comparator(a, b)) < 0`; `a =C b` means
/// `ℝ(comparator(a, b)) = 0`; and `a >C b` means `ℝ(comparator(a, b)) > 0`.
///
/// * Calling `comparator(a, b)` always returns the same value `v` when given a
///   specific pair of values `a` and `b` as its two arguments. Furthermore,
///   `v` is a Number, and `v` is not NaN. Note that this implies that exactly
///   one of `a <C b`, `a =C b`, and `a >C b` will be true for a given pair of
///   `a` and `b`.
/// * Calling `comparator(a, b)` does not modify `obj` or any object on `obj`'s
///   prototype chain.
/// * `a =C a` (reflexivity)
/// * If `a =C b`, then `b =C a` (symmetry)
/// * If `a =C b` and `b =C c`, then `a =C c` (transitivity of `=C`)
/// * If `a <C b` and `b <C c`, then `a <C c` (transitivity of `<C`)
/// * If `a >C b` and `b >C c`, then `a >C c` (transitivity of `>C`)
///
/// > #### Note
/// > The above conditions are necessary and sufficient to ensure that
/// > comparator divides the set S into equivalence classes and that these
/// > equivalence classes are totally ordered.
fn sort_indexed_properties<const SKIP_HOLES: bool, const TYPED_ARRAY: bool>(
    agent: &mut Agent,
    obj: Object,
    len: usize,
    comparator: Option<Function>,
) -> JsResult<Vec<Value>> {
    // 1. Let items be a new empty List.
    let mut items = Vec::with_capacity(len);
    // 2. Let k be 0.
    let mut k = 0;
    // 3. Repeat, while k < len,
    while k < len {
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk: PropertyKey = k.try_into().unwrap();
        // b. If holes is skip-holes, then
        let k_read = if SKIP_HOLES {
            // i. Let kRead be ? HasProperty(obj, Pk).
            has_property(agent, obj, pk)?
        } else {
            // c. Else,
            // i. Assert: holes is read-through-holes.
            // ii. Let kRead be true.
            true
        };
        // d. If kRead is true, then
        if k_read {
            // i. Let kValue be ? Get(obj, Pk).
            let k_value = get(agent, obj, pk)?;
            // ii. Append kValue to items.
            items.push(k_value);
        }
        // e. Set k to k + 1.
        k += 1;
    }
    // 4. Sort items using an implementation-defined sequence of calls to
    // SortCompare. If any such call returns an abrupt completion, stop before
    // performing any further calls to SortCompare and return that Completion
    // Record.
    if TYPED_ARRAY {
        items.sort_by(|_a, _b| compare_typed_array_elements());
    } else {
        let mut error: Option<JsError> = None;
        items.sort_by(|a, b| {
            if error.is_some() {
                // This is dangerous but we don't have much of a choice.
                return Ordering::Equal;
            }
            let result = compare_array_elements(agent, *a, *b, comparator);
            let Ok(result) = result else {
                error = Some(result.unwrap_err());
                return Ordering::Equal;
            };
            result
        });
        if let Some(error) = error {
            return Err(error);
        }
    }
    // 5. Return items.
    Ok(items)
}

/// ### [23.1.3.30.2 CompareArrayElements ( x, y, comparator )](https://tc39.es/ecma262/#sec-comparearrayelements)
/// The abstract operation CompareArrayElements takes arguments x (an
/// ECMAScript language value), y (an ECMAScript language value), and
/// comparator (a function object or undefined) and returns either a normal
/// completion containing a Number or an abrupt completion.
fn compare_array_elements(
    agent: &mut Agent,
    x: Value,
    y: Value,
    comparator: Option<Function>,
) -> JsResult<Ordering> {
    // 1. If x and y are both undefined, return +0𝔽.
    if x.is_undefined() && y.is_undefined() {
        Ok(Ordering::Equal)
    } else if x.is_undefined() {
        // 2. If x is undefined, return 1𝔽.
        Ok(Ordering::Greater)
    } else if y.is_undefined() {
        // 3. If y is undefined, return -1𝔽.
        Ok(Ordering::Less)
    } else
    // 4. If comparator is not undefined, then
    if let Some(comparator) = comparator {
        // a. Let v be ? ToNumber(? Call(comparator, undefined, « x, y »)).
        let v = call_function(
            agent,
            comparator,
            Value::Undefined,
            Some(ArgumentsList(&[x, y])),
        )?;
        let v = to_number(agent, v)?;
        // b. If v is NaN, return +0𝔽.
        // c. Return v.
        if v.is_nan(agent) {
            Ok(Ordering::Equal)
        } else if v.is_sign_positive(agent) {
            Ok(Ordering::Greater)
        } else if v.is_sign_negative(agent) {
            Ok(Ordering::Less)
        } else {
            Ok(Ordering::Equal)
        }
    } else if let (Value::Integer(x), Value::Integer(y)) = (x, y) {
        // Fast path: Avoid string conversions for numbers
        Ok(x.into_i64().cmp(&y.into_i64()))
    } else if let (Ok(x), Ok(y)) = (Number::try_from(x), Number::try_from(y)) {
        // Fast path: Avoid string conversions for numbers.
        // Note: This is probably not correct for NaN's.
        Ok(x.into_f64(agent).total_cmp(&y.into_f64(agent)))
    } else {
        // 5. Let xString be ? ToString(x).
        let x = to_string(agent, x)?;
        // 6. Let yString be ? ToString(y).
        let y = to_string(agent, y)?;
        // 7. Let xSmaller be ! IsLessThan(xString, yString, true).
        // 8. If xSmaller is true, return -1𝔽.
        if is_less_than::<true>(agent, x, y).unwrap() == Some(true) {
            Ok(Ordering::Less)
        } else
        // 9. Let ySmaller be ! IsLessThan(yString, xString, true).
        // 10. If ySmaller is true, return 1𝔽.
        if is_less_than::<true>(agent, y, x).unwrap() == Some(true) {
            Ok(Ordering::Greater)
        } else {
            // 11. Return +0𝔽.
            Ok(Ordering::Equal)
        }
    }
}

fn compare_typed_array_elements() -> Ordering {
    todo!();
}
