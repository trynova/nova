// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [7.3 Operations on Objects](https://tc39.es/ecma262/#sec-operations-on-objects)

use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            keyed_group::KeyedGroup,
            operations_on_iterator_objects::{
                IteratorRecord, get_iterator, if_abrupt_close_iterator, iterator_close_with_error,
                iterator_step_value,
            },
            testing_and_comparison::{is_callable, is_constructor, require_object_coercible},
            type_conversion::{
                to_length, to_object, to_property_key, to_property_key_simple, try_to_length,
            },
        },
        builtins::{
            ArgumentsList, Array, BuiltinConstructorFunction, array_create,
            keyed_collections::map_objects::map_prototype::canonicalize_keyed_collection_key,
            proxy::abstract_operations::{
                try_validate_non_revoked_proxy, validate_non_revoked_proxy,
            },
        },
        execution::{
            Agent, ECMAScriptCodeEvaluationState, Environment, ExecutionContext, JsResult, Realm,
            agent::{ExceptionType, JsError},
            new_class_field_initializer_environment,
        },
        types::{
            BUILTIN_STRING_MEMORY, Function, InternalMethods, IntoFunction, IntoObject, IntoValue,
            Number, Object, ObjectHeapData, OrdinaryObject, PropertyDescriptor, PropertyKey,
            PropertyKeySet, String, Value,
        },
    },
    engine::{
        ScopableCollection, Scoped, ScopedCollection, TryResult, Vm,
        context::{Bindable, GcScope, NoGcScope},
        instanceof_operator,
        rootable::{Rootable, Scopable},
        unwrap_try,
    },
    heap::{Heap, ObjectEntry, WellKnownSymbolIndexes},
};

/// ### [7.3.2 Get ( O, P )](https://tc39.es/ecma262/#sec-get-o-p)
///
/// The abstract operation Get takes arguments O (an Object) and P (a property
/// key) and returns either a normal completion containing an ECMAScript
/// language value or a throw completion. It is used to retrieve the value of a
/// specific property of an object.
#[inline]
pub(crate) fn get<'a, 'b>(
    agent: &mut Agent,
    o: impl IntoObject<'b>,
    p: PropertyKey,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
    let p = p.bind(gc.nogc());
    // 1. Return ? O.[[Get]](P, O).
    o.into_object()
        .internal_get(agent, p.unbind(), o.into_value(), gc)
}

/// ### Try [7.3.2 Get ( O, P )](https://tc39.es/ecma262/#sec-get-o-p)
///
/// The abstract operation Get takes arguments O (an Object) and P (a property
/// key) and returns either a normal completion containing an ECMAScript
/// language value or a throw completion. It is used to retrieve the value of a
/// specific property of an object.
#[inline]
pub(crate) fn try_get<'a, 'gc>(
    agent: &mut Agent,
    o: impl IntoObject<'a>,
    p: PropertyKey,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<Value<'gc>> {
    // 1. Return ? O.[[Get]](P, O).
    o.into_object().try_get(agent, p, o.into_value(), gc)
}

/// ### [7.3.3 GetV ( V, P )](https://tc39.es/ecma262/#sec-getv)
///
/// The abstract operation GetV takes arguments V (an ECMAScript language
/// value) and P (a property key) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. It is used
/// to retrieve the value of a specific property of an ECMAScript language
/// value. If the value is not an object, the property lookup is performed
/// using a wrapper object appropriate for the type of the value.
pub(crate) fn get_v<'gc>(
    agent: &mut Agent,
    v: Value,
    p: PropertyKey,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let v = v.bind(gc.nogc());
    let p = p.bind(gc.nogc());
    // 1. Let O be ? ToObject(V).
    // Optimisation: We avoid allocating a primitive object that would only be
    // used for the internal methods, and instead just use the prototype
    // intrinsics directly.
    let o = match v {
        Value::Undefined | Value::Null => {
            // Call to conversion function to throw error.
            return Err(to_object(agent, v.unbind(), gc.into_nogc()).unwrap_err());
        }
        Value::Boolean(_) => agent
            .current_realm_record()
            .intrinsics()
            .boolean_prototype()
            .into_object(),
        Value::String(_) | Value::SmallString(_) => {
            let v = String::try_from(v).unwrap();
            if let Some(value) = v.get_property_value(agent, p) {
                return Ok(value.unbind());
            }
            agent
                .current_realm_record()
                .intrinsics()
                .string_prototype()
                .into_object()
        }
        Value::Symbol(_) => agent
            .current_realm_record()
            .intrinsics()
            .symbol_prototype()
            .into_object(),
        Value::Number(_) | Value::Integer(_) | Value::SmallF64(_) => agent
            .current_realm_record()
            .intrinsics()
            .number_prototype()
            .into_object(),
        Value::BigInt(_) | Value::SmallBigInt(_) => agent
            .current_realm_record()
            .intrinsics()
            .big_int_prototype()
            .into_object(),
        _ => Object::try_from(v).unwrap(),
    };
    // 2. Return ? O.[[Get]](P, V).
    o.unbind().internal_get(agent, p.unbind(), v.unbind(), gc)
}

/// ### Try [7.3.3 GetV ( V, P )](https://tc39.es/ecma262/#sec-getv)
///
/// The abstract operation GetV takes arguments V (an ECMAScript language
/// value) and P (a property key) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. It is used
/// to retrieve the value of a specific property of an ECMAScript language
/// value. If the value is not an object, the property lookup is performed
/// using a wrapper object appropriate for the type of the value.
pub(crate) fn try_get_v<'gc>(
    agent: &mut Agent,
    v: Value,
    p: PropertyKey,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<JsResult<'gc, Value<'gc>>> {
    let v = v.bind(gc);
    let p = p.bind(gc);
    // 1. Let O be ? ToObject(V).
    // Optimisation: We avoid allocating a primitive object that would only be
    // used for the internal methods, and instead just use the prototype
    // intrinsics directly.
    let o = match v {
        Value::Undefined | Value::Null => {
            // Call to conversion function to throw error.
            return TryResult::Continue(Err(to_object(agent, v, gc).unwrap_err()));
        }
        Value::Boolean(_) => agent
            .current_realm_record()
            .intrinsics()
            .boolean_prototype()
            .into_object(),
        Value::String(_) | Value::SmallString(_) => {
            let v = String::try_from(v).unwrap();
            if let Some(value) = v.get_property_value(agent, p) {
                return TryResult::Continue(Ok(value.bind(gc)));
            }
            agent
                .current_realm_record()
                .intrinsics()
                .string_prototype()
                .into_object()
        }
        Value::Symbol(_) => agent
            .current_realm_record()
            .intrinsics()
            .symbol_prototype()
            .into_object(),
        Value::Number(_) | Value::Integer(_) | Value::SmallF64(_) => agent
            .current_realm_record()
            .intrinsics()
            .number_prototype()
            .into_object(),
        Value::BigInt(_) | Value::SmallBigInt(_) => agent
            .current_realm_record()
            .intrinsics()
            .big_int_prototype()
            .into_object(),
        _ => Object::try_from(v).unwrap(),
    };
    // 2. Return ? O.[[Get]](P, V).
    TryResult::Continue(Ok(o.try_get(agent, p, v, gc)?))
}

/// ### [7.3.4 Set ( O, P, V, Throw )](https://tc39.es/ecma262/#sec-set-o-p-v-throw)
///
/// The abstract operation Set takes arguments O (an Object), P (a property
/// key), V (an ECMAScript language value), and Throw (a Boolean) and returns
/// either a normal completion containing UNUSED or a throw completion. It is
/// used to set the value of a specific property of an object. V is the new
/// value for the property.
pub(crate) fn set<'a>(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    v: Value,
    throw: bool,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let p = p.bind(gc.nogc());
    let scoped_p = p.scope(agent, gc.nogc());
    // 1. Let success be ? O.[[Set]](P, V, O).
    let success = o
        .internal_set(agent, p.unbind(), v, o.into_value(), gc.reborrow())
        .unbind()?;
    // SAFETY: p is not shared.
    let p = unsafe { scoped_p.take(agent) }.bind(gc.nogc());
    // 2. If success is false and Throw is true, throw a TypeError exception.
    if !success && throw {
        return throw_set_error(agent, p.unbind(), gc.into_nogc());
    }
    // 3. Return UNUSED.
    Ok(())
}

pub(crate) fn throw_set_error<'a>(
    agent: &mut Agent,
    p: PropertyKey,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    Err(agent.throw_exception(
        ExceptionType::TypeError,
        format!("Could not set property '{}'.", p.as_display(agent)),
        gc,
    ))
}

/// ### Try [7.3.4 Set ( O, P, V, Throw )](https://tc39.es/ecma262/#sec-set-o-p-v-throw)
///
/// The abstract operation Set takes arguments O (an Object), P (a property
/// key), V (an ECMAScript language value), and Throw (a Boolean) and returns
/// either a normal completion containing UNUSED or a throw completion. It is
/// used to set the value of a specific property of an object. V is the new
/// value for the property.
pub(crate) fn try_set<'a>(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    v: Value,
    throw: bool,
    gc: NoGcScope<'a, '_>,
) -> TryResult<JsResult<'a, ()>> {
    // 1. Let success be ? O.[[Set]](P, V, O).
    let success = o.try_set(agent, p, v, o.into_value(), gc)?;
    // 2. If success is false and Throw is true, throw a TypeError exception.
    if !success && throw {
        return TryResult::Continue(throw_set_error(agent, p, gc));
    }
    // 3. Return UNUSED.
    TryResult::Continue(Ok(()))
}

