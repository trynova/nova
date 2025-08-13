// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use num_traits::ToPrimitive;
use wtf8::Wtf8Buf;

use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{get_iterator_from_method, iterator_to_list},
            operations_on_objects::{
                call_function, get, get_method, invoke, length_of_array_like, set,
                throw_not_callable, try_get, try_set,
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
            ArgumentsList, ArrayBuffer, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic,
            BuiltinIntrinsicConstructor,
            array_buffer::{
                Ordering, get_value_from_buffer, is_detached_buffer, set_value_in_buffer,
            },
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
            BUILTIN_STRING_MEMORY, Function, InternalMethods, IntoNumeric, IntoObject, IntoValue,
            Number, Object, PropertyKey, String, U8Clamped, Value, Viewable, unwrap_try_get_value,
            unwrap_try_get_value_or_unset,
        },
    },
    engine::{
        Scoped, TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
        unwrap_try,
    },
    heap::{IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
    with_typed_array_viewable,
};

use super::abstract_operations::{
    TypedArrayWithBufferWitnessRecords, is_typed_array_out_of_bounds, is_valid_integer_index,
    make_typed_array_with_buffer_witness_record, set_typed_array_from_array_like,
    set_typed_array_from_typed_array, typed_array_byte_length,
    typed_array_create_from_constructor_with_length, typed_array_create_same_type,
    typed_array_length, typed_array_species_create_with_buffer,
    typed_array_species_create_with_length, validate_typed_array,
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

    /// ### [23.2.2.1 %TypedArray%.from ( source \[ , mapper \[ , thisArg \] \] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.from)
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
            let len = values.len(agent).to_i64().unwrap();
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;

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
        Ok(unwrap_try_get_value_or_unset(try_get(
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
        let size =
            with_typed_array_viewable!(o, typed_array_byte_length::<T>(agent, &ta_record, gc));

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
        if with_typed_array_viewable!(o, is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc)) {
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
        gc: GcScope<'gc, '_>,
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
        let o = with_typed_array_viewable!(
            o,
            copy_within_typed_array::<T>(
                agent,
                ta_record.unbind(),
                target.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            )
        );
        // 18. Return O.
        o.map(|o| o.into_value())
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
        let len = with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, nogc));
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
            let k_value = unwrap_try_get_value_or_unset(try_get(agent, o, pk, gc.nogc()));
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

        let o = with_typed_array_viewable!(
            ta_record.object,
            fill_typed_array::<T>(
                agent,
                ta_record.unbind(),
                value.unbind(),
                start.unbind(),
                end.unbind(),
                gc,
            )
        );

        o.map(|o| o.into_value())
    }

    /// ### [23.2.3.10 %TypedArray%.prototype.filter ( callback [ , thisArg ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.filter)
    /// The interpretation and use of the arguments of this method
    /// are the same as for Array.prototype.filter as defined in 23.1.3.8.
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
        let a = with_typed_array_viewable!(
            o,
            filter_typed_array::<T>(
                agent,
                callback.unbind(),
                this_arg.unbind(),
                ta_record.unbind(),
                gc,
            )
        );

        a.map(|a| a.into_value())
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
            let k_value = unwrap_try_get_value_or_unset(try_get(agent, o, pk, gc.nogc()));
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
            let element_k = unwrap_try_get_value_or_unset(try_get(
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
        let result = with_typed_array_viewable!(
            o,
            search_typed_element::<T, true>(
                agent,
                o.unbind(),
                search_element.unbind(),
                k,
                len,
                gc.into_nogc(),
            )
        );

        Ok(result.map_or(-1, |v| v as i64).try_into().unwrap())
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
        let (len, element_size) = with_typed_array_viewable!(
            o,
            (
                typed_array_length::<T>(agent, &ta_record, nogc),
                core::mem::size_of::<T>(),
            )
        );

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
        let sep = sep_string.as_wtf8(agent).to_owned();
        // 6. Let R be the empty String.
        let mut r = Wtf8Buf::with_capacity(len * 3);
        // 7. Let k be 0.
        // 8. Repeat, while k < len,
        let offset = o.byte_offset(agent);
        let viewed_array_buffer = o.get_viewed_array_buffer(agent, gc);
        // Note: Above ToString might have detached the ArrayBuffer or shrunk its length.
        let after_len = if recheck_buffer {
            let is_detached = is_detached_buffer(agent, viewed_array_buffer);
            let ta_record =
                make_typed_array_with_buffer_witness_record(agent, o, Ordering::Unordered, gc);

            with_typed_array_viewable!(o, {
                let is_invalid =
                    is_detached || is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc);
                if is_invalid {
                    None
                } else {
                    Some(typed_array_length::<T>(agent, &ta_record, gc))
                }
            })
        } else {
            // Note: Growable SharedArrayBuffers are a thing, and can change the
            // length at any point in time but they can never shrink the buffer.
            // Hence the TypedArray or any of its indexes are never invalidated.
            Some(len)
        };
        let Some(after_len) = after_len else {
            let count = len.saturating_sub(1);
            let byte_count = count * sep.len();
            let mut buf = Wtf8Buf::with_capacity(byte_count);
            for _ in 0..count {
                buf.push_wtf8(sep);
            }
            return Ok(String::from_wtf8_buf(agent, buf, gc).into_value());
        };
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
            let byte_index_in_buffer = k * element_size + offset;
            // b. Let element be ! Get(O, ! ToString(𝔽(k))).
            let element = with_typed_array_viewable!(
                o,
                get_value_from_buffer::<T>(
                    agent,
                    viewed_array_buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::Unordered,
                    None,
                    gc,
                )
            );
            // i. Let S be ! ToString(element).
            let s = unwrap_try(try_to_string(agent, element, gc)).unwrap();
            // ii. Set R to the string-concatenation of R and S.
            r.push_wtf8(s.as_wtf8(agent));
            // d. Set k to k + 1.
        }
        // 9. Return R.
        Ok(String::from_wtf8_buf(agent, r, gc).into_value().unbind())
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
        let result = with_typed_array_viewable!(
            o,
            search_typed_element::<T, false>(
                agent,
                o,
                search_element.get(agent),
                k,
                len,
                gc.nogc(),
            )
        );

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
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has [[ViewedArrayBuffer]] and [[ArrayLength]] internal slots.
        let o = require_internal_slot_typed_array(agent, this_value, gc)?;
        // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc);
        // 5. If IsTypedArrayOutOfBounds(taRecord) is true, return +0𝔽.
        if with_typed_array_viewable!(o, is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc)) {
            return Ok(Value::pos_zero());
        }
        // 6. Let length be TypedArrayLength(taRecord).
        let length =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc)) as i64;
        // 7. Return 𝔽(length).
        Ok(Value::try_from(length).unwrap())
    }

    /// ### [23.2.3.22 %TypedArray%.prototype.map ( callback [ , thisArg ] )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.map)
    /// The interpretation and use of the arguments of this method are the same as for Array.prototype.map as defined in 23.1.3.21.
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

        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        let Some(callback_fn) = is_callable(callback_fn, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Callback is not callable",
                gc.into_nogc(),
            ));
        };

        let a = with_typed_array_viewable!(
            ta_record.object,
            map_typed_array::<T>(
                agent,
                callback_fn.unbind(),
                this_arg.unbind(),
                ta_record.unbind(),
                gc
            )?
        );
        Ok(a.into_value())
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
            let result = unwrap_try_get_value(try_get(agent, o, pk, gc.nogc()));
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
            let k_value =
                unwrap_try_get_value_or_unset(try_get(agent, scoped_o.get(agent), pk, gc.nogc()));
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;
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
            let result = unwrap_try_get_value_or_unset(try_get(agent, o, pk, gc.nogc()));
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
            let k_value =
                unwrap_try_get_value_or_unset(try_get(agent, scoped_o.get(agent), pk, gc.nogc()));
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
        let o = ta_record.object;
        with_typed_array_viewable!(o, reverse_typed_array::<T>(agent, ta_record, o, gc));
        // 7. Return O.
        Ok(o.into_value())
    }

    /// [23.2.3.26 %TypedArray%.prototype.set ( source [ , offset ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.set)
    /// This method sets multiple values in this TypedArray, reading the values
    /// from source. The details differ based upon the type of source. The optional
    /// offset value indicates the first element index in this TypedArray where
    /// values are written. If omitted, it is assumed to be 0.
    fn set<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let source = arguments.get(0).bind(gc.nogc());
        let offset = arguments.get(1).bind(gc.nogc());
        // 1. Let target be the this value.
        let target = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(target, [[TypedArrayName]]).
        let o = require_internal_slot_typed_array(agent, target, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        with_typed_array_viewable!(
            o,
            set_typed_array::<T>(
                agent,
                o.unbind(),
                source.unbind(),
                offset.unbind(),
                gc.reborrow()
            )
            .unbind()?
        );
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ## [23.2.3.27 %TypedArray%.prototype.slice ( start, end )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.slice)
    /// The interpretation and use of the arguments of this method
    /// are the same as for Array.prototype.slice as defined in 23.1.3.28.
    fn slice<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let start = arguments_list.get(0).bind(gc.nogc());
        let end = arguments_list.get(1).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value.bind(gc.nogc());
        // 2. Let taRecord be ? ValidateTypedArray(O, seq-cst).
        let ta_record = validate_typed_array(agent, o, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        let a = with_typed_array_viewable!(
            o,
            slice_typed_array::<T>(agent, ta_record.unbind(), start.unbind(), end.unbind(), gc)
        );
        // 15. Return A.
        a.map(|a| a.into_value())
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
        let len = with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, nogc));
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
            let k_value = unwrap_try_get_value_or_unset(try_get(agent, o, pk, gc.nogc()));
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
        // 3. Let taRecord be ? ValidateTypedArray(obj, seq-cst).
        let ta_record = validate_typed_array(agent, obj, Ordering::SeqCst, nogc)
            .unbind()?
            .bind(gc.nogc());
        // 4. Let len be TypedArrayLength(taRecord).
        // 5. NOTE: The following closure performs a numeric comparison rather than the string comparison used in 23.1.3.30.
        // 6. Let SortCompare be a new Abstract Closure with parameters (x, y) that captures comparator and performs the following steps when called:
        //    a. Return ? CompareTypedArrayElements(x, y, comparator).
        // 7. Let sortedList be ? SortIndexedProperties(obj, len, SortCompare, read-through-holes).
        if let Some(comparator) = comparator {
            let obj = ta_record.object;
            let scoped_obj = ta_record.object.scope(agent, nogc);

            with_typed_array_viewable!(
                obj,
                sort_comparator_typed_array::<T>(
                    agent,
                    ta_record.unbind(),
                    scoped_obj.clone(),
                    comparator,
                    gc,
                )?
            );

            // 10. Return obj.
            Ok(scoped_obj.get(agent).into_value())
        } else {
            let ta_record = ta_record.unbind();
            let nogc = gc.into_nogc();
            let ta_record = ta_record.bind(nogc);
            let obj = ta_record.object;
            match obj {
                TypedArray::Int8Array(_) => {
                    sort_total_cmp_typed_array::<i8>(agent, ta_record, nogc)
                }
                TypedArray::Uint8Array(_) => {
                    sort_total_cmp_typed_array::<u8>(agent, ta_record, nogc)
                }
                TypedArray::Uint8ClampedArray(_) => {
                    sort_total_cmp_typed_array::<U8Clamped>(agent, ta_record, nogc)
                }
                TypedArray::Int16Array(_) => {
                    sort_total_cmp_typed_array::<i16>(agent, ta_record, nogc)
                }
                TypedArray::Uint16Array(_) => {
                    sort_total_cmp_typed_array::<u16>(agent, ta_record, nogc)
                }
                TypedArray::Int32Array(_) => {
                    sort_total_cmp_typed_array::<i32>(agent, ta_record, nogc)
                }
                TypedArray::Uint32Array(_) => {
                    sort_total_cmp_typed_array::<u32>(agent, ta_record, nogc)
                }
                TypedArray::BigInt64Array(_) => {
                    sort_total_cmp_typed_array::<i64>(agent, ta_record, nogc)
                }
                TypedArray::BigUint64Array(_) => {
                    sort_total_cmp_typed_array::<u64>(agent, ta_record, nogc)
                }
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => {
                    sort_ecmascript_cmp_typed_array::<f16>(agent, ta_record, nogc)
                }
                TypedArray::Float32Array(_) => {
                    sort_ecmascript_cmp_typed_array::<f32>(agent, ta_record, nogc)
                }
                TypedArray::Float64Array(_) => {
                    sort_ecmascript_cmp_typed_array::<f64>(agent, ta_record, nogc)
                }
            };
            Ok(obj.into_value())
        }
    }

    /// ### [23.2.3.30 %TypedArray%.prototype.subarray ( start, end )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.subarray)
    /// This method returns a new TypedArray whose element type is the element type of
    /// this TypedArray and whose ArrayBuffer is the ArrayBuffer of this TypedArray,
    /// referencing the elements in the interval from start (inclusive) to end (exclusive).
    /// If either start or end is negative, it refers to an index from the end of the array,
    /// as opposed to from the beginning.
    fn subarray<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let start = arguments.get(0).bind(gc.nogc());
        let end = arguments.get(1).bind(gc.nogc());
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        let o = require_internal_slot_typed_array(agent, o, gc.nogc()).unbind()?;
        // 4. Let buffer be O.[[ViewedArrayBuffer]].
        let buffer = o.get_viewed_array_buffer(agent, gc.nogc());
        // 5. Let srcRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let src_record =
            make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc.nogc());
        let res = with_typed_array_viewable!(
            src_record.object,
            subarray_typed_array::<T>(
                agent,
                src_record.unbind(),
                start.unbind(),
                end.unbind(),
                buffer.unbind(),
                gc
            )
        );
        res.map(|v| v.into_value())
    }

    /// ### [23.2.3.31 %TypedArray%.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.tolocalestring)
    /// This is a distinct method that implements the same algorithm as Array.prototype.toLocaleString
    /// as defined in 23.1.3.32 except that TypedArrayLength is called in place of performing a
    /// [[Get]] of "length". The implementation of the algorithm may be optimized with the knowledge
    /// that the this value has a fixed length when the underlying buffer is not resizable and whose
    /// integer-indexed properties are not sparse. However, such optimization must not introduce any
    /// observable changes in the specified behaviour of the algorithm. This method is not generic.
    /// ValidateTypedArray is called with the this value and seq-cst as arguments prior to evaluating
    /// the algorithm. If its result is an abrupt completion that exception is thrown instead of
    /// evaluating the algorithm.
    ///
    /// > #### Note
    /// > If the ECMAScript implementation includes the ECMA-402 Internationalization API this method is
    /// > based upon the algorithm for Array.prototype.toLocaleString that is in the ECMA-402 specification.
    fn to_locale_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let array be ? ToObject(this value).
        let ta_record = validate_typed_array(agent, this_value, Ordering::SeqCst, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = ta_record.object;
        let o = o.scope(agent, gc.nogc());
        // 2. Let len be ? LengthOfArrayLike(array).
        let len = with_typed_array_viewable!(
            o.get(agent),
            typed_array_length::<T>(agent, &ta_record, gc.nogc())
        ) as i64;
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
                o.get(agent),
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
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc.nogc()))
                as i64;

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

    /// ### [23.2.3.33 %TypedArray%.prototype.toSorted ( comparator )](https://tc39.es/ecma262/#sec-%typedarray%.prototype.tosorted)
    fn to_sorted<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
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
        if let Some(comparator) = comparator {
            let obj = ta_record.object;
            let a = with_typed_array_viewable!(
                obj,
                to_sorted_comparator_typed_array::<T>(agent, ta_record.unbind(), comparator, gc)
            );
            // 10. Return obj.
            a.map(|a| a.into_value())
        } else {
            let obj = ta_record.object;
            let a = match obj {
                TypedArray::Int8Array(_) => {
                    to_sorted_total_cmp_typed_array::<i8>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Uint8Array(_) => {
                    to_sorted_total_cmp_typed_array::<u8>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Uint8ClampedArray(_) => {
                    to_sorted_total_cmp_typed_array::<U8Clamped>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Int16Array(_) => {
                    to_sorted_total_cmp_typed_array::<i16>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Uint16Array(_) => {
                    to_sorted_total_cmp_typed_array::<u16>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Int32Array(_) => {
                    to_sorted_total_cmp_typed_array::<i32>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Uint32Array(_) => {
                    to_sorted_total_cmp_typed_array::<u32>(agent, ta_record.unbind(), gc)
                }
                TypedArray::BigInt64Array(_) => {
                    to_sorted_total_cmp_typed_array::<i64>(agent, ta_record.unbind(), gc)
                }
                TypedArray::BigUint64Array(_) => {
                    to_sorted_total_cmp_typed_array::<u64>(agent, ta_record.unbind(), gc)
                }
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => {
                    to_sorted_ecmascript_cmp_typed_array::<f16>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Float32Array(_) => {
                    to_sorted_ecmascript_cmp_typed_array::<f32>(agent, ta_record.unbind(), gc)
                }
                TypedArray::Float64Array(_) => {
                    to_sorted_ecmascript_cmp_typed_array::<f64>(agent, ta_record.unbind(), gc)
                }
            };
            // 10. Return obj.
            a.map(|a| a.into_value())
        }
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

    /// ### [23.2.3.36 %TypedArray%.prototype.with ( index, value )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%typedarray%.prototype.with)
    fn with<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
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
        let o = with_typed_array_viewable!(
            ta_record.object,
            with_typed_array::<T>(
                agent,
                ta_record.unbind(),
                index.unbind(),
                value.unbind(),
                gc,
            )
        );
        o.map(|o| o.into_value())
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

pub(crate) fn viewable_slice<'a, T: Viewable>(
    agent: &'a mut Agent,
    ta: TypedArray,
    gc: NoGcScope,
) -> &'a [T] {
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_slice(agent);
    if byte_slice.is_empty() {
        return &[];
    }
    let byte_limit = byte_length.map_or(byte_slice.len(), |l| byte_offset + l);
    byte_slice_to_viewable::<T>(byte_slice, byte_offset, byte_limit)
}

pub(crate) fn byte_slice_to_viewable<T: Viewable>(
    byte_slice: &[u8],
    byte_offset: usize,
    byte_limit: usize,
) -> &[T] {
    if byte_slice.is_empty() || byte_limit > byte_slice.len() {
        return &[];
    }
    let byte_slice = &byte_slice[byte_offset..byte_limit];
    // SAFETY: All bytes in byte_slice are initialized, and all bitwise
    // combinations of T are valid values. Alignment of T's is
    // guaranteed by align_to_mut itself.
    let (head, slice, _) = unsafe { byte_slice.align_to::<T>() };
    if !head.is_empty() {
        panic!("TypedArray is not properly aligned");
    }
    slice
}

fn viewable_slice_mut<'a, T: Viewable>(
    agent: &'a mut Agent,
    ta: TypedArray,
    gc: NoGcScope,
) -> &'a mut [T] {
    let array_buffer = ta.get_viewed_array_buffer(agent, gc);
    let byte_offset = ta.byte_offset(agent);
    let byte_length = ta.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return &mut [];
    }
    let byte_limit = byte_length.map_or(byte_slice.len(), |l| byte_offset + l);
    byte_slice_to_viewable_mut::<T>(byte_slice, byte_offset, byte_limit)
}

pub(crate) fn byte_slice_to_viewable_mut<T: Viewable>(
    byte_slice: &mut [u8],
    byte_offset: usize,
    byte_limit: usize,
) -> &mut [T] {
    if byte_slice.is_empty() || byte_limit > byte_slice.len() {
        return &mut [];
    }
    let byte_slice = &mut byte_slice[byte_offset..byte_limit];
    // SAFETY: All bytes in byte_slice are initialized, and all bitwise
    // combinations of T are valid values. Alignment of T's is
    // guaranteed by align_to_mut itself.
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        panic!("TypedArray is not properly aligned");
    }
    slice
}

fn map_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    callback_fn: Function,
    this_arg: Value,
    ta_record: TypedArrayWithBufferWitnessRecords,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let nogc = gc.nogc();
    let ta_record = ta_record.bind(nogc);
    let callback_fn = callback_fn.scope(agent, nogc);
    let this_arg = this_arg.scope(agent, nogc);
    let o = ta_record.object.scope(agent, nogc);
    // 3. Let len be TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, nogc);

    // 5. Let A be ? TypedArraySpeciesCreate(O, « 𝔽(len) »).
    let a = typed_array_species_create_with_length::<T>(
        agent,
        ta_record.object.unbind(),
        len as i64,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 6. Let k be 0.
    // 7. Repeat, while k < len,
    let a = a.scope(agent, gc.nogc());
    for k in 0..len {
        // 𝔽(k)
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk = PropertyKey::try_from(k).unwrap();
        // b. Let kValue be ! Get(O, Pk).
        let k_value = unwrap_try_get_value_or_unset(try_get(agent, o.get(agent), pk, gc.nogc()));
        // c. Let mappedValue be ? Call(callback, thisArg, « kValue, 𝔽(k), O »).
        let mapped_value = call_function(
            agent,
            callback_fn.get(agent),
            this_arg.get(agent),
            Some(ArgumentsList::from_mut_slice(&mut [
                k_value.unbind(),
                // SAFETY: pk is a PropertyKey::Integer and we want a
                // Value::Integer here; this is exactly correct.
                unsafe { pk.into_value_unchecked() },
                o.get(agent).into_value(),
            ])),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // d. Perform ? Set(A, Pk, mappedValue, true).
        set(
            agent,
            a.get(agent).into_object(),
            pk,
            mapped_value.unbind(),
            true,
            gc.reborrow(),
        )
        .unbind()?
        // e. Set k to k + 1.
    }
    // 8. Return A.
    Ok(a.get(agent).unbind())
}

fn search_typed_element<T: Viewable, const ASCENDING: bool>(
    agent: &mut Agent,
    ta: TypedArray,
    search_element: Value,
    k: usize,
    len: usize,
    gc: NoGcScope,
) -> Option<usize> {
    let search_element = T::try_from_value(agent, search_element)?;
    let slice = viewable_slice::<T>(agent, ta, gc);
    // Length of the TypedArray may have changed between when we measured it
    // and here: We'll never try to access past the boundary of the slice if
    // the backing ArrayBuffer shrank.
    let len = len.min(slice.len());

    if ASCENDING {
        if k >= len {
            return None;
        }
        slice[k..len]
            .iter()
            .position(|&r| r == search_element)
            .map(|pos| pos + k)
    } else {
        if k >= len {
            return None;
        }
        slice[..=k].iter().rposition(|&r| r == search_element)
    }
}

fn reverse_typed_array<T: Viewable>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    ta: TypedArray,
    gc: NoGcScope,
) {
    // 3. Let len be TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, gc);
    // 4. Let middle be floor(len / 2).
    // 5. Let lower be 0.
    // 6. Repeat, while lower ≠ middle,
    //    a. Let upper be len - lower - 1.
    //    b. Let upperP be ! ToString(𝔽(upper)).
    //    c. Let lowerP be ! ToString(𝔽(lower)).
    //    d. Let lowerValue be ! Get(O, lowerP).
    //    e. Let upperValue be ! Get(O, upperP).
    //    f. Perform ! Set(O, lowerP, upperValue, true).
    //    g. Perform ! Set(O, upperP, lowerValue, true).
    //    h. Set lower to lower + 1.
    let slice = viewable_slice_mut::<T>(agent, ta, gc);
    let slice = &mut slice[..len];
    slice.reverse();
}

fn copy_within_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    target: Value,
    start: Value,
    end: Option<Value>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let ta_record = ta_record.bind(gc.nogc());
    let o = ta_record.object;
    let scoped_o = o.scope(agent, gc.nogc());
    let target = target.bind(gc.nogc());
    let start = start.bind(gc.nogc());
    let end = end.bind(gc.nogc());
    // 3. Let len be TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc())
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
    let ta = scoped_o.get(agent).bind(gc);
    let end_bound = (end_index - start_index).max(0).min(len - target_index) as usize;
    let ta_record = make_typed_array_with_buffer_witness_record(agent, ta, Ordering::SeqCst, gc);
    if is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Callback is not callable",
            gc,
        ));
    }
    let after_len = typed_array_length::<T>(agent, &ta_record, gc) as usize;
    let slice = viewable_slice_mut::<T>(agent, ta, gc);
    let slice = &mut slice[..after_len];
    let start_bound = start_index as usize;
    let target_index = target_index as usize;
    let before_len = len as usize;
    if before_len != slice.len() {
        let end_bound = (after_len - target_index)
            .max(0)
            .min(before_len - target_index);
        slice.copy_within(start_bound..end_bound, target_index);
        return Ok(ta);
    }
    if end_bound > 0 {
        slice.copy_within(start_bound..start_bound + end_bound, target_index);
    }
    Ok(ta)
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
    let value = T::from_ne_value(agent, value);
    let slice = viewable_slice_mut::<T>(agent, ta, gc);
    if k >= end_index {
        return Ok(ta);
    }
    let slice = &mut slice[k..end_index];
    slice.fill(value);
    // 20. Return O.
    Ok(ta)
}

