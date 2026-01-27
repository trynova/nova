// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hint::unreachable_unchecked;

use ecmascript_atomics::Ordering;
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
    SmallInteger,
        Agent, AnyTypedArray, ArgumentsList, ArrayIterator, BUILTIN_STRING_MEMORY, Behaviour,
        Builtin, BuiltinFunctionBuilder, BuiltinGetter, BuiltinIntrinsic,
        BuiltinIntrinsicConstructor, CollectionIteratorKind, ExceptionType, InternalMethods,
        JsResult, Number, Numeric, Object, OrdinaryObjectBuilder, Primitive, PropertyKey, Realm,
        String, TryGetResult, TypedArrayAbstractOperations, Value, call_function,
        find_via_predicate, for_any_typed_array, get, get_iterator_from_method, get_method, invoke,
        is_callable, is_constructor, iterator_to_list, length_of_array_like, same_value_zero, set,
        throw_not_callable, to_big_int, to_big_int_primitive, to_boolean, to_integer_or_infinity,
        to_number, to_number_primitive, to_object, to_string, try_get, try_length_of_array_like,
        try_result_into_js, try_to_integer_or_infinity, try_to_string,
        try_typed_array_species_create_with_length, typed_array_create_from_data_block, unwrap_try,
        unwrap_try_get_value, unwrap_try_get_value_or_unset,
    },
    engine::{
        Bindable, GcScope, NoGcScope,
        Scopable,
    },
    heap::{IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

use super::abstract_operations::{
    make_typed_array_with_buffer_witness_record, typed_array_create_from_constructor_with_length,
    typed_array_species_create_with_buffer, typed_array_species_create_with_length,
    validate_typed_array,
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
            crate::ecmascript::ExceptionType::TypeError,
            "Abstract class TypedArray not directly constructable",
            gc.into_nogc(),
        ))
    }

    /// ### [23.2.2.1 %TypedArray%.from ( source \[ , mapper \[ , thisArg \] \] )](https://tc39.es/ecma262/#sec-%typedarray%.from)
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
            .bind(gc.nogc())
            .into_iterator_record() else {
                return Err(throw_not_callable(agent, gc.into_nogc()));
            };
            let values = iterator_to_list(agent, iterator_record.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. Let len be the number of elements in values.
            let len = i64::try_from(values.len(agent)).unwrap();
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
            for (k, k_value) in values.iter(agent).enumerate() {
                // 𝔽(k)
                //  i. Let Pk be ! ToString(𝔽(k)).
                // ii. Let kValue be the first element of values.
                // iii. Remove the first element from values.
                let sk = SmallInteger::from(k as u32);
                let fk = Number::from(sk).into();
                let pk = PropertyKey::from(sk);
                //  iv. If mapping is true, then
                let mapped_value = if let Some(mapper) = &mapping {
                    //  1. Let mappedValue be ? Call(mapper, thisArg, « kValue, 𝔽(k) »).
                    call_function(
                        agent,
                        mapper.get(agent),
                        scoped_this_arg.get(agent),
                        Some(ArgumentsList::from_mut_slice(&mut [
                            k_value.get(gc.nogc()).unbind(),
                            fk,
                        ])),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc())
                } else {
                    // v. Else,
                    //      1. Let mappedValue be kValue.
                    k_value.get(gc.nogc())
                };
                // vi. Perform ? Set(targetObj, Pk, mappedValue, true).
                set(
                    agent,
                    scoped_target_obj.get(agent).into(),
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
            return Ok(target_obj.into());
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
            let fk = Number::from(sk).into();
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
                scoped_target_obj.get(agent).into(),
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
        Ok(target_obj.into())
    }

    /// ### [23.2.2.2 %TypedArray%.of ( ...items )](https://tc39.es/ecma262/#sec-properties-of-the-%typedarray%-intrinsic-object)
    fn of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let arguments = arguments.bind(gc.nogc());

        // 1. Let len be the number of elements in items.
        let len = i64::try_from(arguments.len()).unwrap();

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
        let c = c.unbind();
        arguments.unbind().with_scoped(
            agent,
            |agent, arguments, mut gc| {
                let c = c.bind(gc.nogc());
                let new_obj = typed_array_create_from_constructor_with_length(
                    agent,
                    c.unbind(),
                    len,
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // 5. Let k be 0.
                // 6. Repeat, while k < len,
                let scoped_new_obj = new_obj.scope(agent, gc.nogc());
                for k in 0..len {
                    // a. Let kValue be items[k].
                    let k_value = arguments.get(agent, k as u32, gc.nogc());
                    // b. Let Pk be ! ToString(𝔽(k)).
                    let pk = k.try_into().unwrap();
                    // c. Perform ? Set(newObj, Pk, kValue, true).
                    set(
                        agent,
                        scoped_new_obj.get(agent).into(),
                        pk,
                        k_value.unbind(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // d. Set k to k + 1.
                }
                // 7. Return newObj.
                Ok(scoped_new_obj.get(agent).into())
            },
            gc,
        )
    }

    /// ### [23.2.2.4 get %TypedArray% \[ %Symbol.species% \]](https://tc39.es/ecma262/#sec-get-%typedarray%-%symbol.species%)
    ///
    /// > NOTE: %TypedArray.prototype% methods normally use their **this**
    /// > value's constructor to create a derived object. However, a subclass
    /// > constructor may over-ride that default behaviour by redefining its
    /// > %Symbol.species% property.
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
        .with_prototype_property(typed_array_prototype.into())
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
    /// ### [23.2.3.1 %TypedArray%.prototype.at ( index )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.at)
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
        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);
        let mut o = ta_record.object;
        // 4. Let relativeIndex be ? ToIntegerOrInfinity(index).
        let relative_index = if let Value::Integer(index) = index {
            index.into_i64()
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let result = to_integer_or_infinity(agent, index.unbind(), gc.reborrow())
                .unbind()?
                .into_i64();
            // SAFETY: not shared.
            o = unsafe { scoped_o.take(agent).bind(gc.nogc()) };
            result
        };
        // 5. If relativeIndex ≥ 0, then
        // a. Let k be relativeIndex.
        // 6. Else,
        // a. Let k be len + relativeIndex.
        // 7. If k < 0 or k ≥ len, return undefined.
        let k = calculate_offset_index(relative_index, len);
        let Ok(k) = usize::try_from(k) else {
            return Ok(Value::Undefined);
        };
        if k >= len {
            return Ok(Value::Undefined);
        }
        // 8. Return ! Get(O, ! ToString(𝔽(k))).
        Ok(unwrap_try_get_value_or_unset(try_get(
            agent,
            o.unbind(),
            PropertyKey::Integer(k.try_into().unwrap()),
            None,
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
        Ok(o.viewed_array_buffer(agent).into())
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

        let size = for_any_typed_array!(o, o, {
            // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
            let cached_buffer_byte_length =
                o.get_cached_buffer_byte_length(agent, Ordering::SeqCst);

            // 5. Let size be TypedArrayByteLength(taRecord).
            o.typed_array_byte_length(agent, cached_buffer_byte_length)
        });
        // 6. Return 𝔽(size).
        Ok(Value::from_i64(agent, size as i64, gc))
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

        let offset = for_any_typed_array!(o, o, {
            // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
            let cached_buffer_byte_length =
                o.get_cached_buffer_byte_length(agent, Ordering::SeqCst);

            // 5. If IsTypedArrayOutOfBounds(taRecord) is true, return +0𝔽.
            if o.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length) {
                return Ok(Value::pos_zero());
            }

            // 6. Let offset be O.[[ByteOffset]].
            o.byte_offset(agent)
        });
        // 7. Return 𝔽(offset).
        Ok(Value::from_i64(agent, offset as i64, gc))
    }

    /// ### [23.2.3.6 %TypedArray%.prototype.copyWithin ( target, start \[ , end \] )](https://tc39.es/ecma262/#sec-typedarray-objects)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.copyWithin** as defined in 23.1.3.4.
    fn copy_within<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let target = arguments.get(0).bind(gc.nogc());
        let start = arguments.get(1).bind(gc.nogc());
        let end = arguments.get(2).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value.bind(gc.nogc());
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);
        let (o, new_len, target_index, start_index, end_index) = if let (
            Value::Integer(relative_target),
            Value::Integer(relative_start),
            relative_end @ Value::Integer(_) | relative_end @ Value::Undefined,
        ) = (target, start, end)
        {
            let target_index = calculate_relative_index(relative_target.into_i64(), len);
            let start_index = calculate_relative_index(relative_start.into_i64(), len);
            let end_index = if let Value::Integer(relative_end) = relative_end {
                calculate_relative_index(relative_end.into_i64(), len)
            } else {
                len
            };
            (
                ta_record.object.unbind().bind(gc.into_nogc()),
                len,
                target_index,
                start_index,
                end_index,
            )
        } else {
            Self::copy_within_slow_path(
                agent,
                ta_record.object.unbind(),
                len,
                target.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            )?
        };
        // 16. Let count be min(endIndex - startIndex, len - targetIndex).
        let count = end_index
            .saturating_sub(start_index)
            .min(len.saturating_sub(target_index));
        // 17. If count > 0, then
        if count > 0 {
            // g. Set count to min(count, len - startIndex, len - targetIndex).
            let count = count.min(new_len - start_index).min(new_len - target_index);
            o.copy_within(agent, start_index, target_index, count)
        }
        Ok(o.into())
    }

    #[cold]
    fn copy_within_slow_path<'gc>(
        agent: &mut Agent,
        o: AnyTypedArray,
        len: usize,
        target: Value,
        start: Value,
        end: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, (AnyTypedArray<'gc>, usize, usize, usize, usize)> {
        let o = o.scope(agent, gc.nogc());
        let end = end.scope(agent, gc.nogc());
        let start = start.scope(agent, gc.nogc());
        let target = target.bind(gc.nogc());
        // 4. Let relativeTarget be ? ToIntegerOrInfinity(target).
        // 5. If relativeTarget = -∞, let targetIndex be 0.
        // 6. Else if relativeTarget < 0, let targetIndex be max(len + relativeTarget, 0).
        // 7. Else, let targetIndex be min(relativeTarget, len).
        let target_index =
            calculate_relative_index_value(agent, target.unbind(), len, gc.reborrow()).unbind()?;

        // SAFETY: not shared.
        let start = unsafe { start.take(agent) }.bind(gc.nogc());

        // 8. Let relativeStart be ? ToIntegerOrInfinity(start).
        // 9. If relativeStart = -∞, let startIndex be 0
        // 10. Else if relativeStart < 0, let startIndex be max(len + relativeStart, 0).
        // 11. Else, let startIndex be min(relativeStart, len).
        let start_index =
            calculate_relative_index_value(agent, start.unbind(), len, gc.reborrow()).unbind()?;

        // SAFETY: not shared.
        let end = unsafe { end.take(agent) }.bind(gc.nogc());

        // 12. If end is undefined, let relativeEnd be len; else let
        //     relativeEnd be ? ToIntegerOrInfinity(end).
        // 13. If relativeEnd = -∞, let endIndex be 0.
        // 14. Else if relativeEnd < 0, let endIndex be
        //     max(len + relativeEnd, 0).
        // 15. Else, let endIndex be min(relativeEnd, len).
        let end_index =
            calculate_relative_end_index_value(agent, end.unbind(), len, gc.reborrow()).unbind()?;

        let gc = gc.into_nogc();

        // SAFETY: not shared.
        let o = unsafe { o.take(agent) }.bind(gc);

        let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst);
        if ta_record.is_typed_array_out_of_bounds(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc,
            ));
        }
        let len = ta_record.typed_array_length(agent);
        Ok((ta_record.object, len, target_index, start_index, end_index))
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
        Ok(ArrayIterator::from_object(agent, o.into(), CollectionIteratorKind::KeyAndValue).into())
    }

    /// ### [23.2.3.8 %%TypedArray%.prototype.every ( callback \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.every)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.every** as defined in 23.1.3.6.
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
        let len = ta_record.typed_array_length(agent);
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
            let k_value = unwrap_try_get_value_or_unset(try_get(agent, o, pk, None, gc.nogc()));
            // c. Let testResult be ToBoolean(? Call(callback, thisArg, « kValue, 𝔽(k), O »)).
            let call = call_function(
                agent,
                callback.get(agent),
                this_arg.get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    k_value.unbind(),
                    Number::try_from(k).unwrap().into(),
                    o.unbind().into(),
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

    /// ### [23.2.3.9 %TypedArray%.prototype.fill ( value \[ , start \[ , end \] \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.fill)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.fill** as defined in 23.1.3.7.
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
        let len = ta_record.typed_array_length(agent);
        let (o, value, start_index, count) = if let (
            Ok(value),
            Value::Integer(relative_start),
            relative_end @ Value::Integer(_) | relative_end @ Value::Undefined,
        ) = (Primitive::try_from(value), start, end)
        {
            let value: Numeric = if ta_record.object.is_bigint() {
                to_big_int_primitive(agent, value, gc.nogc())
                    .unbind()?
                    .bind(gc.nogc())
                    .into()
            } else {
                to_number_primitive(agent, value, gc.nogc())
                    .unbind()?
                    .bind(gc.nogc())
                    .into()
            };
            let start_index = calculate_relative_index(relative_start.into_i64(), len);
            let end_index = if let Value::Integer(relative_end) = relative_end {
                calculate_relative_index(relative_end.into_i64(), len)
            } else {
                len
            };
            let o = ta_record.object.unbind();
            let value = value.unbind();
            let gc = gc.into_nogc();
            (
                o.bind(gc),
                value.bind(gc),
                start_index,
                end_index.saturating_sub(start_index),
            )
        } else {
            Self::fill_slow_path(
                agent,
                ta_record.object.unbind(),
                len,
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            )?
        };

        // 18. Let k be startIndex.
        // 19. Repeat, while k < endIndex.
        if count > 0 {
            // a. Let Pk be ! ToString(F(k)).
            // b. Perform ! Set(O, Pk, value, true).
            // c. Set k to k + 1.
            o.fill(agent, value, start_index, count);
        }

        // 20. Return O.
        Ok(o.into())
    }

    #[cold]
    fn fill_slow_path<'gc>(
        agent: &mut Agent,
        o: AnyTypedArray,
        len: usize,
        value: Value,
        start: Value,
        end: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, (AnyTypedArray<'gc>, Numeric<'gc>, usize, usize)> {
        let is_bigint = o.is_bigint();
        let o = o.scope(agent, gc.nogc());
        let start = start.scope(agent, gc.nogc());
        let end = end.scope(agent, gc.nogc());
        let value = value.bind(gc.nogc());
        // 4. If O.[[ContentType]] is bigint,
        let value: Numeric = if is_bigint {
            // set value to ? ToBigInt(value).
            to_big_int(agent, value.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
                .into()
        } else {
            // 5. Otherwise, set value to ? ToNumber(value).
            to_number(agent, value.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
                .into()
        };

        let start_temp = start.get(agent).bind(gc.nogc());
        // SAFETY: not shared.
        let value = unsafe { start.replace_self(agent, value.unbind()) };
        let start = start_temp;

        // 6. Let relativeStart be ? ToIntegerOrInfinity(start).
        // 7. If relativeStart = -∞, let startIndex be 0.
        // 8. Else if relativeStart < 0, let startIndex be
        //    max(len + relativeStart, 0).
        // 9. Else, let startIndex be min(relativeStart, len).
        let start_index =
            calculate_relative_index_value(agent, start.unbind(), len, gc.reborrow()).unbind()?;

        // SAFETY: not shared.
        let end = unsafe { end.take(agent) }.bind(gc.nogc());

        // 10. If end is undefined, let relativeEnd be len; else let
        //     relativeEnd be ? ToIntegerOrInfinity(end).
        // 11. If relativeEnd = -∞, let endIndex be 0.
        // 12. Else if relativeEnd < 0, let endIndex be
        //     max(len + relativeEnd, 0).
        // 13. Else, let endIndex be min(relativeEnd, len).
        let end_index =
            calculate_relative_end_index_value(agent, end.unbind(), len, gc.reborrow()).unbind()?;

        let gc = gc.into_nogc();

        // SAFETY: not shared.
        let value = unsafe { value.take(agent) }.bind(gc);
        // SAFETY: not shared.
        let o = unsafe { o.take(agent) }.bind(gc);

        // 14. Set taRecord to MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst);
        // 15. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
        if ta_record.is_typed_array_out_of_bounds(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc,
            ));
        }
        // 16. Set len to TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);
        // 17. Set endIndex to min(endIndex, len).
        let end_index = end_index.min(len);
        Ok((
            ta_record.object,
            value,
            start_index,
            end_index.saturating_sub(start_index),
        ))
    }

    /// ### [23.2.3.10 %TypedArray%.prototype.filter ( callback \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.filter)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.filter** as defined in 23.1.3.8.
    fn filter<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be the this value.
        let this_value = this_value.bind(gc.nogc());
        let callback = arguments.get(0).bind(gc.nogc());
        let this_arg = arguments.get(1).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback) = is_callable(callback, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };
        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);
        o.unbind()
            .filter(agent, callback.unbind(), this_arg.unbind(), len, gc)
    }

    /// ### [23.2.3.11 %TypedArray%.prototype.find ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.find)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.find** as defined in 23.1.3.9.
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
        let len = ta_record.typed_array_length(agent);
        let o = o.scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len as u64, true, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Value]].
        Ok(find_rec.1)
    }

    /// ### [23.2.3.12 %TypedArray%.prototype.findIndex( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.findindex)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.findIndex** as defined in 23.1.3.10.
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
        let len = ta_record.typed_array_length(agent) as u64;
        let o = o.scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, true, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into())
    }

    /// ### [23.2.3.13 %TypedArray%.prototype.findLast ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.findlast)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.findLastIndex** as defined in 23.1.3.12.
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
        let len = ta_record.typed_array_length(agent) as u64;
        let o = o.scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, ascending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Value]].
        Ok(find_rec.1)
    }

    /// ### [23.2.3.14 %TypedArray%.prototype.findLastIndex ( predicate \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.findlastindex)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.findLastIndex** as defined in 23.1.3.12.
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
        let len = ta_record.typed_array_length(agent) as u64;
        let o = o.scope(agent, gc.nogc());
        // 4. Let findRec be ? FindViaPredicate(O, len, descending, predicate, thisArg).
        let find_rec = find_via_predicate(agent, o, len, false, predicate, this_arg, gc)?;
        // 5. Return findRec.[[Index]].
        Ok(Number::try_from(find_rec.0).unwrap().into())
    }

    /// ### [23.2.3.15 %TypedArray%.prototype.forEach ( callback \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.foreach)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.forEach** as defined in 23.1.3.15.
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
        let len = ta_record.typed_array_length(agent) as u64;
        let mut o = ta_record.object;
        let scoped_o = o.scope(agent, nogc);
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
            let k_value = unwrap_try_get_value_or_unset(try_get(agent, o, pk, None, gc.nogc()));
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
                    o.unbind().into(),
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

    /// ### [23.2.3.16 %TypedArray%.prototype.includes ( searchElement \[ , fromIndex \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.includes)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.includes** as defined in 23.1.3.16.
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
        let len = ta_record.typed_array_length(agent);
        // 4. If len = 0, return false.
        if len == 0 {
            return Ok(false.into());
        };
        let mut o = ta_record.object;
        // 5. Let n be ? ToIntegerOrInfinity(fromIndex).
        let from_index_is_undefined = from_index.is_undefined();
        let n = if let Some(n) =
            try_result_into_js(try_to_integer_or_infinity(agent, from_index, nogc)).unbind()?
        {
            n
        } else {
            let scoped_o = o.scope(agent, nogc);
            let scoped_search_element = search_element.scope(agent, nogc);
            let result =
                to_integer_or_infinity(agent, from_index.unbind(), gc.reborrow()).unbind()?;
            let gc = gc.nogc();
            // SAFETY: not shared.
            unsafe {
                search_element = scoped_search_element.take(agent).bind(gc);
                o = scoped_o.take(agent).bind(gc);
            }
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
        // a. Let k be n.
        // 10. Else,
        // a. Let k be len + n.
        // b. If k < 0, set k to 0.
        let k = calculate_relative_index(n, len);
        // 11. Repeat, while k < len,
        for k in k..len {
            // a. Let elementK be ! Get(O, ! ToString(𝔽(k))).
            let element_k = unwrap_try_get_value_or_unset(try_get(
                agent,
                o,
                PropertyKey::Integer(k.try_into().unwrap()),
                None,
                gc,
            ));
            // b. If SameValueZero(searchElement, elementK) is true, return true.
            if same_value_zero(agent, search_element, element_k) {
                return Ok(true.into());
            }
            // c. Set k to k + 1.
        }
        // 12. Return false.
        Ok(false.into())
    }

    /// ### [23.2.3.17 %TypedArray%.prototype.indexOf ( searchElement \[ , fromIndex \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.indexof)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.indexOf** as defined in 23.1.3.17.
    fn index_of<'gc>(
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
        let len = ta_record.typed_array_length(agent);

        let mut o = ta_record.object;
        // 4. If len = 0, return -1𝔽.
        if len == 0 {
            return Ok((-1).into());
        };
        // 5. Let n be ? ToIntegerOrInfinity(fromIndex).
        let from_index_is_undefined = from_index.is_undefined();
        let n = if let Some(n) =
            try_result_into_js(try_to_integer_or_infinity(agent, from_index, nogc)).unbind()?
        {
            n
        } else {
            let scoped_o = o.scope(agent, nogc);
            let scoped_search_element = search_element.scope(agent, nogc);
            let result =
                to_integer_or_infinity(agent, from_index.unbind(), gc.reborrow()).unbind()?;
            let gc = gc.nogc();
            // SAFETY: not shared.
            unsafe {
                search_element = scoped_search_element.take(agent).bind(gc);
                o = scoped_o.take(agent).bind(gc);
            }
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
        // a. Let k be n.
        // 10. Else,
        // a. Let k be len + n.
        // b. If k < 0, set k to 0.
        let k = calculate_relative_index(n, len);

        // 11. Repeat, while k < len,
        let result = o.search::<true>(agent, search_element, k, len);

        Ok(result.map_or(-1, |v| v as i64).try_into().unwrap())
    }

    /// ### [23.2.3.18 %TypedArray%.prototype.join ( separator )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.join)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.join** as defined in 23.1.3.18.
    ///
    /// This method is not generic. The **this** value must be an object with a
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
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);
        let mut o = ta_record.object;

        // 4. If separator is undefined, let sep be ",".
        let (sep_string, after_len) = if separator.is_undefined() {
            (String::from_small_string(","), len)
        } else if let Ok(sep) = String::try_from(separator) {
            (sep, len)
        } else {
            // 5. Else, let sep be ? ToString(separator).
            let scoped_o = o.scope(agent, nogc);
            let sep_string = to_string(agent, separator.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let nogc = gc.nogc();
            o = scoped_o.get(agent).bind(nogc);
            let (is_out_of_bounds, len) = for_any_typed_array!(o, o, {
                let cached_buffer_byte_length =
                    o.get_cached_buffer_byte_length(agent, Ordering::Unordered);
                let is_out_of_bounds =
                    o.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length);
                (
                    is_out_of_bounds,
                    if is_out_of_bounds {
                        len
                    } else {
                        o.typed_array_length(agent, cached_buffer_byte_length)
                    },
                )
            });
            if is_out_of_bounds {
                // If TypedArray is out of bounds then every Get(O, k) returns
                // undefined. The result is a string comprising only of
                // separators.
                let sep = sep_string.as_wtf8_(agent).to_owned();
                let count = len.saturating_sub(1);
                let byte_count = count * sep.len();
                let mut buf = Wtf8Buf::with_capacity(byte_count);
                for _ in 0..count {
                    buf.push_wtf8(sep);
                }
                return Ok(String::from_wtf8_buf(agent, buf, gc.into_nogc()).into());
            }
            (sep_string, len)
        };
        let o = o.unbind();
        let sep_string = sep_string.unbind();
        let gc = gc.into_nogc();
        let o = o.bind(gc);
        let sep_string = sep_string.bind(gc);
        if len == 0 {
            return Ok(String::EMPTY_STRING.into());
        }

        let sep = sep_string.as_wtf8_(agent).to_owned();
        // 6. Let R be the empty String.
        let mut r = Wtf8Buf::with_capacity(after_len * 3);
        // 7. Let k be 0.
        // 8. Repeat, while k < len,
        for k in 0..len {
            // a. If k > 0, set R to the string-concatenation of R and sep.
            if k > 0 {
                r.push_wtf8(sep);
            }
            // c. If element is not undefined, then
            if k >= after_len {
                // Note: element is undefined if the ViewedArrayBuffer was
                // detached by ToString call, or was shrunk to less than len.
                continue;
            }
            // b. Let element be ! Get(O, ! ToString(𝔽(k))).
            let element = match unwrap_try(try_get(agent, o, k.try_into().unwrap(), None, gc)) {
                TryGetResult::Value(e) => e,
                _ => unreachable!(),
            };
            // i. Let S be ! ToString(element).
            let s = unwrap_try(try_to_string(agent, element, gc));
            // ii. Set R to the string-concatenation of R and S.
            r.push_wtf8(s.as_wtf8_(agent));
            // d. Set k to k + 1.
        }
        // 9. Return R.
        Ok(String::from_wtf8_buf(agent, r, gc).unbind().into())
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
        Ok(ArrayIterator::from_object(agent, o.into(), CollectionIteratorKind::Key).into())
    }

    /// ### [23.2.3.20 %TypedArray%.prototype.lastIndexOf ( searchElement \[ , fromIndex \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.lastindexof)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.lastIndexOf** as defined in 23.1.3.20.
    fn last_index_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let mut search_element = arguments.get(0).bind(gc.nogc());
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
        let len = ta_record.typed_array_length(agent);

        let mut o = ta_record.object;
        // 4. If len = 0,
        if len == 0 {
            // return -1𝔽.
            return Ok((-1).into());
        };
        // 5. If fromIndex is present,
        let k = if let Some(from_index) = from_index {
            // let n be ? ToIntegerOrInfinity(fromIndex);
            let nogc = gc.nogc();
            let n = if let Some(n) =
                try_result_into_js(try_to_integer_or_infinity(agent, from_index, nogc)).unbind()?
            {
                n
            } else {
                let scoped_o = o.scope(agent, nogc);
                let scoped_search_element = search_element.scope(agent, nogc);
                let result =
                    to_integer_or_infinity(agent, from_index.unbind(), gc.reborrow()).unbind()?;
                let gc = gc.nogc();
                // SAFETY: not shared.
                unsafe {
                    search_element = scoped_search_element.take(agent).bind(gc);
                    o = scoped_o.take(agent).bind(gc);
                }
                result
            };
            // 6. If n = -∞,
            if n.is_neg_infinity() {
                // return -1𝔽.
                return Ok((-1).into());
            }
            let n = n.into_i64();
            // 7. If n ≥ 0, then
            let len = u64::try_from(len).unwrap();

            if n >= 0 {
                let n = n.unsigned_abs();
                // a. Let k be min(n, len - 1).
                usize::try_from(n.min(len - 1)).unwrap()
            } else {
                // 8. Else,
                // a. Let k be len + n.
                let Some(k) = len.checked_add_signed(n) else {
                    // Underflow; while k >= 0 is never performed and we
                    // immediately enter step 10. Return -1F.
                    return Ok((-1).into());
                };
                usize::try_from(k).unwrap()
            }
        } else {
            // 5. ... else let n be len - 1.
            len - 1
        };

        // 9. Repeat, while k ≥ 0,
        let result = o.search::<false>(agent, search_element, k, len);

        Ok(result.map_or(-1, |v| v as i64).try_into().unwrap())
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
        let o = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        let o = require_internal_slot_typed_array(agent, o, gc)?;

        let length = for_any_typed_array!(o, o, {
            // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
            let cached_buffer_byte_length =
                o.get_cached_buffer_byte_length(agent, Ordering::SeqCst);

            // 5. If IsTypedArrayOutOfBounds(taRecord) is true, return +0𝔽.
            if o.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length) {
                return Ok(Value::pos_zero());
            }

            // 6. Let length be TypedArrayLength(taRecord).
            o.typed_array_length(agent, cached_buffer_byte_length)
        });
        // 7. Return 𝔽(length).
        Ok(Value::try_from(length as i64).unwrap())
    }

    /// ### [23.2.3.22 %TypedArray%.prototype.map ( callback \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.map)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.map** as defined in 23.1.3.21.
    fn map<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be the this value.
        let o = this_value.bind(gc.nogc());
        let callback_fn = arguments.get(0).bind(gc.nogc());
        let this_arg = arguments.get(1).bind(gc.nogc());
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());

        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);

        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };

        ta_record
            .object
            .unbind()
            .map(agent, callback_fn.unbind(), this_arg.unbind(), len, gc)
    }

    /// ### [23.2.3.23 %TypedArray%.prototype.reduce ( callback \[ , initialValue \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.reduce)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.reduce** as defined in 23.1.3.24.
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
        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);
        let o = ta_record.object;

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
            let result = unwrap_try_get_value(try_get(agent, o, pk, None, gc.nogc()));
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
            let k_value = unwrap_try_get_value_or_unset(try_get(
                agent,
                scoped_o.get(agent),
                pk,
                None,
                gc.nogc(),
            ));
            // c. Set accumulator to ? Call(callback, undefined, « accumulator, kValue, 𝔽(k), O »).
            let result = call_function(
                agent,
                scoped_callback.get(agent),
                Value::Undefined,
                Some(ArgumentsList::from_mut_slice(&mut [
                    accumulator.get(agent),
                    k_value.unbind(),
                    Number::from(k_int).into(),
                    scoped_o.get(agent).into(),
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

    /// ### [23.2.3.24 %TypedArray%.prototype.reduceRight ( callback \[ , initialValue \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.reduceright)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.reduceRight** as defined in 23.1.3.25.
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
        let len = ta_record.typed_array_length(agent);

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
        let mut k = len.checked_sub(1);
        // 7. Let accumulator be undefined.
        // 8. If initialValue is present, then
        //    a. Set accumulator to initialValue.
        let mut accumulator = if let Some(init) = initial_value {
            init.scope(agent, gc.nogc())
        } else if let Some(l) = k {
            // 9. Else,
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::try_from(l).unwrap();
            // b. Set accumulator to ! Get(O, Pk).
            let result = unwrap_try_get_value_or_unset(try_get(agent, o, pk, None, gc.nogc()));
            // c. Set k to k - 1.
            k = l.checked_sub(1);
            result.scope(agent, gc.nogc())
        } else {
            return Ok(Value::Undefined);
        };
        let scoped_callback = callback.scope(agent, gc.nogc());
        let scoped_o = o.scope(agent, gc.nogc());
        // 10. Repeat, while k >= 0,
        while let Some(l) = k {
            let k_int = l.try_into().unwrap();
            // a. Let Pk be ! ToString(𝔽(k)).
            let pk = PropertyKey::Integer(k_int);
            // b. Let kValue be ! Get(O, Pk).
            let k_value = unwrap_try_get_value_or_unset(try_get(
                agent,
                scoped_o.get(agent),
                pk,
                None,
                gc.nogc(),
            ));
            // c. Set accumulator to ? Call(callback, undefined, « accumulator, kValue, 𝔽(k), O »).
            let result = call_function(
                agent,
                scoped_callback.get(agent),
                Value::Undefined,
                Some(ArgumentsList::from_mut_slice(&mut [
                    accumulator.get(agent),
                    k_value.unbind(),
                    Number::from(k_int).into(),
                    scoped_o.get(agent).into(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // SAFETY: accumulator is not shared.
            unsafe { accumulator.replace(agent, result.unbind()) };
            // d. Set k to k - 1.
            k = l.checked_sub(1);
        }
        // 11. Return accumulator.
        Ok(accumulator.get(agent))
    }

    /// ### [23.2.3.25 %TypedArray%.prototype.reverse ( )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.reverse)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.reverse** as defined in 23.1.3.26.
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
        let len = ta_record.typed_array_length(agent);
        let o = ta_record.object;
        o.reverse(agent, len);
        // 7. Return O.
        Ok(o.into())
    }

    /// ### [23.2.3.26 %TypedArray%.prototype.set ( source \[ , offset \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.set)
    ///
    /// This method sets multiple values in this TypedArray, reading the values
    /// from source. The details differ based upon the type of source. The
    /// optional offset value indicates the first element index in this
    /// TypedArray where values are written. If omitted, it is assumed to be 0.
    fn set<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let mut source = arguments.get(0).bind(nogc);
        let offset = arguments.get(1).bind(nogc);
        // 1. Let target be the this value.
        let target = this_value.bind(nogc);
        // 2. Perform ? RequireInternalSlot(target, [[TypedArrayName]]).
        // 3. Assert: target has a [[ViewedArrayBuffer]] internal slot.
        let mut target = require_internal_slot_typed_array(agent, target, nogc)
            .unbind()?
            .bind(nogc);
        // 4. Let targetOffset be ? ToIntegerOrInfinity(offset).
        let target_offset = if let Some(target_offset) =
            try_result_into_js(try_to_integer_or_infinity(agent, offset, nogc)).unbind()?
        {
            target_offset
        } else {
            let scoped_target = target.scope(agent, nogc);
            let scoped_source = source.scope(agent, nogc);
            let target_offset =
                to_integer_or_infinity(agent, offset.unbind(), gc.reborrow()).unbind()?;
            let gc = gc.nogc();
            // SAFETY: not shared.
            unsafe {
                source = scoped_source.take(agent).bind(gc);
                target = scoped_target.take(agent).bind(gc);
            }
            target_offset
        };
        // 5. If targetOffset < 0, throw a RangeError exception.
        if target_offset.is_negative() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "invalid array length",
                gc.into_nogc(),
            ));
        }

        // Note: first three meaningful steps from SetTypedArrayFromXXX are the
        // same in both paths.

        // 2. or 1. Let targetRecord be MakeTypedArrayWithBufferWitnessRecord(target, seq-cst).
        let cached_buffer_byte_length =
            target.get_cached_buffer_byte_length(agent, Ordering::SeqCst);
        // 3. or 2. If IsTypedArrayOutOfBounds(targetRecord) is true, throw a TypeError exception.
        if target.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "TypedArray out of bounds",
                gc.into_nogc(),
            ));
        };
        // 4. or 3. Let targetLength be TypedArrayLength(targetRecord).
        let target_length = target.typed_array_length(agent, cached_buffer_byte_length);

        let source_is_typed_array = AnyTypedArray::try_from(source).is_ok();

        // 6. If source is an Object that has a [[TypedArrayName]] internal slot, then
        let (src, src_length) = if source_is_typed_array {
            // a. Perform ? SetTypedArrayFromTypedArray(target, targetOffset, source).

            let Ok(source) = AnyTypedArray::try_from(source) else {
                // SAFETY: checked above.
                unsafe { unreachable_unchecked() }
            };

            // 6. Let srcRecord be MakeTypedArrayWithBufferWitnessRecord(source, seq-cst).
            let src_record =
                make_typed_array_with_buffer_witness_record(agent, source, Ordering::SeqCst);
            // 7. If IsTypedArrayOutOfBounds(srcRecord) is true, throw a TypeError exception.
            if src_record.is_typed_array_out_of_bounds(agent) {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "TypedArray out of bounds",
                    gc.into_nogc(),
                ));
            }
            // 8. Let srcLength be TypedArrayLength(srcRecord).
            (source.into(), Some(src_record.typed_array_length(agent)))
        } else {
            // 7. Else,
            // a. Perform ? SetTypedArrayFromArrayLike(target, targetOffset, source).

            // 4. Let src be ? ToObject(source).
            let mut src = to_object(agent, source, gc.nogc())
                .unbind()?
                .bind(gc.nogc());
            let len = if let Some(len) =
                try_result_into_js(try_length_of_array_like(agent, src, gc.nogc())).unbind()?
            {
                len as u64
            } else {
                let scoped_target = target.scope(agent, gc.nogc());
                let scoped_src = src.scope(agent, gc.nogc());
                let len = length_of_array_like(agent, src.unbind(), gc.reborrow()).unbind()? as u64;
                // SAFETY: not shared
                unsafe {
                    src = scoped_src.take(agent).bind(gc.nogc());
                    target = scoped_target.take(agent).bind(gc.nogc());
                }
                len
            };
            (src, usize::try_from(len).ok())
        };

        // 15. or 6. If targetOffset = +∞, throw a RangeError exception.
        if target_offset.is_pos_infinity() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "targetOffset must be less than infinity",
                gc.into_nogc(),
            ));
        }
        let target_offset = usize::try_from(target_offset.into_i64() as u64).ok();
        // 16. or 7. If srcLength + targetOffset > targetLength, throw a RangeError exception.
        let (target_offset, src_length) = match (target_offset, src_length) {
            (Some(target_offset), Some(src_length))
                if src_length
                    .checked_add(target_offset)
                    .is_some_and(|r| r <= target_length) =>
            {
                (target_offset, src_length)
            }
            _ => {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "source length out of target bounds",
                    gc.into_nogc(),
                ));
            }
        };

        if source_is_typed_array {
            let Ok(src) = AnyTypedArray::try_from(src) else {
                // SAFETY: checked above.
                unsafe { unreachable_unchecked() }
            };

            if target.is_bigint() != src.is_bigint() {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "source elements are incompatible with target",
                    gc.into_nogc(),
                ));
            }
            target.unbind().set_from_typed_array(
                agent,
                target_offset,
                src.unbind(),
                0,
                src_length,
                gc.into_nogc(),
            )?;
        } else {
            let src = src.scope(agent, gc.nogc());
            let target_is_bigint = target.is_bigint();
            let target = target.scope(agent, gc.nogc());
            // 8. Let k be 0.
            // 9. Repeat, while k < srcLength,
            for k in 0..src_length {
                // a. Let Pk be ! ToString(𝔽(k)).
                let pk = PropertyKey::try_from(k).unwrap();
                // b. Let value be ? Get(src, Pk).
                let value = get(agent, src.get(agent), pk, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // c. Let targetIndex be 𝔽(targetOffset + k).
                let target_index = target_offset + k;
                // d. Perform ? TypedArraySetElement(target, targetIndex, value).
                let value = if target_is_bigint {
                    to_big_int(agent, value.unbind(), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc())
                        .into()
                } else {
                    to_number(agent, value.unbind(), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc())
                        .into()
                };
                target
                    .get(agent)
                    .typed_array_set_element(agent, target_index as i64, value);
                // e. Set k to k + 1.
            }
        };
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [23.2.3.27 %TypedArray%.prototype.slice ( start, end )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.slice)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.slice** as defined in 23.1.3.28.
    fn slice<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let start = arguments_list.get(0).bind(gc.nogc());
        let end = arguments_list.get(1).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value.bind(gc.nogc());
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let srcArrayLength be TypedArrayLength(taRecord).
        let src_array_length = ta_record.typed_array_length(agent);

        let o = ta_record.object;

        let mut recheck_length = false;

        let (o, start_index, end_index) = if let (
            relative_start @ Value::Undefined | relative_start @ Value::Integer(_),
            relative_end @ Value::Undefined | relative_end @ Value::Integer(_),
        ) = (start, end)
        {
            // 4. Let relativeStart be ? ToIntegerOrInfinity(start).
            // 5. If relativeStart = -∞, let startIndex be 0.
            // 6. Else if relativeStart < 0, let startIndex be max(srcArrayLength + relativeStart, 0).
            // 7. Else, let startIndex be min(relativeStart, srcArrayLength).
            let start_index = if let Value::Integer(s) = relative_start {
                calculate_relative_index(s.into_i64(), src_array_length)
            } else {
                // Undefined => NaN => 0
                0
            };
            // 8. If end is undefined, let relativeEnd be srcArrayLength; else let relativeEnd be ? ToIntegerOrInfinity(end).
            // 9. If relativeEnd = -∞, let endIndex be 0.
            // 10. Else if relativeEnd < 0, let endIndex be max(srcArrayLength + relativeEnd, 0).
            // 11. Else, let endIndex be min(relativeEnd, srcArrayLength).
            let end_index = if let Value::Integer(e) = relative_end {
                calculate_relative_index(e.into_i64(), src_array_length)
            } else {
                // Undefined => srcArrayLength
                src_array_length
            };
            (o, start_index, end_index)
        } else {
            // Note: SharedArrayBuffers cannot shrink.
            recheck_length = !o.is_shared();
            Self::slice_slow_path(
                agent,
                o.unbind(),
                src_array_length,
                start.unbind(),
                end.unbind(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        };
        // 12. Let countBytes be max(endIndex - startIndex, 0).
        let mut count = end_index.saturating_sub(start_index);
        // 13. Let A be ? TypedArraySpeciesCreate(O, « 𝔽(countBytes) »).
        let a = if let Some(mut data_block) = try_result_into_js(
            try_typed_array_species_create_with_length(agent, o, count, gc.nogc()),
        )
        .unbind()?
        {
            if recheck_length {
                // a. Set taRecord to MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
                let ta_record =
                    make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst);
                // b. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if ta_record.is_typed_array_out_of_bounds(agent) {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        gc.into_nogc(),
                    ));
                }
                // c. Set endIndex to min(endIndex, TypedArrayLength(taRecord)).
                let end_index = end_index.min(ta_record.typed_array_length(agent));
                // d. Set countBytes to max(endIndex - startIndex, 0).
                count = end_index.saturating_sub(start_index);
            }
            o.set_into_data_block(agent, &mut data_block, start_index, count);
            for_any_typed_array!(
                o,
                o,
                // SAFETY: type checked by the macro.
                unsafe {
                    typed_array_create_from_data_block(agent, o, data_block)
                        .unbind()
                        .bind(gc.into_nogc())
                        .cast::<TA>()
                }
                .into(),
                TA
            )
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let a = typed_array_species_create_with_length(agent, o.unbind(), count, gc.reborrow())
                .unbind()?;
            let gc = gc.into_nogc();
            let a = a.bind(gc);
            // 14. If countBytes > 0, then
            if count == 0 {
                // 15. Return A.
                return Ok(a.into());
            };
            // SAFETY: o is not shared.
            let o = unsafe { scoped_o.take(agent) }.bind(gc);
            // Note: SharedArrayBuffers cannot shrink.
            if !o.is_shared() {
                // a. Set taRecord to MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
                let ta_record =
                    make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst);
                // b. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if ta_record.is_typed_array_out_of_bounds(agent) {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        gc,
                    ));
                }
                // c. Set endIndex to min(endIndex, TypedArrayLength(taRecord)).
                let end_index = end_index.min(ta_record.typed_array_length(agent));
                // d. Set countBytes to max(endIndex - startIndex, 0).
                count = end_index.saturating_sub(start_index);
            }
            a.slice(agent, o, start_index, count);
            a
        };
        // 15. Return A.
        Ok(a.into())
    }

    fn slice_slow_path<'gc>(
        agent: &mut Agent,
        o: AnyTypedArray,
        src_array_length: usize,
        start: Value,
        end: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, (AnyTypedArray<'gc>, usize, usize)> {
        let o = o.scope(agent, gc.nogc());
        let end = end.scope(agent, gc.nogc());
        let start = start.bind(gc.nogc());

        // 4. Let relativeStart be ? ToIntegerOrInfinity(start).
        // 5. If relativeStart = -∞, let startIndex be 0.
        // 6. Else if relativeStart < 0, let startIndex be max(srcArrayLength + relativeStart, 0).
        // 7. Else, let startIndex be min(relativeStart, srcArrayLength).
        let start_index =
            calculate_relative_index_value(agent, start.unbind(), src_array_length, gc.reborrow())
                .unbind()?;

        // SAFETY: not shared.
        let end = unsafe { end.take(agent) }.bind(gc.nogc());

        // 8. If end is undefined, let relativeEnd be srcArrayLength; else let
        //    relativeEnd be ? ToIntegerOrInfinity(end).
        // 9. If relativeEnd = -∞, let endIndex be 0.
        // 10. Else if relativeEnd < 0, let endIndex be max(srcArrayLength + relativeEnd, 0).
        // 11. Else, let endIndex be min(relativeEnd, srcArrayLength).
        let end_index =
            calculate_relative_end_index_value(agent, end.unbind(), src_array_length, gc)?;

        Ok((
            // SAFETY: not shared.
            unsafe { o.take(agent) },
            start_index,
            end_index,
        ))
    }

    /// ### [23.2.3.28 %TypedArray%.prototype.some ( callback \[ , thisArg \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.some)
    ///
    /// The interpretation and use of the arguments of this method are the same
    /// as for **Array.prototype.some** as defined in 23.1.3.29.
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
        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);

        let mut o = ta_record.object;
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback) = is_callable(callback, nogc) else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
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
            let k_value = unwrap_try_get_value_or_unset(try_get(agent, o, pk, None, gc.nogc()));
            // c. Let testResult be ToBoolean(? Call(callback, thisArg, « kValue, 𝔽(k), O »)).
            let call = call_function(
                agent,
                callback.get(agent),
                this_arg.get(agent),
                Some(ArgumentsList::from_mut_slice(&mut [
                    k_value.unbind(),
                    Number::try_from(k).unwrap().unbind().into(),
                    o.unbind().into(),
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

    /// ### [23.2.3.29 %TypedArray%.prototype.sort ( comparator )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.sort)
    ///
    /// This is a distinct method that, except as described below, implements
    /// the same requirements as those of **Array.prototype.sort** as defined
    /// in 23.1.3.30. The implementation of this method may be optimized with
    /// the knowledge that the **this** value is an object that has a fixed
    /// length and whose integer-indexed properties are not sparse.
    ///
    /// This method is not generic. The this value must be an object with a
    /// \[\[TypedArrayName]] internal slot.
    ///
    /// > NOTE: Because **NaN** always compares greater than any other value
    /// > (see CompareTypedArrayElements), **NaN** property values always sort
    /// > to the end of the result when _comparator_ is not provided.
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
        // 3. Let taRecord be ? ValidateTypedArray(obj, seq-cst).
        let ta_record = validate_typed_array(agent, obj, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(gc.nogc());
        // 4. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);

        let obj = ta_record.object;

        // 5. NOTE: The following closure performs a numeric comparison rather than the string comparison used in 23.1.3.30.
        // 6. Let SortCompare be a new Abstract Closure with parameters (x, y) that captures comparator and performs the following steps when called:
        //    a. Return ? CompareTypedArrayElements(x, y, comparator).
        // 7. Let sortedList be ? SortIndexedProperties(obj, len, SortCompare, read-through-holes).
        if let Some(comparator) = comparator {
            let scoped_obj = obj.scope(agent, nogc);

            obj.unbind()
                .sort_with_comparator(agent, len, comparator, gc)?;

            // 10. Return obj.
            // SAFETY: not shared.
            Ok(unsafe { scoped_obj.take(agent) }.into())
        } else {
            let obj = obj.unbind().bind(gc.into_nogc());
            obj.sort(agent, len);

            Ok(obj.into())
        }
    }

    /// ### [23.2.3.30 %TypedArray%.prototype.subarray ( start, end )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.subarray)
    ///
    /// This method returns a new TypedArray whose element type is the element
    /// type of this TypedArray and whose ArrayBuffer is the ArrayBuffer of
    /// this TypedArray, referencing the elements in the interval from _start_
    /// (inclusive) to _end_ (exclusive). If either _start_ or _end_ is
    /// negative, it refers to an index from the end of the array, as opposed
    /// to from the beginning.
    ///
    /// This method is not generic. The *this* value must be an object with a
    /// \[\[TypedArrayName]] internal slot.
    fn subarray<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let start = arguments.get(0).bind(gc.nogc());
        let end = arguments.get(1).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        let o = require_internal_slot_typed_array(agent, o, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 5. Let srcRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let src_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst);

        // 6. If IsTypedArrayOutOfBounds(srcRecord) is true, then
        let src_length = if src_record.is_typed_array_out_of_bounds(agent) {
            // a. Let srcLength be 0.
            0
        } else {
            // 7. Else,
            //  a. Let srcLength be TypedArrayLength(srcRecord).
            src_record.typed_array_length(agent)
        };

        let (o, start_index, new_length) = if let (
            Value::Integer(relative_start),
            relative_end @ Value::Integer(_) | relative_end @ Value::Undefined,
        ) = (start, end)
        {
            // 8. Let relativeStart be ? ToIntegerOrInfinity(start).
            // 9. If relativeStart = -∞, let startIndex be 0.
            // 10. Else if relativeStart < 0, let startIndex be max(srcLength + relativeStart, 0).
            // 11. Else, let startIndex be min(relativeStart, srcLength).
            let start_index = calculate_relative_index(relative_start.into_i64(), src_length);

            // 15. If O.[[ArrayLength]] is auto and end is undefined, then
            let new_length = if o.array_length(agent).is_none() && end.is_undefined() {
                // a. Let argumentsList be « buffer, 𝔽(beginByteOffset) ».
                None
            } else {
                // 16. Else,
                // a. If end is undefined, let relativeEnd be srcLength; else
                //    let relativeEnd be ? ToIntegerOrInfinity(end).
                // b. If relativeEnd = -∞, let endIndex be 0.
                // c. Else if relativeEnd < 0, let endIndex be
                //    max(srcLength + relativeEnd, 0).
                // d. Else, let endIndex be min(relativeEnd, srcLength).
                let end_index = calculate_relative_end_index(relative_end, src_length);
                // e. Let newLength be max(endIndex - startIndex, 0).
                let new_length = end_index.saturating_sub(start_index);
                // f. Let argumentsList be « buffer, 𝔽(beginByteOffset), 𝔽(newLength) ».
                Some(new_length)
            };
            (src_record.object, start_index, new_length)
        } else {
            Self::subarray_slow_path(
                agent,
                src_record.object.unbind(),
                src_length,
                start.unbind(),
                end.unbind(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        };

        // 4. Let buffer be O.[[ViewedArrayBuffer]].
        let buffer = o.viewed_array_buffer(agent);
        // 12. Let elementSize be TypedArrayElementSize(O).
        let element_size = o.typed_array_element_size();
        // 13. Let srcByteOffset be O.[[ByteOffset]].
        let src_byte_offset = o.byte_offset(agent);
        // 14. Let beginByteOffset be srcByteOffset + (startIndex × elementSize).
        let begin_byte_offset =
            src_byte_offset.saturating_add(start_index.saturating_mul(element_size));

        // 17. Return ? TypedArraySpeciesCreate(O, argumentsList).
        typed_array_species_create_with_buffer(
            agent,
            o.unbind(),
            buffer.unbind(),
            begin_byte_offset,
            new_length,
            gc,
        )
        .map(|ta| ta.into())
    }

    fn subarray_slow_path<'gc>(
        agent: &mut Agent,
        o: AnyTypedArray,
        src_length: usize,
        start: Value,
        end: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, (AnyTypedArray<'gc>, usize, Option<usize>)> {
        // 15. If O.[[ArrayLength]] is auto and end is undefined, then
        // a. Let argumentsList be « buffer, 𝔽(beginByteOffset) ».
        let no_new_length_param = o.array_length(agent).is_none() && end.is_undefined();
        let o = o.scope(agent, gc.nogc());
        let end = end.scope(agent, gc.nogc());
        let start = start.bind(gc.nogc());

        // 8. Let relativeStart be ? ToIntegerOrInfinity(start).
        // 9. If relativeStart = -∞, let startIndex be 0.
        // 10. Else if relativeStart < 0, let startIndex be max(srcLength + relativeStart, 0).
        // 11. Else, let startIndex be min(relativeStart, srcLength).
        let start_index =
            calculate_relative_index_value(agent, start.unbind(), src_length, gc.reborrow())
                .unbind()?;

        // SAFETY: not shared.
        let end = unsafe { end.take(agent) }.bind(gc.nogc());

        // 15. If O.[[ArrayLength]] is auto and end is undefined, then
        let new_length = if no_new_length_param {
            // a. Let argumentsList be « buffer, 𝔽(beginByteOffset) ».
            None
        } else {
            // 16. Else,
            // a. If end is undefined, let relativeEnd be srcLength; else
            //    let relativeEnd be ? ToIntegerOrInfinity(end).
            // b. If relativeEnd = -∞, let endIndex be 0.
            // c. Else if relativeEnd < 0, let endIndex be
            //    max(srcLength + relativeEnd, 0).
            // d. Else, let endIndex be min(relativeEnd, srcLength).
            let end_index =
                calculate_relative_end_index_value(agent, end.unbind(), src_length, gc.reborrow())
                    .unbind()?;
            // e. Let newLength be max(endIndex - startIndex, 0).
            let new_length = end_index.saturating_sub(start_index);
            // f. Let argumentsList be « buffer, 𝔽(beginByteOffset), 𝔽(newLength) ».
            Some(new_length)
        };
        Ok((
            // SAFETY: not shared.
            unsafe { o.take(agent) },
            start_index,
            new_length,
        ))
    }

    /// ### [23.2.3.31 %TypedArray%.prototype.toLocaleString ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.tolocalestring)
    ///
    /// This is a distinct method that implements the same algorithm as
    /// **Array.prototype.toLocaleString** as defined in 23.1.3.32 except that
    /// TypedArrayLength is called in place of performing a \[\[Get]] of
    /// "length". The implementation of the algorithm may be optimized with the
    /// knowledge that the **this** value has a fixed length when the
    /// underlying buffer is not resizable and whose integer-indexed properties
    /// are not sparse. However, such optimization must not introduce any
    /// observable changes in the specified behaviour of the algorithm.
    ///
    /// This method is not generic. ValidateTypedArray is called with the this
    /// value and SEQ-CST as arguments prior to evaluating the algorithm. If
    /// its result is an abrupt completion that exception is thrown instead of
    /// evaluating the algorithm.
    ///
    /// > NOTE: If the ECMAScript implementation includes the ECMA-402
    /// > Internationalization API this method is based upon the algorithm for
    /// > **Array.prototype.toLocaleString** that is in the ECMA-402
    /// > specification.
    fn to_locale_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let array be ? ToObject(this value).
        // "ValidateTypedArray is called with the **this** value and SEQ-CST as arguments".
        let ta_record = validate_typed_array(agent, this_value, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let scoped_obj = ta_record.object.scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(array).
        // "TypedArrayLength is called in place of performing a [[Get]] of 'length'"
        let len = ta_record.typed_array_length(agent);
        // 3. Let separator be the implementation-defined list-separator String
        //    value appropriate for the host environment's current locale (such
        //    as ", ").
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
            // b. Let element be ! Get(array, ! ToString(𝔽(k))).
            let element = unwrap_try_get_value_or_unset(try_get(
                agent,
                scoped_obj.get(agent),
                PropertyKey::Integer(k.try_into().unwrap()),
                None,
                gc.nogc(),
            ));
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
                r.push_wtf8(s.as_wtf8_(agent));
            };
            // d. Set k to k + 1.
            k += 1;
        }
        // 7. Return R.
        Ok(String::from_wtf8_buf(agent, r, gc.into_nogc()).into())
    }

    /// ### [23.2.3.32 %TypedArray%.prototype.toReversed ( )](https://tc39.es/ecma262/#sec-array.prototype.tospliced)
    fn to_reversed<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc)?;
        let o = ta_record.object;
        // 3. Let length be TypedArrayLength(taRecord).
        let length = ta_record.typed_array_length(agent);

        // 4. Let A be ? TypedArrayCreateSameType(O, « 𝔽(length) »).
        let a = o.typed_array_create_same_type_and_copy_data(agent, length, gc)?;
        // 5. Let k be 0.
        // 6. Repeat, while k < length,
        {
            let a: AnyTypedArray = a.into();
            // a. Let from be ! ToString(𝔽(length - k - 1)).
            // b. Let Pk be ! ToString(𝔽(k)).
            // c. Let fromValue be ! Get(O, from).
            // d. Perform ! Set(A, Pk, fromValue, true).
            // e. Set k to k + 1.
            a.reverse(agent, length);
        }
        // 7. Return A.
        Ok(a.into())
    }

    /// ### [23.2.3.33 %TypedArray%.prototype.toSorted ( comparator )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.tosorted)
    fn to_sorted<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let comparator = arguments.get(0).bind(gc.nogc());
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
        // 2. Let O be the this value.
        let o = this_value;
        // 3. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 4. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);

        let o = ta_record.object;

        // 5. Let A be ? TypedArrayCreateSameType(O, len).
        let mut a = o
            .typed_array_create_same_type_and_copy_data(agent, len, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 6. NOTE: The following closure performs a numeric comparison rather
        //    than the string comparison used in 23.1.3.34.
        // 7. Let SortCompare be a new Abstract Closure with parameters (x, y)
        //    that captures comparator and performs the following steps when
        //    called:
        //        a. Return ? CompareTypedArrayElements(x, y, comparator).
        // 8. Let sortedList be ? SortIndexedProperties(O, len, SortCompare, read-through-holes).
        // 9. Let j be 0.
        // 10. Repeat, while j < len,
        if let Some(comparator) = comparator {
            let scoped_a = a.scope(agent, gc.nogc());
            let any_a: AnyTypedArray = a.into();
            any_a
                .unbind()
                .sort_with_comparator(agent, len, comparator, gc.reborrow())
                .unbind()?;
            // SAFETY: not shared.
            a = unsafe { scoped_a.take(agent) }.bind(gc.into_nogc());
        } else {
            let a: AnyTypedArray = a.into();
            a.sort(agent, len);
            // a. Perform ! Set(A, ! ToString(𝔽(j)), sortedList[j], true).
            // b. Set j to j + 1.
        }
        // 10. Return obj.
        Ok(a.unbind().into())
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
        Ok(ArrayIterator::from_object(agent, o.into(), CollectionIteratorKind::Value).into())
    }

    /// ### [23.2.3.36 %TypedArray%.prototype.with ( index, value )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.with)
    fn with<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let index = arguments.get(0).bind(gc.nogc());
        let value = arguments.get(1).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let len be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);

        let mut o = ta_record.object;
        let is_bigint = o.is_bigint();

        // 4. Let relativeIndex be ? ToIntegerOrInfinity(index).
        let (relative_index, numeric_value) =
            if let (Value::Integer(index), Ok(value)) = (index, Primitive::try_from(value)) {
                let relative_index = index.into_i64();
                // 7. If O.[[ContentType]] is BIGINT, let numericValue be ? ToBigInt(value).
                let numeric_value: Numeric = if is_bigint {
                    to_big_int_primitive(agent, value, gc.nogc())
                        .unbind()?
                        .bind(gc.nogc())
                        .into()
                } else {
                    // 8. Else, let numericValue be ? ToNumber(value).
                    to_number_primitive(agent, value, gc.nogc())
                        .unbind()?
                        .bind(gc.nogc())
                        .into()
                };
                (relative_index, numeric_value)
            } else {
                let scoped_o = o.scope(agent, gc.nogc());
                let value = value.scope(agent, gc.nogc());
                let relative_index = to_integer_or_infinity(agent, index.unbind(), gc.reborrow())
                    .unbind()?
                    .into_i64();

                // SAFETY: not shared.
                let value = unsafe { value.take(agent) }.bind(gc.nogc());

                // 7. If O.[[ContentType]] is BIGINT, let numericValue be ? ToBigInt(value).
                let numeric_value = if is_bigint {
                    to_big_int(agent, value.unbind(), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc())
                        .into()
                } else {
                    // 8. Else, let numericValue be ? ToNumber(value).
                    to_number(agent, value.unbind(), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc())
                        .into()
                };
                // SAFETY: not shared.
                o = unsafe { scoped_o.take(agent).bind(gc.nogc()) };
                (relative_index, numeric_value)
            };
        let o = o.unbind();
        let numeric_value = numeric_value.unbind();
        let gc = gc.into_nogc();
        let o = o.bind(gc);
        let numeric_value = numeric_value.bind(gc);

        // 5. If relativeIndex ≥ 0, let actualIndex be relativeIndex.
        // 6. Else, let actualIndex be len + relativeIndex.
        let actual_index = calculate_offset_index(relative_index, len);

        // 9. If IsValidIntegerIndex(O, 𝔽(actualIndex)) is false, throw a RangeError exception.
        let Some(actual_index) = o.is_valid_integer_index(agent, actual_index) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Index out of bounds",
                gc,
            ));
        };
        // 10. Let A be ? TypedArrayCreateSameType(O, « 𝔽(len) »).
        let a = o.typed_array_create_same_type_and_copy_data(agent, len, gc)?;
        // 11. Let k be 0.
        // 12. Repeat, while k < len
        //  a. Let Pk be ! ToString(𝔽(k)).
        //  b. If k = actualIndex, let fromValue be numericValue.
        //  c. Else, let fromValue be ! Get(O, Pk).
        //  d. Perform ! Set(A, Pk, fromValue, true).
        //  e. Set k to k + 1.
        let pk = PropertyKey::try_from(actual_index).unwrap();
        let result = unwrap_try(a.try_set(agent, pk, numeric_value.into(), a.into(), None, gc));
        debug_assert!(result.succeeded());
        // 13. Return A.
        Ok(a.into())
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
        if let Ok(o) = AnyTypedArray::try_from(this_value) {
            // 4. Let name be O.[[TypedArrayName]].
            // 5. Assert: name is a String.
            // 6. Return name.
            match o {
                AnyTypedArray::Int8Array(_) => Ok(BUILTIN_STRING_MEMORY.Int8Array.into()),
                AnyTypedArray::Uint8Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint8Array.into()),
                AnyTypedArray::Uint8ClampedArray(_) => {
                    Ok(BUILTIN_STRING_MEMORY.Uint8ClampedArray.into())
                }
                AnyTypedArray::Int16Array(_) => Ok(BUILTIN_STRING_MEMORY.Int16Array.into()),
                AnyTypedArray::Uint16Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint16Array.into()),
                AnyTypedArray::Int32Array(_) => Ok(BUILTIN_STRING_MEMORY.Int32Array.into()),
                AnyTypedArray::Uint32Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint32Array.into()),
                AnyTypedArray::BigInt64Array(_) => Ok(BUILTIN_STRING_MEMORY.BigInt64Array.into()),
                AnyTypedArray::BigUint64Array(_) => Ok(BUILTIN_STRING_MEMORY.BigUint64Array.into()),
                #[cfg(feature = "proposal-float16array")]
                AnyTypedArray::Float16Array(_) => Ok(BUILTIN_STRING_MEMORY.Float16Array.into()),
                AnyTypedArray::Float32Array(_) => Ok(BUILTIN_STRING_MEMORY.Float32Array.into()),
                AnyTypedArray::Float64Array(_) => Ok(BUILTIN_STRING_MEMORY.Float64Array.into()),
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedInt8Array(_) => Ok(BUILTIN_STRING_MEMORY.Int8Array.into()),
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedUint8Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint8Array.into()),
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedUint8ClampedArray(_) => {
                    Ok(BUILTIN_STRING_MEMORY.Uint8ClampedArray.into())
                }
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedInt16Array(_) => Ok(BUILTIN_STRING_MEMORY.Int16Array.into()),
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedUint16Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint16Array.into()),
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedInt32Array(_) => Ok(BUILTIN_STRING_MEMORY.Int32Array.into()),
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedUint32Array(_) => Ok(BUILTIN_STRING_MEMORY.Uint32Array.into()),
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedBigInt64Array(_) => {
                    Ok(BUILTIN_STRING_MEMORY.BigInt64Array.into())
                }
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedBigUint64Array(_) => {
                    Ok(BUILTIN_STRING_MEMORY.BigUint64Array.into())
                }
                #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
                AnyTypedArray::SharedFloat16Array(_) => {
                    Ok(BUILTIN_STRING_MEMORY.Float16Array.into())
                }
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedFloat32Array(_) => {
                    Ok(BUILTIN_STRING_MEMORY.Float32Array.into())
                }
                #[cfg(feature = "shared-array-buffer")]
                AnyTypedArray::SharedFloat64Array(_) => {
                    Ok(BUILTIN_STRING_MEMORY.Float64Array.into())
                }
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
                    .with_value(array_prototype_to_string.into())
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
                    .with_value(typed_array_prototype_values.into())
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
) -> JsResult<'a, AnyTypedArray<'a>> {
    // 1. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
    AnyTypedArray::try_from(o.unbind()).map_err(|_| {
        agent.throw_exception_with_static_message(
            crate::ecmascript::ExceptionType::TypeError,
            "Expected this to be TypedArray",
            gc,
        )
    })
}

