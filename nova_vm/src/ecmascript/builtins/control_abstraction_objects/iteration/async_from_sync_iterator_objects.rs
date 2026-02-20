//! ### [27.1.6 Async-from-Sync Iterator Objects](https://tc39.es/ecma262/#sec-async-from-sync-iterator-objects)
//!
//! An Async-from-Sync Iterator object is an async iterator that adapts a
//! specific synchronous iterator. Async-from-Sync Iterator objects are never
//! directly accessible to ECMAScript code. There is not a named constructor
//! for Async-from-Sync Iterator objects. Instead, Async-from-Sync Iterator
//! objects are created by the CreateAsyncFromSyncIterator abstract operation
//! as needed.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, ExceptionType, IteratorRecord,
        MaybeInvalidIteratorRecord, Object, Promise, PromiseCapability, PromiseReactionHandler,
        Value, call_function, create_iter_result_object, get_object_method,
        if_abrupt_reject_promise_m, inner_promise_then, iterator_close_with_value,
        iterator_complete, iterator_next, iterator_value, unwrap_try,
    },
    engine::{Bindable, GcScope, Scopable, VmIteratorRecord},
};

/// ### [27.1.6.1 CreateAsyncFromSyncIterator ( syncIteratorRecord )](https://tc39.es/ecma262/#sec-createasyncfromsynciterator)
///
/// The abstract operation CreateAsyncFromSyncIterator takes argument
/// syncIteratorRecord (an Iterator Record) and returns an Iterator Record. It
/// is used to create an async Iterator Record from a synchronous Iterator
/// Record.
pub(crate) fn create_async_from_sync_iterator(
    sync_iterator_record: MaybeInvalidIteratorRecord,
) -> VmIteratorRecord {
    // 1. Let asyncIterator be OrdinaryObjectCreate(%AsyncFromSyncIteratorPrototype%, « [[SyncIteratorRecord]] »).
    // 2. Set asyncIterator.[[SyncIteratorRecord]] to syncIteratorRecord.
    let iterator = sync_iterator_record.iterator;
    // 3. Let nextMethod be ! Get(asyncIterator, "next").
    let Some(iterator_record) = sync_iterator_record.into_iterator_record() else {
        return VmIteratorRecord::InvalidIterator { iterator };
    };
    // 4. Let iteratorRecord be the Iterator Record { [[Iterator]]: asyncIterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
    // 5. Return iteratorRecord.
    VmIteratorRecord::AsyncFromSyncGenericIterator(iterator_record)
}

pub(crate) struct AsyncFromSyncIteratorPrototype;