fn sort_total_cmp_typed_array<'a, T: Viewable + Ord>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords<'a>,
    gc: NoGcScope<'a, '_>,
) {
    let ta = ta_record.object;
    let len = typed_array_length::<T>(agent, &ta_record, gc);
    let slice = viewable_slice_mut::<T>(agent, ta, gc);
    let slice = &mut slice[..len];
    slice.sort();
}

fn to_sorted_total_cmp_typed_array<'a, T: Viewable + Ord>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords<'a>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let ta_record = ta_record.bind(gc.nogc());
    let ta = ta_record.object;
    let scoped_ta = ta.scope(agent, gc.nogc());
    // 4. Let len be TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc()) as i64;
    // 5. Let A be ? TypedArrayCreateSameType(O, « 𝔽(len) »).
    let a = typed_array_create_same_type(agent, scoped_ta.get(agent), len, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    let (a_slice, o_slice) =
        split_typed_array_views::<T>(agent, a, scoped_ta.get(agent), gc.nogc());
    let len = len as usize;
    let a_slice = &mut a_slice[..len];
    let o_slice = &o_slice[..len];
    // 9. Let j be 0.
    // 10. Repeat, while j < len,
    //     a. Perform ! Set(A, ! ToString(𝔽(j)), sortedList[j], true).
    //     b. Set j to j + 1.
    a_slice.copy_from_slice(o_slice);
    // 6. NOTE: The following closure performs a numeric comparison rather than
    //    the string comparison used in 23.1.3.34.
    // 7. Let SortCompare be a new Abstract Closure with parameters (x, y) that
    //    captures comparator and performs the following steps when called:
    //    a. Return ? CompareTypedArrayElements(x, y, comparator).
    // 8. Let sortedList be ? SortIndexedProperties(O, len, SortCompare,
    //    read-through-holes).
    a_slice.sort();
    // 11. Return A.
    Ok(a.unbind())
}

