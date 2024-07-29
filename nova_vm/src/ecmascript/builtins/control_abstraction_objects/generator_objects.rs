// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct GeneratorPrototype;

pub(crate) struct GeneratorPrototypeNext;
impl Builtin for GeneratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(GeneratorPrototype::next);
}
impl BuiltinIntrinsic for GeneratorPrototypeNext {
    const INDEX: IntrinsicFunctionIndexes =
        IntrinsicFunctionIndexes::GeneratorFunctionPrototypePrototypeNext;
}
pub(crate) struct GeneratorPrototypeReturn;
impl Builtin for GeneratorPrototypeReturn {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.r#return;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(GeneratorPrototype::r#return);
}
pub(crate) struct GeneratorPrototypeThrow;
impl Builtin for GeneratorPrototypeThrow {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.throw;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(GeneratorPrototype::throw);
}

impl GeneratorPrototype {
    fn next<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn r#return(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn throw<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let iterator_prototype = intrinsics.iterator_prototype();
        let generator_function_prototype = intrinsics.generator_function_prototype();
        let this = intrinsics.generator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(iterator_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.constructor.into())
                    .with_value_readonly(generator_function_prototype.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_builtin_intrinsic_function_property::<GeneratorPrototypeNext>()
            .with_builtin_function_property::<GeneratorPrototypeReturn>()
            .with_builtin_function_property::<GeneratorPrototypeThrow>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Generator.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