impl AsyncFromSyncIteratorPrototype {
    /// ### [27.1.6.2.1 %AsyncFromSyncIteratorPrototype%.next ( \[ value \] )](https://tc39.es/ecma262/#sec-%asyncfromsynciteratorprototype%.next)
    pub(crate) fn next<'gc>(
        agent: &mut Agent,
        sync_iterator_record: IteratorRecord,
        value: Option<Value>,
        mut gc: GcScope<'gc, '_>,
    ) -> Promise<'gc> {
        crate::engine::bind!(let value = value, gc);
        crate::engine::bind!(let sync_iterator_record = sync_iterator_record, gc);
        let iterator = sync_iterator_record.iterator.scope(agent, gc.nogc());
        let next_method = sync_iterator_record.next_method.scope(agent, gc.nogc());
        // 1. Let O be the this value.
        // 2. Assert: O is an Object that has a [[SyncIteratorRecord]] internal slot.
        // 3. Let promiseCapability be ! NewPromiseCapability(%Promise%).
        // 4. Let syncIteratorRecord be O.[[SyncIteratorRecord]].
        // 5. If value is present, then
        // a. Let result be Completion(IteratorNext(syncIteratorRecord, value)).
        // 6. Else,
        // a. Let result be Completion(IteratorNext(syncIteratorRecord)).
        let result = iterator_next(agent, sync_iterator_record, value, gc.reborrow())?;
        // SAFETY: neither is shared.
        let sync_iterator = unsafe {
            let _ = next_method.take(agent).local();
            iterator.take(agent).local()
        };
        match result {
            Ok(result) => {
                // 8. Return AsyncFromSyncIteratorContinuation(result, promiseCapability, syncIteratorRecord, true).
                let promise_capability = PromiseCapability::new(agent, gc.nogc());
                async_from_sync_iterator_continuation(
                    agent,
                    result,
                    promise_capability,
                    sync_iterator,
                    true,
                    gc,
                )
            }
            Err(err) => {
                // 7. IfAbruptRejectPromise(result, promiseCapability).
                Promise::new_rejected(agent, err.value(), gc.into_nogc())
            }
        }
    }

    /// ### [27.1.6.2.2 %AsyncFromSyncIteratorPrototype%.return ( \[ value \] )](https://tc39.es/ecma262/#sec-%asyncfromsynciteratorprototype%.return)
    pub(crate) fn r#return<'gc>(
        agent: &mut Agent,
        sync_iterator: Object,
        value: Option<Value>,
        mut gc: GcScope<'gc, '_>,
    ) -> Promise<'gc> {
        crate::engine::bind!(let value = value, gc);
        crate::engine::bind!(let sync_iterator = sync_iterator, gc);
        let scoped_sync_iterator = sync_iterator.scope(agent, gc.nogc());

        // 1. Let O be the this value.
        // 2. Assert: O is an Object that has a [[SyncIteratorRecord]] internal slot.
        // 3. Let promiseCapability be ! NewPromiseCapability(%Promise%).
        let scoped_promise = PromiseCapability::new(agent, gc.nogc())
            .promise()
            .scope(agent, gc.nogc());
        // 4. Let syncIteratorRecord be O.[[SyncIteratorRecord]].
        // 5. Let syncIterator be syncIteratorRecord.[[Iterator]].
        let value = value.map(|v| v.scope(agent, gc.nogc()));
        // 6. Let return be Completion(GetMethod(syncIterator, "return")).
        let r#return = get_object_method(
            agent,
            sync_iterator,
            BUILTIN_STRING_MEMORY.r#return.to_property_key(),
            gc.reborrow(),
        )?;

        crate::engine::bind!(let value = value.map(|v| v.get(agent).local()), gc);

        // 7. IfAbruptRejectPromise(return, promiseCapability).
        let promise_capability = PromiseCapability {
            promise: scoped_promise.get(agent).local(),
            must_be_unresolved: true,
        };
        let r#return = if_abrupt_reject_promise_m!(agent, r#return, promise_capability, gc);
        // 8. If return is undefined, then
        let Some(r#return) = r#return else {
            // a. Let iteratorResult be CreateIteratorResultObject(value, true).
            let iterator_result = create_iter_result_object(
                agent,
                value.unwrap_or(Value::Undefined),
                true,
                gc.nogc(),
            )
            .expect("Should perform GC here");
            // b. Perform ! Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »).
            unwrap_try(promise_capability.try_resolve(agent, iterator_result.into(), gc.nogc()));
            // c. Return promiseCapability.[[Promise]].
            // SAFETY: scoped_promise is not shared.
            return unsafe { scoped_promise.take(agent).local() };
        };
        // 9. If value is present, then
        // a. Let result be Completion(Call(return, syncIterator, « value »)).
        // 10. Else,
        // a. Let result be Completion(Call(return, syncIterator)).
        let result = {
            let mut value = value;
            call_function(
                agent,
                r#return,
                scoped_sync_iterator.get(agent).local().into(),
                value.as_mut().map(ArgumentsList::from_mut_value),
                gc.reborrow(),
            )?
        };
        // 11. IfAbruptRejectPromise(result, promiseCapability).
        let promise_capability = PromiseCapability {
            promise: scoped_promise.get(agent).local(),
            must_be_unresolved: true,
        };
        let result = if_abrupt_reject_promise_m!(agent, result, promise_capability, gc);
        // 12. If result is not an Object, then
        let Ok(result) = Object::try_from(result) else {
            let gc = gc.into_nogc();
            // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
            let error = agent.create_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator protocol violation: return method returned non-object value",
                gc,
            );
            let promise_capability = PromiseCapability {
                // SAFETY: scoped_promise is not shared.
                promise: unsafe { scoped_promise.take(agent).local() },
                must_be_unresolved: true,
            };
            promise_capability.reject(agent, error, gc);
            // b. Return promiseCapability.[[Promise]].
            return promise_capability.promise();
        };
        // 13. Return AsyncFromSyncIteratorContinuation(result, promiseCapability, syncIteratorRecord, false).
        async_from_sync_iterator_continuation(
            agent,
            result,
            PromiseCapability {
                // SAFETY: scoped_promise is not shared.
                promise: unsafe { scoped_promise.take(agent).local() },
                must_be_unresolved: true,
            },
            // SAFETY: scoped_sync_iterator is not shared.
            unsafe { scoped_sync_iterator.take(agent).local() },
            false,
            gc,
        )
    }

    /// ### [27.1.6.2.3 %AsyncFromSyncIteratorPrototype%.throw ( \[ value \] )](https://tc39.es/ecma262/#sec-%asyncfromsynciteratorprototype%.throw)
    ///
    /// > NOTE: In this specification, value is always provided, but is left
    /// > optional for consistency with
    /// > %AsyncFromSyncIteratorPrototype%.return ( [ value ] ).
    pub(crate) fn throw<'gc>(
        agent: &mut Agent,
        sync_iterator: Object,
        value: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> Promise<'gc> {
        crate::engine::bind!(let value = value, gc);
        // 1. Let O be the this value.
        // 2. Assert: O is an Object that has a [[SyncIteratorRecord]] internal slot.
        // 4. Let syncIteratorRecord be O.[[SyncIteratorRecord]].
        // 5. Let syncIterator be syncIteratorRecord.[[Iterator]].
        crate::engine::bind!(let sync_iterator = sync_iterator, gc);
        let scoped_sync_iterator = sync_iterator.scope(agent, gc.nogc());
        // 3. Let promiseCapability be ! NewPromiseCapability(%Promise%).
        let scoped_promise = PromiseCapability::new(agent, gc.nogc())
            .promise()
            .scope(agent, gc.nogc());
        let value = value.scope(agent, gc.nogc());
        // 6. Let throw be Completion(GetMethod(syncIterator, "throw")).
        let throw = get_object_method(
            agent,
            sync_iterator,
            BUILTIN_STRING_MEMORY.throw.to_property_key(),
            gc.reborrow(),
        )?;
        // 7. IfAbruptRejectPromise(throw, promiseCapability).
        let promise_capability = PromiseCapability {
            promise: scoped_promise.get(agent).local(),
            must_be_unresolved: true,
        };
        let throw = if_abrupt_reject_promise_m!(agent, throw, promise_capability, gc);
        // SAFETY: value is not shared.
        crate::engine::bind!(let value = unsafe { value.take(agent).local() }, gc);
        // 8. If throw is undefined, then
        let Some(throw) = throw else {
            // SAFETY: scoped_sync_iterator is not shared.
            crate::engine::bind!(let sync_iterator = unsafe { scoped_sync_iterator.take(agent).local() }, gc);
            // a. NOTE: If syncIterator does not have a throw method, close it
            //    to give it a chance to clean up before we reject the
            //    capability.
            // b. Let closeCompletion be NormalCompletion(empty).
            // c. Let result be Completion(IteratorClose(syncIteratorRecord, closeCompletion)).
            let result =
                iterator_close_with_value(agent, sync_iterator, Value::Undefined, gc.reborrow())?;
            // d. IfAbruptRejectPromise(result, promiseCapability).
            let promise_capability = PromiseCapability {
                promise: scoped_promise.get(agent).local(),
                must_be_unresolved: true,
            };
            if_abrupt_reject_promise_m!(agent, result, promise_capability, gc);
            // e. NOTE: The next step throws a TypeError to indicate that there
            //    was a protocol violation: syncIterator does not have a throw
            //    method.
            // f. NOTE: If closing syncIterator does not throw then the result
            //    of that operation is ignored, even if it yields a rejected
            //    promise.
            // g. Perform ! Call(promiseCapability.[[Reject]], undefined,
            //    « a newly created TypeError object »).
            let gc = gc.into_nogc();
            let error = agent.create_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator protocol violation: no throw method found",
                gc,
            );
            let promise_capability = PromiseCapability {
                promise: scoped_promise.get(agent).local(),
                must_be_unresolved: true,
            };
            promise_capability.reject(agent, error, gc);
            // h. Return promiseCapability.[[Promise]].
            return promise_capability.promise();
        };

        // 9. If value is present, then
        // a. Let result be Completion(Call(throw, syncIterator, « value »)).
        // 10. Else,
        // a. Let result be Completion(Call(throw, syncIterator)).
        let result = call_function(
            agent,
            throw,
            scoped_sync_iterator.get(agent).local().into(),
            Some(ArgumentsList::from_mut_value(&mut value)),
            gc.reborrow(),
        )?;

        // 11. IfAbruptRejectPromise(result, promiseCapability).
        let promise_capability = PromiseCapability {
            promise: scoped_promise.get(agent).local(),
            must_be_unresolved: true,
        };
        let result = if_abrupt_reject_promise_m!(agent, result, promise_capability, gc);
        // 12. If result is not an Object, then
        let Ok(result) = Object::try_from(result) else {
            let gc = gc.into_nogc();
            // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
            let error = agent.create_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator protocol violation: throw method returned non-object value",
                gc,
            );
            let promise_capability = PromiseCapability {
                // SAFETY: scoped_promise is not shared.
                promise: unsafe { scoped_promise.take(agent).local() },
                must_be_unresolved: true,
            };
            promise_capability.reject(agent, error, gc);
            // b. Return promiseCapability.[[Promise]].
            return promise_capability.promise();
        };

        // 13. Return AsyncFromSyncIteratorContinuation(result, promiseCapability, syncIteratorRecord, true).
        async_from_sync_iterator_continuation(
            agent,
            result,
            PromiseCapability {
                // SAFETY: scoped_promise is not shared.
                promise: unsafe { scoped_promise.take(agent).local() },
                must_be_unresolved: true,
            },
            // SAFETY: scoped_sync_iterator is not shared.
            unsafe { scoped_sync_iterator.take(agent).local() },
            true,
            gc,
        )
    }
}