pub trait ECMAScriptOrd {
    fn ecmascript_cmp(&self, other: &Self) -> std::cmp::Ordering;
}

#[cfg(feature = "proposal-float16array")]
impl ECMAScriptOrd for f16 {
    fn ecmascript_cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.is_nan() {
            if other.is_nan() {
                return std::cmp::Ordering::Equal;
            }
            return std::cmp::Ordering::Greater;
        }
        if other.is_nan() {
            return std::cmp::Ordering::Less;
        }
        if *self == 0.0 && *other == 0.0 {
            if self.is_sign_negative() && other.is_sign_positive() {
                return std::cmp::Ordering::Less;
            }
            if self.is_sign_positive() && other.is_sign_negative() {
                return std::cmp::Ordering::Greater;
            }
            return std::cmp::Ordering::Equal;
        }
        self.partial_cmp(other).unwrap()
    }
}

impl ECMAScriptOrd for f32 {
    fn ecmascript_cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.is_nan() {
            if other.is_nan() {
                return std::cmp::Ordering::Equal;
            }
            return std::cmp::Ordering::Greater;
        }
        if other.is_nan() {
            return std::cmp::Ordering::Less;
        }
        if *self == 0.0 && *other == 0.0 {
            if self.is_sign_negative() && other.is_sign_positive() {
                return std::cmp::Ordering::Less;
            }
            if self.is_sign_positive() && other.is_sign_negative() {
                return std::cmp::Ordering::Greater;
            }
            return std::cmp::Ordering::Equal;
        }
        self.partial_cmp(other).unwrap()
    }
}

