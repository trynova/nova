// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [7.4 Operations on Iterator Objects](https://tc39.es/ecma262/#sec-operations-on-iterator-objects)

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call, call_function, get, get_method, try_get_method},
            type_conversion::to_boolean,
        },
        builtins::{ordinary::ordinary_object_create_with_intrinsics, ArgumentsList},
        execution::{
            agent::{ExceptionType, JsError},
            Agent, JsResult, ProtoIntrinsics,
        },
        types::{
            Function, IntoValue, Object, PropertyDescriptor, PropertyKey, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        Scoped, TryResult,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

/// ### [7.4.1 Iterator Records](https://tc39.es/ecma262/#sec-iterator-records)
///
/// An Iterator Record is a Record value used to encapsulate an Iterator or
/// AsyncIterator along with the next method.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IteratorRecord {
    pub(crate) iterator: Object<'static>,
    pub(crate) next_method: Value<'static>,
    pub(crate) done: bool,
}

/// ### [7.4.2 GetIteratorFromMethod ( obj, method )](https://tc39.es/ecma262/#sec-getiteratorfrommethod)
///
/// The abstract operation GetIteratorFromMethod takes arguments obj (an
/// ECMAScript language value) and method (a function object) and returns
/// either a normal completion containing an Iterator Record or a throw
/// completion.
pub(crate) fn get_iterator_from_method(
    agent: &mut Agent,
    obj: Value,
    method: Function,
    mut gc: GcScope,
) -> JsResult<IteratorRecord> {
    let obj = obj.bind(gc.nogc());
    let method = method.bind(gc.nogc());
    // 1. Let iterator be ? Call(method, obj).
    let iterator = call_function(agent, method.unbind(), obj.unbind(), None, gc.reborrow())?
        .unbind()
        .bind(gc.nogc());

    // 2. If iterator is not an Object, throw a TypeError exception.
    let Ok(iterator) = Object::try_from(iterator) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Iterator is not an object",
            gc.nogc(),
        ));
    };

    let scoped_iterator = iterator.scope(agent, gc.nogc());
    // 3. Let nextMethod be ? Get(iterator, "next").
    let next_method = get(
        agent,
        iterator.unbind(),
        BUILTIN_STRING_MEMORY.next.into(),
        gc.reborrow(),
    )?;

    // 4. Let iteratorRecord be the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
    // 5. Return iteratorRecord.
    Ok(IteratorRecord {
        iterator: scoped_iterator.get(agent).unbind(),
        next_method: next_method.unbind(),
        done: false,
    })
}

/// ### [7.4.3 GetIterator ( obj, kind )](https://tc39.es/ecma262/#sec-getiterator)
///
/// The abstract operation GetIterator takes arguments obj (an ECMAScript
/// language value) and kind (sync or async) and returns either a normal
/// completion containing an Iterator Record or a throw completion.
pub(crate) fn get_iterator(
    agent: &mut Agent,
    obj: Value,
    is_async: bool,
    mut gc: GcScope,
) -> JsResult<IteratorRecord> {
    let obj = obj.bind(gc.nogc());
    let scoped_obj = obj.scope(agent, gc.nogc());
    // 1. If kind is async, then
    let method = if is_async {
        // a. Let method be ? GetMethod(obj, @@asyncIterator).
        let method = get_method(
            agent,
            obj.unbind(),
            PropertyKey::Symbol(WellKnownSymbolIndexes::AsyncIterator.into()),
            gc.reborrow(),
        )?;

        // b. If method is undefined, then
        if method.is_none() {
            // i. Let syncMethod be ? GetMethod(obj, @@iterator).
            let Some(sync_method) = get_method(
                agent,
                scoped_obj.get(agent),
                PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
                gc.reborrow(),
            )?
            else {
                // ii. If syncMethod is undefined, throw a TypeError exception.
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "No iterator on object",
                    gc.nogc(),
                ));
            };

            // iii. Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
            let _sync_iterator_record = get_iterator_from_method(
                agent,
                scoped_obj.get(agent),
                sync_method.unbind(),
                gc.reborrow(),
            )?;

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
            obj.unbind(),
            PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
            gc.reborrow(),
        )?
    };

    // 3. If method is undefined, throw a TypeError exception.
    let Some(method) = method else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Iterator method cannot be undefined",
            gc.nogc(),
        ));
    };

    // 4. Return ? GetIteratorFromMethod(obj, method).
    get_iterator_from_method(agent, scoped_obj.get(agent), method.unbind(), gc.reborrow())
}