/// Calculate an index offset from the start (if positive) or end (if negative)
/// of a TypedArray. If the offset over- or underflows the length then None is
/// returned.
#[inline]
fn calculate_offset_index(relative_index: i64, len: usize) -> i64 {
    if relative_index >= 0 {
        // let actualIndex be relativeIndex
        return relative_index;
    }
    if let Ok(len) = i64::try_from(len) {
        // Else, let actualIndex be len + relativeIndex.
        len.saturating_add(relative_index)
    } else {
        // Can't turn length into a i64... that's pretty weird. At least u64
        // should always work.
        let len = u64::try_from(len).unwrap();
        // SAFETY: len is a positive value larger than i64::MAX; adding a
        // negative number to it cannot under- or overflow.
        let actual_index = unsafe { len.checked_add_signed(relative_index).unwrap_unchecked() };
        i64::try_from(actual_index).unwrap_or(i64::MAX)
    }
}

#[inline]
fn calculate_relative_index(relative_index: i64, len: usize) -> usize {
    // SAFETY: length values in JavaScript are always within 2^53, ie. fit in
    // 64 bits. usize may be 128, 64, or 32 bits but in any case this relation
    // holds.
    let len = unsafe { u64::try_from(len).unwrap_unchecked() };
    let result = if relative_index < 0 {
        len.saturating_add_signed(relative_index)
    } else {
        len.min(relative_index.unsigned_abs())
    };
    // SAFETY: len - abs(relative_index) is at most len, and len was usize.
    unsafe { usize::try_from(result).unwrap_unchecked() }
}

