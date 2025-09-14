// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoValue, String, Value},
    },
    engine::context::GcScope,
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
struct WeakMapPrototypeSet;
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
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        // 3. If CanBeHeldWeakly(key) is false, return false.
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        //         a. If p.[[Key]] is not EMPTY and SameValue(p.[[Key]], key) is true, then
        //                 i. Set p.[[Key]] to EMPTY.
        //                 ii. Set p.[[Value]] to EMPTY.
        //                 iii. Return true.
        // 5. Return false.
        Err(agent.todo("WeakMap.prototype.delete", gc.into_nogc()))
    }

    /// ### [24.3.3.3 WeakMap.prototype.get ( key )](https://tc39.es/ecma262/#sec-weakmap.prototype.get)
    fn get<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        // 3. If CanBeHeldWeakly(key) is false, return false.
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        //        a. If p.[[Key]] is not empty and SameValue(p.[[Key]], key) is true, return true.
        // 5. Return false.
        Err(agent.todo("WeakMap.prototype.get", gc.into_nogc()))
    }

    /// ### [24.3.3.4 WeakMap.prototype.has ( key )](https://tc39.es/ecma262/#sec-weakmap.prototype.has)
    fn has<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        // 3. If CanBeHeldWeakly(key) is false, return false.
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        //        a. If p.[[Key]] is not empty and SameValue(p.[[Key]], key) is true, return true.
        // 5. Return false.
        Err(agent.todo("WeakMap.prototype.has", gc.into_nogc()))
    }

    /// ### [24.3.3.5 WeakMap.prototype.set ( key, value )](https://tc39.es/ecma262/#sec-weakmap.prototype.set)
    fn set<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let M be the this value.
        // 2. Perform ? RequireInternalSlot(M, [[WeakMapData]]).
        // 3. If CanBeHeldWeakly(key) is false, throw a TypeError exception.
        // 4. For each Record { [[Key]], [[Value]] } p of M.[[WeakMapData]], do
        //        a. If p.[[Key]] is not empty and SameValue(p.[[Key]], key) is true, then
        //               i. Set p.[[Value]] to value.
        //               ii. Return M.
        // 5. Let p be the Record { [[Key]]: key, [[Value]]: value }.
        // 6. Append p to M.[[WeakMapData]].
        // 7. Return M.
        Err(agent.todo("WeakMap.prototype.set", gc.into_nogc()))
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
                    .with_value_readonly(BUILTIN_STRING_MEMORY.WeakMap.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