impl ECMAScriptOrd for f64 {
    fn ecmascript_cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.is_nan() {
            if other.is_nan() {
                return std::cmp::Ordering::Equal;
            }
            return std::cmp::Ordering::Greater;
        }
        if other.is_nan() {
            return std::cmp::Ordering::Less;
        }
        if *self == 0.0 && *other == 0.0 {
            if self.is_sign_negative() && other.is_sign_positive() {
                return std::cmp::Ordering::Less;
            }
            if self.is_sign_positive() && other.is_sign_negative() {
                return std::cmp::Ordering::Greater;
            }
            return std::cmp::Ordering::Equal;
        }
        self.partial_cmp(other).unwrap()
    }
}

fn sort_ecmascript_cmp_typed_array<T: Viewable + ECMAScriptOrd>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    gc: NoGcScope,
) {
    let ta = ta_record.object;
    let len = typed_array_length::<T>(agent, &ta_record, gc);
    let slice = viewable_slice_mut::<T>(agent, ta, gc);
    let slice = &mut slice[..len];
    slice.sort_by(|a, b| a.ecmascript_cmp(b));
}

fn to_sorted_ecmascript_cmp_typed_array<'a, T: Viewable + ECMAScriptOrd>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords<'a>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let ta_record = ta_record.bind(gc.nogc());
    let ta = ta_record.object.scope(agent, gc.nogc());
    // 4. Let len be TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc());
    // 5. Let A be ? TypedArrayCreateSameType(O, « 𝔽(len) »).
    let a = typed_array_create_same_type(agent, ta.get(agent), len as i64, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    let (a_slice, o_slice) = split_typed_array_views::<T>(agent, a, ta.get(agent), gc.nogc());
    let a_slice = &mut a_slice[..len];
    let o_slice = &o_slice[..len];
    // 9. Let j be 0.
    // 10. Repeat, while j < len,
    //     a. Perform ! Set(A, ! ToString(𝔽(j)), sortedList[j], true).
    //     b. Set j to j + 1.
    a_slice.copy_from_slice(o_slice);
    // 6. NOTE: The following closure performs a numeric comparison rather than
    //    the string comparison used in 23.1.3.34.
    // 7. Let SortCompare be a new Abstract Closure with parameters (x, y) that
    //    captures comparator and performs the following steps when called:
    //    a. Return ? CompareTypedArrayElements(x, y, comparator).
    // 8. Let sortedList be ? SortIndexedProperties(O, len, SortCompare,
    //    read-through-holes).
    a_slice.sort_by(|a, b| a.ecmascript_cmp(b));
    // 11. Return A.
    Ok(a.unbind())
}

