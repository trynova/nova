// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct PromiseConstructor;
impl Builtin for PromiseConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Promise;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(PromiseConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for PromiseConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Promise;
}
struct PromiseAll;
impl Builtin for PromiseAll {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.all;
}
struct PromiseAllSettled;
impl Builtin for PromiseAllSettled {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all_settled);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.allSettled;
}
struct PromiseAny;
impl Builtin for PromiseAny {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::any);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.any;
}
struct PromiseRace;
impl Builtin for PromiseRace {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::race);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.race;
}
struct PromiseReject;
impl Builtin for PromiseReject {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::reject);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.reject;
}
struct PromiseResolve;
impl Builtin for PromiseResolve {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::resolve);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.resolve;
}
struct PromiseWithResolvers;
impl Builtin for PromiseWithResolvers {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::with_resolvers);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.withResolvers;
}
struct PromiseGetSpecies;
impl Builtin for PromiseGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for PromiseGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}

impl PromiseConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn all(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn all_settled(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn any(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn race(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn reject(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn resolve(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn with_resolvers(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_species(_: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let promise_prototype = intrinsics.promise_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<PromiseConstructor>(agent, realm)
            .with_property_capacity(9)
            .with_builtin_function_property::<PromiseAll>()
            .with_builtin_function_property::<PromiseAllSettled>()
            .with_builtin_function_property::<PromiseAny>()
            .with_prototype_property(promise_prototype.into_object())
            .with_builtin_function_property::<PromiseRace>()
            .with_builtin_function_property::<PromiseReject>()
            .with_builtin_function_property::<PromiseResolve>()
            .with_builtin_function_property::<PromiseWithResolvers>()
            .with_builtin_function_getter_property::<PromiseGetSpecies>()
            .build();
    }
}
