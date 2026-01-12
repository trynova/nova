// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor, proxy::proxy_create,
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, Object, String, Value},
    },
    engine::context::{Bindable, GcScope},
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
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let target = arguments.get(0).bind(gc);
        let handler = arguments.get(1).bind(gc);
        // 1. If NewTarget is undefined, throw a TypeError exception.
        if new_target.is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin Proxy constructor without new is forbidden",
                gc,
            ));
        }
        // 2. Return ? ProxyCreate(target, handler).
        proxy_create(agent, target, handler, gc).map(|proxy| proxy.into())
    }

    /// ### [28.2.2.1 Proxy.revocable ( target, handler )](https://tc39.es/ecma262/#sec-proxy.revocable)
    fn revocable<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let proxy be ? ProxyCreate(target, handler).
        // 2. Let revokerClosure be a new Abstract Closure with no parameters that captures nothing and performs the following steps when called:
        //        a. Let F be the active function object.
        //        b. Let p be F.[[RevocableProxy]].
        //        c. If p is null, return NormalCompletion(undefined).
        //        d. Set F.[[RevocableProxy]] to null.
        //        e. Assert: p is a Proxy exotic object.
        //        f. Set p.[[ProxyTarget]] to null.
        //        g. Set p.[[ProxyHandler]] to null.
        //        h. Return NormalCompletion(undefined).
        // 3. Let revoker be CreateBuiltinFunction(revokerClosure, 0, "", « [[RevocableProxy]] »).
        // 4. Set revoker.[[RevocableProxy]] to proxy.
        // 5. Let result be OrdinaryObjectCreate(%Object.prototype%).
        // 6. Perform ! CreateDataPropertyOrThrow(result, "proxy", proxy).
        // 7. Perform ! CreateDataPropertyOrThrow(result, "revoke", revoker).
        // 8. Return result.
        Err(agent.todo("Proxy.revocable", gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        BuiltinFunctionBuilder::new_intrinsic_constructor::<ProxyConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_builtin_function_property::<ProxyRevocable>()
            .build();
    }
}
