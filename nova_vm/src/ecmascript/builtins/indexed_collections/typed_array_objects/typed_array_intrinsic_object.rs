// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use num_traits::ToPrimitive;

use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{get_iterator_from_method, iterator_to_list},
            operations_on_objects::{
                call_function, get, get_method, length_of_array_like, set, throw_not_callable,
                try_get, try_set,
            },
            testing_and_comparison::{is_array, is_callable, is_constructor, same_value_zero},
            type_conversion::{
                to_big_int, to_boolean, to_integer_or_infinity, to_number, to_object, to_string,
                try_to_integer_or_infinity, try_to_string,
            },
        },
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic,
            BuiltinIntrinsicConstructor,
            array_buffer::{Ordering, get_value_from_buffer, is_detached_buffer},
            indexed_collections::array_objects::{
                array_iterator_objects::array_iterator::{ArrayIterator, CollectionIteratorKind},
                array_prototype::find_via_predicate,
            },
            typed_array::TypedArray,
        },
        execution::{
            Agent, JsResult, Realm,
            agent::{ExceptionType, JsError},
        },
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoNumeric, IntoObject, IntoValue, Number, Object,
            PropertyKey, String, U8Clamped, Value, Viewable,
        },
    },
    engine::{
        Scoped, TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
        unwrap_try,
    },
    heap::{IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

use super::abstract_operations::{
    TypedArrayWithBufferWitnessRecords, is_typed_array_out_of_bounds,
    make_typed_array_with_buffer_witness_record, typed_array_byte_length,
    typed_array_create_from_constructor_with_length, typed_array_create_same_type,
    typed_array_length, validate_typed_array,
};

pub struct TypedArrayIntrinsicObject;

impl Builtin for TypedArrayIntrinsicObject {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.TypedArray;
}
impl BuiltinIntrinsicConstructor for TypedArrayIntrinsicObject {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TypedArray;
}

struct TypedArrayFrom;
impl Builtin for TypedArrayFrom {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayIntrinsicObject::from);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.from;
}
struct TypedArrayOf;
impl Builtin for TypedArrayOf {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayIntrinsicObject::of);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.of;
}
struct TypedArrayGetSpecies;
impl Builtin for TypedArrayGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayIntrinsicObject::get_species);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());
}
impl BuiltinGetter for TypedArrayGetSpecies {}
impl TypedArrayIntrinsicObject {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.throw_exception_with_static_message(
            crate::ecmascript::execution::agent::ExceptionType::TypeError,
            "Abstract class TypedArray not directly constructable",
            gc.into_nogc(),
        ))
    }

    /// ### [23.2.2.1 %TypedArray%.from ( source [ , mapper [ , thisArg ] ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.from)
    fn from<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let source = arguments.get(0).bind(gc.nogc());
        let mapper = arguments.get(1).bind(gc.nogc());
        let this_arg = arguments.get(2).bind(gc.nogc());

        // 1. Let C be the this value.
        let c = this_value;
        // 2. If IsConstructor(C) is false, throw a TypeError exception.
        let Some(c) = is_constructor(agent, c) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a constructor",
                gc.into_nogc(),
            ));
        };
        // 3. If mapper is undefined, then
        let mapping = if mapper.is_undefined() {
            // a. Let mapping be false.
            None
        } else {
            // 3. Else,
            //  a. If IsCallable(mapper) is false, throw a TypeError exception.
            let Some(mapper) = is_callable(mapper, gc.nogc()) else {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "The map function of Array.from is not callable",
                    gc.into_nogc(),
                ));
            };
            //  b. Let mapping be true.
            Some(mapper.scope(agent, gc.nogc()))
        };
        let scoped_c = c.scope(agent, gc.nogc());
        let scoped_source = source.scope(agent, gc.nogc());
        let scoped_this_arg = this_arg.scope(agent, gc.nogc());
        // 5. Let usingIterator be ? GetMethod(source, %Symbol.iterator%).
        let using_iterator = get_method(
            agent,
            source.unbind(),
            WellKnownSymbolIndexes::Iterator.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 6. If usingIterator is not undefined, then
        if let Some(using_iterator) = using_iterator {
            // a. Let values be ? IteratorToList(? GetIteratorFromMethod(source, usingIterator)).
            let Some(iterator_record) = get_iterator_from_method(
                agent,
                scoped_source.get(agent),
                using_iterator.unbind(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc()) else {
                return Err(throw_not_callable(agent, gc.into_nogc()));
            };
            let values = iterator_to_list(agent, iterator_record.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. Let len be the number of elements in values.
            let len = values.len().to_i64().unwrap();
            // c. Let targetObj be ? TypedArrayCreateFromConstructor(C, « 𝔽(len) »).
            let target_obj = typed_array_create_from_constructor_with_length(
                agent,
                scoped_c.get(agent),
                len,
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let scoped_target_obj = target_obj.scope(agent, gc.nogc());
            // d. Let k be 0.
            // e. Repeat, while k < len,
            for (k, k_value) in values.iter().enumerate() {
                // 𝔽(k)
                //  i. Let Pk be ! ToString(𝔽(k)).
                // ii. Let kValue be the first element of values.
                // iii. Remove the first element from values.
                let sk = SmallInteger::from(k as u32);
                let fk = Number::from(sk).into_value();
                let pk = PropertyKey::from(sk);
                //  iv. If mapping is true, then
                let mapped_value = if let Some(mapper) = &mapping {
                    //  1. Let mappedValue be ? Call(mapper, thisArg, « kValue, 𝔽(k) »).
                    call_function(
                        agent,
                        mapper.get(agent),
                        scoped_this_arg.get(agent),
                        Some(ArgumentsList::from_mut_slice(&mut [
                            k_value.get(agent).unbind(),
                            fk,
                        ])),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc())
                } else {
                    // v. Else,
                    //      1. Let mappedValue be kValue.
                    k_value.get(agent)
                };
                // vi. Perform ? Set(targetObj, Pk, mappedValue, true).
                set(
                    agent,
                    scoped_target_obj.get(agent).into_object(),
                    pk,
                    mapped_value.unbind(),
                    true,
                    gc.reborrow(),
                )
                .unbind()?;
                // vii. Set k to k + 1.
            }
            // f. Assert: values is now an empty List.
            // g. Return targetObj.
            let target_obj = scoped_target_obj.get(agent);
            return Ok(target_obj.into_value());
        }
        // 7. NOTE: source is not an iterable object, so assume it is already an array-like object.
        // 8. Let arrayLike be ! ToObject(source).
        let array_like = to_object(agent, scoped_source.get(agent), gc.nogc())
            .unwrap()
            .scope(agent, gc.nogc());
        // 9. Let len be ? LengthOfArrayLike(arrayLike).
        let len = length_of_array_like(agent, array_like.get(agent), gc.reborrow()).unbind()?;
        // 10. Let targetObj be ? TypedArrayCreateFromConstructor(C, « 𝔽(len) »).
        let target_obj = typed_array_create_from_constructor_with_length(
            agent,
            scoped_c.get(agent),
            len,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let scoped_target_obj = target_obj.scope(agent, gc.nogc());
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
            let k_value = get(agent, array_like.get(agent), pk, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // c. If mapping is true, then
            let mapped_value = if let Some(mapper) = &mapping {
                // i. Let mappedValue be ? Call(mapper, thisArg, « kValue, 𝔽(k) »).
                call_function(
                    agent,
                    mapper.get(agent),
                    scoped_this_arg.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [k_value.unbind(), fk])),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc())
            } else {
                // d. Else,
                // i. Let mappedValue be kValue.
                k_value
            };
            // e. Perform ? Set(targetObj, Pk, mappedValue, true).
            set(
                agent,
                scoped_target_obj.get(agent).into_object(),
                pk,
                mapped_value.unbind(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
            // f. Set k to k + 1.
            k += 1;
        }
        let target_obj = scoped_target_obj.get(agent);
        Ok(target_obj.into_value())
    }

    fn is_array<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        is_array(agent, arguments.get(0), gc.into_nogc()).map(Value::Boolean)
    }

    /// ### [23.2.2.2 %TypedArray%.of ( ...items )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-properties-of-the-%typedarray%-intrinsic-object)
    fn of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let arguments = arguments.bind(gc.nogc());

        // 1. Let len be the number of elements in items.
        let len = arguments.len();

        // 2. Let C be the this value.
        let c = this_value;
        // 3. If IsConstructor(C) is false, throw a TypeError exception.
        let Some(c) = is_constructor(agent, c) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a constructor",
                gc.into_nogc(),
            ));
        };
        // 4. Let newObj be ? TypedArrayCreateFromConstructor(C, « 𝔽(len) »).
        let len = u32::try_from(len).unwrap();
        let c = c.scope(agent, gc.nogc());

        arguments.unbind().with_scoped(
            agent,
            |agent, arguments, mut gc| {
                let new_obj = typed_array_create_from_constructor_with_length(
                    agent,
                    c.get(agent),
                    len as i64,
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // 5. Let k be 0.
                // 6. Repeat, while k < len,
                let scoped_new_obj = new_obj.scope(agent, gc.nogc());
                for k in 0..len {
                    // a. Let kValue be items[k].
                    // b. Let Pk be ! ToString(𝔽(k)).
                    let pk = k.into();
                    let k_value = arguments.get(agent, k, gc.nogc());
                    // c. Perform ? Set(newObj, Pk, kValue, true).
                    set(
                        agent,
                        scoped_new_obj.get(agent).into_object(),
                        pk,
                        k_value.unbind(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // d. Set k to k + 1.
                }
                // 7. Return newObj.
                Ok(scoped_new_obj.get(agent).into_value())
            },
            gc,
        )
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(this_value.bind(gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let typed_array_prototype = intrinsics.typed_array_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TypedArrayIntrinsicObject>(
            agent, realm,
        )
        .with_property_capacity(4)
        .with_builtin_function_property::<TypedArrayFrom>()
        .with_builtin_function_property::<TypedArrayOf>()
        .with_prototype_property(typed_array_prototype.into_object())
        .with_builtin_function_getter_property::<TypedArrayGetSpecies>()
        .build();
    }
}

pub(crate) struct TypedArrayPrototype;

struct TypedArrayPrototypeAt;
impl Builtin for TypedArrayPrototypeAt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.at;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::at);
}
struct TypedArrayPrototypeGetBuffer;
impl Builtin for TypedArrayPrototypeGetBuffer {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_buffer;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.buffer.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_buffer);
}
impl BuiltinGetter for TypedArrayPrototypeGetBuffer {}
struct TypedArrayPrototypeGetByteLength;
impl Builtin for TypedArrayPrototypeGetByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteLength;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.byteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_byte_length);
}
impl BuiltinGetter for TypedArrayPrototypeGetByteLength {}
struct TypedArrayPrototypeGetByteOffset;
impl Builtin for TypedArrayPrototypeGetByteOffset {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteOffset;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.byteOffset.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_byte_offset);
}
impl BuiltinGetter for TypedArrayPrototypeGetByteOffset {}
struct TypedArrayPrototypeCopyWithin;
impl Builtin for TypedArrayPrototypeCopyWithin {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.copyWithin;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::copy_within);
}
struct TypedArrayPrototypeEntries;
impl Builtin for TypedArrayPrototypeEntries {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::entries);
}
struct TypedArrayPrototypeEvery;
impl Builtin for TypedArrayPrototypeEvery {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::every);
}
struct TypedArrayPrototypeFill;
impl Builtin for TypedArrayPrototypeFill {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fill;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::fill);
}
struct TypedArrayPrototypeFilter;
impl Builtin for TypedArrayPrototypeFilter {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.filter;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::filter);
}
struct TypedArrayPrototypeFind;
impl Builtin for TypedArrayPrototypeFind {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.find;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find);
}
struct TypedArrayPrototypeFindIndex;
impl Builtin for TypedArrayPrototypeFindIndex {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.findIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find_index);
}
struct TypedArrayPrototypeFindLast;
impl Builtin for TypedArrayPrototypeFindLast {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.findLast;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find_last);
}
struct TypedArrayPrototypeFindLastIndex;
impl Builtin for TypedArrayPrototypeFindLastIndex {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.findLastIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find_last_index);
}
struct TypedArrayPrototypeForEach;
impl Builtin for TypedArrayPrototypeForEach {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::for_each);
}
struct TypedArrayPrototypeIncludes;
impl Builtin for TypedArrayPrototypeIncludes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.includes;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::includes);
}
struct TypedArrayPrototypeIndexOf;
impl Builtin for TypedArrayPrototypeIndexOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.indexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::index_of);
}
struct TypedArrayPrototypeJoin;
impl Builtin for TypedArrayPrototypeJoin {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.join;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::join);
}
struct TypedArrayPrototypeKeys;
impl Builtin for TypedArrayPrototypeKeys {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::keys);
}
struct TypedArrayPrototypeLastIndexOf;
impl Builtin for TypedArrayPrototypeLastIndexOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::last_index_of);
}
struct TypedArrayPrototypeGetLength;
impl Builtin for TypedArrayPrototypeGetLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_length;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.length.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_length);
}
impl BuiltinGetter for TypedArrayPrototypeGetLength {}
struct TypedArrayPrototypeMap;
impl Builtin for TypedArrayPrototypeMap {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.map;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::map);
}
struct TypedArrayPrototypeReduce;
impl Builtin for TypedArrayPrototypeReduce {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reduce;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::reduce);
}
struct TypedArrayPrototypeReduceRight;
impl Builtin for TypedArrayPrototypeReduceRight {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reduceRight;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::reduce_right);
}
struct TypedArrayPrototypeReverse;
impl Builtin for TypedArrayPrototypeReverse {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reverse;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::reverse);
}
struct TypedArrayPrototypeSet;
impl Builtin for TypedArrayPrototypeSet {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.set;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::set);
}
struct TypedArrayPrototypeSlice;
impl Builtin for TypedArrayPrototypeSlice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::slice);
}
struct TypedArrayPrototypeSome;
impl Builtin for TypedArrayPrototypeSome {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.some;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::some);
}
struct TypedArrayPrototypeSort;
impl Builtin for TypedArrayPrototypeSort {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sort;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::sort);
}
struct TypedArrayPrototypeSubarray;
impl Builtin for TypedArrayPrototypeSubarray {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.subarray;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::subarray);
}
struct TypedArrayPrototypeToLocaleString;
impl Builtin for TypedArrayPrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::to_locale_string);
}
struct TypedArrayPrototypeToReversed;
impl Builtin for TypedArrayPrototypeToReversed {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toReversed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::to_reversed);
}
struct TypedArrayPrototypeToSorted;
impl Builtin for TypedArrayPrototypeToSorted {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toSorted;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::to_sorted);
}
struct TypedArrayPrototypeValues;
impl Builtin for TypedArrayPrototypeValues {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::values);
}
impl BuiltinIntrinsic for TypedArrayPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::TypedArrayPrototypeValues;
}
struct TypedArrayPrototypeWith;
impl Builtin for TypedArrayPrototypeWith {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.with;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::with);
}
struct TypedArrayPrototypeGetToStringTag;
impl Builtin for TypedArrayPrototypeGetToStringTag {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_toStringTag_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::ToStringTag.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_to_string_tag);
}
impl BuiltinGetter for TypedArrayPrototypeGetToStringTag {}

