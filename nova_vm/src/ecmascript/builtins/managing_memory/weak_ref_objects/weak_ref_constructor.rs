// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
            ordinary::ordinary_create_from_constructor,
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics, Realm, add_to_kept_objects, agent::ExceptionType,
            can_be_held_weakly, throw_not_weak_key_error,
        },
        types::{BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, String, Value},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct WeakRefConstructor;
impl Builtin for WeakRefConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakRef;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for WeakRefConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakRef;
}

impl WeakRefConstructor {
    /// #### [26.1.1.1 WeakRef ( target )](https://tc39.es/ecma262/#sec-weak-ref-constructor)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let target = arguments.get(0).bind(gc.nogc());
        let new_target = new_target.bind(gc.nogc());
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin WeakRef constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();
        // 2. If CanBeHeldWeakly(target) is false, throw a TypeError exception.
        let Some(target) = can_be_held_weakly(agent, target) else {
            return Err(throw_not_weak_key_error(
                agent,
                target.unbind(),
                gc.into_nogc(),
            ));
        };
        let target = target.scope(agent, gc.nogc());
        // 3. Let weakRef be ? OrdinaryCreateFromConstructor(NewTarget, "%WeakRef.prototype%", « [[WeakRefTarget]] »).
        let Object::WeakRef(weak_ref) = ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::WeakRef,
            gc.reborrow(),
        )
        .unbind()?
        else {
            unreachable!()
        };
        let gc = gc.into_nogc();
        let weak_ref = weak_ref.bind(gc);
        // SAFETY: target not shared.
        let target = unsafe { target.take(agent) }.bind(gc);
        // 4. Perform AddToKeptObjects(target).
        add_to_kept_objects(agent, target);
        // 5. Set weakRef.[[WeakRefTarget]] to target.
        weak_ref.set_target(agent, target);
        // 6. Return weakRef.
        Ok(weak_ref.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let weak_ref_prototype = intrinsics.weak_ref_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakRefConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_ref_prototype.into_object())
            .build();
    }
}