/// ### [7.4.4 IteratorNext ( iteratorRecord [ , value ] )](https://tc39.es/ecma262/#sec-iteratornext)
///
/// The abstract operation IteratorNext takes argument iteratorRecord (an
/// Iterator Record) and optional argument value (an ECMAScript language value)
/// and returns either a normal completion containing an Object or a throw
/// completion.
pub(crate) fn iterator_next<'a>(
    agent: &mut Agent,
    iterator_record: &IteratorRecord,
    // SAFETY: The value is immediately passed to Call and never used again:
    // We don't need to bind/unbind/worry about its lifetime.
    value: Option<Value<'static>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<Object<'a>> {
    // 1. If value is not present, then
    // a. Let result be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
    // 2. Else,
    // a. Let result be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « value »).
    let result = call(
        agent,
        iterator_record.next_method,
        iterator_record.iterator.into(),
        value
            .as_ref()
            .map(|data| ArgumentsList(core::slice::from_ref(data))),
        gc.reborrow(),
    )?
    .unbind();
    let gc = gc.into_nogc();
    let result = result.bind(gc);

    // 3. If result is not an Object, throw a TypeError exception.
    // 4. Return result.
    result
        .try_into()
        .or(Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "The iterator result was not an object",
            gc,
        )))
}

/// ### [7.4.5 IteratorComplete ( iterResult )](https://tc39.es/ecma262/#sec-iteratorcomplete)
///
/// The abstract operation IteratorComplete takes argument iterResult (an
/// Object) and returns either a normal completion containing a Boolean or a
/// throw completion.
pub(crate) fn iterator_complete(
    agent: &mut Agent,
    iter_result: Object,
    mut gc: GcScope,
) -> JsResult<bool> {
    // 1. Return ToBoolean(? Get(iterResult, "done")).
    let done = get(
        agent,
        iter_result,
        BUILTIN_STRING_MEMORY.done.into(),
        gc.reborrow(),
    )?;
    Ok(to_boolean(agent, done))
}

/// ### [7.4.6 IteratorValue ( iterResult )](https://tc39.es/ecma262/#sec-iteratorvalue)
///
/// The abstract operation IteratorValue takes argument iterResult (an
/// Object) and returns either a normal completion containing an ECMAScript
/// language value or a throw completion.
pub(crate) fn iterator_value<'a>(
    agent: &mut Agent,
    iter_result: Object,
    gc: GcScope<'a, '_>,
) -> JsResult<Value<'a>> {
    // 1. Return ? Get(iterResult, "value").
    get(agent, iter_result, BUILTIN_STRING_MEMORY.value.into(), gc)
}

/// ### [7.4.7 IteratorStep ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstep)
///
/// The abstract operation IteratorStep takes argument iteratorRecord (an
/// Iterator Record) and returns either a normal completion containing either
/// an Object or false, or a throw completion. It requests the next value from
/// iteratorRecord.\[\[Iterator\]\] by calling
/// iteratorRecord.\[\[NextMethod\]\] and returns either false indicating that
/// the iterator has reached its end or the IteratorResult object if a next
/// value is available.
///
/// > NOTE: Instead of returning the boolean value false we return an Option
/// > where the false state is None. That way we can pass the Object as is.
pub(crate) fn iterator_step<'a>(
    agent: &mut Agent,
    iterator_record: &IteratorRecord,
    mut gc: GcScope<'a, '_>,
) -> JsResult<Option<Object<'a>>> {
    // 1. Let result be ? IteratorNext(iteratorRecord).
    let result = iterator_next(agent, iterator_record, None, gc.reborrow())?
        .unbind()
        .bind(gc.nogc());
    let scoped_result = result.scope(agent, gc.nogc());

    // 2. Let done be ? IteratorComplete(result).
    let done = iterator_complete(agent, result.unbind(), gc.reborrow())?;

    // 3. If done is true, return false.
    if done {
        return Ok(None);
    }

    // 4. Return result.
    Ok(Some(scoped_result.get(agent).bind(gc.into_nogc())))
}

