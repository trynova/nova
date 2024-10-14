// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [7.3 Operations on Objects](https://tc39.es/ecma262/#sec-operations-on-objects)

use ahash::AHashSet;

use super::{
    operations_on_iterator_objects::{get_iterator, if_abrupt_close_iterator, iterator_close},
    testing_and_comparison::{is_callable, require_object_coercible, same_value},
    type_conversion::{to_length, to_object, to_property_key},
};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::iterator_step_value,
        builtins::{
            array_create,
            keyed_collections::map_objects::map_prototype::canonicalize_keyed_collection_key,
            ArgumentsList, Array, BuiltinConstructorFunction,
        },
        execution::{
            agent::ExceptionType, new_class_field_initializer_environment, Agent,
            ECMAScriptCodeEvaluationState, EnvironmentIndex, ExecutionContext, JsResult,
            RealmIdentifier,
        },
        types::{
            Function, InternalMethods, IntoFunction, IntoObject, IntoValue, Number, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    engine::{instanceof_operator, Vm},
    heap::{Heap, ObjectEntry},
    SmallInteger,
};

/// ### [7.3.1 MakeBasicObject ( internalSlotsList )](https://tc39.es/ecma262/#sec-makebasicobject)
///
/// The abstract operation MakeBasicObject takes argument internalSlotsList (a
/// List of internal slot names) and returns an Object. It is the source of all
/// ECMAScript objects that are created algorithmically, including both
/// ordinary objects and exotic objects. It factors out common steps used in
/// creating all objects, and centralizes object creation. It performs the
/// following steps when called:
///
/// > NOTE: Within this specification, exotic objects are created in abstract
/// > operations such as ArrayCreate and BoundFunctionCreate by first calling
/// > MakeBasicObject to obtain a basic, foundational object, and then
/// > overriding some or all of that object's internal methods. In order to
/// > encapsulate exotic object creation, the object's essential internal
/// > methods are never modified outside those operations.
pub(crate) fn make_basic_object(_agent: &mut Agent, _internal_slots_list: ()) -> Object {
    // 1. Let obj be a newly created object with an internal slot for each name in internalSlotsList.
    // 2. Set obj's essential internal methods to the default ordinary object definitions specified in 10.1.
    // 3. Assert: If the caller will not be overriding both obj's [[GetPrototypeOf]] and [[SetPrototypeOf]] essential
    // internal methods, then internalSlotsList contains [[Prototype]].
    // 4. Assert: If the caller will not be overriding all of obj's [[SetPrototypeOf]], [[IsExtensible]], and
    // [[PreventExtensions]] essential internal methods, then internalSlotsList contains [[Extensible]].
    // 5. If internalSlotsList contains [[Extensible]], set obj.[[Extensible]] to true.
    // 6. Return obj.
    todo!()
}

/// ### [7.3.2 Get ( O, P )](https://tc39.es/ecma262/#sec-get-o-p)
///
/// The abstract operation Get takes arguments O (an Object) and P (a property
/// key) and returns either a normal completion containing an ECMAScript
/// language value or a throw completion. It is used to retrieve the value of a
/// specific property of an object.
pub(crate) fn get(agent: &mut Agent, o: impl IntoObject, p: PropertyKey) -> JsResult<Value> {
    // 1. Return ? O.[[Get]](P, O).
    o.into_object().internal_get(agent, p, o.into_value())
}

/// ### [7.3.3 GetV ( V, P )](https://tc39.es/ecma262/#sec-getv)
///
/// The abstract operation GetV takes arguments V (an ECMAScript language
/// value) and P (a property key) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. It is used
/// to retrieve the value of a specific property of an ECMAScript language
/// value. If the value is not an object, the property lookup is performed
/// using a wrapper object appropriate for the type of the value.
pub(crate) fn get_v(agent: &mut Agent, v: Value, p: PropertyKey) -> JsResult<Value> {
    // 1. Let O be ? ToObject(V).
    let o = to_object(agent, v)?;
    // 2. Return ? O.[[Get]](P, V).
    o.internal_get(agent, p, o.into())
}

/// ### [7.3.4 Set ( O, P, V, Throw )](https://tc39.es/ecma262/#sec-set-o-p-v-throw)
///
/// The abstract operation Set takes arguments O (an Object), P (a property
/// key), V (an ECMAScript language value), and Throw (a Boolean) and returns
/// either a normal completion containing UNUSED or a throw completion. It is
/// used to set the value of a specific property of an object. V is the new
/// value for the property.
pub(crate) fn set(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    v: Value,
    throw: bool,
) -> JsResult<()> {
    // 1. Let success be ? O.[[Set]](P, V, O).
    let success = o.internal_set(agent, p, v, o.into_value())?;
    // 2. If success is false and Throw is true, throw a TypeError exception.
    if !success && throw {
        return Err(agent.throw_exception(
            ExceptionType::TypeError,
            format!("Could not set property '{}'.", p.as_display(agent)),
        ));
    }
    // 3. Return UNUSED.
    Ok(())
}

