// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm},
        fundamental_objects::function_objects::function_constructor::{
            DynamicFunctionKind, create_dynamic_function,
        },
        types::{BUILTIN_STRING_MEMORY, Function, Object, String, Value},
    },
    engine::context::GcScope,
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct AsyncGeneratorFunctionConstructor;
impl Builtin for AsyncGeneratorFunctionConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.AsyncGeneratorFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for AsyncGeneratorFunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::AsyncGeneratorFunction;
}

impl AsyncGeneratorFunctionConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 2. If bodyArg is not present, set bodyArg to the empty String.
        let (parameter_args, body_arg) = if arguments.is_empty() {
            (&[] as &[Value], String::EMPTY_STRING.into())
        } else {
            let (last, others) = arguments.split_last().unwrap();
            (others, *last)
        };
        let constructor = if let Some(new_target) = new_target {
            Function::try_from(new_target).unwrap()
        } else {
            agent.running_execution_context().function.unwrap()
        };
        // 3. Return ? CreateDynamicFunction(C, NewTarget, async, parameterArgs, bodyArg).
        let f = create_dynamic_function(
            agent,
            constructor,
            DynamicFunctionKind::AsyncGenerator,
            parameter_args,
            body_arg,
            gc,
        )?;

        Ok(f.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let function_constructor = intrinsics.function();
        let async_generator_function_prototype = intrinsics.async_generator_function_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AsyncGeneratorFunctionConstructor>(
            agent, realm,
        )
        .with_prototype(function_constructor.into())
        .with_property_capacity(1)
        .with_prototype_property(async_generator_function_prototype.into())
        .build();
    }
}
