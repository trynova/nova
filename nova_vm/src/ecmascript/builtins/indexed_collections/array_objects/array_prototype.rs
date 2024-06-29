use num_traits::Zero;
use small_string::SmallString;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_data_property_or_throw, delete_property_or_throw, get,
                has_property, length_of_array_like, set,
            },
            testing_and_comparison::{is_array, is_callable, is_strictly_equal, same_value_zero},
            type_conversion::{to_boolean, to_integer_or_infinity, to_object, to_string},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{array_species_create, ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{
            Function, IntoFunction, IntoValue, Number, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{Heap, IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
    SmallInteger,
};

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
            Number::Float(_) | Number::Number(_) => {
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
                    return Err(agent.throw_exception(ExceptionType::TypeError, "Array overflow"));
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
                    return Err(agent.throw_exception(ExceptionType::TypeError, "Array overflow"));
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

    fn entries(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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
        if !is_callable(callback_fn) {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Callback is not a function")
            );
        }
        let callback_fn = Function::try_from(callback_fn).unwrap();
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
                let f_k = Number::from(k).into_value();
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

    fn filter(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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
        Ok(Number::from(find_rec.0).into_value())
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
        Ok(Number::from(find_rec.0).into_value())
    }

    fn flat(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn flat_map(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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
        if !is_callable(callback_fn) {
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                "Callback function is not a function",
            ));
        }
        let callback_fn = Function::try_from(callback_fn).unwrap();

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
            for (index, element_k) in data.iter().enumerate() {
                if let Some(element_k) = element_k {
                    if is_strictly_equal(agent, search_element, *element_k) {
                        return Ok((k as u32 + index as u32).into());
                    }
                }
            }
            return Ok((-1).into());
        };
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        // 3. If len = 0, return -1𝔽.
        if len.is_zero() {
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

    fn keys(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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
            for (index, element_k) in data.iter().enumerate().rev() {
                if let Some(element_k) = element_k {
                    if is_strictly_equal(agent, search_element, *element_k) {
                        return Ok((index as u32).into());
                    }
                }
            }
            return Ok((-1).into());
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
        if !is_callable(callback_fn) {
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                "Callback function is not a function",
            ));
        }
        let callback_fn = Function::try_from(callback_fn).unwrap();
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
                        Err(agent
                            .throw_exception(ExceptionType::TypeError, "Could not set property."))
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
                        return Err(agent
                            .throw_exception(ExceptionType::TypeError, "Could not set property."));
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
            return Err(agent.throw_exception(ExceptionType::TypeError, "Array length overflow"));
        }
        if let Object::Array(array) = o {
            // Fast path: Reserve enough room in the array.
            let Heap {
                arrays, elements, ..
            } = &mut agent.heap;
            arrays
                .get_mut(array.into_index())
                .unwrap()
                .unwrap()
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

    fn reduce(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn reduce_right(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn reverse(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn shift(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn slice(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn some(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn sort(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
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

    fn to_reversed(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
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
        let func = if !is_callable(func) {
            agent
                .current_realm()
                .intrinsics()
                .object_prototype_to_string()
                .into_function()
        } else {
            Function::try_from(func).unwrap()
        };
        // 4. Return ? Call(func, array).
        call_function(agent, func, array.into_value(), None)
    }

    fn unshift(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn values(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn with(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.array_prototype();
        let array_constructor = intrinsics.array();
        let array_prototype_values = intrinsics.array_prototype_values();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
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
    if !is_callable(predicate) {
        return Err(agent.throw_exception(ExceptionType::TypeError, "Predicate is not a function"));
    }
    let predicate = Function::try_from(predicate).unwrap();
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
                Number::from(k).into_value(),
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