fn sort_comparator_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords<'a>,
    ta: Scoped<TypedArray>,
    comparator: Scoped<Function>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let ta_record = ta_record.bind(gc.nogc());
    let local_ta = ta_record.object;
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc());
    let slice = viewable_slice::<T>(agent, local_ta, gc.nogc());
    let slice = &slice[..len];
    let mut items: Vec<T> = slice.to_vec();
    let mut error: Option<JsError> = None;
    items.sort_by(|a, b| {
        if error.is_some() {
            return std::cmp::Ordering::Equal;
        }
        let a_val = a.into_ne_value(agent, gc.nogc()).into_value();
        let b_val = b.into_ne_value(agent, gc.nogc()).into_value();
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
        .and_then(|v| v.to_number(agent, gc.reborrow()));
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
    let slice = viewable_slice_mut::<T>(agent, ta.get(agent), gc.nogc());
    let len = len.min(slice.len());
    let slice = &mut slice[..len];
    let copy_len = items.len().min(len);
    slice[..copy_len].copy_from_slice(&items[..copy_len]);
    Ok(())
}

fn to_sorted_comparator_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords<'a>,
    comparator: Scoped<Function>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let ta_record = ta_record.bind(gc.nogc());
    let o = ta_record.object;
    // 4. Let len be TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc());
    let scoped_o = o.scope(agent, gc.nogc());
    // 5. Let A be ? TypedArrayCreateSameType(O, « 𝔽(len) »).
    let a = typed_array_create_same_type(agent, o.unbind(), len as i64, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    let scoped_a = a.scope(agent, gc.nogc());
    let (a_slice, o_slice) = split_typed_array_views::<T>(agent, a, scoped_o.get(agent), gc.nogc());
    let a_slice = &mut a_slice[..len];
    let from_slice = &o_slice[..len];
    a_slice.copy_from_slice(from_slice);
    let mut items: Vec<T> = a_slice.to_vec();
    let mut error: Option<JsError> = None;
    // 6. NOTE: The following closure performs a numeric comparison rather than
    //    the string comparison used in 23.1.3.34.
    // 7. Let SortCompare be a new Abstract Closure with parameters (x, y) that
    //    captures comparator and performs the following steps when called:
    //    a. Return ? CompareTypedArrayElements(x, y, comparator).
    // 8. Let sortedList be ? SortIndexedProperties(O, len, SortCompare,
    //    read-through-holes).
    items.sort_by(|a, b| {
        if error.is_some() {
            return std::cmp::Ordering::Equal;
        }
        let a_val = a.into_ne_value(agent, gc.nogc()).into_value();
        let b_val = b.into_ne_value(agent, gc.nogc()).into_value();
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
        .and_then(|v| v.to_number(agent, gc.reborrow()));
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
    let slice = viewable_slice_mut::<T>(agent, scoped_a.get(agent), gc.nogc());
    let len = len.min(slice.len());
    let slice = &mut slice[..len];
    let copy_len = items.len().min(len);
    // 9. Let j be 0.
    // 10. Repeat, while j < len,
    //     a. Perform ! Set(A, ! ToString(𝔽(j)), sortedList[j], true).
    //     b. Set j to j + 1.
    slice[..copy_len].copy_from_slice(&items[..copy_len]);
    // 11. Return A.
    Ok(scoped_a.get(agent))
}

fn filter_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    callback: Function<'_>,
    this_arg: Value,
    ta_record: TypedArrayWithBufferWitnessRecords,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let o = ta_record.object.bind(gc.nogc());
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc()) as i64;
    let callback = callback.bind(gc.nogc());
    let this_arg = this_arg.bind(gc.nogc());
    let o = o.bind(gc.nogc());
    let callback = callback.scope(agent, gc.nogc());
    let this_arg = this_arg.scope(agent, gc.nogc());
    let scoped_o = o.scope(agent, gc.nogc());
    // 5. Let kept be a new empty List.
    // 6. Let captured be 0.
    let mut kept: Vec<T> = Vec::with_capacity(len.try_into().unwrap());
    // 7. Let k be 0.
    // 8. Repeat, while k < len,
    // b. Let kValue be ! Get(O, Pk).
    let byte_offset = scoped_o.get(agent).byte_offset(agent);
    let byte_length = scoped_o.get(agent).byte_length(agent);
    let local_array_buffer = scoped_o
        .get(agent)
        .get_viewed_array_buffer(agent, gc.nogc());
    let array_buffer = local_array_buffer.scope(agent, gc.nogc());
    let properly_aligned = unsafe {
        local_array_buffer
            .as_slice(agent)
            .align_to::<T>()
            .0
            .is_empty()
    };
    if !properly_aligned {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray is not properly aligned",
            gc.into_nogc(),
        ));
    }
    for k in 0..len {
        let byte_slice = array_buffer.get(agent).as_slice(agent);
        let byte_slice = if let Some(byte_length) = byte_length {
            let end_index = byte_offset + byte_length;
            if end_index <= byte_slice.len() {
                &byte_slice[byte_offset..end_index]
            } else {
                &[]
            }
        } else {
            &byte_slice[byte_offset..]
        };
        let (_, slice, _) = unsafe { byte_slice.align_to::<T>() };
        let index: usize = k.try_into().unwrap();
        let value = slice.get(index).copied();
        let k_value = value.map_or(Value::Undefined, |v| {
            v.into_le_value(agent, gc.nogc()).into_value()
        });
        let result = call_function(
            agent,
            callback.get(agent),
            this_arg.get(agent),
            Some(ArgumentsList::from_mut_slice(&mut [
                k_value.unbind(),
                Number::try_from(k).unwrap().into_value(),
                scoped_o.get(agent).into_value(),
            ])),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let selected = to_boolean(agent, result);
        if selected {
            kept.push(value.unwrap_or(T::default()));
        }
    }
    // 9. Let A be ? TypedArraySpeciesCreate(O, « 𝔽(captured) »).
    let captured = kept.len();
    let o = scoped_o.get(agent).bind(gc.nogc());
    let a = typed_array_species_create_with_length::<T>(
        agent,
        o.unbind(),
        captured as i64,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 10. Let n be 0.
    // 11. For each element e of kept, do
    let array_buffer = a.get_viewed_array_buffer(agent, gc.nogc());
    let byte_offset = a.byte_offset(agent);
    let byte_length = a.byte_length(agent);
    let byte_slice = array_buffer.as_mut_slice(agent);
    if byte_slice.is_empty() {
        return Ok(a.unbind());
    }
    let byte_slice = if let Some(byte_length) = byte_length {
        let end_index = byte_offset + byte_length;
        if end_index > byte_slice.len() {
            return Ok(a.unbind());
        }
        &mut byte_slice[byte_offset..end_index]
    } else {
        &mut byte_slice[byte_offset..]
    };
    with_typed_array_viewable!(
        a,
        {
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<V>() {
                copy_between_same_type_typed_arrays::<T>(&kept, byte_slice)
            } else {
                let (head, slice, _) = unsafe { byte_slice.align_to_mut::<V>() };
                if !head.is_empty() {
                    panic!("ArrayBuffer not correctly aligned");
                }
                let len = kept.len().min(slice.len());
                let slice = &mut slice[..len];
                let kept = &kept[..len];
                copy_between_different_type_typed_arrays::<T, V>(kept, slice);
            }
        },
        V
    );

    // 12. Return A.
    Ok(a.unbind())
}

pub(crate) fn copy_between_different_type_typed_arrays<Src: Viewable, Dst: Viewable>(
    src_slice: &[Src],
    dst_slice: &mut [Dst],
) {
    assert_eq!(Src::IS_BIGINT, Dst::IS_BIGINT);
    if Dst::IS_FLOAT {
        for (dst, src) in dst_slice.iter_mut().zip(src_slice.iter()) {
            *dst = Dst::from_f64(src.into_f64());
        }
    } else if !Dst::IS_FLOAT {
        for (dst, src) in dst_slice.iter_mut().zip(src_slice.iter()) {
            *dst = Dst::from_bits(src.into_bits());
        }
    }
}

pub(crate) fn copy_between_same_type_typed_arrays<T: Viewable>(kept: &[T], byte_slice: &mut [u8]) {
    let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
    if !head.is_empty() {
        panic!("ArrayBuffer not correctly aligned");
    }
    let len = kept.len().min(slice.len());
    let slice = &mut slice[..len];
    let kept = &kept[..len];
    slice.copy_from_slice(kept);
}

pub(crate) fn split_typed_array_views<'a, T: Viewable>(
    agent: &'a mut Agent,
    a: TypedArray<'a>,
    o: TypedArray<'a>,
    gc: NoGcScope<'a, '_>,
) -> (&'a mut [T], &'a [T]) {
    let a_buf = a.get_viewed_array_buffer(agent, gc);
    let o_buf = o.get_viewed_array_buffer(agent, gc);
    let a_buf = a_buf.as_slice(agent);
    let o_buf = o_buf.as_slice(agent);
    if !a_buf.is_empty() || !o_buf.is_empty() {
        assert!(
            !std::ptr::eq(a_buf.as_ptr(), o_buf.as_ptr()),
            "Must not point to the same buffer"
        );
    }
    let a_slice = viewable_slice_mut::<T>(agent, a, gc);
    let a_ptr = a_slice.as_mut_ptr();
    let a_len = a_slice.len();
    let o_aligned = viewable_slice::<T>(agent, o, gc);
    // SAFETY: Confirmed beforehand that the two ArrayBuffers are in separate memory regions.
    let a_aligned = unsafe { std::slice::from_raw_parts_mut(a_ptr, a_len) };
    (a_aligned, o_aligned)
}