/// ### Try [7.3.5] CreateDataProperty ( O, P, V )[https://tc39.es/ecma262/#sec-createdataproperty]
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
pub(crate) fn try_create_data_property<'a>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    value: Value,
    gc: NoGcScope,
) -> TryResult<bool> {
    // 1. Let newDesc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true }.
    let new_desc = PropertyDescriptor {
        value: Some(value.unbind()),
        writable: Some(true),
        get: None,
        set: None,
        enumerable: Some(true),
        configurable: Some(true),
    };
    // 2. Return ? O.[[DefineOwnProperty]](P, newDesc).
    object.try_define_own_property(agent, property_key, new_desc, gc)
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
/// > does exist and is not configurable or if O as not extensible,
/// > [\[DefineOwnProperty]] will return false.
pub(crate) fn create_data_property<'a, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    value: Value,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    let property_key = property_key.bind(gc.nogc());
    // 1. Let newDesc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true }.
    let new_desc = PropertyDescriptor {
        value: Some(value.unbind()),
        writable: Some(true),
        get: None,
        set: None,
        enumerable: Some(true),
        configurable: Some(true),
    };
    // 2. Return ? O.[[DefineOwnProperty]](P, newDesc).
    object.internal_define_own_property(agent, property_key.unbind(), new_desc, gc)
}

/// ### Try [7.3.7 CreateDataPropertyOrThrow ( O, P, V )](https://tc39.es/ecma262/#sec-createdatapropertyorthrow)
///
/// The abstract operation CreateDataPropertyOrThrow takes arguments O (an
/// Object), P (a property key), and V (an ECMAScript language value) and
/// returns either a normal completion containing UNUSED or a throw completion.
/// It is used to create a new own property of an object. at throws a TypeError
/// exception if the requested property update cannot be performed.
pub(crate) fn try_create_data_property_or_throw<'a, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    value: Value,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<JsResult<'gc, ()>> {
    let success = try_create_data_property(agent, object, property_key, value, gc)?;
    if !success {
        TryResult::Continue(Err(agent.throw_exception(
            ExceptionType::TypeError,
            format!(
                "Could not create property '{}'.",
                property_key.as_display(agent)
            ),
            gc,
        )))
    } else {
        TryResult::Continue(Ok(()))
    }
}

/// ### [7.3.7 CreateDataPropertyOrThrow ( O, P, V )](https://tc39.es/ecma262/#sec-createdatapropertyorthrow)
///
/// The abstract operation CreateDataPropertyOrThrow takes arguments O (an
/// Object), P (a property key), and V (an ECMAScript language value) and
/// returns either a normal completion containing UNUSED or a throw completion.
/// It is used to create a new own property of an object. at throws a TypeError
/// exception if the requested property update cannot be performed.
pub(crate) fn create_data_property_or_throw<'a, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    value: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, ()> {
    let property_key = property_key.bind(gc.nogc());
    let scoped_property_key = property_key.scope(agent, gc.nogc());
    let success = create_data_property(agent, object, property_key.unbind(), value, gc.reborrow())
        .unbind()?;
    if !success {
        Err(agent.throw_exception(
            ExceptionType::TypeError,
            format!(
                "Could not create property '{}'.",
                scoped_property_key
                    .get(agent)
                    .bind(gc.nogc())
                    .as_display(agent)
            ),
            gc.into_nogc(),
        ))
    } else {
        Ok(())
    }
}

/// ### Try [7.3.8 DefinePropertyOrThrow ( O, P, desc )](https://tc39.es/ecma262/#sec-definepropertyorthrow)
///
/// The abstract operation DefinePropertyOrThrow takes arguments O (an Object),
/// P (a property key), and desc (a Property Descriptor) and returns either a
/// normal completion containing UNUSED or a throw completion. It is used to
/// call the \[\[DefineOwnProperty]] internal method of an object in a manner
/// that will throw a TypeError exception if tae requested property update
/// cannot be performed.
pub(crate) fn try_define_property_or_throw<'a, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    desc: PropertyDescriptor,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<JsResult<'gc, ()>> {
    // 1. Let success be ? O.[[DefineOwnProperty]](P, desc).
    let success = object.try_define_own_property(agent, property_key, desc, gc)?;
    // 2. If success is false, throw a TypeError exception.
    if !success {
        TryResult::Continue(Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Failed to defined property on object",
            gc,
        )))
    } else {
        // 3. Return UNUSED.
        TryResult::Continue(Ok(()))
    }
}

/// ### [7.3.8 DefinePropertyOrThrow ( O, P, desc )](https://tc39.es/ecma262/#sec-definepropertyorthrow)
///
/// The abstract operation DefinePropertyOrThrow takes arguments O (an Object),
/// P (a property key), and desc (a Property Descriptor) and returns either a
/// normal completion containing UNUSED or a throw completion. It is used to
/// call the \[\[DefineOwnProperty]] internal method of an object in a manner
/// that will throw a TypeError exception if tae requested property update
/// cannot be performed.
pub(crate) fn define_property_or_throw<'a, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    desc: PropertyDescriptor,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, ()> {
    let property_key = property_key.bind(gc.nogc());
    let desc = desc.bind(gc.nogc());
    // 1. Let success be ? O.[[DefineOwnProperty]](P, desc).
    let success = object
        .internal_define_own_property(agent, property_key.unbind(), desc.unbind(), gc.reborrow())
        .unbind()?;
    // 2. If success is false, throw a TypeError exception.
    if !success {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Failed to defined property on object",
            gc.into_nogc(),
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
/// unused or a throw completion. It is used to removeaa specific own property
/// of an object. It throws an exception if the property is not configurable.
pub(crate) fn try_delete_property_or_throw<'a, 'gc>(
    agent: &mut Agent,
    o: impl InternalMethods<'a>,
    p: PropertyKey,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<JsResult<'gc, ()>> {
    // 1. Let success be ? O.[[Delete]](P).
    let success = o.try_delete(agent, p, gc)?;
    // 2. If success is false, throw a TypeError exception.
    if !success {
        TryResult::Continue(Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Failed to delete property",
            gc,
        )))
    } else {
        // 3. Return unused.
        TryResult::Continue(Ok(()))
    }
}

/// ### [7.3.9 DeletePropertyOrThrow ( O, P )](https://tc39.es/ecma262/#sec-deletepropertyorthrow)
///
/// The abstract operation DeletePropertyOrThrow takes arguments O (an Object)
/// and P (a property key) and returns either a normal completion containing
/// unused or a throw completion. It is used to remove a specific own property
/// of an object. It throws an exception if the property is not configurable.
pub(crate) fn delete_property_or_throw<'a, 'gc>(
    agent: &mut Agent,
    o: impl InternalMethods<'a>,
    p: PropertyKey,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, ()> {
    let p = p.bind(gc.nogc());
    // 1. Let success be ? O.[[Delete]](P).
    let success = o
        .internal_delete(agent, p.unbind(), gc.reborrow())
        .unbind()?;
    // 2. If success is false, throw a TypeError exception.
    if !success {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Failed to delete property",
            gc.into_nogc(),
        ))
    } else {
        // 3. Return unused.
        Ok(())
    }
}

/// ### Try [7.3.11 GetMethod ( V, P )](https://tc39.es/ecma262/#sec-getmethod)
///
/// The abstract operation GetMethod takes arguments V (an ECMAScript language
/// value) and P (a property key) and returns either a normal completion
/// containing either a function object or undefined, or a throw completion. It
/// is used to get the value of a specific property of an ECMAScript language
/// value when the value of the property is expected to be a function.
pub(crate) fn try_get_method<'a>(
    agent: &mut Agent,
    v: Value,
    p: PropertyKey,
    gc: NoGcScope<'a, '_>,
) -> TryResult<JsResult<'a, Option<Function<'a>>>> {
    // 1. Let func be ? GetV(V, P).
    let func = match try_get_v(agent, v, p, gc)? {
        Ok(func) => func,
        Err(err) => {
            return TryResult::Continue(Err(err));
        }
    };
    TryResult::Continue(get_method_internal(agent, func, gc))
}

/// ### Try [7.3.11 GetMethod ( V, P )](https://tc39.es/ecma262/#sec-getmethod)
///
/// The abstract operation GetMethod takes arguments V (an object) and P (a
/// property key) and returns either a normal completion containing either a
/// function object or undefined, or a throw completion. It is used to get the
/// value of a specific property of an ECMAScript language value when the value
/// of the property is expected to be a function.
pub(crate) fn try_get_object_method<'a>(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    gc: NoGcScope<'a, '_>,
) -> TryResult<JsResult<'a, Option<Function<'a>>>> {
    // 1. Let func be ? GetV(V, P).
    let func = o.try_get(agent, p, o.into_value(), gc)?;
    TryResult::Continue(get_method_internal(agent, func, gc))
}

/// ### [7.3.11 GetMethod ( V, P )](https://tc39.es/ecma262/#sec-getmethod)
///
/// The abstract operation GetMethod takes arguments V (an ECMAScript language
/// value) and P (a property key) and returns either a normal completion
/// containing either a function object or undefined, or a throw completion. It
/// is used to get the value of a specific property of an ECMAScript language
/// value when the value of the property is expected to be a function.
pub(crate) fn get_method<'a>(
    agent: &mut Agent,
    v: Value,
    p: PropertyKey,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Function<'a>>> {
    let p = p.bind(gc.nogc());
    // 1. Let func be ? GetV(V, P).
    let func = get_v(agent, v, p.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    get_method_internal(agent, func.unbind(), gc.into_nogc())
}

/// ### [7.3.11 GetMethod ( V, P )](https://tc39.es/ecma262/#sec-getmethod)
///
/// The abstract operation GetMethod takes arguments V (an ECMAScript language
/// value) and P (a property key) and returns either a normal completion
/// containing either a function object or undefined, or a throw completion. It
/// is used to get the value of a specific property of an ECMAScript language
/// value when the value of the property is expected to be a function.
pub(crate) fn get_object_method<'a>(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Function<'a>>> {
    let p = p.bind(gc.nogc());
    // 1. Let func be ? GetV(V, P).
    let func = o
        .internal_get(agent, p.unbind(), o.into_value(), gc.reborrow())
        .unbind()?;
    let gc = gc.into_nogc();
    let func = func.bind(gc);
    get_method_internal(agent, func, gc)
}

