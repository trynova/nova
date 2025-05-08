// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! [9.9 Processing Model of WeakRef and FinalizationRegistry Targets](https://tc39.es/ecma262/#sec-weakref-processing-model)

use crate::{
    ecmascript::{
        execution::{Agent, weak_key::WeakKey},
        types::{Object, Value},
    },
    engine::context::NoGcScope,
};

use super::agent::{ExceptionType, JsError};

/// ### [9.10 ClearKeptObjects ( )](https://tc39.es/ecma262/#sec-clear-kept-objects)
///
/// The abstract operation ClearKeptObjects takes no arguments and returns
/// unused. ECMAScript implementations are expected to call ClearKeptObjects
/// when a synchronous sequence of ECMAScript executions completes.
pub(super) fn clear_kept_objects(agent: &mut Agent) {
    // 1. Let agentRecord be the surrounding agent's Agent Record.
    // 2. Set agentRecord.[[KeptAlive]] to a new empty List.
    if agent.kept_alive {
        agent.kept_alive = false;
        for weak_ref_data in agent.heap.weak_refs.iter_mut() {
            if let Some(weak_ref_data) = weak_ref_data.as_mut() {
                weak_ref_data.clear_kept_objects();
            }
        }
    }
    // 3. Return unused.
}

/// ### [9.11 AddToKeptObjects ( value )](https://tc39.es/ecma262/#sec-addtokeptobjects)
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

/// ### [9.13 CanBeHeldWeakly ( v )](https://tc39.es/ecma262/#sec-canbeheldweakly)
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
pub(crate) fn can_be_held_weakly(v: Value) -> Option<WeakKey> {
    // 1. If v is an Object, return true.
    if let Ok(v) = Object::try_from(v) {
        Some(v.into())
    } else if let Value::Symbol(v) = v {
        // 2. If v is a Symbol and KeyForSymbol(v) is undefined, return true.
        // TODO: KeyForSymbol
        Some(WeakKey::Symbol(v))
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
        string_repr.as_str(agent)
    );
    agent.throw_exception(ExceptionType::TypeError, message, gc)
}