/// ### [7.3.5] CreateDataProperty ( O, P, V )[https://tc39.es/ecma262/#sec-createdataproperty]
///
/// The abstract operation CreateDataProperty takes arguments O (an Object), P
/// (a property key), and V (an ECMAScript language value) and returns either a
/// normal completion containing a Boolean or a throw completion. It is used to
/// create a new own property of an object.
///
/// > NOTE: This abstract operation creates a property whose attributes are set
/// > to the same defaults used for properties created by the ECMAScript language
/// > assignment operator. Normally, the property will not already exist. If it
/// > does exist and is not configurable or if O is not extensible,
/// > [\[DefineOwnProperty]] will return false.
pub(crate) fn create_data_property(
    agent: &mut Agent,
    object: impl InternalMethods,
    property_key: PropertyKey,
    value: Value,
) -> JsResult<bool> {
    // 1. Let newDesc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true }.
    let new_desc = PropertyDescriptor {
        value: Some(value),
        writable: Some(true),
        get: None,
        set: None,
        enumerable: Some(true),
        configurable: Some(true),
    };
    // 2. Return ? O.[[DefineOwnProperty]](P, newDesc).
    object.internal_define_own_property(agent, property_key, new_desc)
}

/// ### [7.3.7 CreateDataPropertyOrThrow ( O, P, V )](https://tc39.es/ecma262/#sec-createdatapropertyorthrow)
///
/// The abstract operation CreateDataPropertyOrThrow takes arguments O (an
/// Object), P (a property key), and V (an ECMAScript language value) and
/// returns either a normal completion containing UNUSED or a throw completion.
/// It is used to create a new own property of an object. It throws a TypeError
/// exception if the requested property update cannot be performed.
pub(crate) fn create_data_property_or_throw(
    agent: &mut Agent,
    object: impl InternalMethods,
    property_key: PropertyKey,
    value: Value,
) -> JsResult<()> {
    let success = create_data_property(agent, object, property_key, value)?;
    if !success {
        Err(agent.throw_exception(
            ExceptionType::TypeError,
            format!(
                "Could not create property '{}'.",
                property_key.as_display(agent)
            ),
        ))
    } else {
        Ok(())
    }
}

/// ### [7.3.8 DefinePropertyOrThrow ( O, P, desc )](https://tc39.es/ecma262/#sec-definepropertyorthrow)
///
/// The abstract operation DefinePropertyOrThrow takes arguments O (an Object),
/// P (a property key), and desc (a Property Descriptor) and returns either a
/// normal completion containing UNUSED or a throw completion. It is used to
/// call the \[\[DefineOwnProperty]] internal method of an object in a manner
/// that will throw a TypeError exception if the requested property update
/// cannot be performed.
pub(crate) fn define_property_or_throw(
    agent: &mut Agent,
    object: impl InternalMethods,
    property_key: PropertyKey,
    desc: PropertyDescriptor,
) -> JsResult<()> {
    // 1. Let success be ? O.[[DefineOwnProperty]](P, desc).
    let success = object.internal_define_own_property(agent, property_key, desc)?;
    // 2. If success is false, throw a TypeError exception.
    if !success {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Failed to defined property on object",
        ))
    } else {
        // 3. Return UNUSED.
        Ok(())
    }
}

/// ### [7.3.9 DeletePropertyOrThrow ( O, P )](https://tc39.es/ecma262/#sec-deletepropertyorthrow)
///
/// The abstract operation DeletePropertyOrThrow takes arguments O (an Object)
/// and P (a property key) and returns either a normal completion containing
/// unused or a throw completion. It is used to remove a specific own property
/// of an object. It throws an exception if the property is not configurable.
pub(crate) fn delete_property_or_throw(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
) -> JsResult<()> {
    // 1. Let success be ? O.[[Delete]](P).
    let success = o.internal_delete(agent, p)?;
    // 2. If success is false, throw a TypeError exception.
    if !success {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Failed to delete property",
        ))
    } else {
        // 3. Return unused.
        Ok(())
    }
}

/// ### [7.3.11 GetMethod ( V, P )](https://tc39.es/ecma262/#sec-getmethod)
///
/// The abstract operation GetMethod takes arguments V (an ECMAScript language
/// value) and P (a property key) and returns either a normal completion
/// containing either a function object or undefined, or a throw completion. It
/// is used to get the value of a specific property of an ECMAScript language
/// value when the value of the property is expected to be a function.

pub(crate) fn get_method(
    agent: &mut Agent,
    v: Value,
    p: PropertyKey,
) -> JsResult<Option<Function>> {
    // 1. Let func be ? GetV(V, P).
    let func = get_v(agent, v, p)?;
    // 2. If func is either undefined or null, return undefined.
    if func.is_undefined() || func.is_null() {
        return Ok(None);
    }
    // 3. If IsCallable(func) is false, throw a TypeError exception.
    let func = is_callable(func);
    if func.is_none() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Not a callable object",
        ));
    }
    // 4. Return func.
    Ok(func)
}

/// ### [7.3.12 HasProperty ( O, P )](https://tc39.es/ecma262/#sec-hasproperty)
///
/// The abstract operation HasProperty takes arguments O (an Object) and P (a
/// property key) and returns either a normal completion containing a Boolean
/// or a throw completion. It is used to determine whether an object has a
/// property with the specified property key. The property may be either own or
/// inherited.
pub(crate) fn has_property(agent: &mut Agent, o: Object, p: PropertyKey) -> JsResult<bool> {
    // 1. Return ? O.[[HasProperty]](P).
    o.internal_has_property(agent, p)
}