fn get_method_internal<'a>(
    agent: &mut Agent,
    func: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Option<Function<'a>>> {
    // 2. If func is either undefined or null, return undefined.
    if func.is_undefined() || func.is_null() {
        return Ok(None);
    }
    // 3. If IsCallable(func) is false, throw a TypeError exception.
    let func = is_callable(func, gc);
    if func.is_none() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Not a callable object",
            gc,
        ));
    }
    // 4. Return func.
    Ok(func)
}

/// ### Try [7.3.12 HasProperty ( O, P )](https://tc39.es/ecma262/#sec-hasproperty)
///
/// The abstract operation HasProperty takes arguments O (an Object) and P (a
/// property key) and returns either a normal completion containing a Boolean
/// or a throw completion. It is used to determine whether an object has a
/// property with the specified property key. The property may be either own or
/// inherited.
#[inline(always)]
pub(crate) fn try_has_property(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    gc: NoGcScope,
) -> TryResult<bool> {
    // 1. Return ? O.[[HasProperty]](P).
    o.try_has_property(agent, p, gc)
}

/// ### [7.3.12 HasProperty ( O, P )](https://tc39.es/ecma262/#sec-hasproperty)
///
/// The abstract operation HasProperty takes arguments O (an Object) and P (a
/// property key) and returns either a normal completion containing a Boolean
/// or a throw completion. It is used to determine whether an object has a
/// property with the specified property key. The property may be either own or
/// inherited.
#[inline(always)]
pub(crate) fn has_property<'a>(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    // 1. Return ? O.[[HasProperty]](P).
    o.internal_has_property(agent, p, gc)
}

/// ### Try [7.3.13 HasOwnProperty ( O, P )](https://tc39.es/ecma262/#sec-hasownproperty)
///
/// The abstract operation HasOwnProperty takes arguments O (an Object) and P
/// (a property key) and returns either a normal completion containing a
/// Boolean or a throw completion. It is used to determine whether an object
/// has an own property with the specified property key.
#[inline(always)]
pub(crate) fn try_has_own_property(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    gc: NoGcScope,
) -> TryResult<bool> {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let desc = o.try_get_own_property(agent, p, gc)?;
    // 2. If desc is undefined, return false.
    // 3. Return true.
    TryResult::Continue(desc.is_some())
}

/// ### [7.3.13 HasOwnProperty ( O, P )](https://tc39.es/ecma262/#sec-hasownproperty)
///
/// The abstract operation HasOwnProperty takes arguments O (an Object) and P
/// (a property key) and returns either a normal completion containing a
/// Boolean or a throw completion. It is used to determine whether an object
/// has an own property with the specified property key.
#[inline(always)]
pub(crate) fn has_own_property<'a>(
    agent: &mut Agent,
    o: Object,
    p: PropertyKey,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let desc = o.internal_get_own_property(agent, p, gc)?;
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
pub(crate) fn call<'gc>(
    agent: &mut Agent,
    f: Value,
    v: Value,
    arguments_list: Option<ArgumentsList>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    // 1. If argumentsList is not present, set argumentsList to a new empty List.
    let arguments_list = arguments_list.unwrap_or_default();
    // 2. If IsCallable(F) is false, throw a TypeError exception.
    match is_callable(f, gc.nogc()) {
        None => Err(throw_not_callable(agent, gc.into_nogc()).unbind()),
        // 3. Return ? F.[[Call]](V, argumentsList).
        Some(f) => {
            let current_stack_size = agent.stack_refs.borrow().len();
            let result = f.unbind().internal_call(agent, v, arguments_list, gc);
            agent.stack_refs.borrow_mut().truncate(current_stack_size);
            result
        }
    }
}

#[cold]
#[inline(never)]
pub(crate) fn throw_not_callable<'a>(agent: &mut Agent, gc: NoGcScope<'a, '_>) -> JsError<'a> {
    agent.throw_exception_with_static_message(ExceptionType::TypeError, "Not a callable object", gc)
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
pub(crate) fn set_integrity_level<'a, T: Level>(
    agent: &mut Agent,
    o: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    // 1. Let status be ? O.[[PreventExtensions]]().
    let status = o
        .internal_prevent_extensions(agent, gc.reborrow())
        .unbind()?;
    // 2. If status is false, return false.
    if !status {
        return Ok(false);
    }
    // 3. Let keys be ? O.[[OwnPropertyKeys]]().
    let keys = o
        .internal_own_property_keys(agent, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    let keys = keys.unbind().bind(gc.nogc());
    // 4. If level is SEALED, then
    if T::LEVEL == IntegrityLevel::Sealed {
        // a. For each element k of keys, do
        let mut broke = false;
        let mut i = 0;
        for k in keys.iter() {
            // i. Perform ? DefinePropertyOrThrow(O, k, PropertyDescriptor { [[Configurable]]: false }).
            if let TryResult::Continue(result) = try_define_property_or_throw(
                agent,
                o,
                *k,
                PropertyDescriptor {
                    configurable: Some(false),
                    ..Default::default()
                },
                gc.nogc(),
            ) {
                result.unbind()?.bind(gc.nogc());
            } else {
                broke = true;
                break;
            }
            i += 1;
        }
        if !broke {
            return Ok(true);
        }
        let keys = keys[i..]
            .iter()
            .map(|pk| pk.scope(agent, gc.nogc()))
            .collect::<Vec<_>>();
        for k in keys {
            // i. Perform ? DefinePropertyOrThrow(O, k, PropertyDescriptor { [[Configurable]]: false }).
            define_property_or_throw(
                agent,
                o,
                k.get(agent),
                PropertyDescriptor {
                    configurable: Some(false),
                    ..Default::default()
                },
                gc.reborrow(),
            )
            .unbind()?;
        }
    } else {
        // 5. Else,
        // a. Assert: level is FROZEN.
        // b. For each element k of keys, do
        let mut broke = false;
        let mut i = 0;
        for &k in keys.iter() {
            // i. Let currentDesc be ? O.[[GetOwnProperty]](k).
            let current_desc =
                if let TryResult::Continue(result) = o.try_get_own_property(agent, k, gc.nogc()) {
                    result
                } else {
                    broke = true;
                    break;
                };
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
                if let TryResult::Continue(result) =
                    try_define_property_or_throw(agent, o, k, desc, gc.nogc())
                {
                    result.unbind()?
                } else {
                    broke = true;
                    break;
                };
            }
            i += 1;
        }
        if !broke {
            return Ok(true);
        }
        let keys = keys[i..]
            .iter()
            .map(|pk| pk.scope(agent, gc.nogc()))
            .collect::<Vec<_>>();
        for k in keys {
            // i. Let currentDesc be ? O.[[GetOwnProperty]](k).
            let current_desc = o
                .internal_get_own_property(agent, k.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
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
                define_property_or_throw(agent, o, k.get(agent), desc, gc.reborrow()).unbind()?;
            }
            i += 1;
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
pub(crate) fn test_integrity_level<'a, T: Level>(
    agent: &mut Agent,
    o: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    // 1. Let extensible be ? IsExtensible(O).
    // 2. If extensible is true, return false.
    // 3. NOTE: If the object is extensible, none of its properties are examined.
    if o.internal_is_extensible(agent, gc.reborrow()).unbind()? {
        return Ok(false);
    }

    // 4. Let keys be ? O.[[OwnPropertyKeys]]().
    let keys = o
        .internal_own_property_keys(agent, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    let keys = keys.unbind().bind(gc.nogc());

    let mut broke = false;
    let mut i = 0;
    // 5. For each element k of keys, do
    for &k in keys.iter() {
        // a. Let currentDesc be ? O.[[GetOwnProperty]](k).
        let TryResult::Continue(result) = o.try_get_own_property(agent, k, gc.nogc()) else {
            broke = true;
            break;
        };
        // b. If currentDesc is not undefined, then
        if let Some(current_desc) = result {
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
        i += 1;
    }

    if !broke {
        return Ok(true);
    }

    let keys = keys
        .iter()
        .skip(i)
        .map(|pk| pk.scope(agent, gc.nogc()))
        .collect::<Vec<_>>();

    for k in keys {
        // a. Let currentDesc be ? O.[[GetOwnProperty]](k).
        // b. If currentDesc is not undefined, then
        if let Some(current_desc) = o
            .internal_get_own_property(agent, k.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
        {
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
pub(crate) fn create_array_from_list<'a>(
    agent: &mut Agent,
    elements: &[Value],
    gc: NoGcScope<'a, '_>,
) -> Array<'a> {
    let len = elements.len();
    // 1. Let array be ! ArrayCreate(0).
    let array = array_create(agent, len, len, None, gc).unwrap();
    let array_elements = agent[array].elements;
    agent[array_elements]
        .copy_from_slice(unsafe { core::mem::transmute::<&[Value], &[Option<Value>]>(elements) });
    // 2. Let n be 0.
    // 3. For each element e of elements, do
    // a. Perform ! CreateDataPropertyOrThrow(array, ! ToString(𝔽(n)), e).
    // b. Set n to n + 1.
    // 4. Return array.
    array
}

pub(crate) fn create_array_from_scoped_list<'a>(
    agent: &mut Agent,
    elements: ScopedCollection<Vec<Value>>,
    gc: NoGcScope<'a, '_>,
) -> Array<'a> {
    let elements = elements.take(agent).bind(gc);
    let len = elements.len();
    // 1. Let array be ! ArrayCreate(0).
    let array = array_create(agent, len, len, None, gc).unwrap();
    let slice = array.as_mut_slice(agent).iter_mut().zip(elements);
    {
        for (target, el) in slice {
            *target = Some(el.unbind());
        }
    }
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
pub(crate) fn length_of_array_like<'a>(
    agent: &mut Agent,
    obj: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, i64> {
    let obj = obj.bind(gc.nogc());
    // NOTE: Fast path for Array objects.
    if let Ok(array) = Array::try_from(obj) {
        return Ok(array.len(agent) as i64);
    }

    // 1. Return ℝ(? ToLength(? Get(obj, "length"))).
    let property = get(
        agent,
        obj.unbind(),
        PropertyKey::from(BUILTIN_STRING_MEMORY.length),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    to_length(agent, property.unbind(), gc)
}

/// ### [7.3.18 LengthOfArrayLike ( obj )](https://tc39.es/ecma262/#sec-lengthofarraylike)
///
/// The abstract operation LengthOfArrayLike takes argument obj (an Object) and
/// returns either a normal completion containing a non-negative integer or a
/// throw completion. It returns the value of the "length" property of an
/// array-like object.
pub(crate) fn try_length_of_array_like<'a>(
    agent: &mut Agent,
    obj: Object,
    gc: NoGcScope<'a, '_>,
) -> TryResult<JsResult<'a, i64>> {
    // NOTE: Fast path for Array objects.
    if let Ok(array) = Array::try_from(obj) {
        return TryResult::Continue(Ok(array.len(agent) as i64));
    }

    // 1. Return ℝ(? ToLength(? Get(obj, "length"))).
    let property = try_get(
        agent,
        obj,
        PropertyKey::from(BUILTIN_STRING_MEMORY.length),
        gc,
    )?;
    try_to_length(agent, property, gc)
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
pub(crate) fn create_list_from_array_like<'gc>(
    agent: &mut Agent,
    obj: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Vec<Value<'gc>>> {
    match obj {
        Value::Array(array) if array.is_simple(agent) => {
            let gc = gc.into_nogc();
            Ok(array
                .as_slice(agent)
                .iter()
                .map(|el| el.unwrap_or(Value::Undefined).bind(gc))
                .collect())
        }
        // TODO: TypedArrays
        _ if obj.is_object() => {
            let object = Object::try_from(obj).unwrap();
            // 3. Let len be ? LengthOfArrayLike(obj).
            let len = length_of_array_like(agent, object, gc.reborrow()).unbind()?;
            let len = usize::try_from(len).unwrap();
            // 4. Let list be a new empty list.
            let mut list = Vec::<Value>::with_capacity(len).scope(agent, gc.nogc());
            // 5. Let index be 0.
            // 6. Repeat, while index < len,
            for i in 0..len {
                // a. Let indexName be ! ToString(𝔽(index)).
                // b. Let next be ? Get(obj, indexName).
                let next = get(
                    agent,
                    object,
                    PropertyKey::Integer(SmallInteger::try_from(i as u64).unwrap()),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // d. Append next to list.
                list.push(agent, next);
                // e. Set index to index + 1.
            }
            // 7. Return list.
            let gc = gc.into_nogc();
            Ok(list.take(agent).bind(gc))
        }
        // 2. If obj is not an Object, throw a TypeError exception.
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Not an object",
            gc.into_nogc(),
        )),
    }
}

/// ### [7.3.19 CreateListFromArrayLike ( obj [ , elementTypes ] )](https://tc39.es/ecma262/#sec-createlistfromarraylike)
///
/// The abstract operation CreateListFromArrayLike takes argument obj (an ECMAScript language value)
/// and optional argument elementTypes (a List of names of ECMAScript Language Types) and returns
/// either a normal completion containing a List of ECMAScript language values or a throw
/// completion. It is used to create a List value whose elements are provided by the indexed
/// properties of obj. elementTypes contains the names of ECMAScript Language Types that are allowed
/// for element values of the List that is created.
pub(crate) fn create_property_key_list_from_array_like<'a, 'b>(
    agent: &mut Agent,
    obj: Value,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<'a, ScopedCollection<'b, Vec<PropertyKey<'static>>>> {
    // 1. If validElementTypes is not present, set validElementTypes to all.
    // 2. If obj is not an Object, throw a TypeError exception.
    let Ok(object) = Object::try_from(obj) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Not an object",
            gc.into_nogc(),
        ));
    };
    let object = object.bind(gc.nogc());
    let scoped_object = object.scope(agent, gc.nogc());
    // 3. Let len be ? LengthOfArrayLike(obj).
    let len = length_of_array_like(agent, object.unbind(), gc.reborrow()).unbind()?;
    let len = usize::try_from(len).unwrap();
    // 4. Let list be a new empty List.
    let mut list = Vec::<PropertyKey>::with_capacity(len).scope(agent, gc.nogc());
    // 5. Let index be 0.
    let mut index = 0;
    // 6. Repeat, while index < len,
    while index < len {
        let next = get(
            agent,
            scoped_object.get(agent).unbind(),
            PropertyKey::Integer(SmallInteger::try_from(index as u64).unwrap()).unbind(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        match next {
            Value::String(_) | Value::SmallString(_) => {
                let string_value = String::try_from(next).unwrap();
                let scoped_property_key = unwrap_try(to_property_key_simple(
                    agent,
                    string_value.unbind(),
                    gc.nogc(),
                ));
                list.push(agent, scoped_property_key);
            }
            Value::Symbol(sym) => list.push(agent, sym.into()),
            _ => {
                return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "proxy [[OwnPropertyKeys]] must return an array with only string and symbol elements",
                gc.into_nogc(),
            ));
            }
        }
        index += 1;
    }
    Ok(list)
}