impl TypedArrayPrototype {
    /// ### [23.2.3.1 %TypedArray%.prototype.at ( index )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-array.prototype.at)
    fn at<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let index = arguments.get(0).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let mut o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        // 4. Let relativeIndex be ? ToIntegerOrInfinity(index).
        let relative_index = if let Value::Integer(index) = index {
            index.into_i64()
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let result = to_integer_or_infinity(agent, index.unbind(), gc.reborrow())
                .unbind()?
                .into_i64();
            o = scoped_o.get(agent).bind(gc.nogc());
            result
        };
        // 5. If relativeIndex ≥ 0, then
        let k = if relative_index >= 0 {
            // a. Let k be relativeIndex.
            relative_index
        } else {
            // 6. Else,
            // a. Let k be len + relativeIndex.
            len + relative_index
        };
        // 7. If k < 0 or k ≥ len, return undefined.
        if k < 0 || k >= len {
            return Ok(Value::Undefined);
        };
        // 8. Return ! Get(O, ! ToString(𝔽(k))).
        Ok(unwrap_try(try_get(
            agent,
            o.unbind(),
            PropertyKey::Integer(k.try_into().unwrap()),
            gc.into_nogc(),
        )))
    }

    /// ### [23.2.3.2 get %TypedArray%.prototype.buffer](https://tc39.es/ecma262/#sec-get-%typedarray%.prototype.buffer)
    ///
    /// %TypedArray%.prototype.buffer is an accessor property whose set accessor
    /// function is undefined.
    fn get_buffer<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        // 4. Let buffer be O.[[ViewedArrayBuffer]].
        let o = require_internal_slot_typed_array(agent, this_value, gc)?;

        // 5. Return buffer.
        Ok(o.get_viewed_array_buffer(agent, gc).into_value())
    }

    /// ### [23.2.3.3 get %TypedArray%.prototype.byteLength](https://tc39.es/ecma262/#sec-get-%typedarray%.prototype.bytelength)
    ///
    /// %TypedArray%.prototype.byteLength is an accessor property whose set
    /// accessor function is undefined.
    fn get_byte_length<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        let o = require_internal_slot_typed_array(agent, this_value, gc)?;

        // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc);

        // 5. Let size be TypedArrayByteLength(taRecord).
        let size = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_byte_length::<u8>(agent, &ta_record, gc)
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_byte_length::<u16>(agent, &ta_record, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_byte_length::<f16>(agent, &ta_record, gc),
            TypedArray::Float32Array(_)
            | TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_) => typed_array_byte_length::<u32>(agent, &ta_record, gc),
            TypedArray::Float64Array(_)
            | TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_) => {
                typed_array_byte_length::<u64>(agent, &ta_record, gc)
            }
        };

        // 6. Return 𝔽(size).
        Ok(Value::try_from(size as i64).unwrap())
    }

    /// ### [23.2.3.4 get %TypedArray%.prototype.byteOffset](https://tc39.es/ecma262/#sec-get-%typedarray%.prototype.byteoffset)
    ///
    /// %TypedArray%.prototype.byteOffset is an accessor property whose set
    /// accessor function is undefined.
    fn get_byte_offset<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        let o = require_internal_slot_typed_array(agent, this_value, gc)?;

        // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc);

        // 5. If IsTypedArrayOutOfBounds(taRecord) is true, return +0𝔽.
        if match o {
            TypedArray::Int8Array(_) => is_typed_array_out_of_bounds::<i8>(agent, &ta_record, gc),
            TypedArray::Uint8Array(_) => is_typed_array_out_of_bounds::<u8>(agent, &ta_record, gc),
            TypedArray::Uint8ClampedArray(_) => {
                is_typed_array_out_of_bounds::<U8Clamped>(agent, &ta_record, gc)
            }
            TypedArray::Int16Array(_) => is_typed_array_out_of_bounds::<i16>(agent, &ta_record, gc),
            TypedArray::Uint16Array(_) => {
                is_typed_array_out_of_bounds::<u16>(agent, &ta_record, gc)
            }
            TypedArray::Int32Array(_) => is_typed_array_out_of_bounds::<i32>(agent, &ta_record, gc),
            TypedArray::Uint32Array(_) => {
                is_typed_array_out_of_bounds::<u32>(agent, &ta_record, gc)
            }
            TypedArray::BigInt64Array(_) => {
                is_typed_array_out_of_bounds::<i64>(agent, &ta_record, gc)
            }
            TypedArray::BigUint64Array(_) => {
                is_typed_array_out_of_bounds::<u64>(agent, &ta_record, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => {
                is_typed_array_out_of_bounds::<f16>(agent, &ta_record, gc)
            }
            TypedArray::Float32Array(_) => {
                is_typed_array_out_of_bounds::<f32>(agent, &ta_record, gc)
            }
            TypedArray::Float64Array(_) => {
                is_typed_array_out_of_bounds::<f64>(agent, &ta_record, gc)
            }
        } {
            return Ok(Value::pos_zero());
        }

        // 6. Let offset be O.[[ByteOffset]].
        // 7. Return 𝔽(offset).
        Ok(Value::try_from(o.byte_offset(agent) as i64).unwrap())
    }

    /// ### [23.2.3.6 %TypedArray%.prototype.copyWithin ( target, start [ , end ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-typedarray-objects)
    /// The interpretation and use of the arguments of this method
    /// are the same as for Array.prototype.copyWithin as defined in 23.1.3.4.
    fn copy_within<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let target = arguments.get(0).bind(gc.nogc());
        let start = arguments.get(1).bind(gc.nogc());
        let end = if arguments.len() >= 3 {
            Some(arguments.get(2).bind(gc.nogc()))
        } else {
            None
        };
        // 1. Let O be the this value.
        let o = this_value;

        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        let scoped_o = o.scope(agent, gc.nogc());
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match scoped_o.get(agent) {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        }
        .to_i64()
        .unwrap();
        let end = end.map(|e| e.scope(agent, gc.nogc()));
        let start = start.scope(agent, gc.nogc());
        let target = target.scope(agent, gc.nogc());

        // 4. Let relativeTarget be ? ToIntegerOrInfinity(target).
        // SAFETY: target has not been shared.
        let relative_target =
            to_integer_or_infinity(agent, unsafe { target.take(agent) }, gc.reborrow()).unbind()?;
        // 5. If relativeTarget = -∞, let targetIndex be 0.
        let target_index = if relative_target.is_neg_infinity() {
            0
        } else if relative_target.is_negative() {
            // 6. Else if relativeTarget < 0, let targetIndex be max(len + relativeTarget, 0).
            (len + relative_target.into_i64()).max(0)
        } else {
            // 7. Else, let targetIndex be min(relativeTarget, len).
            relative_target.into_i64().min(len)
        };
        // 8. Let relativeStart be ? ToIntegerOrInfinity(start).
        // SAFETY: start has not been shared.
        let relative_start =
            to_integer_or_infinity(agent, unsafe { start.take(agent) }, gc.reborrow()).unbind()?;
        let start_index = if relative_start.is_neg_infinity() {
            // 9. If relativeStart = -∞, let startIndex be 0
            0
        } else if relative_start.is_negative() {
            // 10. Else if relativeStart < 0, let startIndex be max(len + relativeStart, 0).
            (len + relative_start.into_i64()).max(0)
        } else {
            // 11. Else, let startIndex be min(relativeStart, len).
            relative_start.into_i64().min(len)
        };
        // 12. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
        let end = end.map(|e| unsafe { e.take(agent) }.bind(gc.nogc()));
        let end_index = if end.is_none() || end.unwrap().is_undefined() {
            len
        } else {
            let relative_end =
                to_integer_or_infinity(agent, end.unwrap().unbind(), gc.reborrow()).unbind()?;
            // 13. If relativeEnd = -∞, let endIndex be 0.
            if relative_end.is_neg_infinity() {
                0
            } else if relative_end.is_negative() {
                // 14. Else if relativeEnd < 0, let endIndex be max(len + relativeEnd, 0).
                (len + relative_end.into_i64()).max(0)
            } else {
                // 15. Else, let endIndex be min(relativeEnd, len).
                relative_end.into_i64().min(len)
            }
        };
        let gc = gc.into_nogc();
        let o = scoped_o.get(agent).bind(gc);
        match o {
            TypedArray::Int8Array(_) => {
                copy_within_typed_array::<i8>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Uint8Array(_) => {
                copy_within_typed_array::<u8>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Uint8ClampedArray(_) => {
                copy_within_typed_array::<U8Clamped>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Int16Array(_) => {
                copy_within_typed_array::<i16>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Uint16Array(_) => {
                copy_within_typed_array::<u16>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => {
                copy_within_typed_array::<f16>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Int32Array(_) => {
                copy_within_typed_array::<i32>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Uint32Array(_) => {
                copy_within_typed_array::<u32>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Float32Array(_) => {
                copy_within_typed_array::<f32>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::BigInt64Array(_) => {
                copy_within_typed_array::<i64>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::BigUint64Array(_) => {
                copy_within_typed_array::<u64>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
            TypedArray::Float64Array(_) => {
                copy_within_typed_array::<f64>(
                    agent,
                    o,
                    target_index,
                    start_index,
                    end_index,
                    len,
                    gc,
                )?;
            }
        }
        // 18. Return O.
        Ok(o.into_value())
    }

    /// ### [23.2.3.7 %TypedArray%.prototype.entries ( )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.entries)
    fn entries<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        // 2. Perform ? ValidateTypedArray(O, seq-cst).
        let o = validate_typed_array(agent, this_value, Ordering::SeqCst, gc)?.object;
        // 3. Return CreateArrayIterator(O, key+value).
        Ok(
            ArrayIterator::from_object(agent, o.into_object(), CollectionIteratorKind::KeyAndValue)
                .into_value(),
        )
    }

    /// ### [23.2.3.8 %%TypedArray%.prototype.every ( callback [ , thisArg ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.every)
    fn every<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback = arguments.get(0).bind(nogc);
        let this_arg = arguments.get(1).bind(nogc);
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(nogc);
        let mut o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => typed_array_length::<u8>(agent, &ta_record, nogc),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, nogc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, nogc),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => typed_array_length::<u32>(agent, &ta_record, nogc),
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => typed_array_length::<u64>(agent, &ta_record, nogc),
        };
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback) = is_callable(callback, nogc) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };
        let callback = callback.scope(agent, nogc);
        let this_arg = this_arg.scope(agent, nogc);
        let scoped_o = o.scope(agent, nogc);
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::from(SmallInteger::from(k as u32));
            // b. Let kValue be ! Get(O, Pk).
            let k_value = unwrap_try(try_get(agent, o, pk, gc.nogc()));
            // c. Let testResult be ToBoolean(? Call(callback, thisArg, « kValue, 𝔽(k), O »)).
            let call = call_function(
                agent,
                callback.get(agent),
                this_arg.get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    k_value.unbind(),
                    Number::try_from(k).unwrap().into_value(),
                    o.into_value().unbind(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let test_result = to_boolean(agent, call);
            // d. If testResult is false, return false.
            if !test_result {
                return Ok(false.into());
            }
            // e. Set k to k + 1.
            o = scoped_o.get(agent).bind(gc.nogc());
            k += 1;
        }
        // 7. Return true.
        Ok(true.into())
    }

    /// ### [23.2.3.9 %TypedArray%.prototype.fill ( value [ , start [ , end ] ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.fill)
    /// The interpretation and use of the arguments of this method are
    /// the same as for Array.prototype.fill as defined in 23.1.3.7.
    fn fill<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let value = arguments.get(0).bind(gc.nogc());
        let start = arguments.get(1).bind(gc.nogc());
        let end = arguments.get(2).bind(gc.nogc());

        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let len be TypedArrayLength(taRecord).

        let o = match ta_record.object {
            TypedArray::Int8Array(_) => fill_typed_array::<i8>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Uint8Array(_) => fill_typed_array::<u8>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Uint8ClampedArray(_) => fill_typed_array::<U8Clamped>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Int16Array(_) => fill_typed_array::<i16>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Uint16Array(_) => fill_typed_array::<u16>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Int32Array(_) => fill_typed_array::<i32>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Uint32Array(_) => fill_typed_array::<u32>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::BigInt64Array(_) => fill_typed_array::<i64>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::BigUint64Array(_) => fill_typed_array::<u64>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => fill_typed_array::<f16>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Float32Array(_) => fill_typed_array::<f32>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
            TypedArray::Float64Array(_) => fill_typed_array::<f64>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            ),
        };

        o.map(|o| o.into_value())
    }

    fn filter<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!()
    }

    /// ### 23.2.3.11 %TypedArray%.prototype.find ( predicate [ , thisArg ] )(https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.find)
    ///
    /// The interpretation and use of the arguments of this method are the same as for Array.prototype.find as defined in 23.1.3.9.
    fn find<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let predicate = arguments.get(0).scope(agent, gc.nogc());
        let this_arg = arguments.get(1).scope(agent, gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        let o = o.scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, true, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Value]].
        Ok(find_rec.1)
    }

    // ### 23.2.3.12 %TypedArray%.prototype.findIndex( predicate [ , thisArg ] )(https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.findindex)
    //
    // The interpretation and use of the arguments of this method are the same as for Array.prototype.findIndex as defined in 23.1.3.10.
    fn find_index<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let predicate = arguments.get(0).scope(agent, gc.nogc());
        let this_arg = arguments.get(1).scope(agent, gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        let o = o.into_object().scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, true, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into_value())
    }

    fn find_last<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let predicate = arguments.get(0).scope(agent, gc.nogc());
        let this_arg = arguments.get(1).scope(agent, gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        let o = o.scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Value]].
        Ok(find_rec.1)
    }

    // ### 23.2.3.14 %TypedArray%.prototype.findLastIndex ( predicate [ , thisArg ] )(https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.findlastindex)
    // The interpretation and use of the arguments of this method are the same as for Array.prototype.findLastIndex as defined in 23.1.3.12.
    fn find_last_index<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let predicate = arguments.get(0).scope(agent, gc.nogc());
        let this_arg = arguments.get(1).scope(agent, gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        let o = o.into_object().scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, descending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into_value())
    }

    // ### [ 23.2.3.15 %TypedArray%.prototype.forEach ( callback [ , thisArg ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.foreach)
    // The interpretation and use of the arguments of this method are the same as for Array.prototype.forEach as defined in 23.1.3.15.
    fn for_each<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback = arguments.get(0).bind(nogc);
        let this_arg = arguments.get(1).bind(nogc);
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(nogc);
        // 3. Let len be TypedArrayLength(taRecord).
        let mut o = ta_record.object;
        let scoped_o = o.scope(agent, nogc);
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => typed_array_length::<u8>(agent, &ta_record, nogc),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, nogc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, nogc),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => typed_array_length::<u32>(agent, &ta_record, nogc),
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => typed_array_length::<u64>(agent, &ta_record, nogc),
        };
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback) = is_callable(callback, nogc) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };
        let callback = callback.scope(agent, nogc);
        let this_arg = this_arg.scope(agent, nogc);
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk: PropertyKey = k.try_into().unwrap();
            // b. Let kValue be ! Get(O, Pk).
            let k_value = unwrap_try(try_get(agent, o, pk, gc.nogc()));
            // c. Perform ? Call(callback, thisArg, « kValue, 𝔽(k), O »).
            // // SAFETY: pk is Integer, which is what we want for fk as well.
            let fk = unsafe { pk.into_value_unchecked() };
            call_function(
                agent,
                callback.get(agent),
                this_arg.get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    k_value.unbind(),
                    fk.unbind(),
                    o.into_value().unbind(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // d. Set k to k + 1.
            k += 1;
            o = scoped_o.get(agent).bind(gc.nogc());
        }
        // 7. Return undefined.
        Ok(Value::Undefined)
    }

    // ### [23.2.3.16 %TypedArray%.prototype.includes ( searchElement [ , fromIndex ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.includes)
    // The interpretation and use of the arguments of this method are the same as for Array.prototype.includes as defined in 23.1.3.16.
    fn includes<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let mut search_element = arguments.get(0).bind(nogc);
        let from_index = arguments.get(1).bind(nogc);
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(nogc);
        // 3. Let len be TypedArrayLength(taRecord).
        let mut o = ta_record.object;
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => typed_array_length::<u8>(agent, &ta_record, nogc),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, nogc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, nogc),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => typed_array_length::<u32>(agent, &ta_record, nogc),
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => typed_array_length::<u64>(agent, &ta_record, nogc),
        } as i64;
        // 4. If len = 0, return false.
        if len == 0 {
            return Ok(false.into());
        };
        // 5. Let n be ? ToIntegerOrInfinity(fromIndex).
        let from_index_is_undefined = from_index.is_undefined();
        let n = if let TryResult::Continue(n) = try_to_integer_or_infinity(agent, from_index, nogc)
        {
            n.unbind()?
        } else {
            let scoped_o = o.scope(agent, nogc);
            let scoped_search_element = search_element.scope(agent, nogc);
            let result =
                to_integer_or_infinity(agent, from_index.unbind(), gc.reborrow()).unbind()?;
            let gc = gc.nogc();
            o = scoped_o.get(agent).bind(gc);
            search_element = scoped_search_element.get(agent).bind(gc);
            result
        };
        let o = o.unbind();
        let search_element = search_element.unbind();
        let gc = gc.into_nogc();
        let o = o.bind(gc);
        let search_element = search_element.bind(gc);
        // 6. Assert: If fromIndex is undefined, then n is 0.
        if from_index_is_undefined {
            assert_eq!(n.into_i64(), 0);
        }
        // 7. If n = +∞, return false.
        let n = if n.is_pos_infinity() {
            return Ok(false.into());
        } else if n.is_neg_infinity() {
            // 8. Else if n = -∞, set n to 0.
            0
        } else {
            n.into_i64()
        };
        // 9. If n ≥ 0, then
        let mut k = if n >= 0 {
            // a. Let k be n.
            n
        } else {
            // 10. Else,
            // a. Let k be len + n.
            let k = len + n;
            // b. If k < 0, set k to 0.
            if k < 0 { 0 } else { k }
        };
        // 11. Repeat, while k < len,
        while k < len {
            // a. Let elementK be ! Get(O, ! ToString(𝔽(k))).
            let element_k = unwrap_try(try_get(
                agent,
                o,
                PropertyKey::Integer(k.try_into().unwrap()),
                gc,
            ));
            // b. If SameValueZero(searchElement, elementK) is true, return true.
            if same_value_zero(agent, search_element, element_k) {
                return Ok(true.into());
            }
            // c. Set k to k + 1.
            k += 1
        }
        // 12. Return false.
        Ok(false.into())
    }

    /// ### [23.2.3.17 %TypedArray%.prototype.indexOf ( searchElement \[ , fromIndex \] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.indexof)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for Array.prototype.indexOf as defined in 23.1.3.17.
    fn index_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let mut search_element = arguments.get(0).bind(gc.nogc());
        let from_index = arguments.get(1).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let len be TypedArrayLength(taRecord).
        let mut o = ta_record.object;
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        // 4. If len = 0, return -1𝔽.
        if len == 0 {
            return Ok((-1).into());
        };
        // 5. Let n be ? ToIntegerOrInfinity(fromIndex).
        let from_index_is_undefined = from_index.is_undefined();
        let n = if let TryResult::Continue(n) =
            try_to_integer_or_infinity(agent, from_index, gc.nogc())
        {
            n.unbind()?
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let scoped_search_element = search_element.scope(agent, gc.nogc());
            let result =
                to_integer_or_infinity(agent, from_index.unbind(), gc.reborrow()).unbind()?;
            o = scoped_o.get(agent).bind(gc.nogc());
            search_element = scoped_search_element.get(agent).bind(gc.nogc());
            result
        };
        // 6. Assert: If fromIndex is undefined, then n is 0.
        if from_index_is_undefined {
            assert_eq!(n.into_i64(), 0);
        }
        // 7. If n = +∞, return -1F.
        let n = if n.is_pos_infinity() {
            return Ok((-1).into());
        } else if n.is_neg_infinity() {
            // 8. Else if n = -∞, set n to 0.
            0
        } else {
            n.into_i64()
        };
        // 9. If n ≥ 0, then
        let k = if n >= 0 {
            // a. Let k be n.
            n
        } else {
            // 10. Else,
            // a. Let k be len + n.
            // b. If k < 0, set k to 0.
            (len + n).max(0)
        };

        let k = k as usize;
        let len = len as usize;

        // 11. Repeat, while k < len,
        let result = match o {
            TypedArray::Int8Array(_) => search_typed_element::<i8, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Uint8Array(_) => search_typed_element::<u8, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Uint8ClampedArray(_) => search_typed_element::<U8Clamped, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Int16Array(_) => search_typed_element::<i16, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Uint16Array(_) => search_typed_element::<u16, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Int32Array(_) => search_typed_element::<i32, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Uint32Array(_) => search_typed_element::<u32, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::BigInt64Array(_) => search_typed_element::<i64, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::BigUint64Array(_) => search_typed_element::<u64, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => search_typed_element::<f16, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Float32Array(_) => search_typed_element::<f32, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
            TypedArray::Float64Array(_) => search_typed_element::<f64, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            ),
        };
        Ok(result?.map_or(-1, |v| v as i64).try_into().unwrap())
    }

    /// ### [23.2.3.18 %TypedArray%.prototype.join ( separator )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.join)
    ///
    /// The interpretation and use of the arguments of this method are the
    /// same as for Array.prototype.join as defined in 23.1.3.18.
    ///
    /// This method is not generic. The this value must be an object with a
    /// `[[TypedArrayName]]` internal slot.
    fn join<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let separator = arguments.get(0).bind(nogc);
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(nogc);
        let mut o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let (len, element_size) = match o {
            TypedArray::Int8Array(_) => (
                typed_array_length::<i8>(agent, &ta_record, nogc),
                core::mem::size_of::<i8>(),
            ),
            TypedArray::Uint8Array(_) => (
                typed_array_length::<u8>(agent, &ta_record, nogc),
                core::mem::size_of::<u8>(),
            ),
            TypedArray::Uint8ClampedArray(_) => (
                typed_array_length::<U8Clamped>(agent, &ta_record, nogc),
                core::mem::size_of::<U8Clamped>(),
            ),
            TypedArray::Int16Array(_) => (
                typed_array_length::<i16>(agent, &ta_record, nogc),
                core::mem::size_of::<i16>(),
            ),
            TypedArray::Uint16Array(_) => (
                typed_array_length::<u16>(agent, &ta_record, nogc),
                core::mem::size_of::<u16>(),
            ),
            TypedArray::Int32Array(_) => (
                typed_array_length::<i32>(agent, &ta_record, nogc),
                core::mem::size_of::<i32>(),
            ),
            TypedArray::Uint32Array(_) => (
                typed_array_length::<u32>(agent, &ta_record, nogc),
                core::mem::size_of::<u32>(),
            ),
            TypedArray::BigInt64Array(_) => (
                typed_array_length::<i64>(agent, &ta_record, nogc),
                core::mem::size_of::<i64>(),
            ),
            TypedArray::BigUint64Array(_) => (
                typed_array_length::<u64>(agent, &ta_record, nogc),
                core::mem::size_of::<u64>(),
            ),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => (
                typed_array_length::<f16>(agent, &ta_record, nogc),
                core::mem::size_of::<f16>(),
            ),
            TypedArray::Float32Array(_) => (
                typed_array_length::<f32>(agent, &ta_record, nogc),
                core::mem::size_of::<f32>(),
            ),
            TypedArray::Float64Array(_) => (
                typed_array_length::<f64>(agent, &ta_record, nogc),
                core::mem::size_of::<f64>(),
            ),
        };
        // 4. If separator is undefined, let sep be ",".
        let (sep_string, recheck_buffer) = if separator.is_undefined() {
            (String::from_small_string(","), false)
        } else if let Ok(sep) = String::try_from(separator) {
            (sep, false)
        } else {
            // 5. Else, let sep be ? ToString(separator).
            let scoped_o = o.scope(agent, nogc);
            let result = to_string(agent, separator.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let gc = gc.nogc();
            o = scoped_o.get(agent).bind(gc);
            (result, true)
        };
        let o = o.unbind();
        let sep_string = sep_string.unbind();
        let gc = gc.into_nogc();
        let o = o.bind(gc);
        let sep_string = sep_string.bind(gc);
        if len == 0 {
            return Ok(String::EMPTY_STRING.into_value());
        }
        let owned_sep = match &sep_string {
            String::String(heap_string) => Some(heap_string.as_str(agent).to_owned()),
            String::SmallString(_) => None,
        };
        let sep = match &owned_sep {
            Some(str_data) => str_data.as_str(),
            None => {
                let String::SmallString(sep) = &sep_string else {
                    unreachable!();
                };
                sep.as_str()
            }
        };
        // 6. Let R be the empty String.
        let mut r = std::string::String::with_capacity(len * 3);
        // 7. Let k be 0.
        // 8. Repeat, while k < len,
        let offset = o.byte_offset(agent);
        let viewed_array_buffer = o.get_viewed_array_buffer(agent, gc);
        // Note: Above ToString might have detached the ArrayBuffer or shrunk its length.
        let (is_invalid_typed_array, after_len) = if recheck_buffer {
            let is_detached = is_detached_buffer(agent, viewed_array_buffer);
            let ta_record =
                make_typed_array_with_buffer_witness_record(agent, o, Ordering::Unordered, gc);
            match o {
                TypedArray::Int8Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<i8>(agent, &ta_record, gc),
                    typed_array_length::<i8>(agent, &ta_record, gc),
                ),
                TypedArray::Uint8Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<u8>(agent, &ta_record, gc),
                    typed_array_length::<u8>(agent, &ta_record, gc),
                ),
                TypedArray::Uint8ClampedArray(_) => (
                    is_detached || is_typed_array_out_of_bounds::<U8Clamped>(agent, &ta_record, gc),
                    typed_array_length::<U8Clamped>(agent, &ta_record, gc),
                ),
                TypedArray::Int16Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<i16>(agent, &ta_record, gc),
                    typed_array_length::<i16>(agent, &ta_record, gc),
                ),
                TypedArray::Uint16Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<u16>(agent, &ta_record, gc),
                    typed_array_length::<u16>(agent, &ta_record, gc),
                ),
                TypedArray::Int32Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<i32>(agent, &ta_record, gc),
                    typed_array_length::<i32>(agent, &ta_record, gc),
                ),
                TypedArray::Uint32Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<u32>(agent, &ta_record, gc),
                    typed_array_length::<u32>(agent, &ta_record, gc),
                ),
                TypedArray::BigInt64Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<i64>(agent, &ta_record, gc),
                    typed_array_length::<i64>(agent, &ta_record, gc),
                ),
                TypedArray::BigUint64Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<u64>(agent, &ta_record, gc),
                    typed_array_length::<u64>(agent, &ta_record, gc),
                ),
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<f16>(agent, &ta_record, gc),
                    typed_array_length::<f16>(agent, &ta_record, gc),
                ),
                TypedArray::Float32Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<f32>(agent, &ta_record, gc),
                    typed_array_length::<f32>(agent, &ta_record, gc),
                ),
                TypedArray::Float64Array(_) => (
                    is_detached || is_typed_array_out_of_bounds::<f64>(agent, &ta_record, gc),
                    typed_array_length::<f64>(agent, &ta_record, gc),
                ),
            }
        } else {
            // Note: Growable SharedArrayBuffers are a thing, and can change the
            // length at any point in time but they can never shrink the buffer.
            // Hence the TypedArray or any of its indexes rae never invalidated.
            (false, len)
        };
        for k in 0..len {
            // a. If k > 0, set R to the string-concatenation of R and sep.
            if k > 0 {
                r.push_str(sep);
            }
            // c. If element is not undefined, then
            if is_invalid_typed_array || k >= after_len {
                // Note: element is undefined if the ViewedArrayBuffer was
                // detached by ToString call, or was shrunk to less than len.
                continue;
            }
            let byte_index_in_buffer = k * element_size + offset;
            // b. Let element be ! Get(O, ! ToString(𝔽(k))).
            let element = match o {
                TypedArray::Int8Array(_) => get_value_from_buffer::<i8>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Uint8Array(_) => get_value_from_buffer::<u8>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Uint8ClampedArray(_) => get_value_from_buffer::<U8Clamped>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Int16Array(_) => get_value_from_buffer::<i16>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Uint16Array(_) => get_value_from_buffer::<u16>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Int32Array(_) => get_value_from_buffer::<i32>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Uint32Array(_) => get_value_from_buffer::<u32>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::BigInt64Array(_) => get_value_from_buffer::<i64>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::BigUint64Array(_) => get_value_from_buffer::<u64>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => get_value_from_buffer::<f16>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Float32Array(_) => get_value_from_buffer::<f32>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
                TypedArray::Float64Array(_) => get_value_from_buffer::<f64>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                ),
            };
            // i. Let S be ! ToString(element).
            let s = unwrap_try(try_to_string(agent, element, gc)).unwrap();
            // ii. Set R to the string-concatenation of R and S.
            r.push_str(s.as_str(agent));
            // d. Set k to k + 1.
        }
        // 9. Return R.
        Ok(String::from_string(agent, r, gc).into_value().unbind())
    }

    /// ### [23.2.3.19 %TypedArray%.prototype.keys ( )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.keys)
    fn keys<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        // 2. Perform ? ValidateTypedArray(O, seq-cst).
        let o = validate_typed_array(agent, this_value, Ordering::SeqCst, gc)?.object;
        // 3. Return CreateArrayIterator(O, key).
        Ok(
            ArrayIterator::from_object(agent, o.into_object(), CollectionIteratorKind::Key)
                .into_value(),
        )
    }

    // ### [23.2.3.20 %TypedArray%.prototype.lastIndexOf ( searchElement [ , fromIndex ] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.lastindexof)
    // The interpretation and use of the arguments of this method are the same as for Array.prototype.lastIndexOf as defined in 23.1.3.20.
    fn last_index_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let search_element = arguments.get(0).bind(gc.nogc());
        let from_index = if arguments.len() > 1 {
            Some(arguments.get(1).bind(gc.nogc()))
        } else {
            None
        };
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let len be TypedArrayLength(taRecord).
        let o = ta_record.object;
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        // 4. If len = 0, return -1𝔽.
        if len == 0 {
            return Ok((-1).into());
        };
        let o = o.scope(agent, gc.nogc());
        let from_index = from_index.map(|i| i.scope(agent, gc.nogc()));
        let search_element = search_element.scope(agent, gc.nogc());
        // 5. If fromIndex is present, let n be ? ToIntegerOrInfinity(fromIndex); else let n be len - 1.
        let k = if let Some(from_index) = from_index {
            let n = to_integer_or_infinity(agent, from_index.get(agent), gc.reborrow()).unbind()?;
            // 6. If n = -∞, return -1𝔽.
            if n.is_neg_infinity() {
                return Ok((-1).into());
            }
            // 7. If n ≥ 0, then
            if n.into_i64() >= 0 {
                // a. Let k be min(n, len - 1).
                n.into_i64().min(len - 1)
            } else {
                // Note: n is negative, so n < len + n < len.
                // 8. Else,
                // a. Let k be len + n.
                len + n.into_i64()
            }
        } else {
            len - 1
        };

        let k = k as usize;
        let len = len as usize;

        // 9. Repeat, while k ≥ 0,
        let o = o.get(agent);
        let result = match o {
            TypedArray::Int8Array(_) => search_typed_element::<i8, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Uint8Array(_) => search_typed_element::<u8, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Uint8ClampedArray(_) => search_typed_element::<U8Clamped, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Int16Array(_) => search_typed_element::<i16, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Uint16Array(_) => search_typed_element::<u16, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Int32Array(_) => search_typed_element::<i32, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Uint32Array(_) => search_typed_element::<u32, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::BigInt64Array(_) => search_typed_element::<i64, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::BigUint64Array(_) => search_typed_element::<u64, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => search_typed_element::<f16, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Float32Array(_) => search_typed_element::<f32, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
            TypedArray::Float64Array(_) => search_typed_element::<f64, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            ),
        };
        Ok(result
            .unbind()?
            .map_or(-1, |v| v as i64)
            .try_into()
            .unwrap())
    }

    /// ### [23.2.3.21 get %TypedArray%.prototype.length](https://tc39.es/ecma262/#sec-get-%typedarray%.prototype.length)
    fn get_length<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has [[ViewedArrayBuffer]] and [[ArrayLength]] internal slots.
        let o = require_internal_slot_typed_array(agent, this_value, gc)?;
        // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc);
        // 5. If IsTypedArrayOutOfBounds(taRecord) is true, return +0𝔽.
        if match o {
            TypedArray::Int8Array(_) => is_typed_array_out_of_bounds::<i8>(agent, &ta_record, gc),
            TypedArray::Uint8Array(_) => is_typed_array_out_of_bounds::<u8>(agent, &ta_record, gc),
            TypedArray::Uint8ClampedArray(_) => {
                is_typed_array_out_of_bounds::<U8Clamped>(agent, &ta_record, gc)
            }
            TypedArray::Int16Array(_) => is_typed_array_out_of_bounds::<i16>(agent, &ta_record, gc),
            TypedArray::Uint16Array(_) => {
                is_typed_array_out_of_bounds::<u16>(agent, &ta_record, gc)
            }
            TypedArray::Int32Array(_) => is_typed_array_out_of_bounds::<i32>(agent, &ta_record, gc),
            TypedArray::Uint32Array(_) => {
                is_typed_array_out_of_bounds::<u32>(agent, &ta_record, gc)
            }
            TypedArray::BigInt64Array(_) => {
                is_typed_array_out_of_bounds::<i64>(agent, &ta_record, gc)
            }
            TypedArray::BigUint64Array(_) => {
                is_typed_array_out_of_bounds::<u64>(agent, &ta_record, gc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => {
                is_typed_array_out_of_bounds::<f16>(agent, &ta_record, gc)
            }
            TypedArray::Float32Array(_) => {
                is_typed_array_out_of_bounds::<f32>(agent, &ta_record, gc)
            }
            TypedArray::Float64Array(_) => {
                is_typed_array_out_of_bounds::<f64>(agent, &ta_record, gc)
            }
        } {
            return Ok(Value::pos_zero());
        }
        // 6. Let length be TypedArrayLength(taRecord).
        let length = match o {
            TypedArray::Int8Array(_) => typed_array_length::<i8>(agent, &ta_record, gc),
            TypedArray::Uint8Array(_) => typed_array_length::<u8>(agent, &ta_record, gc),
            TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<U8Clamped>(agent, &ta_record, gc)
            }
            TypedArray::Int16Array(_) => typed_array_length::<i16>(agent, &ta_record, gc),
            TypedArray::Uint16Array(_) => typed_array_length::<u16>(agent, &ta_record, gc),
            TypedArray::Int32Array(_) => typed_array_length::<i32>(agent, &ta_record, gc),
            TypedArray::Uint32Array(_) => typed_array_length::<u32>(agent, &ta_record, gc),
            TypedArray::BigInt64Array(_) => typed_array_length::<i64>(agent, &ta_record, gc),
            TypedArray::BigUint64Array(_) => typed_array_length::<u64>(agent, &ta_record, gc),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc),
            TypedArray::Float32Array(_) => typed_array_length::<f32>(agent, &ta_record, gc),
            TypedArray::Float64Array(_) => typed_array_length::<f64>(agent, &ta_record, gc),
        } as i64;
        // 7. Return 𝔽(length).
        Ok(Value::try_from(length).unwrap())
    }

    fn map<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!()
    }

    /// ### [23.2.3.23 %TypedArray%.prototype.reduce ( callback [ , initialValue ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.reduce)
    /// The interpretation and use of the arguments of this method are
    /// the same as for Array.prototype.reduce as defined in 23.1.3.24.
    fn reduce<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let callback = arguments.get(0).bind(gc.nogc());
        let initial_value = if arguments.len() >= 2 {
            Some(arguments.get(1).bind(gc.nogc()))
        } else {
            None
        };
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_) => typed_array_length::<i8>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint8Array(_) => typed_array_length::<u8>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<U8Clamped>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) => typed_array_length::<i16>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint16Array(_) => typed_array_length::<u16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_) => typed_array_length::<i32>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint32Array(_) => typed_array_length::<u32>(agent, &ta_record, gc.nogc()),
            TypedArray::BigInt64Array(_) => typed_array_length::<i64>(agent, &ta_record, gc.nogc()),
            TypedArray::BigUint64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Float32Array(_) => typed_array_length::<f32>(agent, &ta_record, gc.nogc()),
            TypedArray::Float64Array(_) => typed_array_length::<f64>(agent, &ta_record, gc.nogc()),
        } as i64;
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback) = is_callable(callback, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };
        // 5. If len = 0 and initialValue is not present, throw a TypeError exception.
        if len == 0 && initial_value.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length is 0 and no initial value provided",
                gc.into_nogc(),
            ));
        };
        // 6. Let k be 0.
        let mut k = 0;
        // 7. Let accumulator be undefined.
        // 8. If initialValue is present, then
        //    a. Set accumulator to initialValue.
        let mut accumulator = if let Some(init) = initial_value {
            init.scope(agent, gc.nogc())
        } else {
            // 9. Else,
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();
            // b. Set accumulator to ! Get(O, Pk).
            let result = unwrap_try(try_get(agent, o, pk, gc.nogc()));
            // c. Set k to k + 1.
            k += 1;
            result.scope(agent, gc.nogc())
        };
        let scoped_callback = callback.scope(agent, gc.nogc());
        let scoped_o = o.scope(agent, gc.nogc());
        // 10. Repeat, while k < len,
        while k < len {
            let k_int = k.try_into().unwrap();
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k_int);
            // b. Let kValue be ! Get(O, Pk).
            let k_value = unwrap_try(try_get(agent, scoped_o.get(agent), pk, gc.nogc()));
            // c. Set accumulator to ? Call(callback, undefined, « accumulator, kValue, 𝔽(k), O »).
            let result = call_function(
                agent,
                scoped_callback.get(agent),
                Value::Undefined,
                Some(ArgumentsList::from_mut_slice(&mut [
                    accumulator.get(agent),
                    k_value.unbind(),
                    Number::from(k_int).into_value(),
                    scoped_o.get(agent).into_value(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // SAFETY: accumulator is not shared.
            unsafe { accumulator.replace(agent, result.unbind()) };
            // d. Set k to k + 1.
            k += 1;
        }
        // 11. Return accumulator.
        Ok(accumulator.get(agent))
    }

    /// ### [23.2.3.24 %TypedArray%.prototype.reduceRight ( callback [ , initialValue ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.reduceright)
    /// The interpretation and use of the arguments of this method
    /// are the same as for Array.prototype.reduceRight as defined in 23.1.3.25.
    fn reduce_right<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let callback = arguments.get(0).bind(gc.nogc());
        let initial_value = if arguments.len() >= 2 {
            Some(arguments.get(1).bind(gc.nogc()))
        } else {
            None
        };
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_) => typed_array_length::<i8>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint8Array(_) => typed_array_length::<u8>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<U8Clamped>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) => typed_array_length::<i16>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint16Array(_) => typed_array_length::<u16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_) => typed_array_length::<i32>(agent, &ta_record, gc.nogc()),
            TypedArray::Uint32Array(_) => typed_array_length::<u32>(agent, &ta_record, gc.nogc()),
            TypedArray::BigInt64Array(_) => typed_array_length::<i64>(agent, &ta_record, gc.nogc()),
            TypedArray::BigUint64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Float32Array(_) => typed_array_length::<f32>(agent, &ta_record, gc.nogc()),
            TypedArray::Float64Array(_) => typed_array_length::<f64>(agent, &ta_record, gc.nogc()),
        } as i64;
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback) = is_callable(callback, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };
        // 5. If len = 0 and initialValue is not present, throw a TypeError exception.
        if len == 0 && initial_value.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Array length is 0 and no initial value provided",
                gc.into_nogc(),
            ));
        };
        // 6. Let k be len - 1.
        let mut k = len - 1;
        // 7. Let accumulator be undefined.
        // 8. If initialValue is present, then
        //    a. Set accumulator to initialValue.
        let mut accumulator = if let Some(init) = initial_value {
            init.scope(agent, gc.nogc())
        } else {
            // 9. Else,
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();
            // b. Set accumulator to ! Get(O, Pk).
            let result = unwrap_try(try_get(agent, o, pk, gc.nogc()));
            // c. Set k to k - 1.
            k -= 1;
            result.scope(agent, gc.nogc())
        };
        let scoped_callback = callback.scope(agent, gc.nogc());
        let scoped_o = o.scope(agent, gc.nogc());
        // 10. Repeat, while k < len,
        while k >= 0 {
            let k_int = k.try_into().unwrap();
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k_int);
            // b. Let kValue be ! Get(O, Pk).
            let k_value = unwrap_try(try_get(agent, scoped_o.get(agent), pk, gc.nogc()));
            // c. Set accumulator to ? Call(callback, undefined, « accumulator, kValue, 𝔽(k), O »).
            let result = call_function(
                agent,
                scoped_callback.get(agent),
                Value::Undefined,
                Some(ArgumentsList::from_mut_slice(&mut [
                    accumulator.get(agent),
                    k_value.unbind(),
                    Number::from(k_int).into_value(),
                    scoped_o.get(agent).into_value(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // SAFETY: accumulator is not shared.
            unsafe { accumulator.replace(agent, result.unbind()) };
            // d. Set k to k - 1.
            k -= 1;
        }
        // 11. Return accumulator.
        Ok(accumulator.get(agent))
    }

    /// ### [23.2.3.25 %TypedArray%.prototype.reverse ( )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.reverse)
    /// The interpretation and use of the arguments of this method are the same as for Array.prototype.reverse as defined in 23.1.3.26.
    fn reverse<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc)
            .unbind()?
            .bind(gc);
        // 3. Let len be TypedArrayLength(taRecord).
        let o = ta_record.object;
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => typed_array_length::<u8>(agent, &ta_record, gc),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc)
            }
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => typed_array_length::<u32>(agent, &ta_record, gc),
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => typed_array_length::<u64>(agent, &ta_record, gc),
        } as i64;
        // 4. Let middle be floor(len / 2).
        // 5. Let lower be 0.
        let len = len as usize;
        // 6. Repeat, while lower ≠ middle,
        //    a. Let upper be len - lower - 1.
        //    b. Let upperP be ! ToString(𝔽(upper)).
        //    c. Let lowerP be ! ToString(𝔽(lower)).
        //    d. Let lowerValue be ! Get(O, lowerP).
        //    e. Let upperValue be ! Get(O, upperP).
        //    f. Perform ! Set(O, lowerP, upperValue, true).
        //    g. Perform ! Set(O, upperP, lowerValue, true).
        //    h. Set lower to lower + 1.
        match o {
            TypedArray::Int8Array(_) => reverse_typed_array::<i8>(agent, o, len, gc)?,
            TypedArray::Uint8Array(_) => reverse_typed_array::<u8>(agent, o, len, gc)?,
            TypedArray::Uint8ClampedArray(_) => {
                reverse_typed_array::<U8Clamped>(agent, o, len, gc)?
            }
            TypedArray::Int16Array(_) => reverse_typed_array::<i16>(agent, o, len, gc)?,
            TypedArray::Uint16Array(_) => reverse_typed_array::<u16>(agent, o, len, gc)?,
            TypedArray::Int32Array(_) => reverse_typed_array::<i32>(agent, o, len, gc)?,
            TypedArray::Uint32Array(_) => reverse_typed_array::<u32>(agent, o, len, gc)?,
            TypedArray::BigInt64Array(_) => reverse_typed_array::<i64>(agent, o, len, gc)?,
            TypedArray::BigUint64Array(_) => reverse_typed_array::<u64>(agent, o, len, gc)?,
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => reverse_typed_array::<f16>(agent, o, len, gc)?,
            TypedArray::Float32Array(_) => reverse_typed_array::<f32>(agent, o, len, gc)?,
            TypedArray::Float64Array(_) => reverse_typed_array::<f64>(agent, o, len, gc)?,
        };
        // 7. Return O.
        Ok(o.into_value())
    }

    fn set<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!()
    }

    fn slice<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!()
    }

    /// ### [23.2.3.28 get %TypedArray%.prototype.some](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.some)
    fn some<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let callback = arguments.get(0).bind(nogc);
        let this_arg = arguments.get(1).bind(nogc);
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(nogc);
        let mut o = ta_record.object;
        // 3. Let len be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => typed_array_length::<u8>(agent, &ta_record, nogc),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, nogc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, nogc),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => typed_array_length::<u32>(agent, &ta_record, nogc),
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => typed_array_length::<u64>(agent, &ta_record, nogc),
        };
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback) = is_callable(callback, nogc) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };
        let callback = callback.scope(agent, nogc);
        let this_arg = this_arg.scope(agent, nogc);
        let scoped_o = o.scope(agent, nogc);
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Repeat, while k < len,
        while k < len {
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::from(SmallInteger::from(k as u32));
            // b. Let kValue be ! Get(O, Pk).
            let k_value = unwrap_try(try_get(agent, o, pk, gc.nogc()));
            // c. Let testResult be ToBoolean(? Call(callback, thisArg, « kValue, 𝔽(k), O »)).
            let call = call_function(
                agent,
                callback.get(agent),
                this_arg.get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    k_value.unbind(),
                    Number::try_from(k).unwrap().into_value().unbind(),
                    o.into_value().unbind(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let test_result = to_boolean(agent, call);
            // d. If testResult is true, return true.
            if test_result {
                return Ok(true.into());
            }
            // e. Set k to k + 1.
            o = scoped_o.get(agent).bind(gc.nogc());
            k += 1;
        }
        // 7. Return false.
        Ok(false.into())
    }

    /// ### [23.2.3.29 %TypedArray%.prototype.sort ( comparator )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.sort)
    /// This is a distinct method that, except as described below,
    /// implements the same requirements as those of Array.prototype.sort as defined in 23.1.3.30.
    /// The implementation of this method may be optimized with the knowledge that
    /// the this value is an object that has a fixed length and whose integer-indexed properties are not sparse.
    /// This method is not generic. The this value must be an object with a [[TypedArrayName]] internal slot.
    fn sort<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let comparator = arguments.get(0).bind(nogc);
        // 1. If comparator is not undefined and IsCallable(comparator) is false, throw a TypeError exception.
        let comparator = if comparator.is_undefined() {
            None
        } else if let Some(comparator) = is_callable(comparator, nogc) {
            Some(comparator.scope(agent, nogc))
        } else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "The comparison function must be either a function or undefined",
                gc.into_nogc(),
            ));
        };
        // 2. Let obj be the this value.
        let obj = this_value;
        // 3. Let taRecord be ? ValidateTypedArray(obj, seq-cst).
        let ta_record = validate_typed_array(agent, obj, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(nogc);
        let obj = ta_record.object;
        // 4. Let len be TypedArrayLength(taRecord).
        let len = match obj {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => typed_array_length::<u8>(agent, &ta_record, nogc),
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, nogc)
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, nogc),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => typed_array_length::<u32>(agent, &ta_record, nogc),
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => typed_array_length::<u64>(agent, &ta_record, nogc),
        };
        // 5. NOTE: The following closure performs a numeric comparison rather than the string comparison used in 23.1.3.30.
        // 6. Let SortCompare be a new Abstract Closure with parameters (x, y) that captures comparator and performs the following steps when called:
        //    a. Return ? CompareTypedArrayElements(x, y, comparator).
        // 7. Let sortedList be ? SortIndexedProperties(obj, len, SortCompare, read-through-holes).
        let scoped_obj = obj.scope(agent, nogc);
        if let Some(comparator) = comparator {
            match scoped_obj.get(agent) {
                TypedArray::Int8Array(_) => {
                    sort_comparator_typed_array::<i8>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::Uint8Array(_) => {
                    sort_comparator_typed_array::<u8>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::Uint8ClampedArray(_) => sort_comparator_typed_array::<U8Clamped>(
                    agent,
                    obj.unbind(),
                    len,
                    comparator,
                    gc,
                )?,
                TypedArray::Int16Array(_) => {
                    sort_comparator_typed_array::<i16>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::Uint16Array(_) => {
                    sort_comparator_typed_array::<u16>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::Int32Array(_) => {
                    sort_comparator_typed_array::<i32>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::Uint32Array(_) => {
                    sort_comparator_typed_array::<u32>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::BigInt64Array(_) => {
                    sort_comparator_typed_array::<i64>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::BigUint64Array(_) => {
                    sort_comparator_typed_array::<u64>(agent, obj.unbind(), len, comparator, gc)?
                }
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => {
                    sort_comparator_typed_array::<f16>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::Float32Array(_) => {
                    sort_comparator_typed_array::<f32>(agent, obj.unbind(), len, comparator, gc)?
                }
                TypedArray::Float64Array(_) => {
                    sort_comparator_typed_array::<f64>(agent, obj.unbind(), len, comparator, gc)?
                }
            };
        } else {
            let nogc = gc.into_nogc();
            match scoped_obj.get(agent) {
                TypedArray::Int8Array(_) => {
                    sort_partial_cmp_typed_array::<i8>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::Uint8Array(_) => {
                    sort_partial_cmp_typed_array::<u8>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::Uint8ClampedArray(_) => sort_partial_cmp_typed_array::<U8Clamped>(
                    agent,
                    scoped_obj.get(agent),
                    len,
                    nogc,
                )?,
                TypedArray::Int16Array(_) => {
                    sort_partial_cmp_typed_array::<i16>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::Uint16Array(_) => {
                    sort_partial_cmp_typed_array::<u16>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::Int32Array(_) => {
                    sort_partial_cmp_typed_array::<i32>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::Uint32Array(_) => {
                    sort_partial_cmp_typed_array::<u32>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::BigInt64Array(_) => {
                    sort_partial_cmp_typed_array::<i64>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::BigUint64Array(_) => {
                    sort_partial_cmp_typed_array::<u64>(agent, scoped_obj.get(agent), len, nogc)?
                }
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => {
                    sort_partial_cmp_typed_array::<f16>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::Float32Array(_) => {
                    sort_total_cmp_typed_array::<f32>(agent, scoped_obj.get(agent), len, nogc)?
                }
                TypedArray::Float64Array(_) => {
                    sort_total_cmp_typed_array::<f64>(agent, scoped_obj.get(agent), len, nogc)?
                }
            };
        };
        // 10. Return obj.
        let obj = scoped_obj.get(agent);
        Ok(obj.into_value())
    }

    fn subarray<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!();
    }

    fn to_locale_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!();
    }

    /// ### [23.2.3.32 %TypedArray%.prototype.toReversed ( )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-array.prototype.tospliced)
    fn to_reversed<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 3. Let length be TypedArrayLength(taRecord).
        let len = match o {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                typed_array_length::<u8>(agent, &ta_record, gc.nogc())
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                typed_array_length::<u16>(agent, &ta_record, gc.nogc())
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => typed_array_length::<f16>(agent, &ta_record, gc.nogc()),
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                typed_array_length::<u32>(agent, &ta_record, gc.nogc())
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                typed_array_length::<u64>(agent, &ta_record, gc.nogc())
            }
        } as i64;
        let scoped_o = o.scope(agent, gc.nogc());
        // 4. Let A be ? TypedArrayCreateSameType(O, « 𝔽(length) »).
        let a = typed_array_create_same_type(agent, scoped_o.get(agent), len, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let scope_a = a.scope(agent, gc.nogc());
        // 5. Let k be 0.
        let mut k = 0;
        // 6. Repeat, while k < length,
        while k < len {
            // a. Let from be ! ToString(𝔽(length - k - 1)).
            let from = PropertyKey::Integer((len - k - 1).try_into().unwrap());
            // b. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(k).unwrap();
            // c. Let fromValue be ! Get(O, from).
            let from_value = get(agent, scoped_o.get(agent), from, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // d. Perform ! Set(A, Pk, fromValue, true).
            unwrap_try(try_set(
                agent,
                scope_a.get(agent).into_object(),
                pk,
                from_value.unbind(),
                true,
                gc.nogc(),
            ))
            .unwrap();
            // . Set k to k + 1.
            k += 1;
        }
        // 7. Return A.
        Ok(scope_a.get(agent).into_value())
    }

    fn to_sorted<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!();
    }

    /// ### [23.2.3.35 %TypedArray%.prototype.values ( )](https://tc39.es/ecma262/#sec-get-%typedarray%.prototype-%symbol.tostringtag%)
    fn values<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        // 2. Perform ? ValidateTypedArray(O, seq-cst).
        let o = validate_typed_array(agent, this_value, Ordering::SeqCst, gc)?.object;
        // 3. Return CreateArrayIterator(O, value).
        Ok(
            ArrayIterator::from_object(agent, o.into_object(), CollectionIteratorKind::Value)
                .into_value(),
        )
    }

    fn with<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        todo!();
    }

    /// ### [23.2.3.38 get %TypedArray%.prototype \[ %Symbol.toStringTag% \]](https://tc39.es/ecma262/#sec-get-%typedarray%.prototype-%symbol.tostringtag%)
    fn get_to_string_tag<'gc>(
        _agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Let O be the this value.
        if let Ok(o) = TypedArray::try_from(this_value) {
            // 4. Let name be O.[[TypedArrayName]].
            // 5. Assert: name is a String.
            // 6. Return name.
            match o {
                TypedArray::Int8Array(_) => Ok(BUILTIN_STRING_MEMORY.Int8Array.into()),
                TypedArray::Uint8Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint8Array.into()),
                TypedArray::Uint8ClampedArray(_) => {
                    Ok(BUILTIN_STRING_MEMORY.Uint8ClampedArray.into())
                }
                TypedArray::Int16Array(_) => Ok(BUILTIN_STRING_MEMORY.Int16Array.into()),
                TypedArray::Uint16Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint16Array.into()),
                TypedArray::Int32Array(_) => Ok(BUILTIN_STRING_MEMORY.Int32Array.into()),
                TypedArray::Uint32Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint32Array.into()),
                TypedArray::BigInt64Array(_) => Ok(BUILTIN_STRING_MEMORY.BigInt64Array.into()),
                TypedArray::BigUint64Array(_) => Ok(BUILTIN_STRING_MEMORY.BigUint64Array.into()),
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => Ok(BUILTIN_STRING_MEMORY.Float16Array.into()),
                TypedArray::Float32Array(_) => Ok(BUILTIN_STRING_MEMORY.Float32Array.into()),
                TypedArray::Float64Array(_) => Ok(BUILTIN_STRING_MEMORY.Float64Array.into()),
            }
        } else {
            // 2. If O is not an Object, return undefined.
            // 3. If O does not have a [[TypedArrayName]] internal slot, return undefined.
            Ok(Value::Undefined)
        }
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.typed_array_prototype();
        let typed_array_constructor = intrinsics.typed_array();
        let typed_array_prototype_values = intrinsics.typed_array_prototype_values();
        let array_prototype_to_string = intrinsics.array_prototype_to_string();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(38)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<TypedArrayPrototypeAt>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetBuffer>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetByteLength>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetByteOffset>()
            .with_constructor_property(typed_array_constructor)
            .with_builtin_function_property::<TypedArrayPrototypeCopyWithin>()
            .with_builtin_function_property::<TypedArrayPrototypeEntries>()
            .with_builtin_function_property::<TypedArrayPrototypeEvery>()
            .with_builtin_function_property::<TypedArrayPrototypeFill>()
            .with_builtin_function_property::<TypedArrayPrototypeFilter>()
            .with_builtin_function_property::<TypedArrayPrototypeFind>()
            .with_builtin_function_property::<TypedArrayPrototypeFindIndex>()
            .with_builtin_function_property::<TypedArrayPrototypeFindLast>()
            .with_builtin_function_property::<TypedArrayPrototypeFindLastIndex>()
            .with_builtin_function_property::<TypedArrayPrototypeForEach>()
            .with_builtin_function_property::<TypedArrayPrototypeIncludes>()
            .with_builtin_function_property::<TypedArrayPrototypeIndexOf>()
            .with_builtin_function_property::<TypedArrayPrototypeJoin>()
            .with_builtin_function_property::<TypedArrayPrototypeKeys>()
            .with_builtin_function_property::<TypedArrayPrototypeLastIndexOf>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetLength>()
            .with_builtin_function_property::<TypedArrayPrototypeMap>()
            .with_builtin_function_property::<TypedArrayPrototypeReduce>()
            .with_builtin_function_property::<TypedArrayPrototypeReduceRight>()
            .with_builtin_function_property::<TypedArrayPrototypeReverse>()
            .with_builtin_function_property::<TypedArrayPrototypeSet>()
            .with_builtin_function_property::<TypedArrayPrototypeSlice>()
            .with_builtin_function_property::<TypedArrayPrototypeSome>()
            .with_builtin_function_property::<TypedArrayPrototypeSort>()
            .with_builtin_function_property::<TypedArrayPrototypeSubarray>()
            .with_builtin_function_property::<TypedArrayPrototypeToLocaleString>()
            .with_builtin_function_property::<TypedArrayPrototypeToReversed>()
            .with_builtin_function_property::<TypedArrayPrototypeToSorted>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.toString.into())
                    .with_value(array_prototype_to_string.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_builtin_intrinsic_function_property::<TypedArrayPrototypeValues>()
            .with_builtin_function_property::<TypedArrayPrototypeWith>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetToStringTag>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(typed_array_prototype_values.into_value())
                    .with_enumerable(TypedArrayPrototypeValues::ENUMERABLE)
                    .with_configurable(TypedArrayPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .build();
    }
}

#[inline]
pub(crate) fn require_internal_slot_typed_array<'a>(
    agent: &mut Agent,
    o: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    // 1. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
    TypedArray::try_from(o.unbind()).map_err(|_| {
        agent.throw_exception_with_static_message(
            crate::ecmascript::execution::agent::ExceptionType::TypeError,
            "Expected this to be TypedArray",
            gc,
        )
    })
}

fn search_typed_element<'a, T: Viewable + std::fmt::Debug, const ASCENDING: bool>(
    agent: &mut Agent,
    ta: TypedArray,
    search_element: Value,
    k: usize,
    len: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Option<usize>> {
    let search_element = T::try_from_value(agent, search_element);
    let Some(search_element) = search_element else {
        return Ok(None);
    };
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_slice(agent);
    if byte_slice.is_empty() {
        return Ok(None);
    }
    if byte_offset > byte_slice.len() {
        // Start index is out of bounds.
        return Ok(None);
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            // End index is out of bounds.
            return Ok(None);
        }
        &byte_slice[byte_offset..end_index]
    } else {
        &byte_slice[byte_offset..]
    };
    // SAFETY: All bytes in byte_slice are initialized, and all bitwise
    // combinations of T are valid values. Alignment of T's is
    // guaranteed by align_to itself.
    let (head, slice, _) = unsafe { byte_slice.align_to::<T>() };
    if !head.is_empty() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc,
        ));
    }
    // Length of the TypedArray may have changed between when we measured it
    // and here: We'll never try to access past the boundary of the slice if
    // the backing ArrayBuffer shrank.
    let len = len.min(slice.len());

    if ASCENDING {
        if k >= len {
            return Ok(None);
        }
        Ok(slice[k..len]
            .iter()
            .position(|&r| r == search_element)
            .map(|pos| pos + k))
    } else {
        if k >= len {
            return Ok(None);
        }
        Ok(slice[..=k].iter().rposition(|&r| r == search_element))
    }
}

fn reverse_typed_array<'a, T: Viewable + Copy + std::fmt::Debug>(
    agent: &mut Agent,
    ta: TypedArray,
    len: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc,
        ));
    }
    let slice = &mut slice[..len];
    slice.reverse();
    Ok(())
}