pub(crate) fn split_typed_array_buffers<'a, T: Viewable>(
    agent: &'a mut Agent,
    target: ArrayBuffer,
    target_byte_offset: usize,
    source: ArrayBuffer,
    source_byte_offset: usize,
    target_byte_limit: usize,
) -> (&'a mut [T], &'a [T]) {
    let a_buf = target.as_slice(agent);
    let o_buf = source.as_slice(agent);
    if !a_buf.is_empty() || !o_buf.is_empty() {
        assert!(
            !std::ptr::eq(a_buf.as_ptr(), o_buf.as_ptr()),
            "Must not point to the same buffer"
        );
    }
    let target_slice = target.as_mut_slice(agent);
    let target_slice =
        byte_slice_to_viewable_mut::<T>(target_slice, target_byte_offset, target_byte_limit);
    let target_byte_length = core::mem::size_of_val(target_slice);
    let target_ptr = target_slice.as_mut_ptr();
    let target_len = target_slice.len();
    let source_slice = source.as_mut_slice(agent);
    let source_slice = byte_slice_to_viewable_mut::<T>(
        source_slice,
        source_byte_offset,
        source_byte_offset + target_byte_length,
    );
    // SAFETY: Confirmed beforehand that the two ArrayBuffers are in separate memory regions.
    let target_slice = unsafe { std::slice::from_raw_parts_mut(target_ptr, target_len) };
    (target_slice, source_slice)
}

fn with_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    index: Value,
    value: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let ta_record = ta_record.bind(gc.nogc());
    let index = index.bind(gc.nogc());
    let value = value.bind(gc.nogc());
    let o = ta_record.object;
    let scoped_o = o.scope(agent, gc.nogc());
    let scoped_value = value.scope(agent, gc.nogc());
    // 3. Let len be TypedArrayLength(taRecord).
    let len = typed_array_length::<T>(agent, &ta_record, gc.nogc()) as i64;
    // 4. Let relativeIndex be ? ToIntegerOrInfinity(index).
    // 5. If relativeIndex ≥ 0, let actualIndex be relativeIndex.
    let relative_index = if let Value::Integer(index) = index {
        index.into_i64()
    } else {
        to_integer_or_infinity(agent, index.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_i64()
    };
    // 7. If O.[[ContentType]] is BIGINT, let numericValue be ? ToBigInt(value).
    let numeric_value = if T::IS_BIGINT {
        to_big_int(agent, scoped_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_numeric()
    } else {
        // 8. Else, let numericValue be ? ToNumber(value).
        to_number(agent, scoped_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into_numeric()
    };
    let numeric_value = T::from_ne_value(agent, numeric_value);
    // 5. If relativeIndex ≥ 0, let actualIndex be relativeIndex.
    let actual_index = if relative_index >= 0 {
        relative_index
    } else {
        // 6. Else, let actualIndex be len + relativeIndex.
        len + relative_index
    };
    // 9. If IsValidIntegerIndex(O, 𝔽(actualIndex)) is false, throw a RangeError exception.
    if is_valid_integer_index::<T>(agent, scoped_o.get(agent), actual_index, gc.nogc()).is_none() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Index out of bounds",
            gc.into_nogc(),
        ));
    }
    // 10. Let A be ? TypedArrayCreateSameType(O, « 𝔽(len) »).
    let a = typed_array_create_same_type(agent, scoped_o.get(agent), len, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    // 11. Let k be 0.
    // 12. Repeat, while k < len
    //  a. Let Pk be ! ToString(𝔽(k)).
    //  b. If k = actualIndex, let fromValue be numericValue.
    //  c. Else, let fromValue be ! Get(O, Pk).
    //  d. Perform ! Set(A, Pk, fromValue, true).
    //  e. Set k to k + 1.
    let (a_slice, o_slice) = split_typed_array_views::<T>(agent, a, scoped_o.get(agent), gc.nogc());
    let len = len as usize;
    let a_slice = &mut a_slice[..len];
    let from_slice = &o_slice[..len];
    a_slice.copy_from_slice(from_slice);
    if !from_slice.is_empty() && o_slice.len() == len {
        a_slice[actual_index as usize] = numeric_value;
    }
    // 13. Return A.
    Ok(a.unbind())
}