/// Abstract operation Call specialized for a Function.
pub(crate) fn call_function<'gc>(
    agent: &mut Agent,
    f: Function,
    v: Value,
    arguments_list: Option<ArgumentsList>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let f = f.bind(gc.nogc());
    let arguments_list = arguments_list.unwrap_or_default();
    let current_stack_size = agent.stack_refs.borrow().len();
    let result = f.unbind().internal_call(agent, v, arguments_list, gc);
    agent.stack_refs.borrow_mut().truncate(current_stack_size);
    result
}

pub(crate) fn construct<'a>(
    agent: &mut Agent,
    f: Function,
    arguments_list: Option<ArgumentsList>,
    new_target: Option<Function>,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    let f = f.bind(gc.nogc());
    // 1. If newTarget is not present, set newTarget to F.
    let new_target = new_target.unwrap_or(f);
    // 2. If argumentsList is not present, set argumentsList to a new empty List.
    let arguments_list = arguments_list.unwrap_or_default();
    f.unbind()
        .internal_construct(agent, arguments_list, new_target.unbind(), gc)
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
pub(crate) fn invoke<'a>(
    agent: &mut Agent,
    v: Value,
    p: PropertyKey,
    arguments_list: Option<ArgumentsList>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
    let v = v.bind(gc.nogc());
    let p = p.bind(gc.nogc());
    // 1. If argumentsList is not present, set argumentsList to a new empty List.
    let mut arguments_list = arguments_list.unwrap_or_default();
    // 2. Let func be ? GetV(V, P).
    // 3. Return ? Call(func, V, argumentsList).
    if let TryResult::Continue(func) = try_get_v(agent, v, p, gc.nogc()) {
        call(agent, func.unbind()?, v.unbind(), Some(arguments_list), gc)
    } else {
        // We couldn't get the func without calling into Javascript: No
        // choice, we must scope v and the arguments.
        let scoped_v = v.scope(agent, gc.nogc());
        let v_unbound = v.unbind();
        let p_unbound = p.unbind();
        let func = arguments_list
            .with_scoped(
                agent,
                |agent, _, gc| get_v(agent, v_unbound, p_unbound, gc),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
        call(
            agent,
            func.unbind(),
            scoped_v.get(agent),
            Some(arguments_list),
            gc,
        )
    }
}

/// ### [7.3.21 OrdinaryHasInstance ( C, O )](https://tc39.es/ecma262/#sec-ordinaryhasinstance)
///
/// The abstract operation OrdinaryHasInstance takes arguments C (an ECMAScript
/// language value) and O (an ECMAScript language value) and returns either a
/// normal completion containing a Boolean or a throw completion. It implements
/// the default algorithm for determining if O inherits from the instance
/// object inheritance path provided by C.
pub(crate) fn ordinary_has_instance<'a, 'b>(
    agent: &mut Agent,
    c: impl TryInto<Function<'b>>,
    o: impl IntoValue<'b>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    // 1. If IsCallable(C) is false, return false.
    let Some(c) = is_callable(c, gc.nogc()) else {
        return Ok(false);
    };
    // 2. If C has a [[BoundTargetFunction]] internal slot, then
    if let Function::BoundFunction(c) = c {
        // a. Let BC be C.[[BoundTargetFunction]].
        let bc = agent[c].bound_target_function.bind(gc.nogc());
        // b. Return ? InstanceofOperator(O, BC).
        return instanceof_operator(agent, o, bc.unbind(), gc);
    }
    // 3. If O is not an Object, return false.
    let Ok(o) = Object::try_from(o.into_value()) else {
        return Ok(false);
    };
    // 4. Let P be ? Get(C, "prototype").
    let p = get(
        agent,
        c.unbind(),
        BUILTIN_STRING_MEMORY.prototype.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 5. If P is not an Object, throw a TypeError exception.
    let Ok(p) = Object::try_from(p) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Non-object prototype found",
            gc.into_nogc(),
        ));
    };
    // 6. Repeat,
    is_prototype_of_loop(agent, p.unbind(), o.unbind(), gc)
}

