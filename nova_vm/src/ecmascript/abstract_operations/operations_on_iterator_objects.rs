//! ## [7.4 Operations on Iterator Objects](https://tc39.es/ecma262/#sec-operations-on-iterator-objects)

use super::{
    operations_on_objects::{call, get},
    type_conversion::{to_boolean, to_object},
};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::get_method,
        execution::{
            agent::{ExceptionType, JsError},
            Agent, JsResult,
        },
        types::{Function, Object, OrdinaryObject, PropertyKey, Value},
    },
    heap::WellKnownSymbolIndexes,
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
    let Ok(iterator) = to_object(agent, iterator) else {
        return Err(agent.throw_exception(ExceptionType::TypeError, "Iterator is not an object"));
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
    let method = if is_async {
        // a. Let method be ? GetMethod(obj, @@asyncIterator).
        let method = get_method(
            agent,
            obj,
            PropertyKey::Symbol(WellKnownSymbolIndexes::AsyncIterator.into()),
        )?;

        // b. If method is undefined, then
        if method.is_none() {
            // i. Let syncMethod be ? GetMethod(obj, @@iterator).
            let Some(sync_method) = get_method(
                agent,
                obj,
                PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
            )?
            else {
                // ii. If syncMethod is undefined, throw a TypeError exception.
                return Err(
                    agent.throw_exception(ExceptionType::TypeError, "No iterator on object")
                );
            };

            // iii. Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
            let sync_iterator_record = get_iterator_from_method(agent, obj, sync_method)?;

            // iv. Return CreateAsyncFromSyncIterator(syncIteratorRecord).
            todo!("Implement create_async_from_sync_iterator(sync_iterator_record)")
        } else {
            method
        }
    } else {
        // 2. Else,
        // a. Let method be ? GetMethod(obj, @@iterator).
        get_method(
            agent,
            obj,
            PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
        )?
    };

    // 3. If method is undefined, throw a TypeError exception.
    let Some(method) = method else {
        Err(agent.throw_exception(
            ExceptionType::TypeError,
            "Iterator method cannot be undefined",
        ))
    };

    // 4. Return ? GetIteratorFromMethod(obj, method).
    get_iterator_from_method(agent, obj, method)
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

/// [7.4.7 IteratorStep ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstep)
///
/// The abstract operation IteratorStep takes argument iteratorRecord (an
/// Iterator Record) and returns either a normal completion containing either
/// an Object or false, or a throw completion. It requests the next value from
/// iteratorRecord.\[\[Iterator\]\] by calling iteratorRecord.\[\[NextMethod\]\]
/// and returns either false indicating that the iterator has reached its end
/// or the IteratorResult object if a next value is available.
pub(crate) fn iterator_step(agent: &mut Agent, iterator_record: Object) -> JsResult<Value> {
    // 1. Let result be ? IteratorNext(iteratorRecord).
    let result = iterator_next(agent, iterator_record, None)?;

    // 2. Let done be ? IteratorComplete(result).
    let done = iterator_complete(agent, result)?;

    // 3. If done is true, return false.
    if done.is_true() {
        return Ok(Value::Boolean(false));
    }

    // 4. Return result.
    Ok(result.into())
}

/// [7.4.8 IteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-iteratorclose)
///
/// The abstract operation IteratorClose takes arguments iteratorRecord (an
/// Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an iterator that it should perform
/// any actions it would normally perform when it has reached its completed state.
pub(crate) fn iterator_close(
    agent: &mut Agent,
    iterator_record: Object,
    completion: JsResult<Value>,
) -> JsResult<Value> {
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    // 4. If innerResult.[[Type]] is normal, then
    // a. Let return be innerResult.[[Value]].
    // b. If return is undefined, return ? completion.
    // c. Set innerResult to Completion(Call(return, iterator)).
    // 5. If completion.[[Type]] is throw, return ? completion.
    // 6. If innerResult.[[Type]] is throw, return ? innerResult.
    // 7. If innerResult.[[Value]] is not an Object, throw a TypeError exception.
    // 8. Return ? completion.
    todo!()
}

/// [7.4.9 IfAbruptCloseIterator ( value, iteratorRecord )](https://tc39.es/ecma262/#sec-ifabruptcloseiterator)
///
/// IfAbruptCloseIterator is a shorthand for a sequence of algorithm steps that use an Iterator Record.
pub(crate) fn if_abrupt_close_iterator(
    agent: &mut Agent,
    value: JsResult<Value>,
    iterator_record: Object,
) -> JsResult<Value> {
    // 1. Assert: value is a Completion Record.
    // 2. If value is an abrupt completion, return ? IteratorClose(iteratorRecord, value).
    // 3. Else, set value to value.[[Value]].
    todo!()
}

/// [7.4.10 AsyncIteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-asynciteratorclose)
///
/// The abstract operation AsyncIteratorClose takes arguments iteratorRecord
/// (an Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an async iterator that it should
/// perform any actions it would normally perform when it has reached its
/// completed state.
pub(crate) fn async_iterator_close(
    agent: &mut Agent,
    iterator_record: Object,
    completion: JsResult<Value>,
) -> JsResult<Value> {
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    // 4. If innerResult.[[Type]] is normal, then
    // a. Let return be innerResult.[[Value]].
    // b. If return is undefined, return ? completion.
    // c. Set innerResult to Completion(Call(return, iterator)).
    // d. If innerResult.[[Type]] is normal, set innerResult to Completion(Await(innerResult.[[Value]])).
    // 5. If completion.[[Type]] is throw, return ? completion.
    // 6. If innerResult.[[Type]] is throw, return ? innerResult.
    // 7. If innerResult.[[Value]] is not an Object, throw a TypeError exception.
    // 8. Return ? completion.
    todo!()
}

/// [7.4.11 CreateIterResultObject ( value, done )](https://tc39.es/ecma262/#sec-createiterresultobject)
///
/// The abstract operation CreateIterResultObject takes arguments value (an
/// ECMAScript language value) and done (a Boolean) and returns an Object that
/// conforms to the IteratorResult interface. It creates an object that conforms
/// to the IteratorResult interface.
pub(crate) fn create_iter_result_object(agent: &mut Agent, value: Value, done: bool) -> Value {
    // 1. Let obj be OrdinaryObjectCreate(%Object.prototype%).
    // 2. Perform ! CreateDataPropertyOrThrow(obj, "value", value).
    // 3. Perform ! CreateDataPropertyOrThrow(obj, "done", done).
    // 4. Return obj.
    todo!()
}

/// [7.4.12 CreateListIteratorRecord ( list )](https://tc39.es/ecma262/#sec-createlistiteratorRecord)
///
/// The abstract operation CreateListIteratorRecord takes argument list (a List
/// of ECMAScript language values) and returns an Iterator Record. It creates
/// an Iterator (27.1.1.2) object record whose next method returns the
/// successive elements of list.
pub(crate) fn create_list_iterator_record(agent: &mut Agent, list: &[Value]) -> JsResult<Value> {
    // 1. Let closure be a new Abstract Closure with no parameters that captures list and performs the following steps when called:
    // a. For each element E of list, do
    // i. Perform ? GeneratorYield(CreateIterResultObject(E, false)).
    // b. Return NormalCompletion(undefined).
    // 2. Let iterator be CreateIteratorFromClosure(closure, empty, %IteratorPrototype%).
    // 3. Return the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: %GeneratorFunction.prototype.prototype.next%, [[Done]]: false }.
    todo!()
}

/// [7.4.13 IteratorToList ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratortolist)
///
/// The abstract operation IteratorToList takes argument iteratorRecord (an
/// Iterator Record) and returns either a normal completion containing a List
/// of ECMAScript language values or a throw completion.
pub(crate) fn iterator_to_list(agent: &mut Agent, iterator_record: Object) -> JsResult<Vec<Value>> {
    // 1. Let values be a new empty List.
    let mut values = Vec::new();

    // 2. Let next be true.
    let mut next = Value::Boolean(true);

    // 3. Repeat, while next is not false,
    while next.is_true() {
        // a. Set next to ? IteratorStep(iteratorRecord).
        next = iterator_step(agent, iterator_record)?;

        // b. If next is not false, then
        if !next.is_false() {
            // SAFETY: Because iterator_step returns either false or an object this is always safe.
            let next = unsafe { next.try_into().unwrap_unchecked() };

            // i. Let nextValue be ? IteratorValue(next).
            let next_value = iterator_value(agent, next)?;

            // ii. Append nextValue to values.
            values.push(next_value);
        }
    }

    // 4. Return values.
    Ok(values)
}