/// ### [27.1.6.4 AsyncFromSyncIteratorContinuation ( result, promiseCapability, syncIteratorRecord, closeOnRejection )](https://tc39.es/ecma262/#sec-asyncfromsynciteratorcontinuation)
///
/// The abstract operation AsyncFromSyncIteratorContinuation takes arguments
/// result (an Object), promiseCapability (a PromiseCapability Record for an
/// intrinsic %Promise%), syncIteratorRecord (an Iterator Record), and
/// closeOnRejection (a Boolean) and returns a Promise.
pub(crate) fn async_from_sync_iterator_continuation<'a>(
    agent: &mut Agent,
    result: Object,
    promise_capability: PromiseCapability,
    sync_iterator: Object,
    close_on_rejection: bool,
    mut gc: GcScope,
) -> Promise<'a> {
    crate::engine::bind!(let result = result, gc);
    crate::engine::bind!(let promise_capability = promise_capability, gc);

    let scoped_promise = promise_capability.promise.scope(agent, gc.nogc());
    let sync_iterator = sync_iterator.scope(agent, gc.nogc());
    let scoped_result = result.scope(agent, gc.nogc());
    let must_be_unresolved = promise_capability.must_be_unresolved;
    // 1. NOTE: Because promiseCapability is derived from the intrinsic
    //    %Promise%, the calls to promiseCapability.[[Reject]] entailed by the
    //    use IfAbruptRejectPromise below are guaranteed not to throw.
    // 2. Let done be Completion(IteratorComplete(result)).
    let done = iterator_complete(agent, result, gc.reborrow())?;
    // 3. IfAbruptRejectPromise(done, promiseCapability).
    let promise_capability = PromiseCapability {
        promise: scoped_promise.get(agent).local(),
        must_be_unresolved,
    };
    let done = if_abrupt_reject_promise_m!(agent, done, promise_capability, gc);
    // 4. Let value be Completion(IteratorValue(result)).
    // SAFETY: scoped_result is not shared.
    crate::engine::bind!(let result = unsafe { scoped_result.take(agent).local() }, gc);
    let value = iterator_value(agent, result, gc.reborrow())?;
    // 5. IfAbruptRejectPromise(value, promiseCapability).
    let promise_capability = PromiseCapability {
        promise: scoped_promise.get(agent).local(),
        must_be_unresolved,
    };
    let value = if_abrupt_reject_promise_m!(agent, value, promise_capability, gc);
    // 6. Let valueWrapper be Completion(PromiseResolve(%Promise%, value)).
    let value_wrapper = Promise::resolve(agent, value, gc.reborrow())?;
    // 7. If valueWrapper is an abrupt completion, done is false, and closeOnRejection is true, then
    //         a. Set valueWrapper to Completion(IteratorClose(syncIteratorRecord, valueWrapper)).
    // 8. IfAbruptRejectPromise(valueWrapper, promiseCapability).
    // 9. Let unwrap be a new Abstract Closure with parameters (v) that
    //    captures done and performs the following steps when called:
    //         a. Return CreateIteratorResultObject(v, done).
    // 10. Let onFulfilled be CreateBuiltinFunction(unwrap, 1, "", « »).
    // 11. NOTE: onFulfilled is used when processing the "value" property of an
    //     IteratorResult object in order to wait for its value if it is a
    //     promise and re-package the result in a new "unwrapped"
    //     IteratorResult object.
    let on_fulfilled = PromiseReactionHandler::AsyncFromSyncIterator { done };
    // 12. If done is true, or if closeOnRejection is false, then
    let on_rejected = if done || !close_on_rejection {
        // a. Let onRejected be undefined.
        PromiseReactionHandler::Empty
    } else {
        // 13. Else,
        // a. Let closeIterator be a new Abstract Closure with parameters (error) that captures syncIteratorRecord and performs the following steps when called:
        //         i. Return ? IteratorClose(syncIteratorRecord, ThrowCompletion(error)).
        // b. Let onRejected be CreateBuiltinFunction(closeIterator, 1, "", « »).
        // c. NOTE: onRejected is used to close the Iterator when the "value" property of an IteratorResult object it yields is a rejected promise.
        PromiseReactionHandler::AsyncFromSyncIteratorClose(unsafe {
            sync_iterator.take(agent).local()
        })
    };
    // 14. Perform PerformPromiseThen(valueWrapper, onFulfilled, onRejected, promiseCapability).
    let promise_capability = PromiseCapability {
        promise: scoped_promise.get(agent).local(),
        must_be_unresolved,
    };
    inner_promise_then(
        agent,
        value_wrapper,
        on_fulfilled,
        on_rejected,
        Some(promise_capability),
        gc.nogc(),
    );
    // 15. Return promiseCapability.[[Promise]].
    unsafe { scoped_promise.take(agent).local() }
}
