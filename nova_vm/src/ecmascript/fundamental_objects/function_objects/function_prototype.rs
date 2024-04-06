use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, ordinary_has_instance},
            testing_and_comparison::is_callable,
        },
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Builtin},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{Function, IntoValue, Value},
    },
    heap::{GetHeapData, WellKnownSymbolIndexes},
};

pub(crate) struct FunctionPrototype;

struct FunctionPrototypeApply;
impl Builtin for FunctionPrototypeApply {
    const NAME: &'static str = "apply";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::apply);
}

struct FunctionPrototypeBind;
impl Builtin for FunctionPrototypeBind {
    const NAME: &'static str = "bind";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::bind);
}

struct FunctionPrototypeCall;
impl Builtin for FunctionPrototypeCall {
    const NAME: &'static str = "call";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::call);
}

struct FunctionPrototypeToString;
impl Builtin for FunctionPrototypeToString {
    const NAME: &'static str = "toString";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::to_string);
}

struct FunctionPrototypeHasInstance;
impl Builtin for FunctionPrototypeHasInstance {
    const NAME: &'static str = "[Symbol.hasInstance]";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(FunctionPrototype::has_instance);
}

impl FunctionPrototype {
    fn apply(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        if !is_callable(this_value) {
            return Err(agent.throw_exception(ExceptionType::TypeError, "Not a callable value"));
        };
        let func = Function::try_from(this_value).unwrap();
        let this_arg = args.get(0);
        let arg_array = args.get(1);
        if arg_array.is_undefined() || arg_array.is_null() {
            // TODO: PrepareForTailCall
            return call_function(agent, func, this_arg, None);
        }
        // TODO: let arg_list = create_list_from_array_like(arg_array);
        let elements = match arg_array {
            Value::Array(idx) => {
                agent
                    .heap
                    .arrays
                    .get(idx.into_index())
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .elements
            }
            _ => {
                return Err(
                    agent.throw_exception(ExceptionType::TypeError, "Not a valid arguments array")
                );
            }
        };
        let elements = agent.heap.elements.get(elements.into());
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
        if !is_callable(this_value) {
            return Err(agent.throw_exception(ExceptionType::TypeError, "Not a callable value"));
        };
        let func = Function::try_from(this_value).unwrap();
        // TODO: PrepareForTailCall
        let this_arg = args.get(0);
        let args = ArgumentsList(&args[1..]);
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
                let data = &agent.heap.get(idx).ecmascript_function;
                let _span = data.source_text;
                let _source = data.script_or_module;
                todo!();
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
                let data = agent.heap.get(idx);
                let initial_name = data.initial_name.map_or_else(
                    || "function () {{ [ native code ] }}".into(),
                    |initial_name| match initial_name {
                        crate::ecmascript::types::String::String(idx) => format!(
                            "function {}() {{ [ native code ] }}",
                            agent.heap.get(idx).as_str().unwrap()
                        ),
                        crate::ecmascript::types::String::SmallString(string) => {
                            format!("function {}() {{ [ native code ] }}", string.as_str())
                        }
                    },
                );
                Ok(Value::from_str(&mut agent.heap, &initial_name))
            }
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
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.function_prototype();
        let function_constructor = intrinsics.function();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(6)
            .with_builtin_function_property::<FunctionPrototypeApply>()
            .with_builtin_function_property::<FunctionPrototypeBind>()
            .with_builtin_function_property::<FunctionPrototypeCall>()
            .with_constructor_property(function_constructor)
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
