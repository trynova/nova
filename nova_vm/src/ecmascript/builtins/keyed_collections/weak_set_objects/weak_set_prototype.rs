// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::builtins::weak_set::WeakSet;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::{can_be_held_weakly, throw_not_weak_key_error};
use crate::ecmascript::types::IntoValue;
use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct WeakSetPrototype;

struct WeakSetPrototypeAdd;
impl Builtin for WeakSetPrototypeAdd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.add;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakSetPrototype::add);
}
struct WeakSetPrototypeDelete;
impl Builtin for WeakSetPrototypeDelete {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.delete;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakSetPrototype::delete);
}
struct WeakSetPrototypeHas;
impl Builtin for WeakSetPrototypeHas {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakSetPrototype::has);
}

impl WeakSetPrototype {
    /// ### [24.4.3.1 WeakSet.prototype.add ( value )](https://tc39.es/ecma262/#sec-weakset.prototype.add)
    pub(crate) fn add<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        let value = arguments.get(0).bind(gc);

        // 1. Let S be the this value.
        let s = this_value;
        // 2. Perform ? RequireInternalSlot(S, [[WeakSetData]]).
        let s = require_internal_slot_weak_set(agent, s, gc)?;
        // 3. If CanBeHeldWeakly(value) is false, throw a TypeError exception.
        let Some(value) = can_be_held_weakly(value) else {
            return Err(throw_not_weak_key_error(agent, value.unbind(), gc));
        };
        // 4. For each element e of S.[[WeakSetData]], do
        // a. If e is not empty and SameValue(e, value) is true, then
        // i. Return S.
        // 5. Append value to S.[[WeakSetData]].
        // 6. Return S.
        agent[s].add(value);
        Ok(s.into_value().unbind())
    }

    /// ### [24.4.3.3 WeakSet.prototype.delete ( value )](https://tc39.es/ecma262/#sec-weakset.prototype.delete)
    ///
    /// > Note: The value empty is used as a specification device to indicate
    /// > that an entry has been deleted. Actual implementations may take other
    /// > actions such as physically removing the entry from internal data
    /// > structures.
    fn delete<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        let value = arguments.get(0).bind(gc);

        // 1. Let S be the this value.
        let s = this_value;
        // 2. Perform ? RequireInternalSlot(S, [[WeakSetData]]).
        let s = require_internal_slot_weak_set(agent, s, gc)?;
        // 3. If CanBeHeldWeakly(value) is false, return false.
        let Some(value) = can_be_held_weakly(value) else {
            return Ok(false.into_value());
        };
        // 4. For each element e of S.[[WeakSetData]], do
        // a. If e is not empty and SameValue(e, value) is true, then
        // i. Replace the element of S.[[WeakSetData]] whose value is e with an
        //    element whose value is empty.
        // ii. Return true.
        let deleted = agent[s].delete(value);
        // 5. Return false.
        Ok(deleted.into_value())
    }

    /// ### [24.4.3.4 WeakSet.prototype.has ( value )](https://tc39.es/ecma262/#sec-weakset.prototype.has)
    fn has<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        let value = arguments.get(0).bind(gc);

        // 1. Let S be the this value.
        let s = this_value;
        // 2. Perform ? RequireInternalSlot(S, [[WeakSetData]]).
        let s = require_internal_slot_weak_set(agent, s, gc)?;
        // 3. If CanBeHeldWeakly(value) is false, return false.
        let Some(value) = can_be_held_weakly(value) else {
            return Ok(false.into_value());
        };
        // 4. For each element e of S.[[WeakSetData]], do
        // a. If e is not empty and SameValue(e, value) is true, return true.
        // 5. Return false.
        let result = agent[s].has(value);
        Ok(result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.weak_set_prototype();
        let weak_set_constructor = intrinsics.weak_set();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<WeakSetPrototypeAdd>()
            .with_constructor_property(weak_set_constructor)
            .with_builtin_function_property::<WeakSetPrototypeDelete>()
            .with_builtin_function_property::<WeakSetPrototypeHas>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.WeakSet.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[inline]
fn require_internal_slot_weak_set<'a>(
    agent: &mut Agent,
    o: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, WeakSet<'a>> {
    match o {
        // 1. Perform ? RequireInternalSlot(O, [[WeakSetData]]).
        Value::WeakSet(array_buffer) => Ok(array_buffer.unbind().bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected this to be WeakSet",
            gc,
        )),
    }
}
