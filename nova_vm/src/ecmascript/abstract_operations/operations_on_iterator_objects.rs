// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [7.4 Operations on Iterator Objects](https://tc39.es/ecma262/#sec-operations-on-iterator-objects)

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, get, get_method, get_object_method, try_get_object_method,
            },
            testing_and_comparison::is_callable,
            type_conversion::to_boolean,
        },
        builtins::{ArgumentsList, ordinary::ordinary_object_create_with_intrinsics},
        execution::{
            Agent, JsResult, ProtoIntrinsics,
            agent::{ExceptionType, JsError},
        },
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, PropertyDescriptor,
            PropertyKey, Value,
        },
    },
    engine::{
        ScopableCollection, ScopedCollection, TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

/// ### [7.4.1 Iterator Records](https://tc39.es/ecma262/#sec-iterator-records)
///
/// An Iterator Record is a Record value used to encapsulate an Iterator or
/// AsyncIterator along with the next method.
#[derive(Debug, Clone, Copy)]
pub struct IteratorRecord<'a> {
    pub(crate) iterator: Object<'a>,
    pub(crate) next_method: Function<'a>,
    // Note: The done field doesn't seem to be used anywhere.
    // pub(crate) done: bool,
}

/// SAFETY: Properly implemented as recursive binding.
unsafe impl Bindable for IteratorRecord<'_> {
    type Of<'a> = IteratorRecord<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        Self::Of {
            iterator: self.iterator.unbind(),
            next_method: self.next_method.unbind(),
            // done: self.done,
        }
    }

    #[inline(always)]
    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        Self::Of {
            iterator: self.iterator.bind(gc),
            next_method: self.next_method.bind(gc),
            // done: self.done,
        }
    }
}

/// ### [7.4.2 GetIteratorDirect ( obj )](https://tc39.es/ecma262/#sec-getiteratordirect)
/// The abstract operation GetIteratorDirect takes argument obj (an Object) and returns
/// either a normal completion containing an Iterator Record or a throw completion.
///
/// Note: Different from the spec, this method returns None if the iterator
/// object's next method isn't callable.
pub(crate) fn get_iterator_direct<'gc>(
    agent: &mut Agent,
    obj: Object,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Option<IteratorRecord<'gc>>> {
    let obj = obj.bind(gc.nogc());

    let scoped_obj = obj.scope(agent, gc.nogc());
    // 1. Let nextMethod be ? Get(obj, "next").
    let next_method = get(
        agent,
        obj.unbind(),
        BUILTIN_STRING_MEMORY.next.into(),
        gc.reborrow(),
    )
    .unbind()?;
    let gc = gc.into_nogc();

    let Some(next_method) = is_callable(next_method, gc) else {
        return Ok(None);
    };

    // 2. Let iteratorRecord be the Iterator Record { [[Iterator]]: obj, [[NextMethod]]: nextMethod, [[Done]]: false }.
    let iterator_record = IteratorRecord {
        iterator: scoped_obj.get(agent).bind(gc),
        next_method,
    };

    // 3. Return iteratorRecord.
    Ok(Some(iterator_record))
}

/// ### [7.4.3 GetIteratorFromMethod ( obj, method )](https://tc39.es/ecma262/#sec-getiteratorfrommethod)
///
/// The abstract operation GetIteratorFromMethod takes arguments obj (an
/// ECMAScript language value) and method (a function object) and returns
/// either a normal completion containing an Iterator Record or a throw
/// completion.
///
/// Note: Different from the spec, this method returns None if the iterator
/// object's next method isn't callable.
pub(crate) fn get_iterator_from_method<'a>(
    agent: &mut Agent,
    obj: Value,
    method: Function,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<IteratorRecord<'a>>> {
    let obj = obj.bind(gc.nogc());
    let method = method.bind(gc.nogc());
    // 1. Let iterator be ? Call(method, obj).
    let iterator = call_function(agent, method.unbind(), obj.unbind(), None, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // 2. If iterator is not an Object, throw a TypeError exception.
    let Ok(iterator) = Object::try_from(iterator) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Iterator is not an object",
            gc.into_nogc(),
        ));
    };

    let scoped_iterator = iterator.scope(agent, gc.nogc());
    // 3. Let nextMethod be ? Get(iterator, "next").
    let next_method = get(
        agent,
        iterator.unbind(),
        BUILTIN_STRING_MEMORY.next.into(),
        gc.reborrow(),
    )
    .unbind()?;
    let gc = gc.into_nogc();

    let Some(next_method) = is_callable(next_method, gc) else {
        return Ok(None);
    };

    // 4. Let iteratorRecord be the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
    // 5. Return iteratorRecord.
    Ok(Some(IteratorRecord {
        iterator: scoped_iterator.get(agent).bind(gc),
        next_method,
        // done: false,
    }))
}

