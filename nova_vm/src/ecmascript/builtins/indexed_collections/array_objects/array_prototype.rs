// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cmp::Ordering;

use wtf8::Wtf8Buf;

use crate::ecmascript::abstract_operations::operations_on_objects::{
    invoke, try_create_data_property_or_throw, try_length_of_array_like,
};
use crate::ecmascript::abstract_operations::type_conversion::{
    try_to_integer_or_infinity, try_to_string,
};
use crate::ecmascript::types::InternalMethods;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::rootable::{Rootable, Scopable};
use crate::engine::{ScopableCollection, Scoped, TryResult, unwrap_try};
use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_data_property_or_throw, delete_property_or_throw, get,
                has_property, length_of_array_like, set,
            },
            testing_and_comparison::{is_array, is_callable, is_strictly_equal, same_value_zero},
            type_conversion::{
                to_boolean, to_integer_or_infinity, to_number, to_object, to_string,
            },
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, ArrayHeapData, Behaviour, Builtin, BuiltinIntrinsic, array_create,
            array_species_create,
        },
        execution::{
            Agent, JsResult, Realm,
            agent::{ExceptionType, JsError},
        },
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoFunction, IntoObject, IntoValue, Number, Object,
            PropertyKey, String, Value,
        },
    },
    heap::{Heap, IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

use super::array_iterator_objects::array_iterator::{ArrayIterator, CollectionIteratorKind};

pub(crate) struct ArrayPrototype;

struct ArrayPrototypeAt;
impl Builtin for ArrayPrototypeAt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.at;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::at);
}
struct ArrayPrototypeConcat;
impl Builtin for ArrayPrototypeConcat {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.concat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::concat);
}
struct ArrayPrototypeCopyWithin;
impl Builtin for ArrayPrototypeCopyWithin {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.copyWithin;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::copy_within);
}
struct ArrayPrototypeEntries;
impl Builtin for ArrayPrototypeEntries {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::entries);
}
struct ArrayPrototypeEvery;
impl Builtin for ArrayPrototypeEvery {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::every);
}
struct ArrayPrototypeFill;
impl Builtin for ArrayPrototypeFill {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fill;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::fill);
}
struct ArrayPrototypeFilter;
impl Builtin for ArrayPrototypeFilter {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.filter;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::filter);
}
struct ArrayPrototypeFind;
impl Builtin for ArrayPrototypeFind {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.find;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find);
}
struct ArrayPrototypeFindIndex;
impl Builtin for ArrayPrototypeFindIndex {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.findIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_index);
}
struct ArrayPrototypeFindLast;
impl Builtin for ArrayPrototypeFindLast {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.findLast;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_last);
}
struct ArrayPrototypeFindLastIndex;
impl Builtin for ArrayPrototypeFindLastIndex {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.findLastIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_last_index);
}
struct ArrayPrototypeFlat;
impl Builtin for ArrayPrototypeFlat {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.flat;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::flat);
}
struct ArrayPrototypeFlatMap;
impl Builtin for ArrayPrototypeFlatMap {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.flatMap;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::flat_map);
}
struct ArrayPrototypeForEach;
impl Builtin for ArrayPrototypeForEach {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::for_each);
}
struct ArrayPrototypeIncludes;
impl Builtin for ArrayPrototypeIncludes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.includes;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::includes);
}
struct ArrayPrototypeIndexOf;
impl Builtin for ArrayPrototypeIndexOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.indexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::index_of);
}
struct ArrayPrototypeJoin;
impl Builtin for ArrayPrototypeJoin {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.join;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::join);
}
struct ArrayPrototypeKeys;
impl Builtin for ArrayPrototypeKeys {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::keys);
}
struct ArrayPrototypeLastIndexOf;
impl Builtin for ArrayPrototypeLastIndexOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::last_index_of);
}
struct ArrayPrototypeMap;
impl Builtin for ArrayPrototypeMap {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.map;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::map);
}
struct ArrayPrototypePop;
impl Builtin for ArrayPrototypePop {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.pop;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::pop);
}
struct ArrayPrototypePush;
impl Builtin for ArrayPrototypePush {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.push;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::push);
}
struct ArrayPrototypeReduce;
impl Builtin for ArrayPrototypeReduce {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reduce;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reduce);
}
struct ArrayPrototypeReduceRight;
impl Builtin for ArrayPrototypeReduceRight {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reduceRight;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reduce_right);
}
struct ArrayPrototypeReverse;
impl Builtin for ArrayPrototypeReverse {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reverse;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reverse);
}
struct ArrayPrototypeShift;
impl Builtin for ArrayPrototypeShift {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.shift;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::shift);
}
struct ArrayPrototypeSlice;
impl Builtin for ArrayPrototypeSlice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::slice);
}
struct ArrayPrototypeSome;
impl Builtin for ArrayPrototypeSome {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.some;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::some);
}
struct ArrayPrototypeSort;
impl Builtin for ArrayPrototypeSort {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sort;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::sort);
}
impl BuiltinIntrinsic for ArrayPrototypeSort {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeSort;
}
struct ArrayPrototypeSplice;
impl Builtin for ArrayPrototypeSplice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.splice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::splice);
}
struct ArrayPrototypeToLocaleString;
impl Builtin for ArrayPrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_locale_string);
}
struct ArrayPrototypeToReversed;
impl Builtin for ArrayPrototypeToReversed {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toReversed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_reversed);
}
struct ArrayPrototypeToSorted;
impl Builtin for ArrayPrototypeToSorted {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toSorted;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_sorted);
}
struct ArrayPrototypeToSpliced;
impl Builtin for ArrayPrototypeToSpliced {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toSpliced;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_spliced);
}
struct ArrayPrototypeToString;
impl Builtin for ArrayPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_string);
}
impl BuiltinIntrinsic for ArrayPrototypeToString {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeToString;
}
struct ArrayPrototypeUnshift;
impl Builtin for ArrayPrototypeUnshift {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.unshift;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::unshift);
}
struct ArrayPrototypeValues;
impl Builtin for ArrayPrototypeValues {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::values);
}
impl BuiltinIntrinsic for ArrayPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeValues;
}
struct ArrayPrototypeWith;
impl Builtin for ArrayPrototypeWith {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.with;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::with);
}

impl ArrayPrototype {
    /// ### [23.1.3.1 Array.prototype.at ( index )](https://tc39.es/ecma262/#sec-array.prototype.at)
    fn at<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let mut index = arguments.get(0).bind(nogc);

