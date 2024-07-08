// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct PromisePrototype;

struct PromisePrototypeCatch;
impl Builtin for PromisePrototypeCatch {
    const NAME: String = BUILTIN_STRING_MEMORY.catch;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromisePrototype::catch);
}
struct PromisePrototypeFinally;
impl Builtin for PromisePrototypeFinally {
    const NAME: String = BUILTIN_STRING_MEMORY.finally;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromisePrototype::finally);
}
struct PromisePrototypeThen;
impl Builtin for PromisePrototypeThen {
    const NAME: String = BUILTIN_STRING_MEMORY.then;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromisePrototype::then);
}

impl PromisePrototype {
    fn catch(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn finally(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn then(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.promise_prototype();
        let promise_constructor = intrinsics.promise();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<PromisePrototypeCatch>()
            .with_constructor_property(promise_constructor)
            .with_builtin_function_property::<PromisePrototypeFinally>()
            .with_builtin_function_property::<PromisePrototypeThen>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Promise.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
