// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::builtins::proxy::proxy_create;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::types::IntoValue;
use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct ProxyConstructor;
impl Builtin for ProxyConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Proxy;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for ProxyConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Proxy;
}

struct ProxyRevocable;
impl Builtin for ProxyRevocable {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.revocable;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ProxyConstructor::revocable);
}

impl ProxyConstructor {
    /// ### [28.2.1.1 Proxy ( target, handler )](https://tc39.es/ecma262/#sec-proxy-target-handler)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let gc = gc.into_nogc();
        let target = arguments.get(0);
        let handler = arguments.get(1);
        // 1. If NewTarget is undefined, throw a TypeError exception.
        if new_target.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin Proxy constructor without new is forbidden",
                gc,
            ));
        }
        // 2. Return ? ProxyCreate(target, handler).
        proxy_create(agent, target, handler, gc).map(|proxy| proxy.into_value())
    }

    fn revocable<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        BuiltinFunctionBuilder::new_intrinsic_constructor::<ProxyConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_builtin_function_property::<ProxyRevocable>()
            .build();
    }
}