/// ### [7.4.4 GetIterator ( obj, kind )](https://tc39.es/ecma262/#sec-getiterator)
///
/// The abstract operation GetIterator takes arguments obj (an ECMAScript
/// language value) and kind (sync or async) and returns either a normal
/// completion containing an Iterator Record or a throw completion.
pub(crate) fn get_iterator<'a>(
    agent: &mut Agent,
    obj: Value,
    is_async: bool,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<IteratorRecord<'a>>> {
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
        )
        .unbind()?
        .bind(gc.nogc());

        // b. If method is undefined, then
        if method.is_none() {
            // i. Let syncMethod be ? GetMethod(obj, @@iterator).
            let Some(sync_method) = get_method(
                agent,
                scoped_obj.get(agent),
                PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc()) else {
                // ii. If syncMethod is undefined, throw a TypeError exception.
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "No iterator on object",
                    gc.into_nogc(),
                ));
            };

            // iii. Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
            let _sync_iterator_record = get_iterator_from_method(
                agent,
                scoped_obj.get(agent),
                sync_method.unbind(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());

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
        )
        .unbind()?
        .bind(gc.nogc())
    };

    // 3. If method is undefined, throw a TypeError exception.
    let Some(method) = method else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Iterator method cannot be undefined",
            gc.into_nogc(),
        ));
    };

    // 4. Return ? GetIteratorFromMethod(obj, method).
    get_iterator_from_method(agent, scoped_obj.get(agent), method.unbind(), gc)
}

/// ### [7.4.6 IteratorNext ( iteratorRecord [ , value ] )](https://tc39.es/ecma262/#sec-iteratornext)
///
/// The abstract operation IteratorNext takes argument iteratorRecord (an
/// Iterator Record) and optional argument value (an ECMAScript language value)
/// and returns either a normal completion containing an Object or a throw
/// completion.
pub(crate) fn iterator_next<'a>(
    agent: &mut Agent,
    iterator_record: IteratorRecord,
    // SAFETY: The value is immediately passed to Call and never used again:
    // We don't need to bind/unbind/worry about its lifetime.
    mut value: Option<Value<'static>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    // 1. If value is not present, then
    // a. Let result be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
    // 2. Else,
    // a. Let result be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « value »).
    let result = call_function(
        agent,
        iterator_record.next_method,
        iterator_record.iterator.into(),
        value.as_mut().map(ArgumentsList::from_mut_value),
        gc.reborrow(),
    )
    .unbind()?;
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

/// ### [7.4.7 IteratorComplete ( iterResult )](https://tc39.es/ecma262/#sec-iteratorcomplete)
///
/// The abstract operation IteratorComplete takes argument iterResult (an
/// Object) and returns either a normal completion containing a Boolean or a
/// throw completion.
fn iterator_complete<'a>(
    agent: &mut Agent,
    iter_result: Object,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    // 1. Return ToBoolean(? Get(iterResult, "done")).
    let done = get(agent, iter_result, BUILTIN_STRING_MEMORY.done.into(), gc)?;
    Ok(to_boolean(agent, done))
}

/// ### [7.4.8 IteratorValue ( iterResult )](https://tc39.es/ecma262/#sec-iteratorvalue)
///
/// The abstract operation IteratorValue takes argument iterResult (an
/// Object) and returns either a normal completion containing an ECMAScript
/// language value or a throw completion.
pub(crate) fn iterator_value<'a>(
    agent: &mut Agent,
    iter_result: Object,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
    // 1. Return ? Get(iterResult, "value").
    get(agent, iter_result, BUILTIN_STRING_MEMORY.value.into(), gc)
}

/// ### [7.4.9 IteratorStep ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstep)
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
    iterator_record: IteratorRecord,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Object<'a>>> {
    // 1. Let result be ? IteratorNext(iteratorRecord).
    let result = iterator_next(agent, iterator_record, None, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    let scoped_result = result.scope(agent, gc.nogc());

    // 2. Let done be ? IteratorComplete(result).
    let done = iterator_complete(agent, result.unbind(), gc.reborrow()).unbind()?;

    // 3. If done is true, return false.
    if done {
        return Ok(None);
    }

    // 4. Return result.
    // SAFETY: scoped_result is never shared.
    Ok(Some(unsafe {
        scoped_result.take(agent).bind(gc.into_nogc())
    }))
}