/// ### [7.3.13 HasOwnProperty ( O, P )](https://tc39.es/ecma262/#sec-hasownproperty)
///
/// The abstract operation HasOwnProperty takes arguments O (an Object) and P
/// (a property key) and returns either a normal completion containing a
/// Boolean or a throw completion. It is used to determine whether an object
/// has an own property with the specified property key.
pub(crate) fn has_own_property(agent: &mut Agent, o: Object, p: PropertyKey) -> JsResult<bool> {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let desc = o.internal_get_own_property(agent, p)?;
    // 2. If desc is undefined, return false.
    // 3. Return true.
    Ok(desc.is_some())
}

/// ### [7.3.13 Call ( F, V \[ , argumentsList \] )](https://tc39.es/ecma262/#sec-call)
///
/// The abstract operation Call takes arguments F (an ECMAScript language
/// value) and V (an ECMAScript language value) and optional argument
/// argumentsList (a List of ECMAScript language values) and returns either a
/// normal completion containing an ECMAScript language value or a throw
/// completion. It is used to call the [[Call]] internal method of a function
/// object. F is the function object, V is an ECMAScript language value that is
/// the this value of the [[Call]], and argumentsList is the value passed to
/// the corresponding argument of the internal method. If argumentsList is not
/// present, a new empty List is used as its value.
pub(crate) fn call(
    agent: &mut Agent,
    f: Value,
    v: Value,
    arguments_list: Option<ArgumentsList>,
) -> JsResult<Value> {
    // 1. If argumentsList is not present, set argumentsList to a new empty List.
    let arguments_list = arguments_list.unwrap_or_default();
    // 2. If IsCallable(F) is false, throw a TypeError exception.
    match is_callable(f) {
        None => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Not a callable object",
        )),
        // 3. Return ? F.[[Call]](V, argumentsList).
        Some(f) => f.internal_call(agent, v, arguments_list),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrityLevel {
    Sealed,
    Frozen,
}

pub(crate) trait Level {
    const LEVEL: IntegrityLevel;
}

pub(crate) mod integrity {
    use super::{IntegrityLevel, Level};

    pub(crate) struct Sealed {}
    pub(crate) struct Frozen {}

    impl Level for Sealed {
        const LEVEL: IntegrityLevel = IntegrityLevel::Sealed;
    }

    impl Level for Frozen {
        const LEVEL: IntegrityLevel = IntegrityLevel::Frozen;
    }
}

/// ### [7.3.15 SetIntegrityLevel ( O, level )](https://tc39.es/ecma262/#sec-setintegritylevel)
///
/// The abstract operation SetIntegrityLevel takes arguments O (an Object) and
/// level (SEALED or FROZEN) and returns either a normal completion containing
/// a Boolean or a throw completion. It is used to fix the set of own
/// properties of an object.
pub(crate) fn set_integrity_level<T: Level>(agent: &mut Agent, o: Object) -> JsResult<bool> {
    // 1. Let status be ? O.[[PreventExtensions]]().
    let status = o.internal_prevent_extensions(agent)?;
    // 2. If status is false, return false.
    if !status {
        return Ok(false);
    }
    // 3. Let keys be ? O.[[OwnPropertyKeys]]().
    let keys = o.internal_own_property_keys(agent)?;
    // 4. If level is SEALED, then
    if T::LEVEL == IntegrityLevel::Sealed {
        // a. For each element k of keys, do
        for k in keys {
            // i. Perform ? DefinePropertyOrThrow(O, k, PropertyDescriptor { [[Configurable]]: false }).
            define_property_or_throw(
                agent,
                o,
                k,
                PropertyDescriptor {
                    configurable: Some(false),
                    ..Default::default()
                },
            )?;
        }
    } else {
        // 5. Else,
        // a. Assert: level is FROZEN.
        // b. For each element k of keys, do
        for k in keys {
            // i. Let currentDesc be ? O.[[GetOwnProperty]](k).
            let current_desc = o.internal_get_own_property(agent, k)?;
            // ii. If currentDesc is not undefined, then
            if let Some(current_desc) = current_desc {
                // 1. If IsAccessorDescriptor(currentDesc) is true, then
                let desc = if current_desc.is_accessor_descriptor() {
                    // a. Let desc be the PropertyDescriptor { [[Configurable]]: false }.
                    PropertyDescriptor {
                        configurable: Some(false),
                        ..Default::default()
                    }
                } else {
                    // 2. Else,
                    // a. Let desc be the PropertyDescriptor { [[Configurable]]: false, [[Writable]]: false }.
                    PropertyDescriptor {
                        configurable: Some(false),
                        writable: Some(false),
                        ..Default::default()
                    }
                };
                // 3. Perform ? DefinePropertyOrThrow(O, k, desc).
                define_property_or_throw(agent, o, k, desc)?;
            }
        }
    }
    // 6. Return true.
    Ok(true)
}

/// ### [7.3.16 TestIntegrityLevel ( O, level )](https://tc39.es/ecma262/#sec-testintegritylevel)
///
/// The abstract operation TestIntegrityLevel takes arguments O (an Object) and
/// level (SEALED or FROZEN) and returns either a normal completion containing a
/// Boolean or a throw completion. It is used to determine if the set of own
/// properties of an object are fixed.
pub(crate) fn test_integrity_level<T: Level>(agent: &mut Agent, o: Object) -> JsResult<bool> {
    // 1. Let extensible be ? IsExtensible(O).
    // 2. If extensible is true, return false.
    // 3. NOTE: If the object is extensible, none of its properties are examined.
    if o.internal_is_extensible(agent)? {
        return Ok(false);
    }

    // 4. Let keys be ? O.[[OwnPropertyKeys]]().
    let keys = o.internal_own_property_keys(agent)?;
    // 5. For each element k of keys, do
    for k in keys {
        // a. Let currentDesc be ? O.[[GetOwnProperty]](k).
        // b. If currentDesc is not undefined, then
        if let Some(current_desc) = o.internal_get_own_property(agent, k)? {
            // i. If currentDesc.[[Configurable]] is true, return false.
            if current_desc.configurable == Some(true) {
                return Ok(false);
            }
            // ii. If level is frozen and IsDataDescriptor(currentDesc) is true, then
            if T::LEVEL == IntegrityLevel::Frozen && current_desc.is_data_descriptor() {
                // 1. If currentDesc.[[Writable]] is true, return false.
                if current_desc.writable == Some(true) {
                    return Ok(false);
                }
            }
        }
    }

    // 6. Return true.
    Ok(true)
}