/// ### [7.4.8 IteratorStepValue ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstepvalue)
/// The abstract operation IteratorStepValue takes argument iteratorRecord
/// (an Iterator Record) and returns either a normal completion containing
/// either an ECMAScript language value or done, or a throw completion. It
/// requests the next value from iteratorRecord.[\[Iterator\]] by calling
/// iteratorRecord.[\[NextMethod\]] and returns either done indicating that the
/// iterator has reached its end or the value from the IteratorResult object if
/// a next value is available.
pub(crate) fn iterator_step_value<'a>(
    agent: &mut Agent,
    iterator_record: &mut IteratorRecord,
    mut gc: GcScope<'a, '_>,
) -> JsResult<Option<Value<'a>>> {
    // 1. Let result be Completion(IteratorNext(iteratorRecord)).
    let result = iterator_next(agent, iterator_record, None, gc.reborrow());

    // 2. If result is a throw completion, then
    let result = match result {
        Err(err) => {
            // a. Set iteratorRecord.[[Done]] to true.
            iterator_record.done = true;

            // b. Return ? result.
            return Err(err);
        }
        // 3. Set result to ! result.
        Ok(result) => result.unbind().bind(gc.nogc()),
    };
    let scoped_result = result.scope(agent, gc.nogc());

    // 4. Let done be Completion(IteratorComplete(result)).
    let done = iterator_complete(agent, result.unbind(), gc.reborrow());

    // 5. If done is a throw completion, then
    let done = match done {
        Err(err) => {
            // a. Set iteratorRecord.[[Done]] to true.
            iterator_record.done = true;

            // b. Return ? done.
            return Err(err);
        }
        // 6. Set done to ! done.
        Ok(done) => done,
    };

    // 7. If done is true, then
    if done {
        // a. Set iteratorRecord.[[Done]] to true.
        iterator_record.done = true;

        // b. Return done.
        return Ok(None);
    }

    // 8. Let value be Completion(Get(result, "value")).
    let value = get(
        agent,
        scoped_result.get(agent),
        BUILTIN_STRING_MEMORY.value.into(),
        gc,
    );

    // 9. If value is a throw completion, then
    if value.is_err() {
        // a. Set iteratorRecord.[[Done]] to true.
        iterator_record.done = true;
    }

    // 10. Return ? value.
    value.map(Some)
}

/// ### [7.4.9 IteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-iteratorclose)
///
/// The abstract operation IteratorClose takes arguments iteratorRecord (an
/// Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an iterator that it should perform
/// any actions it would normally perform when it has reached its completed
/// state.
pub(crate) fn iterator_close<T>(
    agent: &mut Agent,
    iterator_record: &IteratorRecord,
    completion: JsResult<T>,
    mut gc: GcScope,
) -> JsResult<T> {
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    let iterator = iterator_record.iterator;
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    let inner_result = get_method(
        agent,
        iterator.into_value(),
        BUILTIN_STRING_MEMORY.r#return.into(),
        gc.reborrow(),
    );
    // 4. If innerResult.[[Type]] is normal, then
    let inner_result = match inner_result {
        Ok(return_function) => {
            // a. Let return be innerResult.[[Value]].
            // b. If return is undefined, return ? completion.
            let Some(return_function) = return_function else {
                return completion;
            };
            // c. Set innerResult to Completion(Call(return, iterator)).
            call_function(
                agent,
                return_function.unbind(),
                iterator.into_value(),
                None,
                gc.reborrow(),
            )
        }
        Err(inner_result) => Err(inner_result),
    };
    // 5. If completion.[[Type]] is throw, return ? completion.
    let completion = completion?;
    // 6. If innerResult.[[Type]] is throw, return ? innerResult.
    let inner_result = inner_result?;
    // 7. If innerResult.[[Value]] is not an Object, throw a TypeError exception.
    if !inner_result.is_object() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Invalid iterator 'return' method return value",
            gc.nogc(),
        ));
    }
    // 8. Return ? completion.
    Ok(completion)
}