/// ### [7.4.10 IteratorStepValue ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstepvalue)
/// The abstract operation IteratorStepValue takes argument iteratorRecord
/// (an Iterator Record) and returns either a normal completion containing
/// either an ECMAScript language value or done, or a throw completion. It
/// requests the next value from iteratorRecord.[\[Iterator\]] by calling
/// iteratorRecord.[\[NextMethod\]] and returns either done indicating that the
/// iterator has reached its end or the value from the IteratorResult object if
/// a next value is available.
pub(crate) fn iterator_step_value<'a>(
    agent: &mut Agent,
    iterator_record: IteratorRecord,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Value<'a>>> {
    // 1. Let result be Completion(IteratorNext(iteratorRecord)).
    let result = iterator_next(agent, iterator_record, None, gc.reborrow());

    // 2. If result is a throw completion, then
    let result = match result {
        Err(err) => {
            // a. Set iteratorRecord.[[Done]] to true.

            // b. Return ? result.
            return Err(err.unbind());
        }
        // 3. Set result to ! result.
        Ok(result) => result.unbind().bind(gc.nogc()),
    };
    let scoped_result = result.scope(agent, gc.nogc());

    // 4. Let done be Completion(IteratorComplete(result)).
    let done = iterator_complete(agent, result.unbind(), gc.reborrow())
        .unbind()
        .bind(gc.nogc());
    // SAFETY: scoped_result is never shared.
    let result = unsafe { scoped_result.take(agent) }.bind(gc.nogc());

    // 5. If done is a throw completion, then
    let done = match done {
        Err(err) => {
            // a. Set iteratorRecord.[[Done]] to true.
            // b. Return ? done.
            return Err(err.unbind());
        }
        // 6. Set done to ! done.
        Ok(done) => done,
    };

    // 7. If done is true, then
    if done {
        // a. Set iteratorRecord.[[Done]] to true.
        // b. Return done.
        return Ok(None);
    }

    // 8. Let value be Completion(Get(result, "value")).
    let value = get(
        agent,
        result.unbind(),
        BUILTIN_STRING_MEMORY.value.into(),
        gc,
    );

    // 9. If value is a throw completion, then
    // a. Set iteratorRecord.[[Done]] to true.
    // 10. Return ? value.
    value.map(Some)
}

/// ### [7.4.11 IteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-iteratorclose)
///
/// The abstract operation IteratorClose takes arguments iteratorRecord (an
/// Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an iterator that it should perform
/// any actions it would normally perform when it has reached its completed
/// state.
pub(crate) fn iterator_close_with_value<'a>(
    agent: &mut Agent,
    iterator: Object,
    completion: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
    let mut iterator = iterator.bind(gc.nogc());
    let completion = completion.scope(agent, gc.nogc());
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    let inner_result = if let TryResult::Continue(inner_result) = try_get_object_method(
        agent,
        iterator,
        BUILTIN_STRING_MEMORY.r#return.into(),
        gc.nogc(),
    ) {
        inner_result
    } else {
        let scoped_iterator = iterator.scope(agent, gc.nogc());
        let inner_result = get_object_method(
            agent,
            iterator.unbind(),
            BUILTIN_STRING_MEMORY.r#return.into(),
            gc.reborrow(),
        )
        .unbind()
        .bind(gc.nogc());
        // SAFETY: scoped_iterator is not shared.
        iterator = unsafe { scoped_iterator.take(agent) }.bind(gc.nogc());
        inner_result
    };
    // 4. If innerResult.[[Type]] is normal, then
    let inner_result = match inner_result {
        Ok(return_function) => {
            // a. Let return be innerResult.[[Value]].
            // b. If return is undefined, return ? completion.
            let Some(return_function) = return_function else {
                // SAFETY: completion is not shared.
                return Ok(unsafe { completion.take(agent) });
            };
            // c. Set innerResult to Completion(Call(return, iterator)).
            call_function(
                agent,
                return_function.unbind(),
                iterator.into_value().unbind(),
                None,
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc())
        }
        Err(inner_result) => Err(inner_result),
    };
    // SAFETY: completion is not shared.
    let completion = unsafe { completion.take(agent) }.bind(gc.nogc());

    // 5. If completion.[[Type]] is throw, return ? completion.
    // 6. If innerResult.[[Type]] is throw, return ? innerResult.
    let inner_result = inner_result.unbind()?.bind(gc.nogc());
    // 7. If innerResult.[[Value]] is not an Object, throw a TypeError exception.
    if !inner_result.is_object() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Invalid iterator 'return' method return value",
            gc.into_nogc(),
        ));
    }
    // 8. Return ? completion.
    Ok(completion.unbind())
}