/// ### [7.3.22 SpeciesConstructor ( O, defaultConstructor )](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-speciesconstructor)
pub(crate) fn species_constructor<'a>(
    agent: &mut Agent,
    o: Object<'a>,
    default_constructor: Function<'a>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Function<'a>> {
    // 1. Let C be ? Get(O, "constructor").
    let c = get(
        agent,
        o,
        BUILTIN_STRING_MEMORY.constructor.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 2. If C is undefined, return defaultConstructor.
    if c.is_undefined() {
        return Ok(default_constructor);
    }
    // 3. If C is not an Object, throw a TypeError exception.
    let Ok(c) = Object::try_from(c) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "constructor property value is not an object",
            gc.into_nogc(),
        ));
    };
    // 4. Let S be ? Get(C, %Symbol.species%).
    let s = get(
        agent,
        c.unbind(),
        WellKnownSymbolIndexes::Species.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 5. If S is either undefined or null, return defaultConstructor.
    if s.is_undefined() || s.is_null() {
        return Ok(default_constructor);
    }
    // 6. If IsConstructor(S) is true, return S.
    if let Some(s) = is_constructor(agent, s) {
        return Ok(s.unbind());
    }
    // 7. Throw a TypeError exception.
    Err(agent.throw_exception_with_static_message(
        ExceptionType::TypeError,
        "constructor species is not a constructor",
        gc.into_nogc(),
    ))
}