/// ### [7.3.17 CreateArrayFromList ( elements )](https://tc39.es/ecma262/#sec-createarrayfromlist)
///
/// The abstract operation CreateArrayFromList takes argument elements (a List
/// of ECMAScript language values) and returns an Array. It is used to create
/// an Array whose elements are provided by elements.
pub(crate) fn create_array_from_list(agent: &mut Agent, elements: &[Value]) -> Array {
    let len = elements.len();
    // 1. Let array be ! ArrayCreate(0).
    let array = array_create(agent, len, len, None).unwrap();
    let array_elements = agent[array].elements;
    agent[array_elements]
        .copy_from_slice(unsafe { std::mem::transmute::<&[Value], &[Option<Value>]>(elements) });
    // 2. Let n be 0.
    // 3. For each element e of elements, do
    // a. Perform ! CreateDataPropertyOrThrow(array, ! ToString(𝔽(n)), e).
    // b. Set n to n + 1.
    // 4. Return array.
    array
}

/// ### [7.3.18 LengthOfArrayLike ( obj )](https://tc39.es/ecma262/#sec-lengthofarraylike)
///
/// The abstract operation LengthOfArrayLike takes argument obj (an Object) and
/// returns either a normal completion containing a non-negative integer or a
/// throw completion. It returns the value of the "length" property of an
/// array-like object.
pub(crate) fn length_of_array_like(agent: &mut Agent, obj: Object) -> JsResult<i64> {
    // NOTE: Fast path for Array objects.
    if let Ok(array) = Array::try_from(obj) {
        return Ok(array.len(agent) as i64);
    }

    // 1. Return ℝ(? ToLength(? Get(obj, "length"))).
    let property = get(agent, obj, PropertyKey::from(BUILTIN_STRING_MEMORY.length))?;
    to_length(agent, property)
}

/// ### [7.3.19 CreateListFromArrayLike ( obj [ , elementTypes ] )](https://tc39.es/ecma262/#sec-createlistfromarraylike)
///
/// The abstract operation CreateListFromArrayLike takes argument obj (an ECMAScript language value)
/// and optional argument elementTypes (a List of names of ECMAScript Language Types) and returns
/// either a normal completion containing a List of ECMAScript language values or a throw
/// completion. It is used to create a List value whose elements are provided by the indexed
/// properties of obj. elementTypes contains the names of ECMAScript Language Types that are allowed
/// for element values of the List that is created.
///
/// NOTE: This implementation doesn't yet support `elementTypes`.
pub(crate) fn create_list_from_array_like(agent: &mut Agent, obj: Value) -> JsResult<Vec<Value>> {
    match obj {
        Value::Array(array) => Ok(array
            .as_slice(agent)
            .iter()
            .map(|el| el.unwrap_or(Value::Undefined))
            .collect()),
        // TODO: TypedArrays
        _ if obj.is_object() => {
            let object = Object::try_from(obj).unwrap();
            // 3. Let len be ? LengthOfArrayLike(obj).
            let len = length_of_array_like(agent, object)?;
            let len = usize::try_from(len).unwrap();
            // 4. Let list be a new empty list.
            let mut list = Vec::with_capacity(len);
            // 5. Let index be 0.
            // 6. Repeat, while index < len,
            for i in 0..len {
                // a. Let indexName be ! ToString(𝔽(index)).
                // b. Let next be ? Get(obj, indexName).
                let next = get(
                    agent,
                    object,
                    PropertyKey::Integer(SmallInteger::try_from(i as u64).unwrap()),
                )?;
                // d. Append next to list.
                list.push(next);
                // e. Set index to index + 1.
            }
            // 7. Return list.
            Ok(list)
        }
        // 2. If obj is not an Object, throw a TypeError exception.
        _ => {
            Err(agent
                .throw_exception_with_static_message(ExceptionType::TypeError, "Not an object"))
        }
    }
}

/// Abstract operation Call specialized for a Function.
pub(crate) fn call_function(
    agent: &mut Agent,
    f: Function,
    v: Value,
    arguments_list: Option<ArgumentsList>,
) -> JsResult<Value> {
    let arguments_list = arguments_list.unwrap_or_default();
    f.internal_call(agent, v, arguments_list)
}

pub(crate) fn construct(
    agent: &mut Agent,
    f: Function,
    arguments_list: Option<ArgumentsList>,
    new_target: Option<Function>,
) -> JsResult<Object> {
    // 1. If newTarget is not present, set newTarget to F.
    let new_target = new_target.unwrap_or(f);
    // 2. If argumentsList is not present, set argumentsList to a new empty List.
    let arguments_list = arguments_list.unwrap_or_default();
    f.internal_construct(agent, arguments_list, new_target)
}

