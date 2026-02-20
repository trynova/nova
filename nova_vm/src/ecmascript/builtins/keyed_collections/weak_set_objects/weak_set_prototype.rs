// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, ExceptionType, JsResult,
        Realm, String, Value, WeakSet, builders::OrdinaryObjectBuilder, can_be_held_weakly,
        throw_not_weak_key_error,
    },
    engine::{Bindable, GcScope, NoGcScope},
    heap::{ArenaAccess, ArenaAccessMut, WellKnownSymbolIndexes},
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
        arguments: ArgumentsList<'_, 'static>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        let gc = gc.into_nogc();
        crate::engine::bind!(let this_value = this_value, gc);
        crate::engine::bind!(let value = arguments.get(0), gc);

        // 1. Let S be the this value.
        let s = this_value;
        // 2. Perform ? RequireInternalSlot(S, [[WeakSetData]]).
        let s = require_internal_slot_weak_set(agent, s, gc)?;
        // 3. If CanBeHeldWeakly(value) is false, throw a TypeError exception.
        let Some(value) = can_be_held_weakly(agent, value) else {
            return Err(throw_not_weak_key_error(agent, value, gc));
        };
        // 4. For each element e of S.[[WeakSetData]], do
        // a. If e is not empty and SameValue(e, value) is true, then
        // i. Return S.
        // 5. Append value to S.[[WeakSetData]].
        // 6. Return S.
        s.get_mut(agent).add(value);
        Ok(s.into())
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
        arguments: ArgumentsList<'_, 'static>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        let gc = gc.into_nogc();
        crate::engine::bind!(let this_value = this_value, gc);
        crate::engine::bind!(let value = arguments.get(0), gc);

        // 1. Let S be the this value.
        let s = this_value;
        // 2. Perform ? RequireInternalSlot(S, [[WeakSetData]]).
        let s = require_internal_slot_weak_set(agent, s, gc)?;
        // 3. If CanBeHeldWeakly(value) is false, return false.
        let Some(value) = can_be_held_weakly(agent, value) else {
            return Ok(false.into());
        };
        // 4. For each element e of S.[[WeakSetData]], do
        // a. If e is not empty and SameValue(e, value) is true, then
        // i. Replace the element of S.[[WeakSetData]] whose value is e with an
        //    element whose value is empty.
        // ii. Return true.
        let deleted = s.get_mut(agent).delete(value);
        // 5. Return false.
        Ok(deleted.into())
    }

    /// ### [24.4.3.4 WeakSet.prototype.has ( value )](https://tc39.es/ecma262/#sec-weakset.prototype.has)
    fn has<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList<'_, 'static>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        let gc = gc.into_nogc();
        crate::engine::bind!(let this_value = this_value, gc);
        crate::engine::bind!(let value = arguments.get(0), gc);

        // 1. Let S be the this value.
        let s = this_value;
        // 2. Perform ? RequireInternalSlot(S, [[WeakSetData]]).
        let s = require_internal_slot_weak_set(agent, s, gc)?;
        // 3. If CanBeHeldWeakly(value) is false, return false.
        let Some(value) = can_be_held_weakly(agent, value) else {
            return Ok(false.into());
        };
        // 4. For each element e of S.[[WeakSetData]], do
        // a. If e is not empty and SameValue(e, value) is true, return true.
        // 5. Return false.
        let result = s.get(agent).local().has(value);
        Ok(result.into())
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
                    .with_value_readonly(BUILTIN_STRING_MEMORY.WeakSet.into())
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
        Value::WeakSet(array_buffer) => Ok(array_buffer),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected this to be WeakSet",
            gc,
        )),
    }
}