/// ### [7.4.11 IteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-iteratorclose)
///
/// The abstract operation IteratorClose takes arguments iteratorRecord (an
/// Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an iterator that it should perform
/// any actions it would normally perform when it has reached its completed
/// state.
pub(crate) fn iterator_close_with_error<'a>(
    agent: &mut Agent,
    iterator: Object,
    completion: JsError,
    mut gc: GcScope<'a, '_>,
) -> JsError<'a> {
    let mut iterator = iterator.bind(gc.nogc());
    let completion = completion.scope(agent, gc.nogc());
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    let inner_result = if let TryResult::Continue(inner_result) = try_get_object_method(
        agent,
        iterator,
        BUILTIN_STRING_MEMORY.r#return.into(),
        gc.nogc(),
    ) {
        inner_result
    } else {
        let scoped_iterator = iterator.scope(agent, gc.nogc());
        let inner_result = get_object_method(
            agent,
            iterator.unbind(),
            BUILTIN_STRING_MEMORY.r#return.into(),
            gc.reborrow(),
        )
        .unbind()
        .bind(gc.nogc());
        // SAFETY: scoped_iterator is not shared.
        iterator = unsafe { scoped_iterator.take(agent) }.bind(gc.nogc());
        inner_result
    };
    // 4. If innerResult.[[Type]] is normal, then
    if let Ok(r#return) = inner_result {
        // a. Let return be innerResult.[[Value]].
        // b. If return is undefined, return ? completion.
        let Some(r#return) = r#return else {
            // SAFETY: completion is not shared.
            return unsafe { completion.take(agent) };
        };
        // c. Set innerResult to Completion(Call(return, iterator)).
        let _ = call_function(
            agent,
            r#return.unbind(),
            iterator.into_value().unbind(),
            None,
            gc.reborrow(),
        );
    }
    // 5. If completion.[[Type]] is throw, return ? completion.
    // SAFETY: completion is not shared.
    unsafe { completion.take(agent) }
}

