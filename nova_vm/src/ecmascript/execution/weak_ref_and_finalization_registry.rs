// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//!## [9.9 Processing Model of WeakRef and FinalizationRegistry Targets](https://tc39.es/ecma262/#sec-weakref-processing-model)

use crate::{
    ecmascript::{
        Agent, ArgumentsList, ExceptionType, FinalizationRegistry, JsError, JsResult, Object,
        Value, WeakKey, call_function, key_for_symbol,
    },
    engine::{
        Global, ScopableCollection,
        Bindable, GcScope, NoGcScope,
        Scopable,
    },
};

/// ## [9.10 ClearKeptObjects ( )](https://tc39.es/ecma262/#sec-clear-kept-objects)
///
/// The abstract operation ClearKeptObjects takes no arguments and returns
/// unused. ECMAScript implementations are expected to call ClearKeptObjects
/// when a synchronous sequence of ECMAScript executions completes.
pub(crate) fn clear_kept_objects(agent: &mut Agent) {
    // 1. Let agentRecord be the surrounding agent's Agent Record.
    // 2. Set agentRecord.[[KeptAlive]] to a new empty List.
    if agent.kept_alive {
        agent.kept_alive = false;
        for weak_ref_data in agent.heap.weak_refs.iter_mut() {
            weak_ref_data.clear_kept_objects();
        }
    }
    // 3. Return unused.
}

/// ## [9.11 AddToKeptObjects ( value )](https://tc39.es/ecma262/#sec-addtokeptobjects)
///
/// The abstract operation AddToKeptObjects takes argument value (an Object or
/// a Symbol) and returns unused.
///
/// > Note: When the abstract operation AddToKeptObjects is called with a
/// > target object or symbol, it adds the target to a list that will point
/// > strongly at the target until ClearKeptObjects is called.
pub(crate) fn add_to_kept_objects(agent: &mut Agent, _value: WeakKey) {
    // 1. Let agentRecord be the surrounding agent's Agent Record.
    // 2. Append value to agentRecord.[[KeptAlive]].
    agent.kept_alive = true;
    // 3. Return unused.
}

pub(crate) struct FinalizationRegistryCleanupJob {
    finalization_registry: Global<FinalizationRegistry<'static>>,
}

impl FinalizationRegistryCleanupJob {
    pub(crate) fn new(agent: &mut Agent, finalization_registry: FinalizationRegistry) -> Self {
        Self {
            finalization_registry: Global::new(agent, finalization_registry.unbind()),
        }
    }
    pub(crate) fn run<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) {
        let finalization_registry = self.finalization_registry.take(agent).bind(gc.nogc());
        // 1. Let cleanupResult be
        //    Completion(CleanupFinalizationRegistry(finalizationRegistry)).
        let cleanup_result =
            cleanup_finalization_registry(agent, finalization_registry.unbind(), gc);
        // 2. If cleanupResult is an abrupt completion, perform any host-defined steps for reporting the error.
        if cleanup_result.is_err() {
            let _ = cleanup_result;
        }
        // 3. Return unused.
    }
}