pub(crate) fn iterator_close_with_error(
    agent: &mut Agent,
    iterator_record: &IteratorRecord,
    completion: JsError,
    mut gc: GcScope,
) -> JsError {
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    let iterator = iterator_record.iterator;
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    let inner_result = get_method(
        agent,
        iterator.into_value(),
        BUILTIN_STRING_MEMORY.r#return.into(),
        gc.reborrow(),
    );
    // 4. If innerResult.[[Type]] is normal, then
    if let Ok(r#return) = inner_result {
        // a. Let return be innerResult.[[Value]].
        // b. If return is undefined, return ? completion.
        let Some(r#return) = r#return else {
            return completion;
        };
        // c. Set innerResult to Completion(Call(return, iterator)).
        let _ = call_function(
            agent,
            r#return.unbind(),
            iterator.into_value(),
            None,
            gc.reborrow(),
        );
    }
    // 5. If completion.[[Type]] is throw, return ? completion.
    completion
}

/// ### [7.4.9 IteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-iteratorclose)
///
/// The abstract operation IteratorClose takes arguments iteratorRecord (an
/// Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an iterator that it should perform
/// any actions it would normally perform when it has reached its completed
/// state.
pub(crate) fn try_iterator_close<T>(
    agent: &mut Agent,
    iterator_record: &IteratorRecord,
    completion: JsResult<T>,
    gc: NoGcScope,
) -> TryResult<JsResult<T>> {
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    let iterator = iterator_record.iterator;
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    let inner_result = try_get_method(
        agent,
        iterator.into_value(),
        BUILTIN_STRING_MEMORY.r#return.into(),
        gc,
    )?;
    // 4. If innerResult.[[Type]] is normal, then
    match inner_result {
        Ok(return_function) => {
            // a. Let return be innerResult.[[Value]].
            match return_function {
                // c. Set innerResult to Completion(Call(return, iterator)).
                // Can't call functions in NoGcScope.
                Some(_) => TryResult::Break(()),
                // b. If return is undefined, return ? completion.
                None => TryResult::Continue(completion),
            }
        }
        Err(inner_result) => {
            match completion {
                // 6. If innerResult.[[Type]] is throw, return ? innerResult.
                Ok(_) => TryResult::Continue(Err(inner_result)),
                // 5. If completion.[[Type]] is throw, return ? completion.
                Err(err) => TryResult::Continue(Err(err)),
            }
        }
    }
}

macro_rules! if_abrupt_close_iterator {
    ($agent:ident, $value:ident, $iterator_record:ident, $gc:ident) => {
        // 1. Assert: value is a Completion Record.
        // 2. If value is an abrupt completion, return ? IteratorClose(iteratorRecord, value).
        if let Err(err) = $value {
            return Err(
                crate::ecmascript::abstract_operations::operations_on_iterator_objects::iterator_close_with_error(
                    $agent,
                    &$iterator_record,
                    err,
                    $gc
                )
            );
        } else if let Ok(value) = $value {
            value.unbind().bind($gc.nogc())
        } else {
            unreachable!();
        }
    };
}

pub(crate) use if_abrupt_close_iterator;

/// ### [7.4.11 AsyncIteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-asynciteratorclose)
///
/// The abstract operation AsyncIteratorClose takes arguments iteratorRecord
/// (an Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an async iterator that it should
/// perform any actions it would normally perform when it has reached its
/// completed state.
pub(crate) fn async_iterator_close<'a>(
    _agent: &mut Agent,
    _iterator_record: &IteratorRecord,
    _completion: JsResult<Value>,
    _gc: GcScope<'a, '_>,
) -> JsResult<Value<'a>> {
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

