// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_objects::try_define_property_or_throw;
use crate::engine::context::GcScope;
use crate::engine::unwrap_try;
use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ordinary::ordinary_object_create_with_intrinsics, ArgumentsList, Behaviour, Builtin,
            BuiltinIntrinsicConstructor,
        },
        execution::{Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        fundamental_objects::function_objects::function_constructor::{
            create_dynamic_function, DynamicFunctionKind,
        },
        types::{
            Function, IntoObject, IntoValue, Object, PropertyDescriptor, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct GeneratorFunctionConstructor;
impl Builtin for GeneratorFunctionConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.GeneratorFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for GeneratorFunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::GeneratorFunction;
}

impl<'gc> GeneratorFunctionConstructor {
    fn constructor(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
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

        // 3. Return ? CreateDynamicFunction(C, NewTarget, generator, parameterArgs, bodyArg).
        let f = create_dynamic_function(
            agent,
            constructor,
            DynamicFunctionKind::Generator,
            parameter_args,
            body_arg,
            gc.reborrow(),
        )?
        .unbind();
        let gc = gc.into_nogc();
        let f = f.bind(gc);
        // 20.2.1.1.1 CreateDynamicFunction ( constructor, newTarget, kind, parameterArgs, bodyArg )
        // 30. If kind is generator, then
        //   a. Let prototype be OrdinaryObjectCreate(%GeneratorFunction.prototype.prototype%).
        let prototype = ordinary_object_create_with_intrinsics(
            agent,
            Some(ProtoIntrinsics::Object),
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .generator_prototype()
                    .into_object(),
            ),
            gc,
        );
        //   b. Perform ! DefinePropertyOrThrow(F, "prototype", PropertyDescriptor { [[Value]]: prototype, [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false }).
        unwrap_try(try_define_property_or_throw(
            agent,
            f,
            BUILTIN_STRING_MEMORY.prototype.to_property_key(),
            PropertyDescriptor {
                value: Some(prototype.into_value()),
                writable: Some(true),
                get: None,
                set: None,
                enumerable: Some(false),
                configurable: Some(false),
            },
            gc,
        ))
        .unwrap();

        Ok(f.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let generator_function_prototype = intrinsics.generator_function_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<GeneratorFunctionConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(generator_function_prototype.into_object())
        .build();
    }
}
