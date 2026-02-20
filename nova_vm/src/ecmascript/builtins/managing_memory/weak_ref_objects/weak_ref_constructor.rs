// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, ExceptionType, Function, JsResult, Object, ProtoIntrinsics,
        Realm, String, Value, add_to_kept_objects, builders::BuiltinFunctionBuilder,
        can_be_held_weakly, ordinary_create_from_constructor, throw_not_weak_key_error,
    },
    engine::{Bindable, GcScope, Scopable},
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
    /// ### [26.1.1.1 WeakRef ( target )](https://tc39.es/ecma262/#sec-weak-ref-constructor)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList<'_, 'static>,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        crate::engine::bind!(let target = arguments.get(0), gc);
        crate::engine::bind!(let new_target = new_target, gc);
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
            return Err(throw_not_weak_key_error(agent, target, gc.into_nogc()));
        };
        let target = target.scope(agent, gc.nogc());
        // 3. Let weakRef be ? OrdinaryCreateFromConstructor(NewTarget, "%WeakRef.prototype%", « [[WeakRefTarget]] »).
        let Object::WeakRef(weak_ref) = ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::WeakRef,
            gc.reborrow(),
        )?
        else {
            unreachable!()
        };
        let gc = gc.into_nogc();
        crate::engine::bind!(let weak_ref = weak_ref, gc);
        // SAFETY: target not shared.
        crate::engine::bind!(let target = unsafe { target.take(agent).local() }, gc);
        // 4. Perform AddToKeptObjects(target).
        add_to_kept_objects(agent, target);
        // 5. Set weakRef.[[WeakRefTarget]] to target.
        weak_ref.set_target(agent, target);
        // 6. Return weakRef.
        Ok(weak_ref.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let weak_ref_prototype = intrinsics.weak_ref_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakRefConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_ref_prototype.into())
            .build();
    }
}
