// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::Index;

use crate::ecmascript::types::HeapNumber;
use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
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
    fn delete(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        todo!()
    }

    fn has(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
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

#[inline(always)]
/// ### [24.5.1 CanonicalizeKeyedCollectionKey ( key )](https://tc39.es/ecma262/#sec-canonicalizekeyedcollectionkey)
/// The abstract operation CanonicalizeKeyedCollectionKey takes argument key
/// (an ECMAScript language value) and returns an ECMAScript language value.
pub(crate) fn canonicalize_keyed_collection_key(
    agent: &impl Index<HeapNumber<'static>, Output = f64>,
    key: Value,
) -> Value {
    // 1. If key is -0𝔽, return +0𝔽.
    if let Value::SmallF64(key) = key {
        // Note: Only f32 should hold -0.
        if key.into_f64() == -0.0 {
            return 0.into();
        }
    } else if let Value::Number(key) = key {
        debug_assert_ne!(agent[key], -0.0, "HeapNumber should never be -0.0");
    }
    // 2. Return key.
    key
}
