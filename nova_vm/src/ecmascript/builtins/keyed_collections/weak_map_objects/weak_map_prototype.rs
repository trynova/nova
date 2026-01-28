// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, ExceptionType, JsResult,
        Realm, String, Value, WeakMap, builders::OrdinaryObjectBuilder, can_be_held_weakly,
        throw_not_weak_key_error,
    },
    engine::{Bindable, GcScope, NoGcScope},
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct WeakMapPrototype;

struct WeakMapPrototypeDelete;
impl Builtin for WeakMapPrototypeDelete {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.delete;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::delete);
}
struct WeakMapPrototypeGet;
impl Builtin for WeakMapPrototypeGet {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::get);
}
struct WeakMapPrototypeHas;
impl Builtin for WeakMapPrototypeHas {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.has;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::has);
}
pub(super) struct WeakMapPrototypeSet;
impl Builtin for WeakMapPrototypeSet {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.set;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakMapPrototype::set);
}

impl WeakMapPrototype {
    /// ### [24.3.3.2 WeakMap.prototype.delete ( key )](https://tc39.es/ecma262/#sec-weakmap.prototype.delete)
    ///
    /// > NOTE: The value EMPTY is used as a specification device to indicate
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
        let key = arguments.get(0).bind(gc);
        // 1. Let M be the this value.
        let m = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        let m = require_internal_slot_weak_map_data(agent, m, gc)?;
        // 3. If CanBeHeldWeakly(key) is false,
        let Some(key) = can_be_held_weakly(agent, key) else {
            // return false.
            return Ok(false.into());
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        // a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, then
        // i. Set p.[[Key]] to EMPTY.
        // ii. Set p.[[Value]] to EMPTY.
        // iii. Return true.
        // 5. Return false.
        Ok(m.delete(agent, key).into())
    }

    /// ### [24.3.3.3 WeakMap.prototype.get ( key )](https://tc39.es/ecma262/#sec-weakmap.prototype.get)
    fn get<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let key = arguments.get(0).bind(gc);
        // 1. Let M be the this value.
        let m = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        let m = require_internal_slot_weak_map_data(agent, m, gc)?;
        // 3. If CanBeHeldWeakly(key) is false,
        let Some(key) = can_be_held_weakly(agent, key) else {
            // return undefined.
            return Ok(Value::Undefined);
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        // a. If p.[[Key]] is not empty and SameValue(p.[[Key]], key) is true, return p.[[Value]].
        // 5. Return undefined.
        Ok(m.get_v(agent, key).unwrap_or(Value::Undefined))
    }

    /// ### [24.3.3.4 WeakMap.prototype.has ( key )](https://tc39.es/ecma262/#sec-weakmap.prototype.has)
    fn has<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let key = arguments.get(0).bind(gc);
        // 1. Let M be the this value.
        let m = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        let m = require_internal_slot_weak_map_data(agent, m, gc)?;
        // 3. If CanBeHeldWeakly(key) is false,
        let Some(key) = can_be_held_weakly(agent, key) else {
            // return false.
            return Ok(false.into());
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        // a. If p.[[Key]] is not empty and SameValue(p.[[Key]], key) is true, return true.
        // 5. Return false.
        Ok(m.has(agent, key).into())
    }

    /// ### [24.3.3.5 WeakMap.prototype.set ( key, value )](https://tc39.es/ecma262/#sec-weakmap.prototype.set)
    fn set<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let key = arguments.get(0).bind(gc);
        let value = arguments.get(1).bind(gc);
        // 1. Let M be the this value.
        let m = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        let m = require_internal_slot_weak_map_data(agent, m, gc)?;
        // 3. If CanBeHeldWeakly(key) is false, throw a TypeError exception.
        let Some(key) = can_be_held_weakly(agent, key) else {
            return Err(throw_not_weak_key_error(agent, key, gc));
        };
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        // a. If p.[[Key]] is not empty and SameValue(p.[[Key]], key) is true, then
        // i. Set p.[[Value]] to value.
        // ii. Return M.
        // 5. Let p be the Record { [[Key]]: key, [[Value]]: value }.
        // 6. Append p to M.[[WeakMapData]].
        m.set(agent, key, value);
        // 7. Return M.
        Ok(m.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.weak_map_prototype();
        let weak_map_constructor = intrinsics.weak_map();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(6)
            .with_prototype(object_prototype)
            .with_constructor_property(weak_map_constructor)
            .with_builtin_function_property::<WeakMapPrototypeDelete>()
            .with_builtin_function_property::<WeakMapPrototypeGet>()
            .with_builtin_function_property::<WeakMapPrototypeHas>()
            .with_builtin_function_property::<WeakMapPrototypeSet>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.WeakMap.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[inline(always)]
fn require_internal_slot_weak_map_data<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, WeakMap<'a>> {
    match value {
        Value::WeakMap(map) => Ok(map.bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Object is not a WeakMap",
            gc,
        )),
    }
}
