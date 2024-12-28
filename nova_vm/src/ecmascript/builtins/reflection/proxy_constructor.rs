// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(ProxyConstructor::behaviour);
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
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn revocable(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        BuiltinFunctionBuilder::new_intrinsic_constructor::<ProxyConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_builtin_function_property::<ProxyRevocable>()
            .build();
    }
}
