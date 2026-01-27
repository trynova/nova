// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinIntrinsic,
        ExceptionType, JsResult, OrdinaryObjectBuilder, Realm, String, Value,
    },
    engine::{Bindable, GcScope},
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
    fn next<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // GeneratorResume: 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
                gc.into_nogc(),
            ));
        };

        // 1. Return ? GeneratorResume(this value, value, empty).
        generator.resume(agent, arguments.get(0), gc)
    }

    /// ### [27.5.1.3 %GeneratorPrototype%.return ( value )](https://tc39.es/ecma262/#sec-generator.prototype.return)
    fn r#return<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let value = arguments.get(0).bind(gc.nogc());
        // 1. Let g be the this value.
        let g = this_value;
        // 2. Let C be Completion Record { [[Type]]: return, [[Value]]: value, [[Target]]: empty }.
        let c = value;

        // 3. Return ? GeneratorResumeAbrupt(g, C, empty).
        // ### 27.5.3.4 GeneratorResumeAbrupt
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(g) = g else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
                gc.into_nogc(),
            ));
        };

        g.unbind().resume_return(agent, c.unbind(), gc)
    }

    fn throw<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // GeneratorResumeAbrupt: 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
                gc.into_nogc(),
            ));
        };

        // 1. Let g be the this value.
        // 2. Let C be ThrowCompletion(exception).
        // 3. Return ? GeneratorResumeAbrupt(g, C, empty).
        generator.resume_throw(agent, arguments.get(0), gc)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let iterator_prototype = intrinsics.iterator_prototype();
        let generator_function_prototype = intrinsics.generator_function_prototype();
        let this = intrinsics.generator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(iterator_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.constructor.into())
                    .with_value_readonly(generator_function_prototype.into())
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
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Generator.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
