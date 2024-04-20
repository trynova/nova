//! ## [7.3 Operations on Objects](https://tc39.es/ecma262/#sec-operations-on-objects)

use super::{
    testing_and_comparison::{is_callable, same_value},
    type_conversion::to_object,
};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, BuiltinFunction, ECMAScriptFunction},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{
            Function, InternalMethods, IntoObject, Object, PropertyDescriptor, PropertyKey, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::GetHeapData,
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
pub(crate) fn get(agent: &mut Agent, o: Object, p: PropertyKey) -> JsResult<Value> {
    // 1. Return ? O.[[Get]](P, O).
    o.internal_get(agent, p, o.into())
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
        return Err(agent.throw_exception(ExceptionType::TypeError, "Could not set property."));
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
/// to the same defaults used for properties created by the ECMAScript language
/// assignment operator. Normally, the property will not already exist. If it
/// does exist and is not configurable or if O is not extensible,
/// \[\[DefineOwnProperty]] will return false.
pub(crate) fn create_data_property(
    agent: &mut Agent,
    object: Object,
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
    object: Object,
    property_key: PropertyKey,
    value: Value,
) -> JsResult<()> {
    let success = create_data_property(agent, object, property_key, value)?;
    if !success {
        Err(agent.throw_exception(ExceptionType::TypeError, "Could not create property"))
    } else {
        Ok(())
    }
}

/// ### [7.3.9 DefinePropertyOrThrow ( O, P, desc )](https://tc39.es/ecma262/#sec-definepropertyorthrow)
///
/// The abstract operation DefinePropertyOrThrow takes arguments O (an Object),
/// P (a property key), and desc (a Property Descriptor) and returns either a
/// normal completion containing UNUSED or a throw completion. It is used to
/// call the \[\[DefineOwnProperty]] internal method of an object in a manner
/// that will throw a TypeError exception if the requested property update
/// cannot be performed.
pub(crate) fn define_property_or_throw(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    desc: PropertyDescriptor,
) -> JsResult<()> {
    // 1. Let success be ? O.[[DefineOwnProperty]](P, desc).
    let success = object.internal_define_own_property(agent, property_key, desc)?;
    // 2. If success is false, throw a TypeError exception.
    if !success {
        Err(agent.throw_exception(
            ExceptionType::TypeError,
            "Failed to defined property on object",
        ))
    } else {
        // 3. Return UNUSED.
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
    if !is_callable(func) {
        return Err(agent.throw_exception(ExceptionType::TypeError, "Not a callable object"));
    }
    // 4. Return func.
    match func {
        Value::BoundFunction(idx) => Ok(Some(Function::from(idx))),
        Value::BuiltinFunction(idx) => Ok(Some(Function::from(idx))),
        Value::ECMAScriptFunction(idx) => Ok(Some(Function::from(idx))),
        _ => unreachable!(),
    }
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
    if !is_callable(f) {
        Err(agent.throw_exception(ExceptionType::TypeError, "Not a callable object"))
    } else {
        // 3. Return ? F.[[Call]](V, argumentsList).
        match f {
            Value::BoundFunction(idx) => {
                Function::from(idx).internal_call(agent, v, arguments_list)
            }
            Value::BuiltinFunction(idx) => {
                BuiltinFunction::from(idx).internal_call(agent, v, arguments_list)
            }
            Value::ECMAScriptFunction(idx) => {
                ECMAScriptFunction::from(idx).internal_call(agent, v, arguments_list)
            }
            _ => unreachable!(),
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
pub(crate) fn ordinary_has_instance(agent: &mut Agent, c: Value, o: Value) -> JsResult<bool> {
    // 1. If IsCallable(C) is false, return false.
    if !is_callable(c) {
        return Ok(false);
    }
    let c = Object::try_from(c).unwrap();
    // 2. If C has a [[BoundTargetFunction]] internal slot, then
    if let Object::BoundFunction(idx) = c {
        // a. Let BC be C.[[BoundTargetFunction]].
        // b. Return ? InstanceofOperator(O, BC).
        let _bc = agent.heap.get(idx).function;
        // return instance_of_operator(o, bc);
    }
    // 3. If O is not an Object, return false.
    let Ok(mut o) = Object::try_from(o) else {
        return Ok(false);
    };
    // 4. Let P be ? Get(C, "prototype").
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.prototype);
    let p = get(agent, c, key)?;
    // 5. If P is not an Object, throw a TypeError exception.
    let Ok(p) = Object::try_from(p) else {
        return Err(agent.throw_exception(ExceptionType::TypeError, "Non-object prototype found"));
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
        Object::BuiltinFunction(idx) => Ok(agent.heap.get(idx).realm),
        Object::ECMAScriptFunction(idx) => Ok(agent.heap.get(idx).ecmascript_function.realm),
        Object::BoundFunction(idx) => {
            // 2. If obj is a bound function exotic object, then
            // a. Let boundTargetFunction be obj.[[BoundTargetFunction]].
            // b. Return ? GetFunctionRealm(boundTargetFunction).
            get_function_realm(agent, agent.heap.get(idx).function)
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
