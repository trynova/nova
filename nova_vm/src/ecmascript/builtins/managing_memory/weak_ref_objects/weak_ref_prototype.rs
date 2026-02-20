// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, ExceptionType, JsResult,
        Realm, String, Value, WeakRef, add_to_kept_objects, builders::OrdinaryObjectBuilder,
    },
    engine::{Bindable, GcScope},
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct WeakRefPrototype;

struct WeakRefPrototypeDeref;
impl Builtin for WeakRefPrototypeDeref {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.deref;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(WeakRefPrototype::deref);
}

impl WeakRefPrototype {
    /// ### [26.1.3.2 WeakRef.prototype.deref ( )](https://tc39.es/ecma262/#sec-weak-ref.prototype.deref)
    fn deref<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList<'_, 'static>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        let gc = gc.into_nogc();
        // 1. Let weakRef be the this value.
        // 2. Perform ? RequireInternalSlot(weakRef, [[WeakRefTarget]]).
        let Value::WeakRef(weak_ref) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Receiver of WeakRef.deref call is not a WeakRef",
                gc,
            ));
        };
        // 3. Return WeakRefDeref(weakRef).
        Ok(weak_ref_deref(agent, weak_ref))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.weak_ref_prototype();
        let weak_ref_constructor = intrinsics.weak_ref();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(object_prototype)
            .with_constructor_property(weak_ref_constructor)
            .with_builtin_function_property::<WeakRefPrototypeDeref>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.WeakRef.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

/// ### [26.1.4.1 WeakRefDeref ( weakRef )](https://tc39.es/ecma262/#sec-weakrefderef)
///
/// The abstract operation WeakRefDeref takes argument weakRef (a WeakRef) and
/// returns an ECMAScript language value.
///
/// > Note: This abstract operation is defined separately from
/// > WeakRef.prototype.deref strictly to make it possible to succinctly define
/// > liveness.
#[inline(always)]
fn weak_ref_deref<'a>(agent: &mut Agent, weak_ref: WeakRef<'a>) -> Value<'a> {
    // 1. Let target be weakRef.[[WeakRefTarget]].
    let target = weak_ref.get_target(agent);
    // 2. If target is not empty, then
    if let Some(target) = target {
        // a. Perform AddToKeptObjects(target).
        add_to_kept_objects(agent, target);
        // b. Return target.
        target.into()
    } else {
        // 3. Return undefined.
        Value::Undefined
    }
}