macro_rules! if_abrupt_close_iterator {
    ($agent:ident, $value:ident, $iterator_record:ident, $gc:ident) => {
        // 1. Assert: value is a Completion Record.
        // 2. If value is an abrupt completion, return ? IteratorClose(iteratorRecord, value).
        if let Err(err) = $value {
            return Err(
                crate::ecmascript::abstract_operations::operations_on_iterator_objects::iterator_close_with_error(
                    $agent,
                    $iterator_record.iterator.unbind(),
                    err.unbind(),
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

/// ### [7.4.13 AsyncIteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-asynciteratorclose)
///
/// The abstract operation AsyncIteratorClose takes arguments iteratorRecord
/// (an Iterator Record) and completion (a Completion Record) and returns a
/// Completion Record. It is used to notify an async iterator that it should
/// perform any actions it would normally perform when it has reached its
/// completed state.
pub(crate) fn async_iterator_close<'a>(
    agent: &mut Agent,
    _iterator_record: &IteratorRecord,
    _completion: JsResult<Value>,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
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
    Err(agent.todo("AsyncIteratorClose", gc.into_nogc()))
}

/// ### [7.4.13 AsyncIteratorClose ( iteratorRecord, completion )](https://tc39.es/ecma262/#sec-asynciteratorclose)
///
/// Note: this is used to perform a "return" function call on async VM iterator
/// close and returns Some(value) if the return function did not throw. If the
/// return function did not exist or threw an error, then None is returned as a
/// sign to the VM to proceed to rethrow the completion.
pub(crate) fn async_vm_iterator_close_with_error<'a>(
    agent: &mut Agent,
    iterator: Object,
    mut gc: GcScope<'a, '_>,
) -> Option<Value<'a>> {
    let mut iterator = iterator.bind(gc.nogc());
    // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
    // 2. Let iterator be iteratorRecord.[[Iterator]].
    // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
    let inner_result = if let TryResult::Continue(inner_result) = try_get_object_method(
        agent,
        iterator,
        BUILTIN_STRING_MEMORY.r#return.into(),
        gc.nogc(),
    ) {
        inner_result
    } else {
        let scoped_iterator = iterator.scope(agent, gc.nogc());
        let inner_result = get_object_method(
            agent,
            iterator.unbind(),
            BUILTIN_STRING_MEMORY.r#return.into(),
            gc.reborrow(),
        )
        .unbind()
        .bind(gc.nogc());
        // SAFETY: scoped_iterator is not shared.
        iterator = unsafe { scoped_iterator.take(agent) }.bind(gc.nogc());
        inner_result
    };
    // 4. If innerResult.[[Type]] is normal, then
    if let Ok(r#return) = inner_result {
        // a. Let return be innerResult.[[Value]].
        // b. If return is undefined, return ? completion.
        let Some(r#return) = r#return else {
            // Note: we return None to signal that completion should be
            // rethrown.
            return None;
        };
        // c. Set innerResult to Completion(Call(return, iterator)).
        let inner_result = call_function(
            agent,
            r#return.unbind(),
            iterator.into_value().unbind(),
            None,
            gc,
        );
        // d. If innerResult.[[Type]] is normal, set innerResult to
        //    Completion(Await(innerResult.[[Value]])).
        if let Ok(value) = inner_result {
            // Note: we return Some to signal that an Await is required.
            return Some(value);
        }
    }
    // 5. If completion.[[Type]] is throw, return ? completion.
    // Note: we return None to signal that completion should be rethrown.
    None
}

/// ### [7.4.14 CreateIterResultObject ( value, done )](https://tc39.es/ecma262/#sec-createiterresultobject)
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
    let Object::Object(obj) =
        ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None, gc)
    else {
        unreachable!()
    };
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
    obj.into_object()
}

/// ### [7.4.15 CreateListIteratorRecord ( list )](https://tc39.es/ecma262/#sec-createlistiteratorRecord)
///
/// The abstract operation CreateListIteratorRecord takes argument list (a List
/// of ECMAScript language values) and returns an Iterator Record. It creates
/// an Iterator (27.1.1.2) object record whose next method returns the
/// successive elements of list.
pub(crate) fn create_list_iterator_record<'a>(
    agent: &mut Agent,
    _list: &[Value],
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
    // 1. Let closure be a new Abstract Closure with no parameters that captures list and performs the following steps when called:
    // a. For each element E of list, do
    // i. Perform ? GeneratorYield(CreateIterResultObject(E, false)).
    // b. Return NormalCompletion(undefined).
    // 2. Let iterator be CreateIteratorFromClosure(closure, empty, %IteratorPrototype%).
    // 3. Return the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: %GeneratorFunction.prototype.prototype.next%, [[Done]]: false }.
    Err(agent.todo("CreateListIteratorRecord", gc.into_nogc()))
}

/// ### [7.4.16 IteratorToList ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratortolist)
///
/// The abstract operation IteratorToList takes argument iteratorRecord (an
/// Iterator Record) and returns either a normal completion containing a List
/// of ECMAScript language values or a throw completion.
pub(crate) fn iterator_to_list<'a, 'b>(
    agent: &mut Agent,
    iterator_record: IteratorRecord,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<'a, ScopedCollection<'b, Vec<Value<'static>>>> {
    // 1. Let values be a new empty List.
    let mut values = Vec::<Value>::new().scope(agent, gc.nogc());

    // 2. Let next be true.
    // 3. Repeat, while next is not false,
    // a. Set next to ? IteratorStep(iteratorRecord).
    // b. If next is not false, then
    while let Some(next) = iterator_step(agent, iterator_record, gc.reborrow())
        .unbind()?
        .bind(gc.nogc())
    {
        // i. Let nextValue be ? IteratorValue(next).
        let next_value = iterator_value(agent, next.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // ii. Append nextValue to values.
        values.push(agent, next_value);
    }

    // 4. Return values.
    Ok(values)
}

impl HeapMarkAndSweep for IteratorRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            iterator,
            next_method,
            // done: _,
        } = self;
        iterator.mark_values(queues);
        next_method.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            iterator,
            next_method,
            // done: _,
        } = self;
        iterator.sweep_values(compactions);
        next_method.sweep_values(compactions);
    }
}
