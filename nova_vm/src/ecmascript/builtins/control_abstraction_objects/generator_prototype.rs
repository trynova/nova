// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::create_iter_result_object,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

use super::generator_objects::GeneratorState;

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

impl<'gc> GeneratorPrototype {
    fn next(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // GeneratorResume: 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
                gc.nogc(),
            ));
        };

        // 1. Return ? GeneratorResume(this value, value, empty).
        Ok(generator.resume(agent, arguments.get(0), gc)?.into_value())
    }

    fn r#return(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let g be the this value.
        // 2. Let C be Completion Record { [[Type]]: return, [[Value]]: value, [[Target]]: empty }.
        // 3. Return ? GeneratorResumeAbrupt(g, C, empty).

        // [27.5.3.4 GeneratorResumeAbrupt ( generator, abruptCompletion, generatorBrand )](https://tc39.es/ecma262/#sec-generatorresumeabrupt)
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
                gc,
            ));
        };
        let generator = generator.bind(gc);

        match agent[generator].generator_state.as_ref().unwrap() {
            // 2. If state is suspended-start, then
            GeneratorState::Suspended { .. } => {
                // NOTE: Since we don't support finally blocks, the behavior in the suspended-yield
                // state is identical to suspended-start. In suspended-yield, the generator would be
                // resumed, but instead of yielding a value, the behavior would be the same as a
                // `return` keyword. Without finally support, this would immediately exit the
                // function execution without running any instructions, and GeneratorStart steps
                // 4.e-l would set the state to completed and return the iter result object.

                // a. Set generator.[[GeneratorState]] to completed.
                // b. NOTE: Once a generator enters the completed state it never leaves it and its
                // associated execution context is never resumed. Any execution state associated
                // with generator can be discarded at this point.
                agent[generator].generator_state = Some(GeneratorState::Completed);

                // c. Set state to completed.
            }
            GeneratorState::Executing => {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "The generator is currently running",
                    gc,
                ))
            }
            GeneratorState::Completed => {}
        };

        // NOTE: If we reach here, state is completed.
        // 3. If state is completed, then
        //    a. If abruptCompletion is a return completion, then
        //       i. Return CreateIterResultObject(abruptCompletion.[[Value]], true).
        Ok(create_iter_result_object(agent, arguments.get(0), true, gc).into_value())
    }

    fn throw(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // GeneratorResumeAbrupt: 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
                gc.nogc(),
            ));
        };

        // 1. Let g be the this value.
        // 2. Let C be ThrowCompletion(exception).
        // 3. Return ? GeneratorResumeAbrupt(g, C, empty).
        Ok(generator
            .resume_throw(agent, arguments.get(0), gc)?
            .into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
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
