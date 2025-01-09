// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct AsyncGeneratorPrototype;

struct AsyncGeneratorPrototypeNext;
impl Builtin for AsyncGeneratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncGeneratorPrototype::next);
}
struct AsyncGeneratorPrototypeReturn;
impl Builtin for AsyncGeneratorPrototypeReturn {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.r#return;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncGeneratorPrototype::r#return);
}
struct AsyncGeneratorPrototypeThrow;
impl Builtin for AsyncGeneratorPrototypeThrow {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.throw;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncGeneratorPrototype::throw);
}

impl AsyncGeneratorPrototype {
    fn next(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn r#return(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn throw(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let async_iterator_prototype = intrinsics.async_iterator_prototype();
        let async_generator_function_prototype = intrinsics.async_generator_function_prototype();
        let this = intrinsics.async_generator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(async_iterator_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.constructor.into())
                    .with_value_readonly(async_generator_function_prototype.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_builtin_function_property::<AsyncGeneratorPrototypeNext>()
            .with_builtin_function_property::<AsyncGeneratorPrototypeReturn>()
            .with_builtin_function_property::<AsyncGeneratorPrototypeThrow>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.AsyncGenerator.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