/// ### [7.3.20 Invoke ( V, P \[ , argumentsList \] )]()
///
/// The abstract operation Invoke takes arguments V (an ECMAScript language
/// value) and P (a property key) and optional argument argumentsList (a List
/// of ECMAScript language values) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. It is used
/// to call a method property of an ECMAScript language value. V serves as both
/// the lookup point for the property and the this value of the call.
/// argumentsList is the list of arguments values passed to the method. If
/// argumentsList is not present, a new empty List is used as its value.
pub(crate) fn invoke(
    agent: &mut Agent,
    v: Value,
    p: PropertyKey,
    arguments_list: Option<ArgumentsList>,
) -> JsResult<Value> {
    // 1. If argumentsList is not present, set argumentsList to a new empty List.
    let arguments_list = arguments_list.unwrap_or_default();
    // 2. Let func be ? GetV(V, P).
    let func = get_v(agent, v, p)?;
    // 3. Return ? Call(func, V, argumentsList).
    call(agent, func, v, Some(arguments_list))
}

/// ### [7.3.21 OrdinaryHasInstance ( C, O )](https://tc39.es/ecma262/#sec-ordinaryhasinstance)
///
/// The abstract operation OrdinaryHasInstance takes arguments C (an ECMAScript
/// language value) and O (an ECMAScript language value) and returns either a
/// normal completion containing a Boolean or a throw completion. It implements
/// the default algorithm for determining if O inherits from the instance
/// object inheritance path provided by C.
pub(crate) fn ordinary_has_instance(
    agent: &mut Agent,
    c: impl TryInto<Function>,
    o: impl IntoValue,
) -> JsResult<bool> {
    // 1. If IsCallable(C) is false, return false.
    let Some(c) = is_callable(c) else {
        return Ok(false);
    };
    // 2. If C has a [[BoundTargetFunction]] internal slot, then
    if let Function::BoundFunction(c) = c {
        // a. Let BC be C.[[BoundTargetFunction]].
        let bc = agent[c].bound_target_function;
        // b. Return ? InstanceofOperator(O, BC).
        return instanceof_operator(agent, o, bc);
    }
    // 3. If O is not an Object, return false.
    let Ok(mut o) = Object::try_from(o.into_value()) else {
        return Ok(false);
    };
    // 4. Let P be ? Get(C, "prototype").
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.prototype);
    let p = get(agent, c, key)?;
    // 5. If P is not an Object, throw a TypeError exception.
    let Ok(p) = Object::try_from(p) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Non-object prototype found",
        ));
    };
    // 6. Repeat,
    loop {
        // a. Set O to ? O.[[GetPrototypeOf]]().
        let o_prototype = o.internal_get_prototype_of(agent)?;
        if let Some(o_prototype) = o_prototype {
            o = o_prototype;
        } else {
            // b. If O is null, return false.
            return Ok(false);
        }
        // c. If SameValue(P, O) is true, return true.
        if same_value(agent, p, o) {
            return Ok(true);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EnumPropKind {
    Key,
    Value,
    KeyValue,
}

pub(crate) trait EnumerablePropertiesKind {
    const KIND: EnumPropKind;
}

pub(crate) mod enumerable_properties_kind {
    use super::{EnumPropKind, EnumerablePropertiesKind};

    pub(crate) struct Key;
    pub(crate) struct Value;
    pub(crate) struct KeyValue;

    impl EnumerablePropertiesKind for Key {
        const KIND: EnumPropKind = EnumPropKind::Key;
    }

    impl EnumerablePropertiesKind for Value {
        const KIND: EnumPropKind = EnumPropKind::Value;
    }

    impl EnumerablePropertiesKind for KeyValue {
        const KIND: EnumPropKind = EnumPropKind::KeyValue;
    }
}

/// ### [7.3.23 EnumerableOwnProperties ( O, kind )](https://tc39.es/ecma262/#sec-enumerableownproperties)
///
/// The abstract operation EnumerableOwnProperties takes arguments O (an
/// Object) and kind (KEY, VALUE, or KEY+VALUE) and returns either a normal
/// completion containing a List of ECMAScript language values or a throw
/// completion.
pub(crate) fn enumerable_own_properties<Kind: EnumerablePropertiesKind>(
    agent: &mut Agent,
    o: Object,
) -> JsResult<Vec<Value>> {
    // 1. Let ownKeys be ? O.[[OwnPropertyKeys]]().
    let own_keys = o.internal_own_property_keys(agent)?;
    // 2. Let results be a new empty List.
    let mut results: Vec<Value> = Vec::with_capacity(own_keys.len());
    // 3. For each element key of ownKeys, do
    for key in own_keys {
        if let PropertyKey::Symbol(_) = key {
            continue;
        }
        // i. Let desc be ? O.[[GetOwnProperty]](key).
        let desc = o.internal_get_own_property(agent, key)?;
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        let Some(desc) = desc else {
            continue;
        };
        if desc.enumerable != Some(true) {
            continue;
        }
        // 1. If kind is KEY, then
        if Kind::KIND == EnumPropKind::Key {
            // a. Append key to results.
            let key_value = match key {
                PropertyKey::Symbol(_) => {
                    unreachable!();
                }
                PropertyKey::Integer(int) => {
                    let int = int.into_i64();
                    String::from_string(agent, format!("{}", int))
                }
                PropertyKey::SmallString(str) => str.into(),
                PropertyKey::String(str) => str.into(),
            };
            results.push(key_value.into_value());
        } else {
            // 2. Else,
            // a. Let value be ? Get(O, key).
            let value = get(agent, o, key)?;
            // b. If kind is VALUE, then
            if Kind::KIND == EnumPropKind::Value {
                // i. Append value to results.
                results.push(value);
            } else {
                // c. Else,
                // i. Assert: kind is KEY+VALUE.
                debug_assert_eq!(Kind::KIND, EnumPropKind::KeyValue);
                let key_value = match key {
                    PropertyKey::Symbol(_) => {
                        unreachable!();
                    }
                    PropertyKey::Integer(int) => {
                        let int = int.into_i64();
                        String::from_string(agent, format!("{}", int))
                    }
                    PropertyKey::SmallString(str) => str.into(),
                    PropertyKey::String(str) => str.into(),
                };
                // ii. Let entry be CreateArrayFromList(« key, value »).
                let entry = create_array_from_list(agent, &[key_value.into_value(), value]);
                // iii. Append entry to results.
                results.push(entry.into_value());
            }
        }
    }
    // 4. Return results.
    Ok(results)
}

/// ### [7.3.25 GetFunctionRealm ( obj )](https://tc39.es/ecma262/#sec-getfunctionrealm)
///
/// The abstract operation GetFunctionRealm takes argument obj (a function
/// object) and returns either a normal completion containing a Realm Record or
/// a throw completion.
pub(crate) fn get_function_realm(
    agent: &mut Agent,
    obj: impl IntoObject,
) -> JsResult<RealmIdentifier> {
    // 1. If obj has a [[Realm]] internal slot, then
    // a. Return obj.[[Realm]].
    let obj = obj.into_object();
    match obj {
        Object::BuiltinFunction(idx) => Ok(agent[idx].realm),
        Object::ECMAScriptFunction(idx) => Ok(agent[idx].ecmascript_function.realm),
        Object::BoundFunction(idx) => {
            // 2. If obj is a bound function exotic object, then
            // a. Let boundTargetFunction be obj.[[BoundTargetFunction]].
            // b. Return ? GetFunctionRealm(boundTargetFunction).
            get_function_realm(agent, agent[idx].bound_target_function)
        }
        // 3. If obj is a Proxy exotic object, then
        // a. Perform ? ValidateNonRevokedProxy(obj).
        // b. Let proxyTarget be obj.[[ProxyTarget]].
        // c. Return ? GetFunctionRealm(proxyTarget).
        // Object::Proxy(idx) => {},
        // 4. Return the current Realm Record.
        // NOTE: Step 4 will only be reached if obj is a non-standard function
        // exotic object that does not have a [[Realm]] internal slot.
        _ => Ok(agent.current_realm_id()),
    }
}

/// ### [7.3.25 CopyDataProperties ( target, source, excludedItems )](https://tc39.es/ecma262/#sec-copydataproperties)
/// The abstract operation CopyDataProperties takes arguments target (an Object), source (an
/// ECMAScript language value), and excludedItems (a List of property keys) and returns either a
/// normal completion containing unused or a throw completion.
///
/// NOTE: This implementation of CopyDataProperties takes an existing target object and populates
/// it, but it does not support excluded items. It can be used to implement the spread operator in
/// object literals, but not the rest operator in object destructuring.
pub(crate) fn copy_data_properties(
    agent: &mut Agent,
    target: OrdinaryObject,
    source: Value,
) -> JsResult<()> {
    // 1. If source is either undefined or null, return unused.
    if source.is_undefined() || source.is_null() {
        return Ok(());
    }
    // 2. Let from be ! ToObject(source).
    let from = to_object(agent, source).unwrap();

    // 3. Let keys be ? from.[[OwnPropertyKeys]]().
    let keys = from.internal_own_property_keys(agent)?;
    // Reserve space in the target's vectors.
    {
        let new_size = agent[target]
            .keys
            .len()
            .checked_add(u32::try_from(keys.len()).unwrap())
            .unwrap();
        let Heap {
            elements, objects, ..
        } = &mut agent.heap;
        objects[target].keys.reserve(elements, new_size);
        objects[target].values.reserve(elements, new_size);
    }

    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // i. Let desc be ? from.[[GetOwnProperty]](nextKey).
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        if let Some(dest) = from.internal_get_own_property(agent, next_key)? {
            if dest.enumerable.unwrap() {
                // 1. Let propValue be ? Get(from, nextKey).
                let prop_value = get(agent, from, next_key)?;
                // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue).
                create_data_property(agent, target, next_key, prop_value).unwrap();
            }
        }
    }

    // 5. Return UNUSED.
    Ok(())
}

/// ### [7.3.25 CopyDataProperties ( target, source, excludedItems )](https://tc39.es/ecma262/#sec-copydataproperties)
/// The abstract operation CopyDataProperties takes arguments target (an Object), source (an
/// ECMAScript language value), and excludedItems (a List of property keys) and returns either a
/// normal completion containing unused or a throw completion.
///
/// NOTE: This implementation of CopyDataProperties also creates the target object with
/// `OrdinaryObjectCreate(%Object.prototype%)`. This can be used to implement the rest operator in
/// object destructuring, but not the spread operator in object literals.
pub(crate) fn copy_data_properties_into_object(
    agent: &mut Agent,
    source: impl IntoObject,
    excluded_items: &AHashSet<PropertyKey>,
) -> JsResult<OrdinaryObject> {
    let from = source.into_object();
    let mut entries = Vec::new();

    // 3. Let keys be ? from.[[OwnPropertyKeys]]().
    // 4. For each element nextKey of keys, do
    for next_key in from.internal_own_property_keys(agent)? {
        // a. Let excluded be false.
        // b. For each element e of excludedItems, do
        //   i. If SameValue(e, nextKey) is true, then
        //     1. Set excluded to true.
        if excluded_items.contains(&next_key) {
            continue;
        }

        // c. If excluded is false, then
        //   i. Let desc be ? from.[[GetOwnProperty]](nextKey).
        //   ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        if let Some(dest) = from.internal_get_own_property(agent, next_key)? {
            if dest.enumerable.unwrap() {
                // 1. Let propValue be ? Get(from, nextKey).
                let prop_value = get(agent, from, next_key)?;
                // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue).
                entries.push(ObjectEntry::new_data_entry(next_key, prop_value));
            }
        }
    }

    Ok(agent.heap.create_object_with_prototype(
        agent
            .current_realm()
            .intrinsics()
            .object_prototype()
            .into_object(),
        &entries,
    ))
}

