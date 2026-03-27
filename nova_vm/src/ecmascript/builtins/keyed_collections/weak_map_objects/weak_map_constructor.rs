// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::hint::unreachable_unchecked;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, ExceptionType, Function, JsResult, Object, ProtoIntrinsics,
        Realm, String, Value, add_entries_from_iterable, builders::BuiltinFunctionBuilder, get,
        is_callable, ordinary_create_from_constructor,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct WeakMapConstructor;
impl Builtin for WeakMapConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakMap;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for WeakMapConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakMap;
}

impl WeakMapConstructor {
    /// ### [24.3.1.1 WeakMap ( \[ iterable \] )](https://tc39.es/ecma262/#sec-weakmap-iterable)
    ///
    /// > NOTE: If the parameter _iterable_ is present, it is expected to be an
    /// > object that implements a %Symbol.iterator% method that returns an
    /// > iterator object that produces a two element array-like object whose
    /// > first element is a value that will be used as a WeakMap key and whose
    /// > second element is the value to associate with that key.
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let iterable = arguments.get(0).bind(gc.nogc());
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "WeakMap Constructor requires 'new'",
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
        let iterable_is_undefined_or_null = iterable.is_undefined() || iterable.is_null();
        let iterable = iterable.scope(agent, gc.nogc());
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%WeakMap.prototype%", « [[WeakMapData]] »).
        let Object::WeakMap(map) = ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::WeakMap,
            gc.reborrow(),
        )
        .unbind()?
        else {
            // SAFETY: ProtoIntrinsics guarded.
            unsafe { unreachable_unchecked() }
        };
        // 3. Set map.[[WeakMapData]] to a new empty List.
        // 4. If iterable is either undefined or null, return map.
        if iterable_is_undefined_or_null {
            return Ok(map.into());
        }
        let scoped_map = map.scope(agent, gc.nogc());
        // 5. Let adder be ? Get(map, "set").
        let adder = get(
            agent,
            map.unbind(),
            BUILTIN_STRING_MEMORY.set.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        let Some(adder) = is_callable(adder, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "expected 'add' to be a function",
                gc.into_nogc(),
            ));
        };
        // 7. Return ? AddEntriesFromIterable(map, iterable, adder).
        add_entries_from_iterable(
            agent,
            // SAFETY: not shared.
            unsafe { scoped_map.take(agent) },
            // SAFETY: not shared.
            unsafe { iterable.take(agent) },
            adder.unbind(),
            gc,
        )
        .map(|m| m.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let weak_map_prototype = intrinsics.weak_map_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakMapConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_map_prototype.into())
            .build();
    }
}