        // 1. Let O be ? ToObject(this value).
        let mut o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let mut scoped_o = None;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = if let TryResult::Continue(len) = try_length_of_array_like(agent, o, gc.nogc()) {
            len.unbind()?
        } else {
            scoped_o = Some(o.scope(agent, gc.nogc()));
            let scoped_index = index.scope(agent, gc.nogc());
            let result = length_of_array_like(agent, o.unbind(), gc.reborrow()).unbind()?;
            o = scoped_o.as_ref().unwrap().get(agent).bind(gc.nogc());
            index = scoped_index.get(agent).bind(gc.nogc());
            result
        };
        // 3. Let relativeIndex be ? ToIntegerOrInfinity(index).
        let relative_index =
            if let TryResult::Continue(len) = try_to_integer_or_infinity(agent, index, gc.nogc()) {
                len.unbind()?.into_i64()
            } else {
                scoped_o = Some(scoped_o.unwrap_or_else(|| o.scope(agent, gc.nogc())));
                let result = to_integer_or_infinity(agent, index.unbind(), gc.reborrow())
                    .unbind()?
                    .into_i64();
                o = scoped_o.unwrap().get(agent).bind(gc.nogc());
                result
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
            get(
                agent,
                o.unbind(),
                PropertyKey::Integer(k.try_into().unwrap()),
                gc,
            )
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
    fn concat<'gc>(
        agent: &mut Agent,
        this_value: Value,
        items: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let mut items = items
            .as_slice()
            .iter()
            .map(|i| i.scope(agent, gc.nogc()))
            .collect::<Vec<_>>();

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let scoped_o = o.scope(agent, gc.nogc());
        // 2. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o.unbind(), 0, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // Optimisation: Reserve space for all Arrays being concatenated.
        if let Object::Array(a) = a {
            let mut total_len = 0u32;
            if let Object::Array(this_value) = scoped_o.get(agent) {
                total_len = total_len.saturating_add(this_value.len(agent));
            }
            items.iter().for_each(|item| {
                if let Value::Array(item) = item.get(agent) {
                    total_len = item.len(agent);
                }
            });
            let Heap {
                arrays, elements, ..
            } = &mut agent.heap;
            arrays[a].elements.reserve(elements, total_len);
        }

        let a = a.scope(agent, gc.nogc());
        // 3. Let n be 0.
        let mut n = 0;
        // 4. Prepend O to items.
        // SAFETY: We're replacing the stored Object value with itself as a
        // Value; their heap root data is the same in either case so on the
        // heap this is a no-op.
        let o_as_value = unsafe {
            scoped_o
                .clone()
                .replace_self(agent, scoped_o.get(agent).into_value())
        };
        items.insert(0, o_as_value);
        // 5. For each element E of items, do
        for e in items {
            // a. Let spreadable be ? IsConcatSpreadable(E).
            let e_is_spreadable = is_concat_spreadable(agent, e.clone(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. If spreadable is true, then
            if let Some(spreadable_e) = e_is_spreadable {
                // i. Let len be ? LengthOfArrayLike(E).
                let len =
                    length_of_array_like(agent, spreadable_e.unbind(), gc.reborrow()).unbind()?;
                // ii. If n + len > 2**53 - 1, throw a TypeError exception.
                if (n + len) > SmallInteger::MAX {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Array overflow",
                        gc.into_nogc(),
                    ));
                }
                // iii. Let k be 0.
                let mut k = 0;
                // iv. Repeat, while k < len,
                while k < len {
                    // 1. Let Pk be ! ToString(𝔽(k)).
                    let pk = PropertyKey::Integer(k.try_into().unwrap());
                    // 2. Let exists be ? HasProperty(E, Pk).
                    let exists = has_property(
                        agent,
                        Object::try_from(e.get(agent)).unwrap(),
                        pk,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // 3. If exists is true, then
                    if exists {
                        // a. Let subElement be ? Get(E, Pk).
                        let sub_element = get(
                            agent,
                            Object::try_from(e.get(agent)).unwrap(),
                            pk,
                            gc.reborrow(),
                        )
                        .unbind()?
                        .bind(gc.nogc());
                        // b. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), subElement).
                        create_data_property_or_throw(
                            agent,
                            a.get(agent),
                            PropertyKey::Integer(n.try_into().unwrap()),
                            sub_element.unbind(),
                            gc.reborrow(),
                        )
                        .unbind()?;
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
                if n >= SmallInteger::MAX {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Array overflow",
                        gc.into_nogc(),
                    ));
                }
                // iii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), E).
                create_data_property_or_throw(
                    agent,
                    a.get(agent),
                    PropertyKey::Integer(n.try_into().unwrap()),
                    e.get(agent),
                    gc.reborrow(),
                )
                .unbind()?;
                // iv. Set n to n + 1.
                n += 1;
            }
        }
        // 6. Perform ? Set(A, "length", 𝔽(n), true).
        set(
            agent,
            a.get(agent),
            BUILTIN_STRING_MEMORY.length.into(),
            Value::try_from(n).unwrap(),
            true,
            gc.reborrow(),
        )
        .unbind()?;
        // 7. Return A.
        Ok(a.get(agent).into_value())
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
    fn copy_within<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let target = arguments.get(0).bind(nogc);
        let start = arguments.get(1).bind(nogc);
        let end = if arguments.len() >= 3 {
            Some(arguments.get(2).bind(nogc))
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
                if count <= 0 {
                    return Ok(array.into_value().unbind());
                }
                let data = array.as_mut_slice(agent);
                data.copy_within((from as usize)..((from + count) as usize), to as usize);

                return Ok(array.into_value().unbind());
            }
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());

        let end = end.map(|e| e.scope(agent, nogc));
        // Note: Start is second to last to be scoped.
        let start = start.scope(agent, nogc);
        // Note: Target is last to be scoped.
        let target = target.scope(agent, nogc);

        // 2. Let len be ? LengthOfArrayLike(O).
        let len: i64 = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;

        // 3. Let relativeTarget be ? ToIntegerOrInfinity(target).
        // SAFETY: target has not been shared.
        let relative_target =
            to_integer_or_infinity(agent, unsafe { target.take(agent) }, gc.reborrow()).unbind()?;

        let to = if relative_target.is_neg_infinity() {
            // 4. If relativeTarget = -∞, let to be 0.
            0
        } else if relative_target.is_negative() {
            // 5. Else if relativeTarget < 0, let to be max(len + relativeTarget, 0).
            (len + relative_target.into_i64()).max(0)
        } else {
            // 6. Else, let to be min(relativeTarget, len).
            relative_target.into_i64().min(len)
        };

        // 7. Let relativeStart be ? ToIntegerOrInfinity(start).
        // SAFETY: start has not been shared.
        let relative_start =
            to_integer_or_infinity(agent, unsafe { start.take(agent) }, gc.reborrow()).unbind()?;

        let from = if relative_start.is_neg_infinity() {
            // 8. If relativeStart = -∞, let from be 0.
            0
        } else if relative_start.is_negative() {
            // 9. Else if relativeStart < 0, let from be max(len + relativeStart, 0).
            (len + relative_start.into_i64()).max(0)
        } else {
            // 10. Else, let from be min(relativeStart, len).
            relative_start.into_i64().min(len)
        };

        // 11. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
        // SAFETY: end has not been shared.
        let end = end.map(|e| unsafe { e.take(agent) }.bind(gc.nogc()));
        let final_end = if end.is_none() || end.unwrap().is_undefined() {
            len
        } else {
            let relative_end =
                to_integer_or_infinity(agent, end.unwrap().unbind(), gc.reborrow()).unbind()?;
            // 12. If relativeEnd = -∞, let final be 0.
            if relative_end.is_neg_infinity() {
                0
            } else if relative_end.is_negative() {
                // 13. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
                (len + relative_end.into_i64()).max(0)
            } else {
                // 14. Else, let final be min(relativeEnd, len).
                relative_end.into_i64().min(len)
            }
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
            let from_present =
                has_property(agent, o.get(agent), from_key, gc.reborrow()).unbind()?;
            // d. If fromPresent is true, then
            if from_present {
                // i. Let fromValue be ? Get(O, fromKey).
                let from_value = get(agent, o.get(agent), from_key, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Perform ? Set(O, toKey, fromValue, true).
                set(
                    agent,
                    o.get(agent),
                    to_key,
                    from_value.unbind(),
                    true,
                    gc.reborrow(),
                )
                .unbind()?;
            } else {
                // e. Else,
                // i. Assert: fromPresent is false.
                // ii. Perform ? DeletePropertyOrThrow(O, toKey).
                delete_property_or_throw(agent, o.get(agent), to_key, gc.reborrow()).unbind()?;
            }
            // f. Set from to from + direction.
            from += direction;
            // g. Set to to to + direction.
            to += direction;
            // h. Set count to count - 1.
            count -= 1;
        }
        // 19. Return O.
        Ok(o.get(agent).into_value())
    }

    fn entries<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? ToObject(this value).
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected this to be an object",
                gc.into_nogc(),
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
    fn every<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(stack_callback_fn) = is_callable(callback_fn.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not a function",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn never escapes this call.
        let callback_fn = unsafe { callback_fn.replace_self(agent, stack_callback_fn.unbind()) };
        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Let testResult be ToBoolean(? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »)).
                let f_k = Number::try_from(k).unwrap().into_value();
                let test_result = call_function(
                    agent,
                    callback_fn.get(agent),
                    this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [k_value.unbind(), f_k])),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
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
    fn fill<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let value = arguments.get(0).bind(nogc);
        let start = arguments.get(1).bind(nogc);
        let end = arguments.get(2).bind(nogc);
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
                data[k..final_end].fill(Some(value.unbind()));
                return Ok(value.into_value().unbind());
            }
        };
        let value = value.scope(agent, nogc);
        let start = start.scope(agent, nogc);
        let end = end.scope(agent, nogc);
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start =
            to_integer_or_infinity(agent, start.get(agent), gc.reborrow()).unbind()?;

        // 4. If relativeStart = -∞, let k be 0.
        let mut k = if relative_start.is_neg_infinity() {
            0
        } else if relative_start.is_negative() {
            // 5. Else if relativeStart < 0, let k be max(len + relativeStart, 0).
            (len + relative_start.into_i64()).max(0)
        } else {
            // 6. Else, let k be min(relativeStart, len).
            len.min(relative_start.into_i64())
        };

        // 7. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
        let final_end = if end.get(agent).is_undefined() {
            len
        } else {
            let relative_end =
                to_integer_or_infinity(agent, end.get(agent), gc.reborrow()).unbind()?;
            // 8. If relativeEnd = -∞, let final be 0.
            if relative_end.is_neg_infinity() {
                0
            } else if relative_end.is_negative() {
                // 9. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
                (len + relative_end.into_i64()).max(0)
            } else {
                // 10. Else, let final be min(relativeEnd, len).
                len.min(relative_end.into_i64())
            }
        };

        // 11. Repeat, while k < final,
        while k < final_end {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Perform ? Set(O, Pk, value, true).
            set(
                agent,
                o.get(agent).unbind(),
                pk,
                value.get(agent).unbind(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
            // c. Set k to k + 1.
            k += 1;
        }
        // 12. Return O.
        Ok(o.get(agent).into_value())
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
    fn filter<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(stack_callback_fn) = is_callable(callback_fn.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not callable",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn never escapes this call.
        let callback_fn = unsafe { callback_fn.replace_self(agent, stack_callback_fn.unbind()) };
        // 4. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o.get(agent), 0, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Let to be 0.
        let mut to: u32 = 0;
        let mut scoped_k_value: Scoped<Value> = Value::Undefined.scope_static(gc.nogc());
        // 7. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::from(SmallInteger::try_from(k).unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // SAFETY: scoped_k_value never escapes this call
                unsafe { scoped_k_value.replace(agent, k_value.unbind()) };
                // ii. Let selected be ToBoolean(? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »)).
                let result = call_function(
                    agent,
                    callback_fn.get(agent),
                    this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        k_value.unbind(),
                        k.try_into().unwrap(),
                        o.get(agent).into_value(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let selected = to_boolean(agent, result);
                // iii. If selected is true, then
                if selected {
                    // 1. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(to)), kValue).
                    create_data_property_or_throw(
                        agent,
                        a.get(agent),
                        to.into(),
                        scoped_k_value.get(agent),
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // 2. Set to to to + 1.
                    to += 1;
                }
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 8. Return A.
        Ok(a.get(agent).into_value())
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
    fn find<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let predicate = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, true, predicate, this_arg, gc)?;
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
    fn find_index<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let predicate = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, true, predicate, this_arg, gc)?;
        // 4. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into_value())
    }

    /// ### [23.1.3.11 Array.prototype.findLast ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.findlast)
    fn find_last<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let predicate = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let findRec be ? FindViaPredicate(O, len, descending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg, gc)?;
        // 4. Return findRec.[[Value]].
        Ok(find_rec.1)
    }

    /// ### [23.1.3.12 Array.prototype.findLastIndex ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.findlastindex)
    fn find_last_index<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let predicate = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let findRec be ? FindViaPredicate(O, len, descending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg, gc)?;
        // 4. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into_value())
    }

    /// ### [23.1.3.13 Array.prototype.flat ( \[ depth \] )]()
    fn flat<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let depth = arguments.get(0).scope(agent, nogc);
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let sourceLen be ? LengthOfArrayLike(O).
        let source_len =
            length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()? as usize;
        // 3. Let depthNum be 1.
        let mut depth_num = 1;
        // 4. If depth is not undefined, then
        if !depth.get(agent).is_undefined() {
            // a. Set depthNum to ? ToIntegerOrInfinity(depth).
            depth_num = to_integer_or_infinity(agent, depth.get(agent), gc.reborrow())
                .unbind()?
                .into_i64();
        }
        // b. If depthNum < 0, set depthNum to 0.
        if depth_num < 0 {
            depth_num = 0;
        }
        // 5. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o.get(agent), 0, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 6. Perform ? FlattenIntoArray(A, O, sourceLen, 0, depthNum).
        flatten_into_array(
            agent,
            a.clone(),
            o,
            source_len,
            0,
            Some(depth_num as usize),
            None,
            None,
            gc.reborrow(),
        )
        .unbind()?;
        // 7. Return A.
        Ok(a.get(agent).into_value())
    }

    /// ### [23.1.3.14 Array.prototype.flatMap ( mapperFunction \[ , thisArg \] )](https://tc39.es/ecma262/#sec-array.prototype.flatmap)
    fn flat_map<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let mapper_function = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let sourceLen be ? LengthOfArrayLike(O).
        let source_len =
            length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()? as usize;
        // 3. If IsCallable(mapperFunction) is false, throw a TypeError exception.
        let Some(stack_mapper_function) = is_callable(mapper_function.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Mapper function is not callable",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn is not shared.
        let mapper_function =
            unsafe { mapper_function.replace_self(agent, stack_mapper_function.unbind()) };

        // 4. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o.get(agent), 0, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 5. Perform ? FlattenIntoArray(A, O, sourceLen, 0, 1, mapperFunction, thisArg).
        flatten_into_array(
            agent,
            a.clone(),
            o,
            source_len,
            0,
            Some(1),
            Some(mapper_function),
            Some(this_arg),
            gc.reborrow(),
        )
        .unbind()?;
        // 6. Return A.
        Ok(a.get(agent).into_value())
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
    fn for_each<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;

        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(stack_callback_fn) = is_callable(callback_fn.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn is not shared.
        let callback_fn = unsafe { callback_fn.replace_self(agent, stack_callback_fn.unbind()) };

        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Perform ? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »).
                call_function(
                    agent,
                    callback_fn.get(agent),
                    this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        k_value.unbind(),
                        k.try_into().unwrap(),
                        o.get(agent).into_value(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?;
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
    fn includes<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_element = arguments.get(0).bind(nogc);
        let from_index = arguments.get(1).bind(nogc);
        if let (Value::Array(array), Value::Undefined | Value::Integer(_)) =
            (this_value, from_index)
        {
            let len = array.len(agent);
            if len == 0 {
                return Ok(false.into());
            }

            let k = if let Value::Integer(n) = from_index {
                let n = n.into_i64();

                if n >= len as i64 {
                    return Ok(false.into());
                }

                if n >= 0 {
                    n as usize
                } else {
                    let result = len as i64 + n;
                    if result < 0 { 0 } else { result as usize }
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
        let from_index_is_undefined = from_index.is_undefined();
        let from_index = from_index.scope(agent, nogc);
        let search_element = search_element.scope(agent, nogc);
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If len = 0, return false.
        if len == 0 {
            return Ok(false.into());
        }
        // 4. Let n be ? ToIntegerOrInfinity(fromIndex).
        let n = to_integer_or_infinity(agent, from_index.get(agent), gc.reborrow()).unbind()?;
        // 5. Assert: If fromIndex is undefined, then n is 0.
        if from_index_is_undefined {
            assert_eq!(n.into_i64(), 0);
        }
        // 6. If n = +∞, return false.
        let n = if n.is_pos_infinity() {
            return Ok(false.into());
        } else if n.is_neg_infinity() {
            // 7. Else if n = -∞, set n to 0.
            0
        } else {
            n.into_i64()
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
            if k < 0 { 0 } else { k }
        };
        // 10. Repeat, while k < len,
        while k < len {
            // a. Let elementK be ? Get(O, ! ToString(𝔽(k))).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            let element_k = get(agent, o.get(agent), pk, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. If SameValueZero(searchElement, elementK) is true, return true.
            if same_value_zero(agent, search_element.get(agent), element_k) {
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
    fn index_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_element = arguments.get(0).bind(nogc);
        let from_index = arguments.get(1).bind(nogc);
        if let (Value::Array(array), Value::Undefined | Value::Integer(_)) =
            (this_value, from_index)
        {
            let len = array.len(agent);
            if len == 0 {
                return Ok((-1).into());
            }

            let k = if let Value::Integer(n) = from_index {
                let n = n.into_i64();

                if n >= len as i64 {
                    return Ok((-1).into());
                }

                if n >= 0 {
                    n as usize
                } else {
                    let result = len as i64 + n;
                    if result < 0 { 0 } else { result as usize }
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
        let from_index_is_undefined = from_index.is_undefined();
        let from_index = from_index.scope(agent, nogc);
        let search_element = search_element.scope(agent, nogc);
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If len = 0, return -1𝔽.
        if len == 0 {
            return Ok((-1).into());
        }
        // 4. Let n be ? ToIntegerOrInfinity(fromIndex).
        let n = to_integer_or_infinity(agent, from_index.get(agent), gc.reborrow()).unbind()?;
        // 5. Assert: If fromIndex is undefined, then n is 0.
        if from_index_is_undefined {
            assert_eq!(n.into_i64(), 0);
        }
        // 6. If n = +∞, return -1𝔽.
        let n = if n.is_pos_infinity() {
            return Ok((-1).into());
        } else if n.is_neg_infinity() {
            // 7. Else if n = -∞, set n to 0.
            0
        } else {
            n.into_i64()
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
            if k < 0 { 0 } else { k }
        };
        // 10. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let elementK be ? Get(O, Pk).
                let element_k = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. If IsStrictlyEqual(searchElement, elementK) is true, return 𝔽(k).
                if is_strictly_equal(agent, search_element.get(agent), element_k) {
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
    fn join<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let separator = arguments.get(0).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        if len == 0 {
            return Ok(String::EMPTY_STRING.into_value());
        }
        let len = len as usize;
        // 3. If separator is undefined, let sep be ",".
        let separator = if separator.get(agent).is_undefined() {
            String::from_small_string(",").scope_static()
        } else {
            // 4. Else, let sep be ? ToString(separator).
            let sep = to_string(agent, separator.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: separator is not shared.
            // Note: Separator is likely a small string so this is a very cheap.
            unsafe { separator.replace_self(agent, sep.unbind()) }
        };
        // 5. Let R be the empty String.
        let mut r = Wtf8Buf::with_capacity(len * 10);
        // 6. Let k be 0.
        // 7. Repeat, while k < len,
        // b. Let element be ? Get(O, ! ToString(𝔽(k))).
        {
            let element = get(agent, o.get(agent), 0.into(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // c. If element is neither undefined nor null, then
            if !element.is_undefined() && !element.is_null() {
                // i. Let S be ? ToString(element).
                let s = to_string(agent, element.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Set R to the string-concatenation of R and S.
                r.push_wtf8(s.as_wtf8(agent));
            }
        }
        for k in 1..len {
            // a. If k > 0, set R to the string-concatenation of R and sep.
            r.push_wtf8(separator.get(agent).as_wtf8(agent));
            // b. Let element be ? Get(O, ! ToString(𝔽(k))).
            let element = get(
                agent,
                o.get(agent),
                SmallInteger::try_from(k as u64).unwrap().into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // c. If element is neither undefined nor null, then
            if !element.is_undefined() && !element.is_null() {
                // i. Let S be ? ToString(element).
                let s = to_string(agent, element.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Set R to the string-concatenation of R and S.
                r.push_wtf8(s.as_wtf8(agent));
            }
            // d. Set k to k + 1.
        }
        // 8. Return R.
        Ok(String::from_wtf8_buf(agent, r, gc.into_nogc()).into_value())
    }

    fn keys<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? ToObject(this value).
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected this to be an object",
                gc.into_nogc(),
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
    fn last_index_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_element = arguments.get(0).bind(nogc);
        let from_index = if arguments.len() > 1 {
            Some(arguments.get(1).bind(nogc))
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
                    if result < 0 { 0 } else { result as usize }
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
        let from_index = from_index.map(|i| i.scope(agent, nogc));
        let search_element = search_element.scope(agent, nogc);
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If len = 0, return -1𝔽.
        if len == 0 {
            return Ok((-1).into());
        }
        // 4. If fromIndex is present, let n be ? ToIntegerOrInfinity(fromIndex); else let n be len - 1.
        let mut k = if let Some(from_index) = from_index {
            let n = to_integer_or_infinity(agent, from_index.get(agent), gc.reborrow()).unbind()?;
            // 5. If n = -∞, return -1𝔽.
            if n.is_neg_infinity() {
                return Ok((-1).into());
            }
            // 6. If n ≥ 0, then
            if n.into_i64() >= 0 {
                // a. Let k be min(n, len - 1).
                n.into_i64().min(len - 1)
            } else {
                // Note: n is negative, so n < len + n < len.
                // 7. Else,
                // a. Let k be len + n.
                len + n.into_i64()
            }
        } else {
            len - 1
        };

        // 8. Repeat, while k ≥ 0,
        while k >= 0 {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let elementK be ? Get(O, Pk).
                let element_k = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. If IsStrictlyEqual(searchElement, elementK) is true, return 𝔽(k).
                if is_strictly_equal(agent, search_element.get(agent), element_k) {
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
    fn map<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(stack_callback_fn) = is_callable(callback_fn.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn is not shared.
        let callback_fn = unsafe { callback_fn.replace_self(agent, stack_callback_fn.unbind()) };
        // 4. Let A be ? ArraySpeciesCreate(O, len).
        let a = array_species_create(agent, o.get(agent), len as usize, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k.try_into().unwrap());
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Let mappedValue be ? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »).
                let mapped_value = call_function(
                    agent,
                    callback_fn.get(agent),
                    this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        k_value.unbind(),
                        k.try_into().unwrap(),
                        o.get(agent).into_value(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?;
                // iii. Perform ? CreateDataPropertyOrThrow(A, Pk, mappedValue).
                create_data_property_or_throw(
                    agent,
                    a.get(agent),
                    pk,
                    mapped_value.unbind(),
                    gc.reborrow(),
                )
                .unbind()?;
            }
            // d. Set k to k + 1.
            k += 1;
        }
        // 7. Return A.
        Ok(a.get(agent).into_value())
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
    fn pop<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
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
                            gc.into_nogc(),
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
                            gc.into_nogc(),
                        ));
                    }
                    return Ok(last_element);
                }
                // Last element was a hole; this means we'd need to look into
                // the prototype chain. We're not going to do that.
            }
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If len = 0, then
        if len == 0 {
            // a. Perform ? Set(O, "length", +0𝔽, true).
            set(
                agent,
                o.get(agent),
                BUILTIN_STRING_MEMORY.length.into(),
                0.into(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
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
            let element = get(agent, o.get(agent), index, gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());
            // e. Perform ? DeletePropertyOrThrow(O, index).
            delete_property_or_throw(agent, o.get(agent), index, gc.reborrow()).unbind()?;
            // f. Perform ? Set(O, "length", newLen, true).
            set(
                agent,
                o.get(agent),
                BUILTIN_STRING_MEMORY.length.into(),
                new_len.try_into().unwrap(),
                true,
                gc,
            )?;
            // g. Return element.
            Ok(element.get(agent))
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
    fn push<'gc>(
        agent: &mut Agent,
        this_value: Value,
        items: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let mut len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let argCount be the number of elements in items.
        let arg_count = items.len();
        // 4. If len + argCount > 2**53 - 1, throw a TypeError exception.
        if (len + arg_count as i64) > SmallInteger::MAX {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length overflow",
                gc.into_nogc(),
            ));
        }
        if let Object::Array(array) = o.get(agent) {
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
                o.get(agent),
                PropertyKey::Integer(len.try_into().unwrap()),
                *e,
                true,
                gc.reborrow(),
            )
            .unbind()?;
            // b. Set len to len + 1.
            len += 1;
        }
        // 6. Perform ? Set(O, "length", 𝔽(len), true).
        let len: Value = len.try_into().unwrap();
        set(
            agent,
            o.get(agent),
            BUILTIN_STRING_MEMORY.length.into(),
            len,
            true,
            gc.reborrow(),
        )
        .unbind()?;

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
    fn reduce<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).scope(agent, nogc);
        let initial_value = if arguments.len() >= 2 {
            Some(arguments.get(1).scope(agent, nogc))
        } else {
            None
        };

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;

        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(stack_callback_fn) = is_callable(callback_fn.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn is not shared.
        let callback_fn = unsafe { callback_fn.replace_self(agent, stack_callback_fn.unbind()) };

        // 4. If len = 0 and initialValue is not present, throw a TypeError exception.
        if len == 0 && initial_value.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length is 0 and no initial value provided",
                gc.into_nogc(),
            ));
        }

        // 5. Let k be 0.
        let mut k = 0;
        // 6. Let accumulator be undefined.
        // 7. If initialValue is present,
        // a. Set accumulator to initialValue.
        let initial_value_is_none = initial_value.is_none();
        let mut accumulator = initial_value.unwrap_or(Value::Undefined.scope_static(gc.nogc()));

        // 8. Else,
        if initial_value_is_none {
            // a. Let kPresent be false.
            let mut k_present = false;

            // b. Repeat, while kPresent is false and k < len,
            while !k_present && k < len {
                // i. Let Pk be ! ToString(𝔽(k)).
                let pk = PropertyKey::Integer(k.try_into().unwrap());

                // ii. Set kPresent to ? HasProperty(O, Pk).
                k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;

                // iii. If kPresent is true, then
                if k_present {
                    // 1. Set accumulator to ? Get(O, Pk).
                    let result = get(agent, o.get(agent), pk, gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());
                    // SAFETY: accumulator is not shared.
                    unsafe { accumulator.replace(agent, result.unbind()) };
                }

                // iv. Set k to k + 1.
                k += 1;
            }

            // c. If kPresent is false, throw a TypeError exception.
            if !k_present {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Array length is 0 and no initial value provided",
                    gc.into_nogc(),
                ));
            }
        }

        // 9. Repeat, while k < len,
        while k < len {
            let k_int = k.try_into().unwrap();
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k_int);

            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;

            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());

                // ii. Set accumulator to ? Call(callbackfn, undefined, « accumulator, kValue, 𝔽(k), O »).
                let result = call_function(
                    agent,
                    callback_fn.get(agent),
                    Value::Undefined,
                    Some(ArgumentsList::from_mut_slice(&mut [
                        accumulator.get(agent),
                        k_value.unbind(),
                        Number::from(k_int).into_value(),
                        o.get(agent).into_value(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: accumulator is not shared.
                unsafe { accumulator.replace(agent, result.unbind()) };
            }

            // d. Set k to k + 1.
            k += 1;
        }

        // 10. Return accumulator.
        Ok(accumulator.get(agent))
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
    fn reduce_right<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).scope(agent, nogc);
        let initial_value = if arguments.len() >= 2 {
            Some(arguments.get(1).scope(agent, nogc))
        } else {
            None
        };

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);

        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;

        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(stack_callback_fn) = is_callable(callback_fn.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not a function",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn is not shared outside this call.
        let callback_fn = unsafe { callback_fn.replace_self(agent, stack_callback_fn.unbind()) };

        // 4. If len = 0 and initialValue is not present, throw a TypeError exception.
        if len == 0 && initial_value.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length is 0 and no initial value provided",
                gc.into_nogc(),
            ));
        }

        // 5. Let k be len - 1.
        let mut k = len - 1;
        // 6. Let accumulator be undefined.
        // 7. If initialValue is present, then
        // a. Set accumulator to initialValue.
        let no_initial_value = initial_value.is_none();
        let mut accumulator = initial_value.unwrap_or(Value::Undefined.scope_static(gc.nogc()));

        // 8. Else,
        if no_initial_value {
            // a. Let kPresent be false.
            let mut k_present = false;

            // b. Repeat, while kPresent is false and k ≥ 0,
            while !k_present && k >= 0 {
                // i. Let Pk be ! ToString(𝔽(k)).
                let pk = PropertyKey::try_from(k).unwrap();

                // ii. Set kPresent to ? HasProperty(O, Pk).
                k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;

                // iii. If kPresent is true, then
                if k_present {
                    // 1. Set accumulator to ? Get(O, Pk).
                    let result = get(agent, o.get(agent), pk, gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());
                    // SAFETY: Accumulator is not shared outside this call.
                    unsafe { accumulator.replace(agent, result.unbind()) };
                }

                // iv. Set k to k - 1.
                k -= 1;
            }

            // c. If kPresent is false, throw a TypeError exception.
            if !k_present {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Array length is 0 and no initial value provided",
                    gc.into_nogc(),
                ));
            }
        }

        // 9. Repeat, while k ≥ 0,
        while k >= 0 {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();

            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;

            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());

                // ii. Set accumulator to ? Call(callbackfn, undefined, « accumulator, kValue, 𝔽(k), O »).
                let result = call_function(
                    agent,
                    callback_fn.get(agent),
                    Value::Undefined,
                    Some(ArgumentsList::from_mut_slice(&mut [
                        accumulator.get(agent),
                        k_value.unbind(),
                        Number::try_from(k).unwrap().into(),
                        o.get(agent).into_value(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: Accumulator is not shared outside this call.
                unsafe { accumulator.replace(agent, result.unbind()) };
            }

            // d. Set k to k - 1.
            k -= 1;
        }

        // 10. Return accumulator.
        Ok(accumulator.get(agent))
    }

    fn reverse<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        if let Value::Array(array) = this_value {
            // Fast path: Array is dense and contains no descriptors. No JS
            // functions can thus be called by shift.
            if array.is_trivial(agent) && array.is_dense(agent) {
                array.as_mut_slice(agent).reverse();
                return Ok(array.into_value().unbind());
            }
        }

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let middle be floor(len / 2).
        let middle = len / 2;
        // 4. Let lower be 0.
        let mut lower: i64 = 0;
        // 5. Repeat, while lower ≠ middle,
        while lower != middle {
            // a. Let upper be len - lower - 1.
            let upper = len - lower - 1;
            // b. Let upperP be ! ToString(𝔽(upper)).
            let upper_p = PropertyKey::Integer(upper.try_into().unwrap());
            // c. Let lowerP be ! ToString(𝔽(lower)).
            let lower_p = PropertyKey::Integer(lower.try_into().unwrap());
            // d. Let lowerExists be ? HasProperty(O, lowerP).
            let lower_exists =
                has_property(agent, o.get(agent), lower_p, gc.reborrow()).unbind()?;
            // e. If lowerExists is true, then
            let lower_value = if lower_exists {
                // i. Let lowerValue be ? Get(O, lowerP).
                Some(
                    get(agent, o.get(agent), lower_p, gc.reborrow())
                        .unbind()?
                        .scope(agent, gc.nogc()),
                )
            } else {
                None
            };
            // f. Let upperExists be ? HasProperty(O, upperP).
            let upper_exists =
                has_property(agent, o.get(agent), upper_p, gc.reborrow()).unbind()?;
            // g. If upperExists is true, then
            let upper_value = if upper_exists {
                // i. Let upperValue be ? Get(O, upperP).
                Some(
                    get(agent, o.get(agent), upper_p, gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc()),
                )
            } else {
                None
            };

            match (lower_value, upper_value) {
                // h. If lowerExists is true and upperExists is true, then
                (Some(lower_value), Some(upper_value)) => {
                    // i. Perform ? Set(O, lowerP, upperValue, true).
                    set(
                        agent,
                        o.get(agent),
                        lower_p,
                        upper_value.unbind(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // ii. Perform ? Set(O, upperP, lowerValue, true).
                    set(
                        agent,
                        o.get(agent),
                        upper_p,
                        lower_value.get(agent),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                }
                // i. Else if lowerExists is false and upperExists is true, then
                (None, Some(upper_value)) => {
                    // i. Perform ? Set(O, lowerP, upperValue, true).
                    set(
                        agent,
                        o.get(agent),
                        lower_p,
                        upper_value.unbind(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // ii. Perform ? DeletePropertyOrThrow(O, upperP).
                    delete_property_or_throw(agent, o.get(agent), upper_p, gc.reborrow())
                        .unbind()?;
                }
                // j. Else if lowerExists is true and upperExists is false, then
                (Some(lower_value), None) => {
                    // i. Perform ? DeletePropertyOrThrow(O, lowerP).
                    delete_property_or_throw(agent, o.get(agent), lower_p, gc.reborrow())
                        .unbind()?;
                    // ii. Perform ? Set(O, upperP, lowerValue, true).
                    set(
                        agent,
                        o.get(agent),
                        upper_p,
                        lower_value.get(agent),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                }
                // k. Else,
                (None, None) => {
                    // i. Assert: lowerExists and upperExists are both false.
                    // ii. NOTE: No action is required.
                    assert!(!(lower_exists && upper_exists));
                }
            }
            //    l. Set lower to lower + 1.
            lower += 1;
        }
        // 6. Return O.
        Ok(o.get(agent).into_value())
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
    fn shift<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        if let Value::Array(array) = this_value {
            if array.is_empty(agent) {
                if agent[array].elements.len_writable {
                    return Ok(Value::Undefined);
                } else {
                    // This will throw
                    set(
                        agent,
                        array.into_object().unbind(),
                        BUILTIN_STRING_MEMORY.length.into(),
                        0.into(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    unreachable!();
                }
            }
            if array.is_trivial(agent) && array.is_dense(agent) {
                // Fast path: Array is dense and contains no descriptors. No JS
                // functions can thus be called by shift.
                let slice = array.as_mut_slice(agent);
                let first = slice[0].unwrap().bind(gc.nogc());
                slice.copy_within(1.., 0);
                *slice.last_mut().unwrap() = None;
                let array_data = &mut agent[array];
                if array_data.elements.len_writable {
                    array_data.elements.len -= 1;
                    return Ok(first.unbind());
                } else {
                    // This will throw
                    set(
                        agent,
                        array.into_object().unbind(),
                        BUILTIN_STRING_MEMORY.length.into(),
                        (array.len(agent) - 1).into(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    unreachable!();
                }
            }
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If len = 0, then
        if len == 0 {
            // a. Perform ? Set(O, "length", +0𝔽, true).
            set(
                agent,
                o.get(agent),
                BUILTIN_STRING_MEMORY.length.into(),
                0.into(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
            // b. Return undefined.
            return Ok(Value::Undefined);
        }
        // 4. Let first be ? Get(O, "0").
        let first = get(agent, o.get(agent), 0.into(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 5. Let k be 1.
        let mut k = 1;
        // 6. Repeat, while k < len,
        while k < len {
            // a. Let from be ! ToString(𝔽(k)).
            let from = k.try_into().unwrap();
            // b. Let to be ! ToString(𝔽(k - 1)).
            let to = (k - 1).try_into().unwrap();
            // c. Let fromPresent be ? HasProperty(O, from).
            let from_present = has_property(agent, o.get(agent), from, gc.reborrow()).unbind()?;
            // d. If fromPresent is true, then
            if from_present {
                // i. Let fromValue be ? Get(O, from).
                let from_value = get(agent, o.get(agent), from, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Perform ? Set(O, to, fromValue, true).
                set(
                    agent,
                    o.get(agent),
                    to,
                    from_value.unbind(),
                    true,
                    gc.reborrow(),
                )
                .unbind()?;
            } else {
                // e. Else,
                // i. Assert: fromPresent is false.
                // ii. Perform ? DeletePropertyOrThrow(O, to).
                delete_property_or_throw(agent, o.get(agent), to, gc.reborrow()).unbind()?;
            }
            // f. Set k to k + 1.
            k += 1;
        }
        // 7. Perform ? DeletePropertyOrThrow(O, ! ToString(𝔽(len - 1))).
        delete_property_or_throw(
            agent,
            o.get(agent),
            (len - 1).try_into().unwrap(),
            gc.reborrow(),
        )
        .unbind()?;
        // 8. Perform ? Set(O, "length", 𝔽(len - 1), true).
        set(
            agent,
            o.get(agent),
            BUILTIN_STRING_MEMORY.length.into(),
            (len - 1).try_into().unwrap(),
            true,
            gc.reborrow(),
        )
        .unbind()?;
        // 9. Return first.
        Ok(first.get(agent))
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
    fn slice<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let start = arguments.get(0).bind(nogc);
        let end = arguments.get(1).bind(nogc);
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
                let array = array.scope(agent, nogc);
                let count = end.saturating_sub(start);
                let a = array_species_create(
                    agent,
                    array.get(agent).into_object(),
                    count,
                    gc.reborrow(),
                )
                .unbind()?
                .scope(agent, gc.nogc());
                if count == 0 {
                    set(
                        agent,
                        a.get(agent),
                        BUILTIN_STRING_MEMORY.length.into(),
                        0.into(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    return Ok(a.get(agent).into_value());
                }
                if let Object::Array(a) = a.get(agent)
                    && a.len(agent) as usize == count
                    && a.is_trivial(agent)
                    && a.as_slice(agent).iter().all(|el| el.is_none())
                {
                    // Array full of holes
                    let source_data = array.get(agent).as_slice(agent)[start..end].as_ptr();
                    let destination_data = a.as_mut_slice(agent).as_mut_ptr();
                    // SAFETY: Source and destination are properly aligned
                    // and valid for reads/writes. They do not overlap.
                    // From JS point of view, setting data properties to
                    // the destination would not call any JS code so this
                    // is spec-wise correct.
                    unsafe { core::ptr::copy_nonoverlapping(source_data, destination_data, count) };
                    set(
                        agent,
                        a.into_object(),
                        BUILTIN_STRING_MEMORY.length.into(),
                        Number::try_from(count).unwrap().into_value(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    return Ok(a.into_value());
                }
                let mut k = start;
                let mut n = 0u32;
                while k < end {
                    // a. Let Pk be ! ToString(𝔽(k)).
                    // b. Let kPresent be ? HasProperty(O, Pk).
                    // Note: Array is dense, we do not need to check this.
                    // c. If kPresent is true, then
                    // i. Let kValue be ? Get(O, Pk).
                    let k_value = array.get(agent).as_slice(agent)[k].unwrap();
                    // ii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), kValue).
                    create_data_property_or_throw(
                        agent,
                        a.get(agent),
                        n.into(),
                        k_value,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // d. Set k to k + 1.
                    k += 1;
                    // e. Set n to n + 1.
                    n += 1;
                }
                // 15. Perform ? Set(A, "length", 𝔽(n), true).
                set(
                    agent,
                    a.get(agent).into_object(),
                    BUILTIN_STRING_MEMORY.length.into(),
                    n.into(),
                    true,
                    gc.reborrow(),
                )
                .unbind()?;
                // 16. Return A.
                return Ok(a.get(agent).into_value());
            }
        }
        let start = start.scope(agent, nogc);
        let end = end.scope(agent, nogc);
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()? as usize;
        // 3. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start =
            to_integer_or_infinity(agent, start.get(agent), gc.reborrow()).unbind()?;
        // 4. If relativeStart = -∞, let k be 0.
        let mut k = if relative_start.is_neg_infinity() {
            0
        } else if relative_start.is_negative() {
            // 5. Else if relativeStart < 0, let k be max(len + relativeStart, 0).
            (len as i64 + relative_start.into_i64()).max(0) as usize
        } else {
            // 6. Else, let k be min(relativeStart, len).
            (relative_start.into_i64() as usize).min(len)
        };

        // 7. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
        let end = end.get(agent).bind(gc.nogc());
        let final_end = if end.is_undefined() {
            len
        } else {
            let relative_end =
                to_integer_or_infinity(agent, end.unbind(), gc.reborrow()).unbind()?;
            // 8. If relativeEnd = -∞, let final be 0.
            if relative_end.is_neg_infinity() {
                0
            } else if relative_end.is_negative() {
                // 9. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
                (len as i64 + relative_end.into_i64()).max(0) as usize
            } else {
                // 10. Else, let final be min(relativeEnd, len).
                (relative_end.into_i64() as usize).min(len)
            }
        };
        // 11. Let count be max(final - k, 0).
        let count = final_end.saturating_sub(k);
        // 12. Let A be ? ArraySpeciesCreate(O, count).
        let a = array_species_create(agent, o.get(agent), count, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 13. Let n be 0.
        let mut n = 0u32;
        // 14. Repeat, while k < final,
        while k < final_end {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = k.try_into().unwrap();
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), kValue).
                create_data_property_or_throw(
                    agent,
                    a.get(agent),
                    n.into(),
                    k_value.unbind(),
                    gc.reborrow(),
                )
                .unbind()?;
            }
            // d. Set k to k + 1.
            k += 1;
            // e. Set n to n + 1.
            n += 1;
        }
        // 15. Perform ? Set(A, "length", 𝔽(n), true).
        set(
            agent,
            a.get(agent),
            BUILTIN_STRING_MEMORY.length.into(),
            n.into(),
            true,
            gc.reborrow(),
        )
        .unbind()?;
        // 16. Return A.
        Ok(a.get(agent).into_value())
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
    fn some<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback_fn = arguments.get(0).scope(agent, nogc);
        let this_arg = arguments.get(1).scope(agent, nogc);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        let Some(stack_callback_fn) = is_callable(callback_fn.get(agent), gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback function is not callable",
                gc.into_nogc(),
            ));
        };
        // SAFETY: callback_fn is never shared before this call.
        let callback_fn = unsafe { callback_fn.replace_self(agent, stack_callback_fn.unbind()) };
        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = k.try_into().unwrap();
            // b. Let kPresent be ? HasProperty(O, Pk).
            let k_present = has_property(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
            // c. If kPresent is true, then
            if k_present {
                // i. Let kValue be ? Get(O, Pk).
                let k_value = get(agent, o.get(agent), pk, gc.reborrow()).unbind()?;
                // ii. Let testResult be ToBoolean(? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »)).
                let test_result = call_function(
                    agent,
                    callback_fn.get(agent),
                    this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        k_value.unbind(),
                        k.try_into().unwrap(),
                        o.get(agent).into_value(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
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
    fn sort<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let comparator = args.get(0);
        // 1. If comparator is not undefined and IsCallable(comparator) is false, throw a TypeError exception.
        let comparator = if comparator.is_undefined() {
            None
        } else if let Some(comparator) = is_callable(comparator, gc.nogc()) {
            Some(comparator.scope(agent, gc.nogc()))
        } else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "",
                gc.into_nogc(),
            ));
        };
        // 2. Let obj be ? ToObject(this value).
        let obj = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 3. Let len be ? LengthOfArrayLike(obj).
        let len =
            usize::try_from(length_of_array_like(agent, obj.get(agent), gc.reborrow()).unbind()?)
                .unwrap();
        // 4. Let SortCompare be a new Abstract Closure with parameters (x, y)
        // that captures comparator and performs the following steps when
        // called:
        //   a. Return ? CompareArrayElements(x, y, comparator).
        // 5. Let sortedList be ? SortIndexedProperties(obj, len, SortCompare,
        // skip-holes).
        let sorted_list =
            sort_indexed_properties::<true>(agent, obj.get(agent), len, comparator, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
        // 6. Let itemCount be the number of elements in sortedList.
        let item_count = sorted_list.len();
        let sorted_list = sorted_list.scope(agent, gc.nogc());
        // 7. Let j be 0.
        // 8. Repeat, while j < itemCount,
        for (j, value) in sorted_list.iter(agent).enumerate() {
            // a. Perform ? Set(obj, ! ToString(𝔽(j)), sortedList[j], true).
            set(
                agent,
                obj.get(agent),
                j.try_into().unwrap(),
                value.get(gc.nogc()).unbind(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
            // b. Set j to j + 1.
        }
        // 9. NOTE: The call to SortIndexedProperties in step 5 uses
        // skip-holes. The remaining indices are deleted to preserve the number
        // of holes that were detected and excluded from the sort.

        // 10. Repeat, while j < len,
        for j in item_count..len {
            // a. Perform ? DeletePropertyOrThrow(obj, ! ToString(𝔽(j))).
            delete_property_or_throw(agent, obj.get(agent), j.try_into().unwrap(), gc.reborrow())
                .unbind()?;
            // b. Set j to j + 1.
        }
        // 11. Return obj.
        Ok(obj.get(agent).into_value())
    }

    fn splice<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let start = arguments.get(0).scope(agent, nogc);
        let delete_count = arguments.get(1).scope(agent, nogc);
        let items = if arguments.len() > 2 {
            arguments[2..]
                .iter()
                .map(|v| v.scope(agent, nogc))
                .collect()
        } else {
            vec![]
        };
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start =
            to_integer_or_infinity(agent, start.get(agent), gc.reborrow()).unbind()?;
        let actual_start = if relative_start.is_neg_infinity() {
            // 4. If relativeStart = -∞, let actualStart be 0.
            0
        } else if relative_start.is_negative() {
            // 5. Else if relativeStart < 0, let actualStart be max(len + relativeStart, 0).
            (len as i64 + relative_start.into_i64()).max(0) as usize
        } else {
            // 6. Else, let actualStart be min(relativeStart, len).
            (relative_start.into_i64().min(len as i64)) as usize
        };
        // 7. Let itemCount be the number of elements in items.
        let item_count = items.len();
        // 8. If start is not present, then
        let actual_delete_count = if arguments.is_empty() {
            // a. Let actualDeleteCount be 0.
            0
        } else if arguments.len() == 1 {
            // 9. Else if deleteCount is not present, then
            // a. Let actualDeleteCount be len - actualStart.
            len as usize - actual_start
        } else {
            // 10. Else,
            // a. Let dc be ? ToIntegerOrInfinity(deleteCount).
            let dc =
                to_integer_or_infinity(agent, delete_count.get(agent), gc.reborrow()).unbind()?;
            // b. Let actualDeleteCount be the result of clamping dc between 0 and len - actualStart.
            (dc.into_i64().max(0) as usize).min(len as usize - actual_start)
        };
        // 11. If len + itemCount - actualDeleteCount > 2**53 - 1, throw a TypeError exception.
        if len as usize + item_count - actual_delete_count > SmallInteger::MAX as usize {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Target index overflowed",
                gc.into_nogc(),
            ));
        }
        // 12. Let A be ? ArraySpeciesCreate(O, actualDeleteCount).
        let a = array_species_create(agent, o.get(agent), actual_delete_count, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 13. Let k be 0.
        let mut k = 0;
        // 14. Repeat, while k < actualDeleteCount,
        while k < actual_delete_count {
            // a. Let from be ! ToString(𝔽(actualStart + k)).
            let from = (actual_start + k).try_into().unwrap();
            // b. If ? HasProperty(O, from) is true, then
            if has_property(agent, o.get(agent), from, gc.reborrow()).unbind()? {
                // i. Let fromValue be ? Get(O, from).
                let from_value = get(agent, o.get(agent), from, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(k)), fromValue).
                create_data_property_or_throw(
                    agent,
                    a.get(agent),
                    k.try_into().unwrap(),
                    from_value.unbind(),
                    gc.reborrow(),
                )
                .unbind()?;
            }
            // c. Set k to k + 1.
            k += 1;
        }
        // 15. Perform ? Set(A, "length", 𝔽(actualDeleteCount), true).
        set(
            agent,
            a.get(agent),
            BUILTIN_STRING_MEMORY.length.into(),
            (actual_delete_count as i64).try_into().unwrap(),
            true,
            gc.reborrow(),
        )
        .unbind()?;
        match item_count.cmp(&actual_delete_count) {
            // 16. If itemCount < actualDeleteCount, then
            Ordering::Less => {
                // a. Set k to actualStart.
                k = actual_start;
                // b. Repeat, while k < (len - actualDeleteCount),
                while k < (len as usize - actual_delete_count) {
                    // i. Let from be ! ToString(𝔽(k + actualDeleteCount)).
                    let from = (k + actual_delete_count).try_into().unwrap();
                    // ii. Let to be ! ToString(𝔽(k + itemCount)).
                    let to = (k + item_count).try_into().unwrap();
                    // iii. If ? HasProperty(O, from) is true, then
                    if has_property(agent, o.get(agent), from, gc.reborrow()).unbind()? {
                        // 1. Let fromValue be ? Get(O, from).
                        let from_value = get(agent, o.get(agent), from, gc.reborrow())
                            .unbind()?
                            .bind(gc.nogc());
                        // 2. Perform ? Set(O, to, fromValue, true).
                        set(
                            agent,
                            o.get(agent),
                            to,
                            from_value.unbind(),
                            true,
                            gc.reborrow(),
                        )
                        .unbind()?;
                    } else {
                        // iv. Else,
                        // 1. Perform ? DeletePropertyOrThrow(O, to).
                        delete_property_or_throw(agent, o.get(agent), to, gc.reborrow())
                            .unbind()?;
                    }
                    k += 1;
                    // v. Set k to k + 1.
                }
                // c. Set k to len.
                k = len as usize;
                // d. Repeat, while k > (len - actualDeleteCount + itemCount),
                while k > (len as usize - actual_delete_count + item_count) {
                    // i. Perform ? DeletePropertyOrThrow(O, ! ToString(𝔽(k - 1))).
                    delete_property_or_throw(
                        agent,
                        o.get(agent),
                        (k - 1).try_into().unwrap(),
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // ii. Set k to k - 1.
                    k -= 1;
                }
            }
            Ordering::Greater => {
                // 17. Else if itemCount > actualDeleteCount, then
                // a. Set k to (len - actualDeleteCount).
                k = len as usize - actual_delete_count;
                // b. Repeat, while k > actualStart,
                while k > actual_start {
                    // i. Let from be ! ToString(𝔽(k + actualDeleteCount - 1)).
                    let from = (k + actual_delete_count - 1).try_into().unwrap();
                    // ii. Let to be ! ToString(𝔽(k + itemCount - 1)).
                    let to = (k + item_count - 1).try_into().unwrap();
                    // iii. If ? HasProperty(O, from) is true, then
                    if has_property(agent, o.get(agent), from, gc.reborrow()).unbind()? {
                        // 1. Let fromValue be ? Get(O, from).
                        let from_value = get(agent, o.get(agent), from, gc.reborrow())
                            .unbind()?
                            .bind(gc.nogc());
                        // 2. Perform ? Set(O, to, fromValue, true).
                        set(
                            agent,
                            o.get(agent),
                            to,
                            from_value.unbind(),
                            true,
                            gc.reborrow(),
                        )
                        .unbind()?;
                    } else {
                        // iv. Else,
                        // 1. Perform ? DeletePropertyOrThrow(O, to).
                        delete_property_or_throw(agent, o.get(agent), to, gc.reborrow())
                            .unbind()?;
                    }
                    // v. Set k to k - 1.
                    k -= 1;
                }
            }
            _ => (),
        };
        // 18. Set k to actualStart.
        k = actual_start;
        // 19. For each element E of items, do
        for e in items {
            // a. Perform ? Set(O, ! ToString(𝔽(k)), E, true).
            set(
                agent,
                o.get(agent),
                k.try_into().unwrap(),
                e.get(agent),
                true,
                gc.reborrow(),
            )
            .unbind()?;
            // b. Set k to k + 1.
            k += 1;
        }
        // 20. Perform ? Set(O, "length", 𝔽(len - actualDeleteCount + itemCount), true).
        set(
            agent,
            o.get(agent),
            BUILTIN_STRING_MEMORY.length.into(),
            (len as i64 - actual_delete_count as i64 + item_count as i64)
                .try_into()
                .unwrap(),
            true,
            gc,
        )?;
        // 21. Return A.
        Ok(a.get(agent).into_value())
    }

    /// ### [23.1.3.32 Array.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-array.prototype.tolocalestring)
    /// An ECMAScript implementation that includes the ECMA-402 Internationalization
    /// API must implement this method as specified in the ECMA-402 specification.
    /// If an ECMAScript implementation does not include the ECMA-402 API the
    /// following specification of this method is used.
    ///
    /// > #### Note 1
    /// > The first edition of ECMA-402 did not include a replacement specification
    /// > for this method. The meanings of the optional parameters to this method
    /// > are defined in the ECMA-402 specification; implementations that do not
    /// > include ECMA-402 support must not use those parameter positions for
    /// > anything else.
    ///
    /// > #### Note 2
    /// > This method converts the elements of the array to Strings using their
    /// > toLocaleString methods, and then concatenates these Strings, separated
    /// > by occurrences of an implementation-defined locale-sensitive separator
    /// > String. This method is analogous to toString except that it is intended
    /// > to yield a locale-sensitive result corresponding with conventions of
    /// > the host environment's current locale.
    ///
    /// > #### Note 3
    /// > This method is intentionally generic; it does not require that its this
    /// > value be an Array. Therefore it can be transferred to other kinds of
    /// > objects for use as a method.
    fn to_locale_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let array be ? ToObject(this value).
        let array = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(array).
        let array = array.scope(agent, gc.nogc());
        let len = length_of_array_like(agent, array.get(agent), gc.reborrow()).unbind()?;
        // 3. Let separator be the implementation-defined list-separator String value appropriate for the host environment's current locale (such as ", ").
        let separator = ", ";
        // 4. Let R be the empty String.
        let mut r = Wtf8Buf::new();
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Repeat, while k < len,
        while k < len {
            // a. If k > 0, set R to the string-concatenation of R and separator.
            if k > 0 {
                r.push_str(separator);
            };
            // b. Let element be ? Get(array, ! ToString(𝔽(k))).
            let element = get(
                agent,
                array.get(agent),
                PropertyKey::Integer(k.try_into().unwrap()),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // c. If element is neither undefined nor null, then
            if !element.is_undefined() && !element.is_null() {
                //  i. Let S be ? ToString(? Invoke(element, "toLocaleString")).
                let argument = invoke(
                    agent,
                    element.unbind(),
                    BUILTIN_STRING_MEMORY.toLocaleString.into(),
                    None,
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let s = to_string(agent, argument.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                //  ii. Set R to the string-concatenation of R and S.
                r.push_wtf8(s.as_wtf8(agent));
            };
            // d. Set k to k + 1.
            k += 1;
        }
        // 7. Return R.
        Ok(String::from_wtf8_buf(agent, r, gc.into_nogc()).into_value())
    }

    fn to_reversed<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        if let Value::Array(array) = this_value {
            // Fast path: Array is dense and contains no descriptors. No JS
            // functions can thus be called by to_reversed.
            if array.is_trivial(agent) && array.is_dense(agent) {
                let array = array.unbind().bind(gc.into_nogc());
                let cloned_array = array.to_cloned(agent);
                cloned_array.as_mut_slice(agent).reverse();
                return Ok(cloned_array.into_value());
            }
        }

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let scoped_o = o.scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.unbind(), gc.reborrow()).unbind()?;
        // 3. Let A be ? ArrayCreate(len).
        let a = array_create(agent, len as usize, len as usize, None, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let k be 0.
        let mut k = 0;
        // 5. Repeat, while k < len,
        while k < len {
            //    a. Let from be ! ToString(𝔽(len - k - 1)).
            let from = PropertyKey::Integer((len - k - 1).try_into().unwrap());
            //    b. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();
            //    c. Let fromValue be ? Get(O, from).
            let from_value = get(agent, scoped_o.get(agent), from, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            //    d. Perform ! CreateDataPropertyOrThrow(A, Pk, fromValue).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                a.get(agent),
                pk,
                from_value.unbind(),
                gc.nogc(),
            ))
            .unwrap();
            //    e. Set k to k + 1.
            k += 1;
        }
        // 6. Return A.
        Ok(a.get(agent).into_value())
    }

    /// ### [23.1.3.34 Array.prototype.toSorted ( comparator )](https://tc39.es/ecma262/#sec-array.prototype.tosorted)
    fn to_sorted<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let comparator = args.get(0);
        // 1. If comparator is not undefined and IsCallable(comparator) is false, throw a TypeError exception.
        let comparator = if comparator.is_undefined() {
            None
        } else if let Some(comparator) = is_callable(comparator, gc.nogc()) {
            Some(comparator.scope(agent, gc.nogc()))
        } else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "The comparison function must be either a function or undefined",
                gc.into_nogc(),
            ));
        };
        // 2. Let o be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 3. Let len be ? LengthOfArrayLike(obj).
        let len =
            usize::try_from(length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?)
                .unwrap();
        // 4. Let A be ? ArrayCreate(len).
        let a = array_create(agent, len, len, None, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 5. Let SortCompare be a new Abstract Closure with parameters (x, y)
        // that captures comparator and performs the following steps when
        // called:
        //   a. Return ? CompareArrayElements(x, y, comparator).
        // 6. Let sortedList be ? SortIndexedProperties(O, len, SortCompare, read-through-holes).
        let sorted_list =
            sort_indexed_properties::<false>(agent, o.get(agent), len, comparator, gc.reborrow())
                .unbind()?;
        let gc = gc.into_nogc();
        let sorted_list = sorted_list.bind(gc);
        let a = a.get(agent).bind(gc);

        // 7. Let j be 0.
        // 8. Repeat, while j < len,
        //  a. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(j)), sortedList[j]).
        //  b. Set j to j + 1.
        // Fast path: Copy sorted items directly into array.
        let sorted_list_as_slice = sorted_list.as_slice();
        // SAFETY: Value has a nice optimisation for Option<Value>, so a Value
        // slice is always a slice of Some(Value).
        let sorted_list_as_slice = unsafe {
            core::mem::transmute::<&[Value<'_>], &[Option<Value<'static>>]>(sorted_list_as_slice)
        };
        let slice = a.as_mut_slice(agent);
        slice.copy_from_slice(sorted_list_as_slice);
        // 9. Return A.
        Ok(a.into_value())
    }

    fn to_spliced<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let start = arguments.get(0).scope(agent, gc.nogc());
        let skip_count = arguments.get(1).scope(agent, gc.nogc());
        let items = if arguments.len() > 2 {
            arguments[2..]
                .iter()
                .map(|v| v.scope(agent, gc.nogc()))
                .collect()
        } else {
            vec![]
        };
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len =
            usize::try_from(length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?)
                .unwrap();
        // 3. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start =
            to_integer_or_infinity(agent, start.get(agent), gc.reborrow()).unbind()?;
        let actual_start = if relative_start.is_neg_infinity() {
            // 4. If relativeStart = -∞, let actualStart be 0.
            0
        } else if relative_start.is_negative() {
            // 5. Else if relativeStart < 0, let actualStart be max(len + relativeStart, 0).
            (len as i64 + relative_start.into_i64()).max(0) as usize
        } else {
            // 6. Else, let actualStart be min(relativeStart, len).
            (relative_start.into_i64().min(len as i64)) as usize
        };
        // 7. Let insertCount be the number of elements in items.
        let insert_count = items.len();
        // 8. If start is not present, then
        let actual_skip_count = if arguments.is_empty() {
            // a. Let actualSkipCount be 0.
            0
        } else if arguments.len() == 1 {
            // 9. Else if skipCount is not present, then
            // a. Let actualSkipCount be len - actualStart.
            len - actual_start
        } else {
            // 10. Else,
            // a. Let dc be ? ToIntegerOrInfinity(skipCount).
            let dc =
                to_integer_or_infinity(agent, skip_count.get(agent), gc.reborrow()).unbind()?;
            // b. Let actualSkipCount be the result of clamping dc between 0 and len - actualStart.
            (dc.into_i64().max(0) as usize).min(len - actual_start)
        };
        // 11. Let newLen be len + insertCount - actualSkipCount.
        let new_len = len + insert_count - actual_skip_count;
        if new_len > SmallInteger::MAX as usize {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Target index overflowed",
                gc.into_nogc(),
            ));
        };
        // 13. Let A be ? ArrayCreate(newLen).
        let a = array_create(agent, new_len, new_len, None, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let scoped_a = a.scope(agent, gc.nogc());
        // 14. Let i be 0.
        let mut i = 0;
        // 15. Let r be actualStart + actualSkipCount.
        let mut r = actual_start + actual_skip_count;
        // 16. Repeat, while i < actualStart,
        while i < actual_start {
            // a. Let Pi be ! ToString(𝔽(i)).
            let pi = i.try_into().unwrap();
            // b. Let iValue be ? Get(O, Pi).
            let i_value = get(agent, o.get(agent), pi, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // c. Perform ! CreateDataPropertyOrThrow(A, Pi, iValue).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                scoped_a.get(agent),
                pi,
                i_value.unbind(),
                gc.nogc(),
            ))
            .unwrap();
            // d. Set i to i + 1.
            i += 1;
        }
        // 17. For each element E of items, do
        for e in items {
            // a. Let Pi be ! ToString(𝔽(i)).
            let pi = i.try_into().unwrap();
            // b. Perform ! CreateDataPropertyOrThrow(A, Pi, E).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                scoped_a.get(agent),
                pi,
                e.get(agent).unbind(),
                gc.nogc(),
            ))
            .unwrap();
            // d. Set i to i + 1.
            i += 1;
        }
        // 18. Repeat, while i < newLen,
        while i < new_len {
            // a. Let Pi be ! ToString(𝔽(i)).
            let pi = i.try_into().unwrap();
            // b. Let from be ! ToString(𝔽(r)).
            let from = r.try_into().unwrap();
            // c. Let fromValue be ? Get(O, from).
            let from_value = get(agent, o.get(agent), from, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // d. Perform ! CreateDataPropertyOrThrow(A, Pi, fromValue).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                scoped_a.get(agent),
                pi,
                from_value.unbind(),
                gc.nogc(),
            ))
            .unwrap();
            // d. Set i to i + 1.
            i += 1;
            // f. Set r to r + 1.
            r += 1;
        }
        let a = scoped_a.get(agent);
        // 19. Return A.
        Ok(a.into_value())
    }

    /// ### [23.1.3.36 Array.prototype.toString ( )](https://tc39.es/ecma262/#sec-array.prototype.tostring)
    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        // 1. Let array be ? ToObject(this value).
        let array = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let func be ? Get(array, "join").
        let func = get(
            agent,
            array.get(agent),
            BUILTIN_STRING_MEMORY.join.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 3. If IsCallable(func) is false, set func to the intrinsic function %Object.prototype.toString%.
        let func = is_callable(func, gc.nogc()).unwrap_or_else(|| {
            agent
                .current_realm_record()
                .intrinsics()
                .object_prototype_to_string()
                .into_function()
                .bind(gc.nogc())
        });
        // 4. Return ? Call(func, array).
        call_function(
            agent,
            func.unbind(),
            array.get(agent).into_value(),
            None,
            gc,
        )
    }

    /// ### [23.1.3.37 Array.prototype.unshift ( ...items )](https://tc39.es/ecma262/#sec-array.prototype.unshift)
    ///
    /// This method prepends the arguments to the start of the array, such that
    /// their order within the array is the same as the order in which they appear
    /// in the argument list.
    ///
    /// > ### Note
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be an Array. Therefore it can be transferred to other
    /// > kinds of objects for use as a method.
    fn unshift<'gc>(
        agent: &mut Agent,
        this_value: Value,
        items: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.unbind();
        // Fast path: Array is dense and contains no descriptors. No JS
        // functions can thus be called by unshift.
        if let Value::Array(array) = this_value {
            let len = array.len(agent);
            let arg_count = items.len();
            let final_len = u32::try_from(len as u64 + arg_count as u64);
            if let Ok(final_len) = final_len
                && array.is_trivial(agent)
                && array.is_dense(agent)
                && array.length_writable(agent)
            {
                // Fast path: Reserve enough room in the array and set array length.
                let Heap {
                    arrays, elements, ..
                } = &mut agent.heap;
                arrays[array].elements.reserve(elements, final_len);
                agent[array].elements.len += arg_count as u32;
                // Fast path: Copy old items to the end of array,
                // copy new items to the front of the array.
                let slice = array.as_mut_slice(agent);
                slice.copy_within(..len as usize, arg_count);
                slice[..arg_count].copy_from_slice(unsafe {
                    // SAFETY: Option<Value> is an extra variant of the Value enum.
                    // The transmute effectively turns Value into Some(Value).
                    core::mem::transmute::<&[Value], &[Option<Value>]>(items.as_slice())
                });
                return Ok(final_len.into());
            }
        }
        let items = items
            .iter()
            .map(|v| v.scope(agent, gc.nogc()))
            .collect::<Vec<_>>();
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let argCount be the number of elements in items.
        let arg_count = items.len();
        // 4. If argCount > 0, then
        if arg_count > 0 {
            // a. If len + argCount > 2**53 - 1, throw a TypeError exception.
            if (len + arg_count as i64) > SmallInteger::MAX {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Array length overflow",
                    gc.into_nogc(),
                ));
            }
            // b. Let k be len.
            let mut k = len;
            // c. Repeat, while k > 0,
            while k > 0 {
                // i. Let from be ! ToString(𝔽(k - 1)).
                let from = (k - 1).try_into().unwrap();
                // ii. Let to be ! ToString(𝔽(k + argCount - 1)).
                let to = (k + arg_count as i64 - 1).try_into().unwrap();
                // iii. Let fromPresent be ? HasProperty(O, from).
                let from_present =
                    has_property(agent, o.get(agent), from, gc.reborrow()).unbind()?;
                // iv. If fromPresent is true, then
                if from_present {
                    // 1. Let fromValue be ? Get(O, from).
                    let from_value = get(agent, o.get(agent), from, gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());
                    // 2. Perform ? Set(O, to, fromValue, true).
                    set(
                        agent,
                        o.get(agent),
                        to,
                        from_value.unbind(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                } else {
                    // v. Else,
                    // 1. Assert: fromPresent is false.
                    // 2. Perform ? DeletePropertyOrThrow(O, to).
                    delete_property_or_throw(agent, o.get(agent), to, gc.reborrow()).unbind()?;
                }
                // vi. Set k to k - 1.
                k -= 1;
            }
            // d. Let j be +0𝔽.
            // e. For each element E of items, do
            for (j, e) in items.iter().enumerate() {
                // i. Perform ? Set(O, ! ToString(j), E, true).
                // ii. Set j to j + 1𝔽.
                set(
                    agent,
                    o.get(agent),
                    j.try_into().unwrap(),
                    e.get(agent),
                    true,
                    gc.reborrow(),
                )
                .unbind()?;
            }
        }
        // 5. Perform ? Set(O, "length", 𝔽(len + argCount), true).
        let len: Value = (len + arg_count as i64).try_into().unwrap();
        set(
            agent,
            o.get(agent),
            BUILTIN_STRING_MEMORY.length.into(),
            len,
            true,
            gc.reborrow(),
        )
        .unbind()?;
        // 6. Return 𝔽(len + argCount).
        Ok(len)
    }

    fn values<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? ToObject(this value).
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected this to be an object",
                gc.into_nogc(),
            ));
        };
        // 2. Return CreateArrayIterator(O, value).
        Ok(ArrayIterator::from_object(agent, o, CollectionIteratorKind::Value).into_value())
    }

    /// ### [23.1.3.39 Array.prototype.with ( index, value )](https://tc39.es/ecma262/#sec-array.prototype.with)
    fn with<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let index = arguments.get(0).bind(nogc);
        let value = arguments.get(1).bind(nogc);
        // Fast path: Array is dense and contains no descriptors. No JS
        // functions can thus be called by with.
        if let (Value::Array(array), Value::Integer(index)) = (this_value, index)
            && array.is_trivial(agent)
            && array.is_dense(agent)
        {
            let relative_index = index.into_i64();
            let len = array.len(agent) as i64;
            let actual_index = if relative_index >= 0 {
                relative_index
            } else {
                len + relative_index
            };
            if actual_index >= len || actual_index < 0 {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "invalid or out-of-range index",
                    gc.into_nogc(),
                ));
            }
            // Fast path: Set new value in cloned array.
            let cloned_array = array.to_cloned(agent);
            cloned_array.as_mut_slice(agent)[actual_index as usize] = Some(value.unbind());
            return Ok(cloned_array.into_value().unbind().bind(gc.into_nogc()));
        }
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        let index = index.scope(agent, nogc);
        let value = value.scope(agent, nogc);
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o.get(agent), gc.reborrow()).unbind()?;
        // 3. Let relativeIndex be ? ToIntegerOrInfinity(index).
        let relative_index = to_integer_or_infinity(agent, index.get(agent), gc.reborrow())
            .unbind()?
            .into_i64();
        // 4. If relativeIndex ≥ 0, let actualIndex be relativeIndex.
        let actual_index = if relative_index >= 0 {
            relative_index
        // 5. Else, let actualIndex be len + relativeIndex.
        } else {
            len + relative_index
        };
        // 6. If actualIndex ≥ len or actualIndex < 0, throw a RangeError exception.
        if actual_index >= len || actual_index < 0 {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "invalid or out-of-range index",
                gc.into_nogc(),
            ));
        }
        // 7. Let A be ? ArrayCreate(len).
        let a = array_create(agent, len as usize, len as usize, None, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 8. Let k be 0.
        let mut k = 0;
        // 9. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();
            // b. If k = actualIndex, let fromValue be value.
            let from_value = if k == actual_index {
                value.get(agent).bind(gc.nogc())
            // c. Else, let fromValue be ? Get(O, Pk).
            } else {
                get(agent, o.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc())
            };
            // d. Perform ! CreateDataPropertyOrThrow(A, Pk, fromValue).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                a.get(agent),
                pk,
                from_value.unbind(),
                gc.nogc(),
            ))
            .unwrap();
            // e. Set k to k + 1.
            k += 1;
        }
        // 10. Return A.
        Ok(a.get(agent).bind(gc.into_nogc()).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
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
fn is_concat_spreadable<'a>(
    agent: &mut Agent,
    scoped_o: Scoped<Value>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Object<'a>>> {
    let o = scoped_o.get(agent).bind(gc.nogc());
    // 1. If O is not an Object, return false.
    let Ok(o) = Object::try_from(o) else {
        return Ok(None);
    };
    // 2. Let spreadable be ? Get(O, @@isConcatSpreadable).
    let spreadable = get(
        agent,
        o.unbind(),
        WellKnownSymbolIndexes::IsConcatSpreadable.into(),
        gc.reborrow(),
    )
    .unbind()?;

    let gc = gc.into_nogc();
    let o = Object::try_from(scoped_o.get(agent).bind(gc)).unwrap();

    // 3. If spreadable is not undefined, return ToBoolean(spreadable).
    if !spreadable.is_undefined() {
        let spreadable = to_boolean(agent, spreadable);
        if spreadable {
            // SAFETY: scoped_o is not shared.
            Ok(Some(o))
        } else {
            Ok(None)
        }
    } else {
        // 4. Return ? IsArray(O).
        let o_is_array = is_array(agent, o, gc)?;
        if o_is_array { Ok(Some(o)) } else { Ok(None) }
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
pub(crate) fn find_via_predicate<'gc, T: 'static + Rootable + InternalMethods<'static>>(
    agent: &mut Agent,
    o: Scoped<T>,
    len: i64,
    ascending: bool,
    predicate: Scoped<Value>,
    this_arg: Scoped<Value>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, (i64, Value<'gc>)> {
    // 1. If IsCallable(predicate) is false, throw a TypeError exception.
    let Some(stack_predicate) = is_callable(predicate.get(agent), gc.nogc()) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Predicate is not a function",
            gc.into_nogc(),
        ));
    };
    // SAFETY: We're only ever called in a way that gives ownership of
    // predicate to us.
    let predicate = unsafe { predicate.replace_self(agent, stack_predicate.unbind()) };
    // 4. For each integer k of indices, do
    fn check<'gc, T: 'static + Rootable + InternalMethods<'static>>(
        agent: &mut Agent,
        o: Scoped<T>,
        predicate: Scoped<Function>,
        this_arg: Scoped<Value>,
        k: i64,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<(i64, Value<'gc>)>> {
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk = PropertyKey::Integer(k.try_into().unwrap());
        // b. NOTE: If O is a TypedArray, the following invocation of Get will
        // return a normal completion.
        // c. Let kValue be ? Get(O, Pk).
        let k_value = get(agent, o.get(agent), pk, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let scoped_k_value = k_value.scope(agent, gc.nogc());

        // d. Let testResult be ? Call(predicate, thisArg, « kValue, 𝔽(k), O »).
        let test_result = call_function(
            agent,
            predicate.get(agent),
            this_arg.get(agent),
            Some(ArgumentsList::from_mut_slice(&mut [
                k_value.unbind(),
                Number::try_from(k).unwrap().into_value(),
                o.get(agent).into_value(),
            ])),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // e. If ToBoolean(testResult) is true, return the Record { [[Index]]: 𝔽(k), [[Value]]: kValue }.
        if to_boolean(agent, test_result) {
            // SAFETY: scoped_k_value is never shared.
            Ok(Some((k, unsafe { scoped_k_value.take(agent) })))
        } else {
            Ok(None)
        }
    }

    // 2. If direction is ascending, then
    if ascending {
        // a. Let indices be a List of the integers in the interval from 0 (inclusive) to len (exclusive), in ascending order.
        for k in 0..len {
            let result = match check(
                agent,
                o.clone(),
                predicate.clone(),
                this_arg.clone(),
                k,
                gc.reborrow(),
            ) {
                Ok(result) => result,
                Err(err) => {
                    return Err(err.unbind());
                }
            };
            if let Some((index, value)) = result {
                return Ok((index, value.unbind().bind(gc.into_nogc())));
            }
        }
    } else {
        // 3. Else,
        // a. Let indices be a List of the integers in the interval from 0 (inclusive) to len (exclusive), in descending order.
        for k in (0..len).rev() {
            let result = match check(
                agent,
                o.clone(),
                predicate.clone(),
                this_arg.clone(),
                k,
                gc.reborrow(),
            ) {
                Ok(result) => result,
                Err(err) => return Err(err.unbind()),
            };
            if let Some((index, value)) = result {
                return Ok((index, value.unbind().bind(gc.into_nogc())));
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
fn flatten_into_array<'a>(
    agent: &mut Agent,
    target: Scoped<Object>,
    source: Scoped<Object>,
    source_len: usize,
    start: usize,
    depth: Option<usize>,
    mapper_function: Option<Scoped<Function>>,
    this_arg: Option<Scoped<Value>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, usize> {
    // 1. Assert: If mapperFunction is present, then IsCallable(mapperFunction) is true, thisArg is present, and depth is 1.
    assert!(mapper_function.is_none() || this_arg.is_some() && depth == Some(1));
    // 2. Let targetIndex be start.
    let mut target_index = start;
    // 3. Let sourceIndex be +0𝔽.
    // 4. Repeat, while ℝ(sourceIndex) < sourceLen,
    for source_index in 0..source_len {
        // a. Let P be ! ToString(sourceIndex).
        let source_index_number = Number::try_from(source_index).unwrap();
        let p = PropertyKey::try_from(source_index).unwrap();
        // b. Let exists be ? HasProperty(source, P).
        let exists = has_property(agent, source.get(agent), p, gc.reborrow()).unbind()?;
        // c. If exists is true, then
        if !exists {
            // d. Set sourceIndex to sourceIndex + 1𝔽.
            continue;
        }
        // i. Let element be ? Get(source, P).
        let element = get(agent, source.get(agent), p, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // ii. If mapperFunction is present, then
        let element = if let Some(mapper_function) = &mapper_function {
            // 1. Set element to ? Call(mapperFunction, thisArg, « element, sourceIndex, source »).
            call_function(
                agent,
                mapper_function.get(agent),
                this_arg.as_ref().unwrap().get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    element.unbind(),
                    source_index_number.into_value(),
                    source.get(agent).into_value(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        } else {
            element
        };
        // iii. Let shouldFlatten be false.
        let mut should_flatten = false;
        // iv. If depth > 0, then
        if depth.is_none_or(|depth| depth > 0) {
            // 1. Set shouldFlatten to ? IsArray(element).
            should_flatten = is_array(agent, element, gc.nogc()).unbind()?;
        }
        // v. If shouldFlatten is true, then
        if should_flatten {
            // Note: Element is necessarily an Array.
            let element = Object::try_from(element).unwrap().scope(agent, gc.nogc());
            let new_depth = depth.map(|depth| depth - 1);
            // 3. Let elementLen be ? LengthOfArrayLike(element).
            let element_len =
                length_of_array_like(agent, element.get(agent), gc.reborrow()).unbind()? as usize;
            // 4. Set targetIndex to ? FlattenIntoArray(target, element, elementLen, targetIndex, newDepth).
            target_index = flatten_into_array(
                agent,
                target.clone(),
                element,
                element_len,
                target_index,
                new_depth,
                None,
                None,
                gc.reborrow(),
            )
            .unbind()?;
        } else {
            // vi. Else,
            // 1. If targetIndex ≥ 2**53 - 1, throw a TypeError exception.
            if target_index >= SmallInteger::MAX as usize {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Target index overflowed",
                    gc.into_nogc(),
                ));
            }
            // 2. Perform ? CreateDataPropertyOrThrow(target, ! ToString(𝔽(targetIndex)), element).
            create_data_property_or_throw(
                agent,
                target.get(agent),
                target_index.try_into().unwrap(),
                element.unbind(),
                gc.reborrow(),
            )
            .unbind()?;
            // 3. Set targetIndex to targetIndex + 1.
            target_index += 1;
        }
        // d. Set sourceIndex to sourceIndex + 1𝔽.
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
fn sort_indexed_properties<'gc, 'scope, const SKIP_HOLES: bool>(
    agent: &mut Agent,
    obj: Object,
    len: usize,
    comparator: Option<Scoped<'scope, Function<'static>>>,
    mut gc: GcScope<'gc, 'scope>,
) -> JsResult<'gc, Vec<Value<'gc>>> {
    let obj = obj.scope(agent, gc.nogc());
    // 1. Let items be a new empty List.
    let mut items = Vec::with_capacity(len);
    // 2. Let k be 0.
    let mut k = 0;
    // 3. Repeat, while k < len,
    while k < len {
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk: PropertyKey<'static> = k.try_into().unwrap();
        // b. If holes is skip-holes, then
        let k_read = if SKIP_HOLES {
            // i. Let kRead be ? HasProperty(obj, Pk).
            has_property(agent, obj.get(agent), pk, gc.reborrow()).unbind()?
        } else {
            // c. Else,
            // i. Assert: holes is read-through-holes.
            // ii. Let kRead be true.
            true
        };
        // d. If kRead is true, then
        if k_read {
            // i. Let kValue be ? Get(obj, Pk).
            let k_value = get(agent, obj.get(agent), pk, gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());
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
    let mut error: Option<JsError> = None;
    items.sort_by(|a, b| {
        if error.is_some() {
            // This is dangerous but we don't have much of a choice.
            return Ordering::Equal;
        }
        let result = compare_array_elements(agent, a, b, comparator.clone(), gc.reborrow());
        match result {
            Ok(result) => result,
            Err(err) => {
                error = Some(err.unbind());
                Ordering::Equal
            }
        }
    });
    if let Some(error) = error {
        return Err(error);
    }
    let gc = gc.into_nogc();
    // 5. Return items.
    Ok(items.into_iter().map(|v| v.get(agent).bind(gc)).collect())
}

/// ### [23.1.3.30.2 CompareArrayElements ( x, y, comparator )](https://tc39.es/ecma262/#sec-comparearrayelements)
/// The abstract operation CompareArrayElements takes arguments x (an
/// ECMAScript language value), y (an ECMAScript language value), and
/// comparator (a function object or undefined) and returns either a normal
/// completion containing a Number or an abrupt completion.
fn compare_array_elements<'a>(
    agent: &mut Agent,
    scoped_x: &Scoped<Value>,
    scoped_y: &Scoped<Value>,
    comparator: Option<Scoped<Function>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Ordering> {
    let x = scoped_x.get(agent).bind(gc.nogc());
    let y = scoped_y.get(agent).bind(gc.nogc());
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
            comparator.get(agent),
            Value::Undefined,
            Some(ArgumentsList::from_mut_slice(&mut [x.unbind(), y.unbind()])),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let v = to_number(agent, v.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
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
        let (x, y) = if let TryResult::Continue(x) = try_to_string(agent, x, gc.nogc()) {
            (x.unbind()?.bind(gc.nogc()), y)
        } else {
            let x = to_string(agent, x.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            (x, scoped_y.get(agent).bind(gc.nogc()))
        };
        // 6. Let yString be ? ToString(y).
        let (x, y) = if let TryResult::Continue(y) = try_to_string(agent, y, gc.nogc()) {
            (x, y.unbind()?.bind(gc.nogc()))
        } else {
            let x = x.scope(agent, gc.nogc());
            let y = to_string(agent, y.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            (x.get(agent).bind(gc.nogc()), y)
        };
        // 7. Let xSmaller be ! IsLessThan(xString, yString, true).
        // 8. If xSmaller is true, return -1𝔽.
        // 9. Let ySmaller be ! IsLessThan(yString, xString, true).
        // 10. If ySmaller is true, return 1𝔽.
        // 11. Return +0𝔽.
        // TODO: this gives UTF-8 lexicographic ordering, not UTF-16.
        Ok(x.as_wtf8(agent).cmp(y.as_wtf8(agent)))
    }
}
