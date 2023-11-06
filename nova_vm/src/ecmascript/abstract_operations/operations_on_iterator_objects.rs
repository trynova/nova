//! ## [7.4 Operations on Iterator Objects](https://tc39.es/ecma262/#sec-operations-on-iterator-objects)

use super::{
    operations_on_objects::{call, get},
    type_conversion::{to_boolean, to_object},
};
use crate::ecmascript::{
    execution::{agent::ExceptionType, Agent, JsResult},
    types::{Function, Object, PropertyKey, Value},
};

/// [7.4.2 GetIteratorFromMethod ( obj, method )](https://tc39.es/ecma262/#sec-getiteratorfrommethod)
///
/// The abstract operation GetIteratorFromMethod takes arguments obj (an
/// ECMAScript language value) and method (a function object) and returns
/// either a normal completion containing an Iterator Record or a throw
/// completion.
pub(crate) fn get_iterator_from_method(
    agent: &mut Agent,
    obj: Value,
    method: Function,
) -> JsResult<Object> {
    // 1. Let iterator be ? Call(method, obj).
    let iterator = call(agent, method.into(), obj, None)?;

    // 2. If iterator is not an Object, throw a TypeError exception.
    let iterator = if let Ok(iterator) = to_object(agent, iterator) {
        iterator
    } else {
        return Err(agent.throw_exception(ExceptionType::TypeError, "Iterator is not object"));
    };

    // 3. Let nextMethod be ? Get(iterator, "next").
    let property = PropertyKey::from_str(&mut agent.heap, "next");
    let next_method = get(agent, iterator, property)?;

    // 4. Let iteratorRecord be the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
    // 5. Return iteratorRecord.
    todo!()
}

/// [7.4.3 GetIterator ( obj, kind )](https://tc39.es/ecma262/#sec-getiterator)
///
/// The abstract operation GetIterator takes arguments obj (an ECMAScript
/// language value) and kind (sync or async) and returns either a normal
/// completion containing an Iterator Record or a throw completion.
pub(crate) fn get_iterator(agent: &mut Agent, obj: Value, is_async: bool) -> JsResult<Object> {
    // 1. If kind is async, then
    // a. Let method be ? GetMethod(obj, @@asyncIterator).
    // b. If method is undefined, then
    // i. Let syncMethod be ? GetMethod(obj, @@iterator).
    // ii. If syncMethod is undefined, throw a TypeError exception.
    // iii. Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
    // iv. Return CreateAsyncFromSyncIterator(syncIteratorRecord).
    // 2. Else,
    // a. Let method be ? GetMethod(obj, @@iterator).
    // 3. If method is undefined, throw a TypeError exception.
    // 4. Return ? GetIteratorFromMethod(obj, method).
    todo!()
}

/// [7.4.4 IteratorNext ( iteratorRecord [ , value ] )](https://tc39.es/ecma262/#sec-iteratornext)
///
/// The abstract operation IteratorNext takes argument iteratorRecord (an
/// Iterator Record) and optional argument value (an ECMAScript language value)
/// and returns either a normal completion containing an Object or a throw
/// completion.
pub(crate) fn iterator_next(
    agent: &mut Agent,
    iterator_record: Object,
    value: Option<Value>,
) -> JsResult<Object> {
    // 1. If value is not present, then
    // a. Let result be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
    // 2. Else,
    // a. Let result be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « value »).
    // 3. If result is not an Object, throw a TypeError exception.
    // 4. Return result.
    todo!()
}

/// [7.4.5 IteratorComplete ( iterResult )](https://tc39.es/ecma262/#sec-iteratorcomplete)
///
/// The abstract operation IteratorComplete takes argument iterResult (an
/// Object) and returns either a normal completion containing a Boolean or a
/// throw completion.
pub(crate) fn iterator_complete(agent: &mut Agent, iter_result: Object) -> JsResult<Value> {
    // 1. Return ToBoolean(? Get(iterResult, "done")).
    let property = PropertyKey::from_str(&mut agent.heap, "done");
    let done = get(agent, iter_result, property)?;
    to_boolean(agent, done)
}

/// [7.4.6 IteratorValue ( iterResult )](https://tc39.es/ecma262/#sec-iteratorvalue)
///
/// The abstract operation IteratorValue takes argument iterResult (an
/// Object) and returns either a normal completion containing an ECMAScript
/// language value or a throw completion.
pub(crate) fn iterator_value(agent: &mut Agent, iter_result: Object) -> JsResult<Value> {
    // 1. Return ? Get(iterResult, "value").
    let property = PropertyKey::from_str(&mut agent.heap, "value");
    get(agent, iter_result, property)
}