pub(crate) fn is_prototype_of_loop<'a>(
    agent: &mut Agent,
    o: Object,
    v: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let mut v = v.bind(gc.nogc());
    {
        let gc = gc.nogc();
        loop {
            let proto = v.try_get_prototype_of(agent, gc);
            let TryResult::Continue(proto) = proto else {
                break;
            };
            if let Some(proto) = proto {
                v = proto;
                if o == v {
                    return Ok(true);
                }
            } else {
                return Ok(false);
            }
        }
    }
    let o = o.scope(agent, gc.nogc());
    loop {
        let proto = v
            .unbind()
            .internal_get_prototype_of(agent, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        if let Some(proto) = proto {
            v = proto;
            if o.get(agent) == v {
                return Ok(true);
            }
        } else {
            return Ok(false);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EnumPropKind {
    Value,
    KeyValue,
}

pub(crate) trait EnumerablePropertiesKind {
    const KIND: EnumPropKind;
}

pub(crate) mod enumerable_properties_kind {
    use super::{EnumPropKind, EnumerablePropertiesKind};

    pub(crate) struct EnumerateKeys;
    pub(crate) struct EnumerateValues;
    pub(crate) struct EnumerateKeysAndValues;

    impl EnumerablePropertiesKind for EnumerateValues {
        const KIND: EnumPropKind = EnumPropKind::Value;
    }

    impl EnumerablePropertiesKind for EnumerateKeysAndValues {
        const KIND: EnumPropKind = EnumPropKind::KeyValue;
    }
}

/// ### [7.3.23 EnumerableOwnKeys ( O, kind )](https://tc39.es/ecma262/#sec-enumerableownproperties)
///
/// The abstract operation EnumerableOwnKeys takes arguments O (an
/// Object) and returns either a normal completion containing a List of
/// ECMAScript property keys or a throw completion.
pub(crate) fn scoped_enumerable_own_keys<'a, 'b>(
    agent: &mut Agent,
    o: Scoped<'b, Object<'static>>,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<'a, ScopedCollection<'b, Vec<PropertyKey<'static>>>> {
    // Note: Only Proxy and possibly Module and EmbedderObject can run JS in
    // [[OwnPropertyKeys]] and [[GetOwnProperty]] calls.
    if !matches!(
        o.get(agent),
        Object::Proxy(_) | Object::Module(_) | Object::EmbedderObject(_)
    ) {
        let gc = gc.into_nogc();
        let o = o.get(agent).bind(gc);
        let keys = unwrap_try(o.try_own_property_keys(agent, gc))
            .into_iter()
            .filter_map(|key| {
                // 1. If key is a String, then
                if !key.is_string() {
                    return None;
                }
                // i. Let desc be ? O.[[GetOwnProperty]](key).
                let desc = unwrap_try(o.try_get_own_property(agent, key, gc));
                // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
                if desc?.enumerable != Some(true) {
                    return None;
                }
                // 1. If kind is KEY, then
                // a. Append key to results.
                Some(key)
            })
            .collect::<Vec<PropertyKey>>();
        return Ok(keys.scope(agent, gc));
    }
    // 1. Let ownKeys be ? O.[[OwnPropertyKeys]]().
    let own_string_keys = o
        .get(agent)
        .internal_own_property_keys(agent, gc.reborrow())
        .unbind()?
        .bind(gc.nogc())
        .into_iter()
        // 1. If key is a String, then
        .filter_map(|key| {
            if key.is_string() {
                Some(key.scope(agent, gc.nogc()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // 2. Let results be a new empty List.
    // 3. For each element key of ownKeys, do
    let results = own_string_keys
        .into_iter()
        .filter_map(|scoped_key| {
            let key = scoped_key.get(agent).bind(gc.nogc());
            // i. Let desc be ? O.[[GetOwnProperty]](key).
            let desc =
                match o
                    .get(agent)
                    .internal_get_own_property(agent, key.unbind(), gc.reborrow())
                {
                    Ok(desc) => desc,
                    // Note: Returning Some(Err(_)) makes the filter not skip
                    // errors while collecting into a single JsResult<Vec<_>>
                    // makes the collect stop if an Err(_) is found.
                    Err(err) => return Some(Err(err.unbind())),
                };
            // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
            if desc?.enumerable != Some(true) {
                // SAFETY: results are not shared.
                let _ = unsafe { scoped_key.take(agent) };
                return None;
            }
            // 1. If kind is KEY, then
            // a. Append key to results.
            Some(Ok(scoped_key))
        })
        .collect::<JsResult<Vec<_>>>()?;
    let gc = gc.into_nogc();
    let results = results
        .into_iter()
        // SAFETY: results are not shared.
        .map(|p| unsafe { p.take(agent).bind(gc) })
        .collect::<Vec<_>>();

    // 4. Return results.
    Ok(results.scope(agent, gc))
}

/// ### [7.3.23 EnumerableOwnProperties ( O, kind )](https://tc39.es/ecma262/#sec-enumerableownproperties)
///
/// The abstract operation EnumerableOwnProperties takes arguments O (an
/// Object) and kind (KEY, VALUE, or KEY+VALUE) and returns either a normal
/// completion containing a List of ECMAScript language values or a throw
/// completion.
pub(crate) fn enumerable_own_properties<'gc, Kind: EnumerablePropertiesKind>(
    agent: &mut Agent,
    o: Object,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Vec<Value<'gc>>> {
    let mut o = o.bind(gc.nogc());
    let mut scoped_o = None;
    // 1. Let ownKeys be ? O.[[OwnPropertyKeys]]().
    let mut own_keys =
        if let TryResult::Continue(own_keys) = o.try_own_property_keys(agent, gc.nogc()) {
            own_keys
        } else {
            scoped_o = Some(o.scope(agent, gc.nogc()));
            let result = o
                .unbind()
                .internal_own_property_keys(agent, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            o = scoped_o.as_ref().unwrap().get(agent).bind(gc.nogc());
            result
        };
    // 2. Let results be a new empty List.
    let mut results: Vec<Value> = Vec::with_capacity(own_keys.len());
    // 3. For each element key of ownKeys, do
    let mut broke = false;
    let mut i = 0;
    for &key in own_keys.iter() {
        if let PropertyKey::Symbol(_) = key {
            i += 1;
            continue;
        }
        // i. Let desc be ? O.[[GetOwnProperty]](key).
        let TryResult::Continue(desc) = o.try_get_own_property(agent, key, gc.nogc()) else {
            broke = true;
            break;
        };
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        let Some(desc) = desc else {
            i += 1;
            continue;
        };
        if desc.enumerable != Some(true) {
            i += 1;
            continue;
        }
        // 1. If kind is KEY, then
        // 2. Else,
        // a. Let value be ? Get(O, key).

        // Optimisation: If [[GetOwnProperty]] has returned us a Value, we
        // shouldn't need to call [[Get]] except if the object is a Proxy.
        let value = if desc.value.is_none() || matches!(o, Object::Proxy(_)) {
            if let TryResult::Continue(value) = try_get(agent, o, key, gc.nogc()) {
                value
            } else {
                broke = true;
                break;
            }
        } else {
            desc.value.unwrap()
        };
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
                    String::from_string(agent, int.to_string(), gc.nogc())
                }
                PropertyKey::SmallString(str) => str.into(),
                PropertyKey::String(str) => str.into(),
            };
            // ii. Let entry be CreateArrayFromList(« key, value »).
            let entry = create_array_from_list(agent, &[key_value.into_value(), value], gc.nogc());
            // iii. Append entry to results.
            results.push(entry.into_value());
        }
        i += 1;
    }
    if broke {
        // drop the keys we already got.
        let _ = own_keys.drain(..i);
        let scoped_o = scoped_o.unwrap_or_else(|| o.scope(agent, gc.nogc()));
        enumerable_own_properties_slow::<Kind>(
            agent,
            scoped_o,
            own_keys.unbind(),
            results.unbind(),
            gc,
        )
    } else {
        // 4. Return results.
        Ok(results.unbind().bind(gc.into_nogc()))
    }
}

fn enumerable_own_properties_slow<'gc, Kind: EnumerablePropertiesKind>(
    agent: &mut Agent,
    o: Scoped<Object>,
    own_keys: Vec<PropertyKey>,
    results: Vec<Value>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Vec<Value<'gc>>> {
    let own_keys = own_keys.scope(agent, gc.nogc());
    let mut results = results.scope(agent, gc.nogc());
    for key in own_keys.iter(agent) {
        let local_key = key.get(gc.nogc());
        if local_key.is_symbol() {
            continue;
        }
        // i. Let desc be ? O.[[GetOwnProperty]](key).
        let desc = o
            .get(agent)
            .internal_get_own_property(agent, local_key.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        let Some(desc) = desc else {
            continue;
        };
        if desc.enumerable != Some(true) {
            continue;
        }
        // 1. If kind is KEY, then
        // 2. Else,
        // a. Let value be ? Get(O, key).
        let value = get(
            agent,
            o.get(agent),
            key.get(gc.nogc()).unbind(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // b. If kind is VALUE, then
        if Kind::KIND == EnumPropKind::Value {
            // i. Append value to results.
            results.push(agent, value);
        } else {
            // c. Else,
            // i. Assert: kind is KEY+VALUE.
            debug_assert_eq!(Kind::KIND, EnumPropKind::KeyValue);
            let key_value =
                String::try_from(key.get(gc.nogc()).convert_to_value(agent, gc.nogc())).unwrap();
            // ii. Let entry be CreateArrayFromList(« key, value »).
            let entry = create_array_from_list(
                agent,
                &[key_value.into_value().unbind(), value.unbind()],
                gc.nogc(),
            );
            // iii. Append entry to results.
            results.push(agent, entry.into_value());
        }
    }
    Ok(results.take(agent))
}

/// ### [7.3.23 EnumerableOwnProperties ( O )](https://tc39.es/ecma262/#sec-enumerableownproperties)
///
/// The abstract operation EnumerableOwnProperties takes arguments O (an
/// Object) and returns either a normal completion containing a List of
/// property keys or a throw completion.
pub(crate) fn enumerable_own_keys<'gc>(
    agent: &mut Agent,
    o: Object,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
    if let Object::Object(o) = o {
        return Ok(ordinary_enumerable_own_keys(agent, o, gc.into_nogc()));
    }
    let mut o = o.bind(gc.nogc());
    let mut scoped_o = None;
    // 1. Let ownKeys be ? O.[[OwnPropertyKeys]]().
    let mut own_keys =
        if let TryResult::Continue(own_keys) = o.try_own_property_keys(agent, gc.nogc()) {
            own_keys
        } else {
            scoped_o = Some(o.scope(agent, gc.nogc()));
            let result = o
                .unbind()
                .internal_own_property_keys(agent, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            o = scoped_o.as_ref().unwrap().get(agent).bind(gc.nogc());
            result
        };
    // 2. Let results be a new empty List.
    let mut results: Vec<PropertyKey> = Vec::with_capacity(own_keys.len());
    // 3. For each element key of ownKeys, do
    let mut broke = false;
    let mut i = 0;
    for &key in own_keys.iter() {
        if let PropertyKey::Symbol(_) = key {
            i += 1;
            continue;
        }
        // i. Let desc be ? O.[[GetOwnProperty]](key).
        let TryResult::Continue(desc) = o.try_get_own_property(agent, key, gc.nogc()) else {
            broke = true;
            break;
        };
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        let Some(desc) = desc else {
            i += 1;
            continue;
        };
        if desc.enumerable != Some(true) {
            i += 1;
            continue;
        }
        // 1. If kind is KEY, then
        // a. Append key to results.
        results.push(key);
        i += 1;
    }
    if broke {
        // drop the keys we already got.
        let _ = own_keys.drain(..i);
        let scoped_o = scoped_o.unwrap_or_else(|| o.scope(agent, gc.nogc()));
        enumerable_own_keys_slow(agent, scoped_o, own_keys.unbind(), results.unbind(), gc)
    } else {
        // 4. Return results.
        Ok(results.unbind().bind(gc.into_nogc()))
    }
}

fn ordinary_enumerable_own_keys<'gc>(
    agent: &mut Agent,
    o: OrdinaryObject,
    gc: NoGcScope<'gc, '_>,
) -> Vec<PropertyKey<'gc>> {
    let ObjectHeapData { keys, values, .. } = agent[o];
    // 1. Let keys be a new empty List.
    let mut integer_keys = vec![];
    let mut result_keys = Vec::with_capacity(keys.len() as usize);

    // 3. For each own property key P of O such that P is a String and P is not an array index, in
    //    ascending chronological order of property creation, do
    for (index, key) in agent[keys].iter().enumerate() {
        // SAFETY: Keys are all PropertyKeys reinterpreted as Values without
        // conversion.
        let key = unsafe { PropertyKey::from_value_unchecked(key.unwrap()) };
        match key {
            PropertyKey::Integer(integer_key) => {
                let enumerable = agent
                    .heap
                    .elements
                    .get_descriptor(values, index)
                    .is_none_or(|desc| desc.is_enumerable());
                if !enumerable {
                    continue;
                }
                let key_value = integer_key.into_i64();
                if (0..u32::MAX as i64).contains(&key_value) {
                    // Integer property key! This requires sorting
                    integer_keys.push(key_value as u32);
                } else {
                    result_keys.push(key.bind(gc));
                }
            }
            PropertyKey::Symbol(_) => {
                // Symbol keys are never considered enumerable.
                continue;
            }
            // a. Append P to keys.
            _ => {
                let enumerable = agent
                    .heap
                    .elements
                    .get_descriptor(values, index)
                    .is_none_or(|desc| desc.is_enumerable());
                if !enumerable {
                    continue;
                }
                result_keys.push(key.bind(gc))
            }
        }
    }

    // 2. For each own property key P of O such that P is an array index,
    if !integer_keys.is_empty() {
        // in ascending numeric index order, do
        integer_keys.sort();
        // a. Append P to keys.
        result_keys.splice(0..0, integer_keys.into_iter().map(|key| key.into()));
    }

    // 5. Return keys.
    result_keys
}

fn enumerable_own_keys_slow<'gc>(
    agent: &mut Agent,
    o: Scoped<Object>,
    own_keys: Vec<PropertyKey>,
    results: Vec<PropertyKey>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
    let own_keys = own_keys.scope(agent, gc.nogc());
    let mut results = results.scope(agent, gc.nogc());
    for key in own_keys.iter(agent) {
        let local_key = key.get(gc.nogc());
        if local_key.is_symbol() {
            continue;
        }
        // i. Let desc be ? O.[[GetOwnProperty]](key).
        let desc = o
            .get(agent)
            .internal_get_own_property(agent, local_key.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        let Some(desc) = desc else {
            continue;
        };
        if desc.enumerable != Some(true) {
            continue;
        }
        // 1. If kind is KEY, then
        // a. Append key to results.
        results.push(agent, key.get(gc.nogc()));
    }
    Ok(results.take(agent))
}

/// ### [7.3.25 GetFunctionRealm ( obj )](https://tc39.es/ecma262/#sec-getfunctionrealm)
///
/// The abstract operation GetFunctionRealm takes argument obj (a function
/// object) and returns either a normal completion containing a Realm Record or
/// a throw completion.
pub(crate) fn get_function_realm<'a, 'gc>(
    agent: &mut Agent,
    obj: impl IntoObject<'a>,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, Realm<'gc>> {
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
            get_function_realm(agent, agent[idx].bound_target_function, gc)
        }
        // 3. If obj is a Proxy exotic object, then
        Object::Proxy(obj) => {
            // a. Perform ? ValidateNonRevokedProxy(obj).
            let obj = validate_non_revoked_proxy(agent, obj, gc)?;
            // b. Let proxyTarget be obj.[[ProxyTarget]].
            let proxy_target = obj.target;
            // c. Return ? GetFunctionRealm(proxyTarget).
            get_function_realm(agent, proxy_target, gc)
        }
        // 4. Return the current Realm Record.
        // NOTE: Step 4 will only be reached if obj is a non-standard function
        // exotic object that does not have a [[Realm]] internal slot.
        _ => Ok(agent.current_realm_id_internal()),
    }
}

/// ### [7.3.25 GetFunctionRealm ( obj )](https://tc39.es/ecma262/#sec-getfunctionrealm)
///
/// The abstract operation GetFunctionRealm takes argument obj (a function
/// object) and returns either a normal completion containing a Realm Record or
/// a throw completion.
///
/// NOTE: This method returns None for revoked Proxies, instead of throwing an
/// error.
pub(crate) fn try_get_function_realm<'a, 'gc>(
    agent: &Agent,
    obj: impl IntoObject<'a>,
    gc: NoGcScope<'gc, '_>,
) -> Option<Realm<'gc>> {
    // 1. If obj has a [[Realm]] internal slot, then
    // a. Return obj.[[Realm]].
    let obj = obj.into_object();
    match obj {
        Object::BuiltinFunction(idx) => Some(agent[idx].realm),
        Object::ECMAScriptFunction(idx) => Some(agent[idx].ecmascript_function.realm),
        Object::BoundFunction(idx) => {
            // 2. If obj is a bound function exotic object, then
            // a. Let boundTargetFunction be obj.[[BoundTargetFunction]].
            // b. Return ? GetFunctionRealm(boundTargetFunction).
            try_get_function_realm(agent, agent[idx].bound_target_function, gc)
        }
        // 3. If obj is a Proxy exotic object, then
        Object::Proxy(obj) => {
            // a. Perform ? ValidateNonRevokedProxy(obj).
            let obj = try_validate_non_revoked_proxy(agent, obj, gc)?;
            // b. Let proxyTarget be obj.[[ProxyTarget]].
            let proxy_target = obj.target;
            // c. Return ? GetFunctionRealm(proxyTarget).
            try_get_function_realm(agent, proxy_target, gc)
        }
        // 4. Return the current Realm Record.
        // NOTE: Step 4 will only be reached if obj is a non-standard function
        // exotic object that does not have a [[Realm]] internal slot.
        _ => Some(agent.current_realm_id_internal()),
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
pub(crate) fn copy_data_properties<'a>(
    agent: &mut Agent,
    target: OrdinaryObject,
    source: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let mut target = target.bind(gc.nogc());
    // 1. If source is either undefined or null, return unused.
    if source.is_undefined() || source.is_null() {
        return Ok(());
    }
    // 2. Let from be ! ToObject(source).
    let mut from = to_object(agent, source, gc.nogc()).unwrap();
    let mut scoped_target = None;
    let mut scoped_from = None;

    // 3. Let keys be ? from.[[OwnPropertyKeys]]().
    let mut keys = if let TryResult::Continue(keys) = from.try_own_property_keys(agent, gc.nogc()) {
        keys
    } else {
        scoped_target = Some(target.scope(agent, gc.nogc()));
        scoped_from = Some(from.scope(agent, gc.nogc()));
        let keys = from
            .unbind()
            .internal_own_property_keys(agent, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        target = scoped_target.as_ref().unwrap().get(agent).bind(gc.nogc());
        from = scoped_from.as_ref().unwrap().get(agent).bind(gc.nogc());
        keys
    };
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
    let mut broke = false;
    let mut i = 0;
    for &next_key in keys.iter() {
        // i. Let desc be ? from.[[GetOwnProperty]](nextKey).
        let TryResult::Continue(dest) = from.try_get_own_property(agent, next_key, gc.nogc())
        else {
            broke = true;
            break;
        };
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        if let Some(dest) = dest {
            if dest.enumerable.unwrap() {
                // 1. Let propValue be ? Get(from, nextKey).
                let TryResult::Continue(prop_value) = try_get(agent, from, next_key, gc.nogc())
                else {
                    broke = true;
                    break;
                };
                // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue).
                assert!(
                    try_create_data_property(agent, target, next_key, prop_value, gc.nogc())
                        .is_continue()
                );
            }
        }
        i += 1;
    }

    if broke {
        let _ = keys.drain(..i);
        let target = scoped_target.unwrap_or_else(|| target.scope(agent, gc.nogc()));
        let from = scoped_from.unwrap_or_else(|| from.scope(agent, gc.nogc()));
        copy_data_properties_slow(agent, target, from, keys.unbind(), gc)
    } else {
        // 5. Return UNUSED.
        Ok(())
    }
}

fn copy_data_properties_slow<'a>(
    agent: &mut Agent,
    target: Scoped<OrdinaryObject>,
    from: Scoped<Object>,
    keys: Vec<PropertyKey>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let keys = keys.scope(agent, gc.nogc());
    for next_key in keys.iter(agent) {
        // i. Let desc be ? from.[[GetOwnProperty]](nextKey).
        // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        if let Some(dest) = from
            .get(agent)
            .internal_get_own_property(agent, next_key.get(gc.nogc()).unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
        {
            if dest.enumerable.unwrap() {
                // 1. Let propValue be ? Get(from, nextKey).
                let prop_value = get(
                    agent,
                    from.get(agent),
                    next_key.get(gc.nogc()).unbind(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue).
                unwrap_try(try_create_data_property(
                    agent,
                    target.get(agent),
                    next_key.get(gc.nogc()).unbind(),
                    prop_value,
                    gc.nogc(),
                ));
            }
        }
    }

    Ok(())
}

/// ### Try [7.3.25 CopyDataProperties ( target, source, excludedItems )](https://tc39.es/ecma262/#sec-copydataproperties)
/// The abstract operation CopyDataProperties takes arguments target (an Object), source (an
/// ECMAScript language value), and excludedItems (a List of property keys) and returns either a
/// normal completion containing unused or a throw completion.
///
/// NOTE: This implementation of CopyDataProperties also creates the target object with
/// `OrdinaryObjectCreate(%Object.prototype%)`. This can be used to implement the rest operator in
/// object destructuring, but not the spread operator in object literals.
pub(crate) fn try_copy_data_properties_into_object<'a, 'b>(
    agent: &mut Agent,
    source: impl IntoObject<'b>,
    excluded_items: &PropertyKeySet,
    gc: NoGcScope<'a, '_>,
) -> TryResult<OrdinaryObject<'a>> {
    let from = source.into_object();
    let mut entries = Vec::new();

    // 3. Let keys be ? from.[[OwnPropertyKeys]]().
    // 4. For each element nextKey of keys, do
    for next_key in from.try_own_property_keys(agent, gc)? {
        // a. Let excluded be false.
        // b. For each element e of excludedItems, do
        //   i. If SameValue(e, nextKey) is true, then
        //     1. Set excluded to true.
        if excluded_items.contains(agent, next_key) {
            continue;
        }

        // c. If excluded is false, then
        //   i. Let desc be ? from.[[GetOwnProperty]](nextKey).
        //   ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        if let Some(dest) = from.try_get_own_property(agent, next_key, gc)? {
            if dest.enumerable.unwrap() {
                // 1. Let propValue be ? Get(from, nextKey).
                let prop_value = if let Some(prop_value) = dest.value {
                    prop_value
                } else {
                    try_get(agent, from, next_key, gc)?
                };
                // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue).
                entries.push(ObjectEntry::new_data_entry(next_key, prop_value));
            }
        }
    }

    TryResult::Continue(
        agent.heap.create_object_with_prototype(
            agent
                .current_realm_record()
                .intrinsics()
                .object_prototype()
                .into_object(),
            &entries,
        ),
    )
}

/// ### [7.3.25 CopyDataProperties ( target, source, excludedItems )](https://tc39.es/ecma262/#sec-copydataproperties)
/// The abstract operation CopyDataProperties takes arguments target (an Object), source (an
/// ECMAScript language value), and excludedItems (a List of property keys) and returns either a
/// normal completion containing unused or a throw completion.
///
/// NOTE: This implementation of CopyDataProperties also creates the target object with
/// `OrdinaryObjectCreate(%Object.prototype%)`. This can be used to implement the rest operator in
/// object destructuring, but not the spread operator in object literals.
pub(crate) fn copy_data_properties_into_object<'a, 'b>(
    agent: &mut Agent,
    source: impl IntoObject<'b>,
    excluded_items: ScopedCollection<PropertyKeySet>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, OrdinaryObject<'a>> {
    let from = source.into_object().bind(gc.nogc());
    let scoped_from = from.scope(agent, gc.nogc());
    let mut entries = Vec::new();

    // 3. Let keys be ? from.[[OwnPropertyKeys]]().
    let mut keys = from
        .unbind()
        .internal_own_property_keys(agent, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    let from = scoped_from.get(agent).bind(gc.nogc());
    // 4. For each element nextKey of keys, do
    let mut broke = false;
    let mut i = 0;
    for next_key in keys.iter() {
        // a. Let excluded be false.
        // b. For each element e of excludedItems, do
        //   i. If SameValue(e, nextKey) is true, then
        //     1. Set excluded to true.
        if excluded_items.contains(agent, *next_key) {
            i += 1;
            continue;
        }

        let next_key = *next_key;

        // c. If excluded is false, then
        //   i. Let desc be ? from.[[GetOwnProperty]](nextKey).
        let TryResult::Continue(desc) = from.try_get_own_property(agent, next_key, gc.nogc())
        else {
            broke = true;
            break;
        };
        //   ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        if let Some(desc) = desc {
            if desc.enumerable.unwrap() {
                // 1. Let propValue be ? Get(from, nextKey).
                let TryResult::Continue(prop_value) =
                    try_get(agent, from.unbind(), next_key, gc.nogc())
                else {
                    broke = true;
                    break;
                };
                // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue).
                entries.push(ObjectEntry::new_data_entry(next_key, prop_value));
            }
        }
        i += 1;
    }

    let object = agent
        .heap
        .create_object_with_prototype(
            agent
                .current_realm_record()
                .intrinsics()
                .object_prototype()
                .into_object(),
            &entries,
        )
        .bind(gc.nogc());

    if broke {
        let _ = keys.drain(..i);
        copy_data_properties_into_object_slow(
            agent,
            scoped_from,
            excluded_items,
            keys.unbind(),
            object.unbind(),
            gc,
        )
    } else {
        // Drop the excluded items set.
        let _ = excluded_items.take(agent);
        Ok(object.unbind())
    }
}

fn copy_data_properties_into_object_slow<'a>(
    agent: &mut Agent,
    from: Scoped<Object>,
    excluded_items: ScopedCollection<PropertyKeySet>,
    keys: Vec<PropertyKey>,
    object: OrdinaryObject,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, OrdinaryObject<'a>> {
    let keys = keys.scope(agent, gc.nogc());
    let object = object.scope(agent, gc.nogc());
    for next_key in keys.iter(agent) {
        // a. Let excluded be false.
        // b. For each element e of excludedItems, do
        //   i. If SameValue(e, nextKey) is true, then
        //     1. Set excluded to true.
        let local_next_key = next_key.get(gc.nogc());
        if excluded_items.contains(agent, local_next_key) {
            continue;
        }

        // c. If excluded is false, then
        //   i. Let desc be ? from.[[GetOwnProperty]](nextKey).
        //   ii. If desc is not undefined and desc.[[Enumerable]] is true, then
        if let Some(desc) = from
            .get(agent)
            .internal_get_own_property(agent, local_next_key.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
        {
            if desc.enumerable.unwrap() {
                // 1. Let propValue be ? Get(from, nextKey).
                let prop_value = get(
                    agent,
                    from.get(agent),
                    next_key.get(gc.nogc()).unbind(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue).
                unwrap_try(try_create_data_property_or_throw(
                    agent,
                    object.get(agent),
                    next_key.get(gc.nogc()),
                    prop_value,
                    gc.nogc(),
                ))
                .unwrap();
            }
        }
    }
    // Drop the excluded items set.
    let _ = excluded_items.take(agent);
    Ok(object.get(agent).bind(gc.into_nogc()))
}

/// [7.3.33 InitializeInstanceElements ( O, constructor )](https://tc39.es/ecma262/#sec-initializeinstanceelements)
///
/// The abstract operation InitializeInstanceElements takes arguments O (an
/// Object) and constructor (an ECMAScript function object) and returns either
/// a normal completion containing unused or a throw completion.
pub(crate) fn initialize_instance_elements<'a>(
    agent: &mut Agent,
    o: Object,
    constructor: BuiltinConstructorFunction,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let o = o.bind(gc.nogc());
    let constructor = constructor.bind(gc.nogc());
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
        // run one after the other. Instead we compile all of the initializers
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
        let decl_env = new_class_field_initializer_environment(agent, f, o, outer_env, gc.nogc());
        agent.push_execution_context(ExecutionContext {
            ecmascript_code: Some(ECMAScriptCodeEvaluationState {
                lexical_environment: Environment::Function(decl_env.unbind()),
                variable_environment: Environment::Function(decl_env.unbind()),
                private_environment: outer_priv_env,
                is_strict_mode: true,
                source_code,
            }),
            function: Some(f.unbind()),
            realm: agent[constructor].realm,
            script_or_module: None,
        });
        let bytecode = bytecode.scope(agent, gc.nogc());
        let result = Vm::execute(agent, bytecode, None, gc).into_js_result();
        agent.pop_execution_context();
        result?;
    }
    Ok(())
}

/// ### [7.3.34 AddValueToKeyedGroup ( groups, key, value )](https://tc39.es/ecma262/#sec-add-value-to-keyed-group)
/// The abstract operation AddValueToKeyedGroup takes arguments groups (a List of Records with fields
/// [[Key]] (an ECMAScript language value) and [[Elements]] (a List of ECMAScript language values)),
/// key (an ECMAScript language value), and value (an ECMAScript language value) and returns UNUSED.
pub(crate) fn add_value_to_keyed_group<K: 'static + Rootable + Copy + Into<Value<'static>>>(
    agent: &mut Agent,
    groups: &mut ScopedCollection<Box<KeyedGroup>>,
    key: K,
    value: Value,
) {
    // 1. For each Record { [[Key]], [[Elements]] } g of groups, do
    // a. If SameValue(g.[[Key]], key) is true, then
    // i. Assert: Exactly one element of groups meets this criterion.
    // ii. Append value to g.[[Elements]].
    // iii. Return UNUSED.
    // 2. Let group be the Record { [[Key]]: key, [[Elements]]: « value » }.
    // 3. Append group to groups.
    if core::any::TypeId::of::<K>() == core::any::TypeId::of::<PropertyKey>() {
        // SAFETY: K is PropertyKey, so it is safe to transmute_copy.
        let key = unsafe { core::mem::transmute_copy::<K, PropertyKey>(&key) };
        groups.add_property_keyed_value(agent, key, value);
    } else if core::any::TypeId::of::<K>() == core::any::TypeId::of::<Value>() {
        // SAFETY: K is Value, so it is safe to transmute_copy.
        let key = unsafe { core::mem::transmute_copy::<K, Value>(&key) };
        groups.add_collection_keyed_value(agent, key, value);
    } else {
        unreachable!()
    }

    // 4. Return UNUSED.
}

/// ### [7.3.35 GroupBy ( items, callback, keyCoercion )](https://tc39.es/ecma262/#sec-groupby)
///
/// The abstract operation GroupBy takes arguments items (an ECMAScript language value), callback
/// (an ECMAScript language value), and keyCoercion (property or collection) and returns either a
/// normal completion containing a List of Records with fields [[Key]] (an ECMAScript language
/// value) and [[Elements]] (a List of ECMAScript language values), or a throw completion.
///
/// Note: This version is for "property" keyCoercion.
pub(crate) fn group_by_property<'gc>(
    agent: &mut Agent,
    items: Value,
    callback_fn: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Box<KeyedGroup<'gc>>> {
    let items = items.bind(gc.nogc());
    let callback_fn = callback_fn.bind(gc.nogc());
    // 1. Perform ? RequireObjectCoercible(iterable).
    require_object_coercible(agent, items, gc.nogc())
        .unbind()?
        .bind(gc.nogc());

    // 2. If IsCallable(callback) is false, throw a TypeError exception.
    let Some(callback_fn) = is_callable(callback_fn, gc.nogc()) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Callback is not callable",
            gc.into_nogc(),
        ));
    };
    let callback_fn = callback_fn.scope(agent, gc.nogc());

    // 3. Let groups be a new empty List.
    let mut groups = KeyedGroup::new(gc.nogc()).scope(agent, gc.nogc());

    // 4. Let iteratorRecord be ? GetIterator(iterable).
    let Some(IteratorRecord {
        iterator,
        next_method,
        ..
    }) = get_iterator(agent, items.unbind(), false, gc.reborrow())
        .unbind()?
        .bind(gc.nogc())
    else {
        return Err(throw_not_callable(agent, gc.into_nogc()));
    };

    let iterator = iterator.scope(agent, gc.nogc());
    let next_method = next_method.scope(agent, gc.nogc());

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
                gc.nogc(),
            );

            // ii. Return ? IteratorClose(iteratorRecord, error).
            return Err(iterator_close_with_error(
                agent,
                iterator.get(agent),
                error.unbind(),
                gc,
            ));
        }

        // b. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(
            agent,
            IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());

        // c. If next is DONE, then
        //   i. Return groups.
        let Some(next) = next else {
            return Ok(groups.take(agent));
        };

        // d. Let value be next.
        let value = next.unbind().bind(gc.nogc());
        let scoped_value = value.scope(agent, gc.nogc());

        // 𝔽(k)
        let fk = Number::try_from(k).unwrap().into_value();

        // e. Let key be Completion(Call(callback, undefined, « value, 𝔽(k) »)).
        let key = call_function(
            agent,
            callback_fn.get(agent),
            Value::Undefined,
            Some(ArgumentsList::from_mut_slice(&mut [value.unbind(), fk])),
            gc.reborrow(),
        );

        // f. IfAbruptCloseIterator(key, iteratorRecord).
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent),
            next_method: next_method.get(agent),
        };
        let key = if_abrupt_close_iterator!(agent, key, iterator_record, gc);

        // g. If keyCoercion is property, then
        // i. Set key to Completion(ToPropertyKey(key)).
        let key = to_property_key(agent, key.unbind(), gc.reborrow()).unbind();

        // ii. IfAbruptCloseIterator(key, iteratorRecord).
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent),
            next_method: next_method.get(agent),
        };
        let key = if_abrupt_close_iterator!(agent, key, iterator_record, gc);

        // SAFETY: Not shared.
        let value = unsafe { scoped_value.take(agent) };
        // i. Perform AddValueToKeyedGroup(groups, key, value).
        add_value_to_keyed_group(agent, &mut groups, key.unbind(), value);

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
pub(crate) fn group_by_collection<'gc>(
    agent: &mut Agent,
    items: Value,
    callback_fn: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Box<KeyedGroup<'gc>>> {
    let items = items.bind(gc.nogc());
    let callback_fn = callback_fn.bind(gc.nogc());
    // 1. Perform ? RequireObjectCoercible(iterable).
    require_object_coercible(agent, items, gc.nogc())
        .unbind()?
        .bind(gc.nogc());

    // 2. If IsCallable(callback) is false, throw a TypeError exception.
    let Some(callback_fn) = is_callable(callback_fn, gc.nogc()) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Callback is not callable",
            gc.into_nogc(),
        ));
    };
    let callback_fn = callback_fn.scope(agent, gc.nogc());

    // 3. Let groups be a new empty List.
    let mut groups = KeyedGroup::new(gc.nogc()).scope(agent, gc.nogc());

    // 4. Let iteratorRecord be ? GetIterator(iterable).
    let Some(IteratorRecord {
        iterator,
        next_method,
        ..
    }) = get_iterator(agent, items.unbind(), false, gc.reborrow())
        .unbind()?
        .bind(gc.nogc())
    else {
        return Err(throw_not_callable(agent, gc.into_nogc()));
    };

    let iterator = iterator.scope(agent, gc.nogc());
    let next_method = next_method.scope(agent, gc.nogc());

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
                gc.nogc(),
            );

            // ii. Return ? IteratorClose(iteratorRecord, error).
            return Err(iterator_close_with_error(
                agent,
                iterator.get(agent),
                error.unbind(),
                gc,
            ));
        }

        // b. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(
            agent,
            IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());

        // c. If next is DONE, then
        //   i. Return groups.
        let Some(next) = next else {
            return Ok(groups.take(agent));
        };

        // d. Let value be next.
        let value = next.unbind().bind(gc.nogc());
        let scoped_value = value.scope(agent, gc.nogc());

        // 𝔽(k)
        let fk = Number::try_from(k).unwrap().into_value();

        // e. Let key be Completion(Call(callback, undefined, « value, 𝔽(k) »)).
        let key = call_function(
            agent,
            callback_fn.get(agent),
            Value::Undefined,
            Some(ArgumentsList::from_mut_slice(&mut [value.unbind(), fk])),
            gc.reborrow(),
        )
        .unbind()
        .bind(gc.nogc());

        // f. IfAbruptCloseIterator(key, iteratorRecord).
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent),
            next_method: next_method.get(agent),
        };
        let key = if_abrupt_close_iterator!(agent, key, iterator_record, gc);

        // h. Else,
        // i. Assert: keyCoercion is collection.
        // ii. Set key to CanonicalizeKeyedCollectionKey(key).
        let key = canonicalize_keyed_collection_key(agent, key);

        // SAFETY: Not shared.
        let value = unsafe { scoped_value.take(agent) };
        // i. Perform AddValueToKeyedGroup(groups, key, value).
        add_value_to_keyed_group(agent, &mut groups, key.unbind(), value);

        // j. Set k to k + 1.
        k += 1;
    }
}