/// ## [9.12 CleanupFinalizationRegistry ( finalizationRegistry )](https://tc39.es/ecma262/#sec-cleanup-finalization-registry)
///
/// The abstract operation CleanupFinalizationRegistry takes argument
/// finalizationRegistry (a FinalizationRegistry) and returns either a normal
/// completion containing unused or a throw completion.
pub(crate) fn cleanup_finalization_registry<'gc>(
    agent: &mut Agent,
    finalization_registry: FinalizationRegistry,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, ()> {
    let finalization_registry = finalization_registry.bind(gc.nogc());
    // 1. Assert: finalizationRegistry has [[Cells]] and [[CleanupCallback]]
    //    internal slots.
    // 2. Let callback be finalizationRegistry.[[CleanupCallback]].
    let (callback, mut queue) = finalization_registry.get_cleanup_queue(agent);
    if queue.is_empty() {
        return Ok(());
    }
    let value = queue.pop().unwrap();
    // 3. While finalizationRegistry.[[Cells]] contains a Record cell such that
    //    cell.[[WeakRefTarget]] is empty, an implementation may perform the
    //    following steps:
    //         a. Choose any such cell.
    //         b. Remove cell from finalizationRegistry.[[Cells]].
    //         c. Perform ? HostCallJobCallback(callback, undefined, « cell.[[HeldValue]] »).
    if queue.len() == 1 {
        // 2. Return ? Call(jobCallback.[[Callback]], V, argumentsList).
        let _ = call_function(
            agent,
            callback.unbind(),
            Value::Undefined,
            Some(ArgumentsList::from_mut_value(&mut value.unbind())),
            gc,
        )?;
    } else {
        let scoped_callback = callback.scope(agent, gc.nogc());
        let finalization_registry = finalization_registry.scope(agent, gc.nogc());
        let queue = queue.scope(agent, gc.nogc());
        let result = call_function(
            agent,
            callback.unbind(),
            Value::Undefined,
            Some(ArgumentsList::from_mut_value(&mut value.unbind())),
            gc.reborrow(),
        )
        .unbind()
        .bind(gc.nogc());
        let mut err_and_i = None;
        if let Err(err) = result {
            err_and_i = Some((err.unbind(), 0));
        } else {
            for (i, value) in queue.iter(agent).enumerate() {
                let value = value.get(gc.nogc());
                let result = call_function(
                    agent,
                    scoped_callback.get(agent),
                    Value::Undefined,
                    Some(ArgumentsList::from_mut_value(&mut value.unbind())),
                    gc.reborrow(),
                )
                .unbind()
                .bind(gc.nogc());
                if let Err(err) = result {
                    err_and_i = Some((err.unbind(), i));
                    break;
                }
            }
        }
        // If an error was thrown by a cleanup callback, we interrupt any
        // further cleanup and add any leftover cleanups back into the
        // FinalizationRegistry. This will re-request cleanup of the registry
        // if necessary.
        if let Some((err, i)) = err_and_i {
            let err = err.unbind();
            let gc = gc.into_nogc();
            let err = err.bind(gc);
            let mut queue = queue.take(agent).bind(gc);
            // Drain all elements that were found so far.
            queue.drain(0..i);
            let finalization_registry = unsafe { finalization_registry.take(agent) }.bind(gc);
            let _ = unsafe { scoped_callback.take(agent) };
            if !queue.is_empty() {
                finalization_registry.add_cleanups(agent, queue);
            }
            return Err(err);
        }
    }
    // 4. Return unused.
    Ok(())
}

/// ## [9.13 CanBeHeldWeakly ( v )](https://tc39.es/ecma262/#sec-canbeheldweakly)
///
/// The abstract operation CanBeHeldWeakly takes argument v (an ECMAScript
/// language value) and returns a Boolean. It returns true if and only if v is
/// suitable for use as a weak reference. Only values that are suitable for use
/// as a weak reference may be a key of a WeakMap, an element of a WeakSet, the
/// target of a WeakRef, or one of the targets of a FinalizationRegistry.
///
/// > NOTE: A language value without language identity can be manifested
/// > without prior reference and is unsuitable for use as a weak reference. A
/// > Symbol value produced by Symbol.for, unlike other Symbol values, does
/// > not have language identity and is unsuitable for use as a weak reference.
/// > Well-known symbols are likely to never be collected, but are nonetheless
/// > treated as suitable for use as a weak reference because they are limited
/// > in number and therefore manageable by a variety of implementation
/// > approaches. However, any value associated to a well-known symbol in a
/// > live WeakMap is unlikely to be collected and could “leak” memory
/// > resources in implementations.
///
/// > NOTE: We return an option of a WeakKey enum instead of a boolean.
pub(crate) fn can_be_held_weakly<'a>(agent: &Agent, v: Value<'a>) -> Option<WeakKey<'a>> {
    // 1. If v is an Object, return true.
    if let Ok(v) = Object::try_from(v) {
        Some(v.into())
    } else if let Value::Symbol(v) = v {
        // 2. If v is a Symbol and KeyForSymbol(v) is undefined, return true.
        if key_for_symbol(agent, v).is_some() {
            None
        } else {
            Some(WeakKey::Symbol(v))
        }
    } else {
        // 3. Return false.
        None
    }
}

pub(crate) fn throw_not_weak_key_error<'a>(
    agent: &mut Agent,
    target: Value,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    let string_repr = target.try_string_repr(agent, gc);
    let message = format!(
        "{} is not a non-null object or unique symbol",
        string_repr.to_string_lossy_(agent)
    );
    agent.throw_exception(ExceptionType::TypeError, message, gc)
}
