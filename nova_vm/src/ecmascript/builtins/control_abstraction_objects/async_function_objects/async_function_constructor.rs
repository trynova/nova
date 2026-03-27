// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, DynamicFunctionKind, Function, JsResult, Object, Realm,
        String, Value, builders::BuiltinFunctionBuilder, create_dynamic_function,
    },
    engine::{Bindable, GcScope},
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct AsyncFunctionConstructor;
impl Builtin for AsyncFunctionConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.AsyncFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for AsyncFunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::AsyncFunction;
}

impl AsyncFunctionConstructor {
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
        Ok(create_dynamic_function(
            agent,
            constructor,
            DynamicFunctionKind::Async,
            parameter_args,
            body_arg.unbind(),
            gc,
        )?
        .into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let async_function_prototype = intrinsics.async_function_prototype();
        let function_constructor = intrinsics.function();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AsyncFunctionConstructor>(agent, realm)
            .with_prototype(function_constructor)
            .with_property_capacity(1)
            .with_prototype_property(async_function_prototype.into())
            .build();
    }
}