/// [7.3.33 InitializeInstanceElements ( O, constructor )](https://tc39.es/ecma262/#sec-initializeinstanceelements)
///
/// The abstract operation InitializeInstanceElements takes arguments O (an
/// Object) and constructor (an ECMAScript function object) and returns either
/// a normal completion containing unused or a throw completion.
pub(crate) fn initialize_instance_elements(
    agent: &mut Agent,
    o: Object,
    constructor: BuiltinConstructorFunction,
) -> JsResult<()> {
    // 1. Let methods be the value of constructor.[[PrivateMethods]].
    // 2. For each PrivateElement method of methods, do
    // a. Perform ? PrivateMethodOrAccessorAdd(O, method).
    // TODO: Private properties and methods.
    // 3. Let fields be the value of constructor.[[Fields]].
    // 4. For each element fieldRecord of fields, do
    // a. Perform ? DefineField(O, fieldRecord).
    // 5. Return unused.
    let constructor_data = &agent[constructor];
    if let Some(bytecode) = constructor_data.compiled_initializer_bytecode {
        // Note: The code here looks quite a bit different from what the spec
        // says. For one, the spec is bugged and doesn't consider default
        // constructors at all. Second, we compile field initializers into
        // the ECMAScript class constructors directly, so our code only needs
        // to work for builtin constructors.
        // Third, the spec defines the initializers as individual functions
        // run one after the other. Instea we compile all of the initializers
        // into a single bytecode executable associated with the constructor.
        // The problem then becomes how to run this executable as an ECMAScript
        // function.
        // To do this, we need a new execution context that points to a new
        // Function environment. The function environment should be lexically a
        // child of the class constructor's creating environment.
        let f = constructor.into_function();
        let outer_env = constructor_data.environment;
        let outer_priv_env = constructor_data.private_environment;
        let source_code = constructor_data.source_code;
        let decl_env = new_class_field_initializer_environment(agent, f, o, outer_env);
        agent.execution_context_stack.push(ExecutionContext {
            ecmascript_code: Some(ECMAScriptCodeEvaluationState {
                lexical_environment: EnvironmentIndex::Function(decl_env),
                variable_environment: EnvironmentIndex::Function(decl_env),
                private_environment: outer_priv_env,
                is_strict_mode: true,
                source_code,
            }),
            function: Some(f),
            realm: agent[constructor].realm,
            script_or_module: None,
        });
        let _ = Vm::execute(agent, bytecode, None).into_js_result()?;
        agent.execution_context_stack.pop();
    }
    Ok(())
}

