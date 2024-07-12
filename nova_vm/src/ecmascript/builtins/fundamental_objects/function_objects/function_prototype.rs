// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, ordinary_has_instance},
            testing_and_comparison::is_callable,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        scripts_and_modules::ScriptOrModule,
        types::{
            Function, IntoFunction, IntoObject, IntoValue, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct FunctionPrototype;
impl Builtin for FunctionPrototype {
    const NAME: String = String::EMPTY_STRING;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(Self::behaviour);
}
impl BuiltinIntrinsicConstructor for FunctionPrototype {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::FunctionPrototype;
}

struct FunctionPrototypeApply;
impl Builtin for FunctionPrototypeApply {
    const NAME: String = BUILTIN_STRING_MEMORY.apply;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::apply);
}

struct FunctionPrototypeBind;
impl Builtin for FunctionPrototypeBind {
    const NAME: String = BUILTIN_STRING_MEMORY.bind;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::bind);
}

struct FunctionPrototypeCall;
impl Builtin for FunctionPrototypeCall {
    const NAME: String = BUILTIN_STRING_MEMORY.call;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::call);
}

struct FunctionPrototypeToString;
impl Builtin for FunctionPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::to_string);
}

struct FunctionPrototypeHasInstance;
impl Builtin for FunctionPrototypeHasInstance {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_hasInstance_;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::has_instance);
}

impl FunctionPrototype {
    fn behaviour(_: &mut Agent, _: Value, _: ArgumentsList) -> JsResult<Value> {
        Ok(Value::Undefined)
    }

    fn apply(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        let Some(func) = is_callable(this_value) else {
            return Err(agent.throw_exception(ExceptionType::TypeError, "Not a callable value"));
        };
        let this_arg = args.get(0);
        let arg_array = args.get(1);
        if arg_array.is_undefined() || arg_array.is_null() {
            // TODO: PrepareForTailCall
            return call_function(agent, func, this_arg, None);
        }
        // TODO: let arg_list = create_list_from_array_like(arg_array);
        let elements = match arg_array {
            Value::Array(array) => array.as_slice(agent),
            _ => {
                return Err(
                    agent.throw_exception(ExceptionType::TypeError, "Not a valid arguments array")
                );
            }
        };
        let args: Vec<Value> = elements
            .iter()
            .map(|value| value.unwrap_or(Value::Undefined))
            .collect();
        let arg_list = ArgumentsList(&args);
        // TODO: PrepareForTailCall
        call_function(agent, func, this_arg, Some(arg_list))
    }

    fn bind(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn call(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        let Some(func) = is_callable(this_value) else {
            return Err(agent.throw_exception(ExceptionType::TypeError, "Not a callable value"));
        };
        // TODO: PrepareForTailCall
        let this_arg = args.get(0);
        let args = ArgumentsList(if args.len() > 0 { &args[1..] } else { &args });
        call_function(agent, func, this_arg, Some(args))
    }

    fn to_string(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // Let func be the this value.
        let Ok(func) = Function::try_from(this_value) else {
            // 5. Throw a TypeError exception.
            return Err(agent.throw_exception(ExceptionType::TypeError, "Not a callable value"));
        };

        match func {
            // 2. If func is an Object, func has a [[SourceText]] internal slot,
            // func.[[SourceText]] is a sequence of Unicode code points, and
            // HostHasSourceTextAvailable(func) is true, then
            Function::ECMAScriptFunction(idx) => {
                // a. Return CodePointsToString(func.[[SourceText]]).
                let data = &agent[idx].ecmascript_function;
                let span = data.source_text;
                let source = data.script_or_module;
                match source {
                    ScriptOrModule::Script(script) => {
                        let source_text = agent[script].source_text
                            [(span.start as usize)..(span.end as usize)]
                            .to_string();
                        Ok(Value::from_string(agent, source_text))
                    }
                    ScriptOrModule::Module(_) => todo!(),
                }
            }
            // 4. If func is an Object and IsCallable(func) is true, return an
            // implementation-defined String source code representation of func.
            // The representation must have the syntax of a NativeFunction.
            Function::BoundFunction(_) => todo!(),
            // 3. If func is a built-in function object, return an
            // implementation-defined String source code representation of func.
            // The representation must have the syntax of a NativeFunction.
            // Additionally, if func has an [[InitialName]] internal slot and
            // func.[[InitialName]] is a String, the portion of the returned
            // String that would be matched by NativeFunctionAccessor_opt
            // PropertyName must be the value of func.[[InitialName]].
            Function::BuiltinFunction(idx) => {
                let data = &agent[idx];
                let initial_name = data.initial_name.map_or_else(
                    || "function () {{ [ native code ] }}".into(),
                    |initial_name| match initial_name {
                        crate::ecmascript::types::String::String(idx) => {
                            format!("function {}() {{ [ native code ] }}", agent[idx].as_str())
                        }
                        crate::ecmascript::types::String::SmallString(string) => {
                            format!("function {}() {{ [ native code ] }}", string.as_str())
                        }
                    },
                );
                Ok(Value::from_string(agent, initial_name))
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolvingFunction(_) => {
                // Promise resolving functions have no initial name.
                Ok(Value::from_static_str(
                    agent,
                    "function () { [ native code ] }",
                ))
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }

        // NOTE: NativeFunction means the following string:
        // `function <?:"get"/"set"> <?:name> (<?:parameters>) { [ native code ] }``
        // <?:...> is an optional template part.
    }

    fn has_instance(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        let v = args.get(0);
        let f = this_value;
        ordinary_has_instance(agent, f, v).map(|result| result.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        ThrowTypeError::create_intrinsic(agent, realm);

        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype().into_object();
        let throw_type_error = intrinsics.throw_type_error().into_function();
        let function_constructor = intrinsics.function();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<FunctionPrototype>(agent, realm)
            .with_property_capacity(8)
            .with_prototype(object_prototype)
            // 10.2.4 AddRestrictedFunctionProperties ( F, realm )
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.caller.into())
                    .with_configurable(true)
                    .with_enumerable(false)
                    .with_getter_and_setter_functions(throw_type_error, throw_type_error)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.arguments.into())
                    .with_configurable(true)
                    .with_enumerable(false)
                    .with_getter_and_setter_functions(throw_type_error, throw_type_error)
                    .build()
            })
            .with_builtin_function_property::<FunctionPrototypeApply>()
            .with_builtin_function_property::<FunctionPrototypeBind>()
            .with_builtin_function_property::<FunctionPrototypeCall>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.constructor.into())
                    .with_enumerable(false)
                    .with_value(function_constructor.into())
                    .build()
            })
            .with_builtin_function_property::<FunctionPrototypeToString>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::HasInstance.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<FunctionPrototypeHasInstance>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .build();
    }
}

struct ThrowTypeError;
impl Builtin for ThrowTypeError {
    const NAME: String = String::EMPTY_STRING;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(Self::behaviour);
}
impl BuiltinIntrinsic for ThrowTypeError {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ThrowTypeError;
}

impl ThrowTypeError {
    fn behaviour(agent: &mut Agent, _: Value, _: ArgumentsList) -> JsResult<Value> {
        Err(agent.throw_exception(ExceptionType::TypeError, "'caller', 'callee', and 'arguments' properties may not be accessed on strict mode functions or the arguments objects for calls to them"))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        BuiltinFunctionBuilder::new_intrinsic_function::<ThrowTypeError>(agent, realm).build();
    }
}
