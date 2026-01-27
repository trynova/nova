// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, ExceptionType,
        FinalizationRegistry, JsResult, OrdinaryObjectBuilder, Realm, String, Value,
        can_be_held_weakly, same_value,
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct FinalizationRegistryPrototype;

struct FinalizationRegistryPrototypeRegister;
impl Builtin for FinalizationRegistryPrototypeRegister {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.register;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(FinalizationRegistryPrototype::register);
}
struct FinalizationRegistryPrototypeUnregister;
impl Builtin for FinalizationRegistryPrototypeUnregister {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.unregister;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(FinalizationRegistryPrototype::unregister);
}

impl FinalizationRegistryPrototype {
    /// ### [26.2.3.2 FinalizationRegistry.prototype.register ( target, heldValue \[ , unregisterToken \] )](https://tc39.es/ecma262/#sec-finalization-registry.prototype)
    ///
    /// > NOTE: Based on the algorithms and definitions in this specification,
    /// > _cell_.\[\[HeldValue]] is live when _finalizationRegistry_.\[\[Cells]]
    /// > contains _cell_; however, this does not necessarily mean that
    /// > _cell_.\[\[UnregisterToken]] or _cell_.\[\[Target]] are live. For
    /// > example, registering an object with itself as its unregister token
    /// > would not keep the object alive forever.
    fn register<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let target = arguments.get(0).bind(gc);
        let held_value = arguments.get(1).bind(gc);
        let unregister_token = arguments.get(2).bind(gc);
        // 1. Let finalizationRegistry be the this value.
        let finalization_registry = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(finalizationRegistry, [[Cells]]).
        let finalization_registry =
            require_internal_slot_finalization_registry(agent, finalization_registry, gc)?;
        // 3. If CanBeHeldWeakly(target) is false, throw a TypeError exception.
        let Some(target) = can_be_held_weakly(agent, target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "expected target to be an object or symbol",
                gc,
            ));
        };
        // 4. If SameValue(target, heldValue) is true, throw a TypeError exception.
        if same_value(agent, target, held_value) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "target cannot be the held value",
                gc,
            ));
        }
        // 5. If CanBeHeldWeakly(unregisterToken) is false, then
        let unregister_token = if unregister_token.is_undefined() {
            // b. Set unregisterToken to empty.
            None
        } else if let Some(unregister_token) = can_be_held_weakly(agent, unregister_token) {
            Some(unregister_token)
        } else {
            // a. If unregisterToken is not undefined,
            // throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "unregisterToken must be undefined, object, or a symbol",
                gc,
            ));
        };
        // 6. Let cell be the Record {
        //        [[WeakRefTarget]]: target,
        //        [[HeldValue]]: heldValue,
        //        [[UnregisterToken]]: unregisterToken
        //    }.
        // 7. Append cell to finalizationRegistry.[[Cells]].
        finalization_registry.register(agent, target, held_value, unregister_token);
        // 8. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [26.2.3.3 FinalizationRegistry.prototype.unregister ( unregisterToken )](https://tc39.es/ecma262/#sec-finalization-registry.prototype.unregister)
    fn unregister<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let unregister_token = arguments.get(0).bind(gc);
        // 1. Let finalizationRegistry be the this value.
        let finalization_registry = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(finalizationRegistry, [[Cells]]).
        let finalization_registry =
            require_internal_slot_finalization_registry(agent, finalization_registry, gc)?;
        // 3. If CanBeHeldWeakly(unregisterToken) is false, throw a TypeError exception.
        let Some(unregister_token) = can_be_held_weakly(agent, unregister_token) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "unregisterToken must be undefined, object, or a symbol",
                gc,
            ));
        };
        // 4. Let removed be false.
        // 5. For each Record { [[WeakRefTarget]], [[HeldValue]], [[UnregisterToken]] }
        //    cell of finalizationRegistry.[[Cells]], do
        // a. If cell.[[UnregisterToken]] is not empty and
        //    SameValue(cell.[[UnregisterToken]], unregisterToken) is true,
        //    then
        // i. Remove cell from finalizationRegistry.[[Cells]].
        // ii. Set removed to true.
        // 6. Return removed.
        Ok(finalization_registry
            .unregister(agent, unregister_token)
            .into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.finalization_registry_prototype();
        let finalization_registry_constructor = intrinsics.finalization_registry();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(4)
            .with_prototype(object_prototype)
            .with_constructor_property(finalization_registry_constructor)
            .with_builtin_function_property::<FinalizationRegistryPrototypeRegister>()
            .with_builtin_function_property::<FinalizationRegistryPrototypeUnregister>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.FinalizationRegistry.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

fn require_internal_slot_finalization_registry<'gc>(
    agent: &mut Agent,
    finalization_registry: Value<'gc>,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, FinalizationRegistry<'gc>> {
    // 1. Perform ? RequireInternalSlot(finalizationRegistry, [[Cells]]).
    FinalizationRegistry::try_from(finalization_registry).map_err(|_| {
        agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected this to be FinalizationRegistry",
            gc,
        )
    })
}