#[inline]
fn calculate_relative_end_index(relative_index: Value, len: usize) -> usize {
    let relative_index = if let Value::Integer(i) = relative_index {
        i.into_i64()
    } else {
        return len;
    };
    // SAFETY: length values in JavaScript are always within 2^53, ie. fit in
    // 64 bits. usize may be 128, 64, or 32 bits but in any case this relation
    // holds.
    let len = unsafe { u64::try_from(len).unwrap_unchecked() };
    let result = if relative_index < 0 {
        len.saturating_add_signed(relative_index)
    } else {
        len.min(relative_index.unsigned_abs())
    };
    // SAFETY: len - abs(relative_index) is at most len, and len was usize.
    unsafe { usize::try_from(result).unwrap_unchecked() }
}

#[inline]
fn calculate_relative_index_value<'gc>(
    agent: &mut Agent,
    index: Value,
    len: usize,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, usize> {
    // 1. Let relativeIndex be ? ToIntegerOrInfinity(index).
    let relative_index = to_integer_or_infinity(agent, index.unbind(), gc).unbind()?;
    // 5. If relativeIndex = -∞, let result be 0.
    if relative_index.is_neg_infinity() {
        Ok(0)
    } else {
        Ok(calculate_relative_index(relative_index.into_i64(), len))
    }
}

#[inline]
fn calculate_relative_end_index_value<'gc>(
    agent: &mut Agent,
    index: Value,
    len: usize,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, usize> {
    if index.is_undefined() {
        return Ok(len);
    }
    // 1. Let relativeIndex be ? ToIntegerOrInfinity(index).
    let relative_index = to_integer_or_infinity(agent, index.unbind(), gc).unbind()?;
    // 5. If relativeIndex = -∞, let result be 0.
    if relative_index.is_neg_infinity() {
        Ok(0)
    } else {
        Ok(calculate_relative_index(relative_index.into_i64(), len))
    }
}