fn copy_within_typed_array<'a, T: Viewable + std::fmt::Debug>(
    agent: &mut Agent,
    ta: TypedArray,
    target_index: i64,
    start_index: i64,
    end_index: i64,
    before_len: i64,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let end_bound = (end_index - start_index)
        .max(0)
        .min(before_len - target_index) as usize;
    let ta_record = make_typed_array_with_buffer_witness_record(agent, ta, Ordering::SeqCst, gc);
    if is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Callback is not callable",
            gc,
        ));
    }
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let len = typed_array_length::<T>(agent, &ta_record, gc) as usize;
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(());
    }
    if byte_offset > byte_slice.len() {
        return Ok(());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc,
        ));
    }
    let slice = &mut slice[..len];
    let start_bound = start_index as usize;
    let target_index = target_index as usize;
    let before_len = before_len as usize;
    if before_len != slice.len() {
        let end_bound = (len - target_index).max(0).min(before_len - target_index);
        slice.copy_within(start_bound..end_bound, target_index);
        return Ok(());
    }
    if end_bound > 0 {
        slice.copy_within(start_bound..start_bound + end_bound, target_index);
    }
    Ok(())
}

fn fill_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    value: Value,
    start: Value,
    end: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let value = value.bind(gc.nogc());
    let start = start.bind(gc.nogc());
    let end = end.bind(gc.nogc());
    let o = ta_record.object;
    let scoped_o = o.scope(agent, gc.nogc());
    let scoped_value = value.scope(agent, gc.nogc());
    let start = start.scope(agent, gc.nogc());
    let end = end.scope(agent, gc.nogc());
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc()) as i64;
    let value = if T::IS_BIGINT {
        // 4. If O.[[ContentType]] is bigint, set value to ? ToBigInt(value).
        to_big_int(agent, scoped_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_numeric()
    } else {
        // 5. Otherwise, set value to ? ToNumber(value).
        to_number(agent, scoped_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_numeric()
    };
    let value = value.scope(agent, gc.nogc());
    // 6. Let relativeStart be ? ToIntegerOrInfinity(start).
    let relative_start = to_integer_or_infinity(agent, start.get(agent), gc.reborrow()).unbind()?;
    // 7. If relativeStart = -∞, let startIndex be 0.
    let start_index = if relative_start.is_neg_infinity() {
        0
    } else if relative_start.is_negative() {
        // 8. Else if relativeStart < 0, let startIndex be max(len + relativeStart, 0).
        (len + relative_start.into_i64()).max(0)
    } else {
        // 9. Else, let startIndex be min(relativeStart, len).
        len.min(relative_start.into_i64())
    };
    // 10. If end is undefined, let relativeEnd be len; else let relativeEnd be ? ToIntegerOrInfinity(end).
    let end_index = if end.get(agent).is_undefined() {
        len
    } else {
        let relative_end = to_integer_or_infinity(agent, end.get(agent), gc.reborrow()).unbind()?;
        // 11. If relativeEnd = -∞, let endIndex be 0.
        if relative_end.is_neg_infinity() {
            0
        } else if relative_end.is_negative() {
            // 12. Else if relativeEnd < 0, let endIndex be max(len + relativeEnd, 0).
            (len + relative_end.into_i64()).max(0)
        } else {
            // 13. Else, let endIndex be min(relativeEnd, len).
            len.min(relative_end.into_i64())
        }
    };
    let gc = gc.into_nogc();
    let ta = scoped_o.get(agent).bind(gc);
    let value = value.get(agent).bind(gc);
    // 14. Set taRecord to MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
    let ta_record = make_typed_array_with_buffer_witness_record(agent, ta, Ordering::SeqCst, gc);
    // 15. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
    if is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Callback is not callable",
            gc,
        ));
    };
    // 16. Set len to TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, gc) as i64;
    // 17. Set endIndex to min(endIndex, len).
    let end_index = len.min(end_index) as usize;
    // 18. Let k be startIndex.
    let k = start_index as usize;
    // 19. Repeat, while k < endIndex,
    let value = if cfg!(target_endian = "little") {
        T::from_le_value(agent, value)
    } else {
        T::from_be_value(agent, value)
    };
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(ta);
    }
    if byte_offset > byte_slice.len() {
        // We shouldn't be out of bounds.
        unreachable!();
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            // We shouldn't be out of bounds.
            unreachable!()
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc,
        ));
    }
    if k >= end_index {
        return Ok(ta);
    }
    let slice = &mut slice[k..end_index];
    slice.fill(value);
    // 20. Return O.
    Ok(ta)
}