fn subarray_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    src_record: TypedArrayWithBufferWitnessRecords,
    start: Value,
    end: Value,
    buffer: ArrayBuffer<'_>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let src_record = src_record.bind(gc.nogc());
    let start = start.bind(gc.nogc());
    let end = end.bind(gc.nogc());
    let scoped_o = src_record.object.scope(agent, gc.nogc());
    let start = start.scope(agent, gc.nogc());
    let end = end.scope(agent, gc.nogc());
    // 6. If IsTypedArrayOutOfBounds(srcRecord) is true, then
    let src_length = if is_typed_array_out_of_bounds::<T>(agent, &src_record, gc.nogc()) {
        // a. Let srcLength be 0.
        0
    } else {
        // 7. Else,
        //  a. Let srcLength be TypedArrayLength(srcRecord).
        typed_array_length::<T>(agent, &src_record, gc.nogc())
    } as i64;
    // 8. Let relativeStart be ? ToIntegerOrInfinity(start).
    let relative_start = to_integer_or_infinity(agent, start.get(agent), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    // 9. If relativeStart = -∞, let startIndex be 0.
    let start_index = if relative_start.is_neg_infinity() {
        0
    } else if relative_start.is_negative() {
        // 10. Else if relativeStart < 0, let startIndex be max(srcLength + relativeStart, 0).
        (src_length + relative_start.into_i64()).max(0)
    } else {
        // 11. Else, let startIndex be min(relativeStart, srcLength).
        relative_start.into_i64().min(src_length)
    };
    // 12. Let elementSize be TypedArrayElementSize(O).
    let element_size = core::mem::size_of::<T>() as i64;
    // 13. Let srcByteOffset be O.[[ByteOffset]].
    let src_byte_offset = scoped_o.get(agent).byte_offset(agent) as i64;
    // 14. Let beginByteOffset be srcByteOffset + (startIndex × elementSize).
    let begin_byte_offset = src_byte_offset + (start_index * element_size);
    // 15. If O.[[ArrayLength]] is auto and end is undefined, then
    let (array_buffer, byte_offset, length) = if scoped_o.get(agent).array_length(agent).is_none()
        && end.get(agent).is_undefined()
    {
        // a. Let argumentsList be « buffer, 𝔽(beginByteOffset) ».
        (buffer, begin_byte_offset, None)
    } else {
        // 16. Else,
        // a. If end is undefined, let relativeEnd be srcLength; else let relativeEnd be ? ToIntegerOrInfinity(end).
        let end_index = if end.get(agent).is_undefined() {
            src_length
        } else {
            let integer_or_infinity = to_integer_or_infinity(agent, end.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            if integer_or_infinity.is_neg_infinity() {
                // b. If relativeEnd = -∞, let endIndex be 0.
                0
            } else if integer_or_infinity.is_negative() {
                // c. Else if relativeEnd < 0, let endIndex be max(srcLength + relativeEnd, 0).
                (src_length + integer_or_infinity.into_i64()).max(0)
            } else {
                // d. Else, let endIndex be min(relativeEnd, srcLength).
                integer_or_infinity.into_i64().min(src_length)
            }
        };
        // e. Let newLength be max(endIndex - startIndex, 0).
        let new_length = (end_index - start_index).max(0);
        // f. Let argumentsList be « buffer, 𝔽(beginByteOffset), 𝔽(newLength) ».
        (buffer, begin_byte_offset, Some(new_length))
    };
    // 17. Return ? TypedArraySpeciesCreate(O, argumentsList).
    typed_array_species_create_with_buffer::<T>(
        agent,
        scoped_o.get(agent),
        array_buffer,
        byte_offset,
        length,
        gc,
    )
}

fn set_typed_array<'a, T: Viewable + std::fmt::Debug>(
    agent: &mut Agent,
    o: TypedArray,
    source: Value,
    offset: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let o = o.bind(gc.nogc());
    let source = source.bind(gc.nogc());
    let offset = offset.bind(gc.nogc());
    let scoped_o = o.scope(agent, gc.nogc());
    let scoped_source = source.scope(agent, gc.nogc());
    // 3. Assert: target has a [[ViewedArrayBuffer]] internal slot.
    // 4. Let targetOffset be ? ToIntegerOrInfinity(offset).
    let target_offset = to_integer_or_infinity(agent, offset.unbind(), gc.reborrow()).unbind()?;
    // 5. If targetOffset < 0, throw a RangeError exception.
    if target_offset.is_negative() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "invalid array length",
            gc.into_nogc(),
        ));
    }
    // 6. If source is an Object that has a [[TypedArrayName]] internal slot, then
    if let Ok(source) = TypedArray::try_from(scoped_source.get(agent)).bind(gc.nogc()) {
        // a. Perform ? SetTypedArrayFromTypedArray(target, targetOffset, source).
        with_typed_array_viewable!(
            source,
            set_typed_array_from_typed_array::<T, V>(
                agent,
                // SAFETY: not shared.
                unsafe { scoped_o.take(agent) },
                target_offset,
                source.unbind(),
                gc.into_nogc()
            ),
            V
        )?;
    } else {
        // 7. Else,
        //  a. Perform ? SetTypedArrayFromArrayLike(target, targetOffset, source).
        with_typed_array_viewable!(
            scoped_o.get(agent),
            set_typed_array_from_array_like::<T>(
                agent,
                scoped_o,
                target_offset,
                scoped_source,
                gc
            )?
        )
    }
    Ok(())
}