/// ### [7.3.34 AddValueToKeyedGroup ( groups, key, value )](https://tc39.es/ecma262/#sec-add-value-to-keyed-group)
/// The abstract operation AddValueToKeyedGroup takes arguments groups (a List of Records with fields
/// [[Key]] (an ECMAScript language value) and [[Elements]] (a List of ECMAScript language values)),
/// key (an ECMAScript language value), and value (an ECMAScript language value) and returns UNUSED.
pub(crate) fn add_value_to_keyed_group<K: Copy + Into<Value>>(
    agent: &mut Agent,
    groups: &mut Vec<GroupByRecord<K>>,
    key: K,
    value: Value,
) -> JsResult<()> {
    // 1. For each Record { [[Key]], [[Elements]] } g of groups, do
    for g in groups.iter_mut() {
        // a. If SameValue(g.[[Key]], key) is true, then
        if same_value(agent, g.key, key) {
            // i. Assert: Exactly one element of groups meets this criterion.
            // ii. Append value to g.[[Elements]].
            g.elements.push(value);

            // iii. Return UNUSED.
            return Ok(());
        }
    }

    // 2. Let group be the Record { [[Key]]: key, [[Elements]]: « value » }.
    let group = GroupByRecord {
        key,
        elements: vec![value],
    };

    // 3. Append group to groups.
    groups.push(group);

    // 4. Return UNUSED.
    Ok(())
}

#[derive(Debug)]
pub(crate) struct GroupByRecord<K: Copy + Into<Value>> {
    pub(crate) key: K,
    pub(crate) elements: Vec<Value>,
}