fn sort_partial_cmp_typed_array<'a, T: Viewable + std::fmt::Debug + PartialOrd>(
    agent: &mut Agent,
    ta: TypedArray,
    len: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc,
        ));
    }
    let slice = &mut slice[..len];
    slice.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Ok(())
}

fn sort_total_cmp_typed_array<'a, T: Viewable + std::fmt::Debug + PartialOrd + 'static>(
    agent: &mut Agent,
    ta: TypedArray,
    len: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc,
        ));
    }
    let slice = &mut slice[..len];
    let mut items: Vec<T> = slice.to_vec();
    items.sort_by(|a: &T, b: &T| {
        let left = a.into_le_value(agent, gc).into_value();
        let right = b.into_le_value(agent, gc).into_value();
        if left.is_nan(agent) && right.is_nan(agent) {
            std::cmp::Ordering::Equal
        } else if left.is_nan(agent) || left.is_pos_zero(agent) && right.is_neg_zero(agent) {
            std::cmp::Ordering::Greater
        } else if right.is_nan(agent) || left.is_neg_zero(agent) && right.is_pos_zero(agent) {
            std::cmp::Ordering::Less
        } else {
            a.partial_cmp(b).unwrap()
        }
    });
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (_, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    let len = len.min(slice.len());
    let slice = &mut slice[..len];
    let copy_len = items.len().min(len);
    slice[..copy_len].copy_from_slice(&items[..copy_len]);
    Ok(())
}

