//! ## [7.3 Operations on Objects](https://tc39.es/ecma262/#sec-operations-on-objects)

use super::{testing_and_comparison::is_callable, type_conversion::to_object};
use crate::ecmascript::{
    execution::{agent::JsError, Agent, JsResult},
    types::{Function, InternalMethods, Object, PropertyKey, Value},
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
pub(crate) fn make_basic_object(agent: &mut Agent, internal_slots_list: ()) -> Object {
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
    o.get(agent, p, o.into())
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
    o.get(agent, p, o.into())
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
        return Err(JsError {});
    }
    // 4. Return func.
    match func {
        Value::Function(idx) => Ok(Some(Function::from(idx))),
        _ => unreachable!(),
    }
}

/// ### [7.3.14 Call ( F, V \[ , argumentsList \] )](https://tc39.es/ecma262/#sec-call)
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
    arguments_list: Option<&[Value]>,
) -> JsResult<Value> {
    // 1. If argumentsList is not present, set argumentsList to a new empty List.
    let arguments_list = arguments_list.unwrap_or(&[]);
    // 2. If IsCallable(F) is false, throw a TypeError exception.
    if !is_callable(f) {
        Err(JsError {})
    } else {
        // 3. Return ? F.[[Call]](V, argumentsList).
        if let Value::Function(idx) = f {
            Function::from(idx).call(agent, v, arguments_list)
        } else {
            unreachable!();
        }
    }
}

/// Abstract operation Call specialized for a Function.
pub(crate) fn call_function(
    agent: &mut Agent,
    f: Function,
    v: Value,
    arguments_list: Option<&[Value]>,
) -> JsResult<Value> {
    let arguments_list = arguments_list.unwrap_or(&[]);
    f.call(agent, v, arguments_list)
}