/// ### [7.3.35 GroupBy ( items, callback, keyCoercion )](https://tc39.es/ecma262/#sec-groupby)
///
/// The abstract operation GroupBy takes arguments items (an ECMAScript language value), callback
/// (an ECMAScript language value), and keyCoercion (property or collection) and returns either a
/// normal completion containing a List of Records with fields [[Key]] (an ECMAScript language
/// value) and [[Elements]] (a List of ECMAScript language values), or a throw completion.
///
/// Note: This version is for "property" keyCoercion.
pub(crate) fn group_by_property(
    agent: &mut Agent,
    items: Value,
    callback_fn: Value,
) -> JsResult<Vec<GroupByRecord<PropertyKey>>> {
    // 1. Perform ? RequireObjectCoercible(iterable).
    require_object_coercible(agent, items)?;

    // 2. If IsCallable(callback) is false, throw a TypeError exception.
    let Some(callback_fn) = is_callable(callback_fn) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Callback is not callable",
        ));
    };

    // 3. Let groups be a new empty List.
    let mut groups: Vec<GroupByRecord<PropertyKey>> = vec![];

    // 4. Let iteratorRecord be ? GetIterator(iterable).
    let mut iterator_record = get_iterator(agent, items, false)?;

    // 5. Let k be 0.
    let mut k = 0;

    // 6. Repeat,
    loop {
        // NOTE: The actual max size of an array is u32::MAX
        // a. If k ≥ 2**53 - 1, then
        if k >= u32::MAX as usize {
            // i. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Maximum array size of 2**53-1 exceeded",
            );

            // ii. Return ? IteratorClose(iteratorRecord, error).
            return iterator_close(agent, &iterator_record, Err(error));
        }

        // b. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(agent, &mut iterator_record)?;

        // c. If next is DONE, then
        //   i. Return groups.
        let Some(next) = next else {
            return Ok(groups);
        };

        // d. Let value be next.
        let value = next;

        let sk = SmallInteger::try_from(k as u64).unwrap();
        // 𝔽(k)
        let fk = Number::from(sk).into_value();

        // e. Let key be Completion(Call(callback, undefined, « value, 𝔽(k) »)).
        let key = call_function(
            agent,
            callback_fn,
            Value::Undefined,
            Some(ArgumentsList(&[value, fk])),
        );

        // f. IfAbruptCloseIterator(key, iteratorRecord).
        let key = if_abrupt_close_iterator(agent, key, &iterator_record)?;

        // g. If keyCoercion is property, then
        // i. Set key to Completion(ToPropertyKey(key)).
        let key = to_property_key(agent, key);

        // ii. IfAbruptCloseIterator(key, iteratorRecord).
        let key = if_abrupt_close_iterator(agent, key, &iterator_record)?;

        // i. Perform AddValueToKeyedGroup(groups, key, value).
        add_value_to_keyed_group(agent, &mut groups, key, value)?;

        // j. Set k to k + 1.
        k += 1;
    }
}

/// ### [7.3.35 GroupBy ( items, callback, keyCoercion )](https://tc39.es/ecma262/#sec-groupby)
///
/// The abstract operation GroupBy takes arguments items (an ECMAScript language value), callback
/// (an ECMAScript language value), and keyCoercion (property or collection) and returns either a
/// normal completion containing a List of Records with fields [[Key]] (an ECMAScript language
/// value) and [[Elements]] (a List of ECMAScript language values), or a throw completion.
///
/// Note: This version is for "collection" keyCoercion.
pub(crate) fn group_by_collection(
    agent: &mut Agent,
    items: Value,
    callback_fn: Value,
) -> JsResult<Vec<GroupByRecord<Value>>> {
    // 1. Perform ? RequireObjectCoercible(iterable).
    require_object_coercible(agent, items)?;

    // 2. If IsCallable(callback) is false, throw a TypeError exception.
    let Some(callback_fn) = is_callable(callback_fn) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Callback is not callable",
        ));
    };

    // 3. Let groups be a new empty List.
    let mut groups: Vec<GroupByRecord<Value>> = vec![];

    // 4. Let iteratorRecord be ? GetIterator(iterable).
    let mut iterator_record = get_iterator(agent, items, false)?;

    // 5. Let k be 0.
    let mut k = 0;

    // 6. Repeat,
    loop {
        // NOTE: The actual max size of an array is u32::MAX
        // a. If k ≥ 2**53 - 1, then
        if k >= u32::MAX as usize {
            // i. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Maximum array size of 2**53-1 exceeded",
            );

            // ii. Return ? IteratorClose(iteratorRecord, error).
            return iterator_close(agent, &iterator_record, Err(error));
        }

        // b. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(agent, &mut iterator_record)?;

        // c. If next is DONE, then
        //   i. Return groups.
        let Some(next) = next else {
            return Ok(groups);
        };

        // d. Let value be next.
        let value = next;

        let sk = SmallInteger::try_from(k as u64).unwrap();
        // 𝔽(k)
        let fk = Number::from(sk).into_value();

        // e. Let key be Completion(Call(callback, undefined, « value, 𝔽(k) »)).
        let key = call_function(
            agent,
            callback_fn,
            Value::Undefined,
            Some(ArgumentsList(&[value, fk])),
        );

        // f. IfAbruptCloseIterator(key, iteratorRecord).
        let key = if_abrupt_close_iterator(agent, key, &iterator_record)?;

        // h. Else,
        // i. Assert: keyCoercion is collection.
        // ii. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(agent, key);

        // i. Perform AddValueToKeyedGroup(groups, key, value).
        add_value_to_keyed_group(agent, &mut groups, key, value)?;

        // j. Set k to k + 1.
        k += 1;
    }
}
