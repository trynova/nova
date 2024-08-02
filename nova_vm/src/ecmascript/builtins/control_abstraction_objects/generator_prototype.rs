// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::create_iter_result_object,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{
            agent::{ExceptionType, JsError},
            Agent, JsResult, RealmIdentifier,
        },
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::Vm,
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

use super::generator_objects::GeneratorState;

pub(crate) struct GeneratorPrototype;

pub(crate) struct GeneratorPrototypeNext;
impl Builtin for GeneratorPrototypeNext {
    const NAME: String = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(GeneratorPrototype::next);
}
impl BuiltinIntrinsic for GeneratorPrototypeNext {
    const INDEX: IntrinsicFunctionIndexes =
        IntrinsicFunctionIndexes::GeneratorFunctionPrototypePrototypeNext;
}
pub(crate) struct GeneratorPrototypeReturn;
impl Builtin for GeneratorPrototypeReturn {
    const NAME: String = BUILTIN_STRING_MEMORY.r#return;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(GeneratorPrototype::r#return);
}
pub(crate) struct GeneratorPrototypeThrow;
impl Builtin for GeneratorPrototypeThrow {
    const NAME: String = BUILTIN_STRING_MEMORY.throw;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(GeneratorPrototype::throw);
}

impl GeneratorPrototype {
    fn next(agent: &mut Agent, this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        // [27.5.3.3 GeneratorResume ( generator, value, generatorBrand )](https://tc39.es/ecma262/#sec-generatorresume)
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
            ));
        };

        match agent[generator].generator_state.as_ref().unwrap() {
            GeneratorState::SuspendedStart { .. } => {
                // 3. Assert: state is either suspended-start or suspended-yield.
                // 4. Let genContext be generator.[[GeneratorContext]].
                // 5. Let methodContext be the running execution context.
                // 6. Suspend methodContext.
                // 7. Set generator.[[GeneratorState]] to executing.
                let Some(GeneratorState::SuspendedStart {
                    executable,
                    execution_context,
                }) = agent[generator]
                    .generator_state
                    .replace(GeneratorState::Executing)
                else {
                    unreachable!()
                };
                // 8. Push genContext onto the execution context stack; genContext is now the
                // running execution context.
                agent.execution_context_stack.push(execution_context);
                // 9. Resume the suspended evaluation of genContext using NormalCompletion(value) as
                // the result of the operation that suspended it. Let result be the value returned
                // by the resumed computation.
                // 10. Assert: When we return here, genContext has already been removed from the
                // execution context stack and methodContext is the currently running execution
                // context.
                // 11. Return ? result.
                let result = Vm::execute(agent, &executable).into_js_result();

                // GeneratorStart
                // 4. f. Remove acGenContext from the execution context stack and restore the
                // execution context that is at the top of the execution context stack as the
                // running execution context.
                agent.execution_context_stack.pop().unwrap();
                // g. Set acGenerator.[[GeneratorState]] to completed.
                // h. NOTE: Once a generator enters the completed state it never leaves it and its
                // associated execution context is never resumed. Any execution state associated
                // with acGenerator can be discarded at this point.
                agent[generator].generator_state = Some(GeneratorState::Completed);

                // i. If result is a normal completion, then
                //    i. Let resultValue be undefined.
                // j. Else if result is a return completion, then
                //    i. Let resultValue be result.[[Value]].
                // k. Else,
                //    i. Assert: result is a throw completion.
                //    ii. Return ? result.
                // l. Return CreateIterResultObject(resultValue, true).
                result.map(|result_value| {
                    create_iter_result_object(agent, result_value, true).into_value()
                })
            }
            GeneratorState::Executing => Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "The generator is currently running",
            )),
            GeneratorState::Completed => {
                // 2. If state is completed, return CreateIterResultObject(undefined, true).
                Ok(create_iter_result_object(agent, Value::Undefined, true).into_value())
            }
        }
    }

    fn r#return(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // [27.5.3.4 GeneratorResumeAbrupt ( generator, abruptCompletion, generatorBrand )](https://tc39.es/ecma262/#sec-generatorresumeabrupt)
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
            ));
        };

        match agent[generator].generator_state.as_ref().unwrap() {
            // 2. If state is suspended-start, then
            GeneratorState::SuspendedStart { .. } => {
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
                ))
            }
            GeneratorState::Completed => {}
        };

        // NOTE: If we reach here, state is completed.
        // 3. If state is completed, then
        //    a. If abruptCompletion is a return completion, then
        //       i. Return CreateIterResultObject(abruptCompletion.[[Value]], true).
        Ok(create_iter_result_object(agent, arguments.get(0), true).into_value())
    }

    fn throw(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // [27.5.3.4 GeneratorResumeAbrupt ( generator, abruptCompletion, generatorBrand )](https://tc39.es/ecma262/#sec-generatorresumeabrupt)
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::Generator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Generator expected",
            ));
        };

        match agent[generator].generator_state.as_ref().unwrap() {
            // 2. If state is suspended-start, then
            GeneratorState::SuspendedStart { .. } => {
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
                ))
            }
            // 3. If state is completed, then
            GeneratorState::Completed => {}
        };

        // NOTE: If we reach here, state is completed.
        // 3. If state is completed, then
        //    b. Return ? abruptCompletion.
        Err(JsError::new(arguments.get(0)))
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