/// ### [7.4.12 CreateIterResultObject ( value, done )](https://tc39.es/ecma262/#sec-createiterresultobject)
///
/// The abstract operation CreateIterResultObject takes arguments value (an
/// ECMAScript language value) and done (a Boolean) and returns an Object that
/// conforms to the IteratorResult interface. It creates an object that
/// conforms to the IteratorResult interface.
pub(crate) fn create_iter_result_object<'a>(
    agent: &mut Agent,
    value: Value,
    done: bool,
    gc: NoGcScope<'a, '_>,
) -> Object<'a> {
    // 1. Let obj be OrdinaryObjectCreate(%Object.prototype%).
    let obj =
        ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None, gc);
    // 2. Perform ! CreateDataPropertyOrThrow(obj, "value", value).
    obj.property_storage().set(
        agent,
        BUILTIN_STRING_MEMORY.value.to_property_key(),
        PropertyDescriptor::new_data_descriptor(value),
    );
    // 3. Perform ! CreateDataPropertyOrThrow(obj, "done", done).
    obj.property_storage().set(
        agent,
        BUILTIN_STRING_MEMORY.done.to_property_key(),
        PropertyDescriptor::new_data_descriptor(done.into()),
    );
    // 4. Return obj.
    obj
}

/// ### [7.4.13 CreateListIteratorRecord ( list )](https://tc39.es/ecma262/#sec-createlistiteratorRecord)
///
/// The abstract operation CreateListIteratorRecord takes argument list (a List
/// of ECMAScript language values) and returns an Iterator Record. It creates
/// an Iterator (27.1.1.2) object record whose next method returns the
/// successive elements of list.
pub(crate) fn create_list_iterator_record<'a>(
    _agent: &mut Agent,
    _list: &[Value],
    _gc: GcScope<'a, '_>,
) -> JsResult<Value<'a>> {
    // 1. Let closure be a new Abstract Closure with no parameters that captures list and performs the following steps when called:
    // a. For each element E of list, do
    // i. Perform ? GeneratorYield(CreateIterResultObject(E, false)).
    // b. Return NormalCompletion(undefined).
    // 2. Let iterator be CreateIteratorFromClosure(closure, empty, %IteratorPrototype%).
    // 3. Return the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: %GeneratorFunction.prototype.prototype.next%, [[Done]]: false }.
    todo!()
}

/// ### [7.4.14 IteratorToList ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratortolist)
///
/// The abstract operation IteratorToList takes argument iteratorRecord (an
/// Iterator Record) and returns either a normal completion containing a List
/// of ECMAScript language values or a throw completion.
pub(crate) fn iterator_to_list<'a, 'b>(
    agent: &mut Agent,
    iterator_record: &IteratorRecord,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<Vec<Scoped<'b, Value<'static>>>> {
    // 1. Let values be a new empty List.
    let mut values = Vec::new();

    // 2. Let next be true.
    // 3. Repeat, while next is not false,
    // a. Set next to ? IteratorStep(iteratorRecord).
    // b. If next is not false, then
    while let Some(next) = iterator_step(agent, iterator_record, gc.reborrow())? {
        // i. Let nextValue be ? IteratorValue(next).
        // ii. Append nextValue to values.
        values.push(
            iterator_value(agent, next.unbind(), gc.reborrow())?
                .unbind()
                .bind(gc.nogc())
                .scope(agent, gc.nogc()),
        );
    }

    // 4. Return values.
    Ok(values)
}

impl HeapMarkAndSweep for IteratorRecord {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            iterator,
            next_method,
            done: _,
        } = self;
        iterator.mark_values(queues);
        next_method.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            iterator,
            next_method,
            done: _,
        } = self;
        iterator.sweep_values(compactions);
        next_method.sweep_values(compactions);
    }
}