fn slice_typed_array<'a, SrcType: Viewable + std::fmt::Debug>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    start: Value,
    end: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let ta_record = ta_record.bind(gc.nogc());
    let start = start.bind(gc.nogc());
    let end = end.bind(gc.nogc());
    let o = ta_record.object;
    let o = o.scope(agent, gc.nogc());
    let end = end.scope(agent, gc.nogc());
    // 3. Let srcArrayLength be TypedArrayLength(taRecord).
    let src_array_length = typed_array_length::<SrcType>(agent, &ta_record, gc.nogc()) as i64;
    // 4. Let relativeStart be ? ToIntegerOrInfinity(start).
    let relative_start = to_integer_or_infinity(agent, start.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    // 5. If relativeStart = -∞, let startIndex be 0.
    let start_index = if relative_start.is_neg_infinity() {
        0
    } else if relative_start.is_negative() {
        // 6. Else if relativeStart < 0, let startIndex be max(srcArrayLength + relativeStart, 0).
        (src_array_length + relative_start.into_i64()).max(0)
    } else {
        // 7. Else, let startIndex be min(relativeStart, srcArrayLength).
        relative_start.into_i64().min(src_array_length)
    };
    // 8. If end is undefined, let relativeEnd be srcArrayLength; else let relativeEnd be ? ToIntegerOrInfinity(end).
    // SAFETY: end is not shared.
    let end = unsafe { end.take(agent) }.bind(gc.nogc());
    let end_index = if end.is_undefined() {
        src_array_length
    } else {
        let integer_or_infinity =
            to_integer_or_infinity(agent, end.unbind(), gc.reborrow()).unbind()?;
        if integer_or_infinity.is_neg_infinity() {
            // 9. If relativeEnd = -∞, let endIndex be 0.
            0
        } else if integer_or_infinity.is_negative() {
            // 10. Else if relativeEnd < 0, let endIndex be max(srcArrayLength + relativeEnd, 0).
            (src_array_length + integer_or_infinity.into_i64()).max(0)
        } else {
            // 11. Else, let endIndex be min(relativeEnd, srcArrayLength).
            integer_or_infinity.into_i64().min(src_array_length)
        }
    };
    // 12. Let countBytes be max(endIndex - startIndex, 0).
    let count_bytes = (end_index - start_index).max(0) as usize;
    // 13. Let A be ? TypedArraySpeciesCreate(O, « 𝔽(countBytes) »).
    let a = typed_array_species_create_with_length::<SrcType>(
        agent,
        o.get(agent),
        count_bytes as i64,
        gc.reborrow(),
    )
    .unbind()?;
    let gc = gc.into_nogc();
    let a = a.bind(gc);
    // 14. If countBytes > 0, then
    if count_bytes == 0 {
        // 15. Return A.
        return Ok(a);
    };
    // SAFETY: o is not shared.
    let o = unsafe { o.take(agent) }.bind(gc);
    // a. Set taRecord to MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
    let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc);
    // b. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
    if is_typed_array_out_of_bounds::<SrcType>(agent, &ta_record, gc) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc,
        ));
    };
    // c. Set endIndex to min(endIndex, TypedArrayLength(taRecord)).
    let end_index =
        end_index.min(typed_array_length::<SrcType>(agent, &ta_record, gc) as i64) as usize;
    with_typed_array_viewable!(
        a,
        {
            let start_index = start_index as usize;
            // d. Set countBytes to max(endIndex - startIndex, 0).
            // e. Let srcType be TypedArrayElementType(O).
            // f. Let targetType be TypedArrayElementType(A).
            // g. If srcType is targetType, then
            if core::any::TypeId::of::<SrcType>() == core::any::TypeId::of::<TargetType>() {
                // i. NOTE: The transfer must be performed in a manner that
                //    preserves the bit-level encoding of the source data.
                // ii. Let srcBuffer be O.[[ViewedArrayBuffer]].
                // iii. Let targetBuffer be A.[[ViewedArrayBuffer]].
                // iv. Let elementSize be TypedArrayElementSize(O).
                // v. Let srcByteOffset be O.[[ByteOffset]].
                // vi. Let srcByteIndex be (startIndex × elementSize) + srcByteOffset.
                // vii. Let targetByteIndex be A.[[ByteOffset]].
                // viii. Let endByteIndex be targetByteIndex + (countBytes × elementSize).
                // ix. Repeat, while targetByteIndex < endByteIndex,
                //  1. Let value be GetValueFromBuffer(srcBuffer, srcByteIndex, uint8, true, unordered).
                //  2. Perform SetValueInBuffer(targetBuffer, targetByteIndex, uint8, value, true, unordered).
                //  3. Set srcByteIndex to srcByteIndex + 1.
                //  4. Set targetByteIndex to targetByteIndex + 1.
                let a_buf = a.get_viewed_array_buffer(agent, gc);
                let o_buf = o.get_viewed_array_buffer(agent, gc);
                if a_buf == o_buf {
                    slice_typed_array_same_buffer_same_type::<SrcType>(
                        agent,
                        a,
                        o,
                        start_index,
                        count_bytes,
                        gc,
                    );
                } else {
                    let (a_slice, o_slice) = split_typed_array_views::<SrcType>(agent, a, o, gc);
                    let end_index = end_index.min(o_slice.len());
                    let src_slice = &o_slice[start_index..end_index];
                    let dst_slice = &mut a_slice[..src_slice.len()];
                    dst_slice.copy_from_slice(src_slice);
                }
            } else {
                // h. Else,
                // i. Let n be 0.
                // ii. Let k be startIndex.
                // iii. Repeat, while k < endIndex,
                //      1. Let Pk be ! ToString(𝔽(k)).
                //      2. Let kValue be ! Get(O, Pk).
                //      3. Perform ! Set(A, ! ToString(𝔽(n)), kValue, true).
                //      4. Set k to k + 1.
                //      5. Set n to n + 1.
                let a_buf = a.get_viewed_array_buffer(agent, gc);
                let o_buf = o.get_viewed_array_buffer(agent, gc);
                if a_buf == o_buf {
                    slice_typed_array_same_buffer_different_type(
                        agent,
                        a,
                        o,
                        start_index,
                        end_index,
                        gc,
                    );
                } else {
                    let a_slice = viewable_slice_mut::<TargetType>(agent, a, gc);
                    let a_ptr = a_slice.as_mut_ptr();
                    let a_len = a_slice.len();
                    let o_aligned = viewable_slice::<SrcType>(agent, o, gc);
                    // SAFETY: Confirmed beforehand that the two ArrayBuffers are in separate memory regions.
                    let a_aligned = unsafe { std::slice::from_raw_parts_mut(a_ptr, a_len) };
                    let src_slice = &o_aligned[start_index..end_index];
                    let dst_slice = &mut a_aligned[..src_slice.len()];
                    copy_between_different_type_typed_arrays::<SrcType, TargetType>(
                        src_slice, dst_slice,
                    )
                }
            }
        },
        TargetType
    );
    Ok(a.unbind())
}

fn slice_typed_array_same_buffer_same_type<T: Viewable>(
    agent: &mut Agent,
    a: TypedArray,
    o: TypedArray,
    start_index: usize,
    count_bytes: usize,
    gc: NoGcScope,
) {
    // i. NOTE: The transfer must be performed in a manner that
    //    preserves the bit-level encoding of the source data.
    // ii. Let srcBuffer be O.[[ViewedArrayBuffer]].
    // iii. Let targetBuffer be A.[[ViewedArrayBuffer]].
    let buffer = a.get_viewed_array_buffer(agent, gc);
    // iv. Let elementSize be TypedArrayElementSize(O).
    let element_size = core::mem::size_of::<T>();
    // v. Let srcByteOffset be O.[[ByteOffset]].
    let src_byte_offset = o.byte_offset(agent);
    // vi. Let srcByteIndex be (startIndex × elementSize) + srcByteOffset.
    let mut src_byte_index = (start_index * element_size) + src_byte_offset;
    // vii. Let targetByteIndex be A.[[ByteOffset]].
    let mut target_byte_index = a.byte_offset(agent);
    // viii. Let endByteIndex be targetByteIndex + (countBytes × elementSize).
    let end_byte_index = target_byte_index + (count_bytes * element_size);
    // ix. Repeat, while targetByteIndex < endByteIndex,
    while target_byte_index < end_byte_index {
        //  1. Let value be GetValueFromBuffer(srcBuffer, srcByteIndex, uint8, true, unordered).
        let value = get_value_from_buffer::<u8>(
            agent,
            buffer,
            src_byte_index,
            true,
            Ordering::Unordered,
            None,
            gc,
        );
        //  2. Perform SetValueInBuffer(targetBuffer, targetByteIndex, uint8, value, true, unordered).
        set_value_in_buffer::<u8>(
            agent,
            buffer,
            target_byte_index,
            value,
            true,
            Ordering::Unordered,
            None,
        );
        //  3. Set srcByteIndex to srcByteIndex + 1.
        src_byte_index += 1;
        //  4. Set targetByteIndex to targetByteIndex + 1.
        target_byte_index += 1;
    }
}

fn slice_typed_array_same_buffer_different_type(
    agent: &mut Agent,
    a: TypedArray,
    o: TypedArray,
    start_index: usize,
    end_index: usize,
    gc: NoGcScope,
) {
    // i. Let n be 0.
    let mut n: usize = 0;
    // ii. Let k be startIndex.
    let mut k = start_index;
    // iii. Repeat, while k < endIndex,
    while k < end_index {
        // 1. Let Pk be ! ToString(𝔽(k)).
        // 2. Let kValue be ! Get(O, Pk).
        let k_value =
            unwrap_try_get_value(o.try_get(agent, k.try_into().unwrap(), o.into_value(), None, gc));
        // 3. Perform ! Set(A, ! ToString(𝔽(n)), kValue, true).
        debug_assert!(unwrap_try(a.try_set(
            agent,
            n.try_into().unwrap(),
            k_value,
            a.into_value(),
            gc
        )));
        // 4. Set k to k + 1.
        k += 1;
        // 5. Set n to n + 1.
        n += 1;
    }
}
