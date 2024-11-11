// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        fundamental_objects::function_objects::function_constructor::{
            create_dynamic_function, DynamicFunctionKind,
        },
        types::{Function, IntoObject, IntoValue, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct AsyncFunctionConstructor;
impl Builtin for AsyncFunctionConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.AsyncFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(AsyncFunctionConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for AsyncFunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::AsyncFunction;
}

impl AsyncFunctionConstructor {
    fn behaviour(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,

        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        // 2. If bodyArg is not present, set bodyArg to the empty String.
        let (parameter_args, body_arg) = if arguments.is_empty() {
            (&[] as &[Value], String::EMPTY_STRING.into_value())
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
        Ok(create_dynamic_function(
            agent,
            gc,
            constructor,
            DynamicFunctionKind::Async,
            parameter_args,
            body_arg,
        )?
        .into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let async_function_prototype = intrinsics.async_function_prototype();
        let function_constructor = intrinsics.function();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AsyncFunctionConstructor>(agent, realm)
            .with_prototype(function_constructor.into_object())
            .with_property_capacity(1)
            .with_prototype_property(async_function_prototype.into_object())
            .build();
    }
}