fn sort_comparator_typed_array<'a, T: Viewable + Copy + std::fmt::Debug>(
    agent: &mut Agent,
    ta: TypedArray,
    len: usize,
    comparator: Scoped<'_, Function<'static>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let array_buffer = ta.get_viewed_array_buffer(agent, gc.nogc());
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() || len == 0 {
        return Ok(());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc.into_nogc(),
        ));
    }
    let slice = &mut slice[..len];
    let mut items: Vec<T> = slice.to_vec();
    let mut error: Option<JsError> = None;
    items.sort_by(|a, b| {
        if error.is_some() {
            return std::cmp::Ordering::Equal;
        }
        let a_val = a.into_le_value(agent, gc.nogc()).into_value();
        let b_val = b.into_le_value(agent, gc.nogc()).into_value();
        let result = call_function(
            agent,
            comparator.get(agent),
            Value::Undefined,
            Some(ArgumentsList::from_mut_slice(&mut [
                a_val.unbind(),
                b_val.unbind(),
            ])),
            gc.reborrow(),
        )
        .unbind()
        .map(|v| v.to_number(agent, gc.reborrow()))
        .and_then(|r| r);
        let num = match result {
            Ok(n) => n,
            Err(e) => {
                error = Some(e.unbind());
                return std::cmp::Ordering::Equal;
            }
        };
        if num.is_nan(agent) {
            std::cmp::Ordering::Equal
        } else if num.is_sign_positive(agent) {
            std::cmp::Ordering::Greater
        } else if num.is_sign_negative(agent) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    });
    if let Some(error) = error {
        return Err(error);
    }
    let array_buffer = ta.get_viewed_array_buffer(agent, gc.nogc());
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    let (_, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    let len = len.min(slice.len());
    let slice = &mut slice[..len];
    let copy_len = items.len().min(len);
    slice[..copy_len].copy_from_slice(&items[..copy_len]);
    Ok(())
}
