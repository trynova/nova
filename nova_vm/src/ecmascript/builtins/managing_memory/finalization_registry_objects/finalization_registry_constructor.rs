// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::hint::unreachable_unchecked;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinFunctionBuilder,
        BuiltinIntrinsicConstructor, ExceptionType, Function, JsResult, Object, ProtoIntrinsics,
        Realm, String, Value, is_callable, ordinary_create_from_constructor,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::{ArenaAccess, IntrinsicConstructorIndexes},
};

pub(crate) struct FinalizationRegistryConstructor;
impl Builtin for FinalizationRegistryConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.FinalizationRegistry;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for FinalizationRegistryConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::FinalizationRegistry;
}

impl FinalizationRegistryConstructor {
    /// ### [26.2.1.1 FinalizationRegistry ( cleanupCallback )](https://tc39.es/ecma262/#sec-finalization-registry-cleanup-callback)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let cleanup_callback = arguments.get(0).bind(gc.nogc());
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "FinalizationRegistry Constructor requires 'new'",
                gc.into_nogc(),
            ));
        };
        let Ok(new_target) = Function::try_from(new_target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Function Proxies not yet supported",
                gc.into_nogc(),
            ));
        };
        // 2. If IsCallable(cleanupCallback) is false, throw a TypeError
        //    exception.
        let Some(cleanup_callback) = is_callable(cleanup_callback, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "cleanupCallback is not a function",
                gc.into_nogc(),
            ));
        };
        let cleanup_callback = cleanup_callback.scope(agent, gc.nogc());
        // 3. Let finalizationRegistry be ?
        //    OrdinaryCreateFromConstructor(
        //      NewTarget,
        //      "%FinalizationRegistry.prototype%",
        //      « [[Realm]], [[CleanupCallback]], [[Cells]] »
        //    ).
        let Object::FinalizationRegistry(finalization_registry) = ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::FinalizationRegistry,
            gc.reborrow(),
        )
        .unbind()?
        else {
            // SAFETY: ProtoIntrinsics guarded.
            unsafe { unreachable_unchecked() }
        };
        let gc = gc.into_nogc();
        let finalization_registry = finalization_registry.bind(gc);
        // SAFETY: not shared.
        let cleanup_callback = unsafe { cleanup_callback.take(agent) }.bind(gc);
        // 4. Let fn be the active function object.
        // 5. Set finalizationRegistry.[[Realm]] to fn.[[Realm]].
        let realm = match agent.active_function_object(gc) {
            Function::BoundFunction(_) => {
                unreachable!("bound function constructing FinalizationRegistry")
            }
            Function::BuiltinFunction(f) => f.get(agent).realm.bind(gc),
            Function::ECMAScriptFunction(f) => f.get(agent).ecmascript_function.realm.bind(gc),
            Function::BuiltinConstructorFunction(f) => f.get(agent).realm.bind(gc),
            Function::BuiltinPromiseResolvingFunction(_) => {
                unreachable!("builtin promise resolving function constructing FinalizationRegistry")
            }
            Function::BuiltinPromiseFinallyFunction(_) => {
                unreachable!("builtin promise finally function constructing FinalizationRegistry")
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        };
        // 6. Set finalizationRegistry.[[CleanupCallback]] to
        //    HostMakeJobCallback(cleanupCallback).
        // 7. Set finalizationRegistry.[[Cells]] to a new empty List.
        // SAFETY: initialising new FR.
        unsafe { finalization_registry.initialise(agent, realm, cleanup_callback) };
        // 8. Return finalizationRegistry.
        Ok(finalization_registry.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let finalization_registry_prototype = intrinsics.finalization_registry_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<FinalizationRegistryConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(finalization_registry_prototype.into())
        .build();
    }
}
