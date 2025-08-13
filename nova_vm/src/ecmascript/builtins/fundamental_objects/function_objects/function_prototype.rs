// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::ControlFlow;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_list_from_array_like, has_own_property,
                ordinary_has_instance, try_get, try_has_own_property,
            },
            testing_and_comparison::is_callable,
            type_conversion::to_integer_or_infinity_number,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinFunction, BuiltinIntrinsic,
            BuiltinIntrinsicConstructor, SetFunctionNamePrefix,
            bound_function::bound_function_create, set_function_name,
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, InternalSlots, IntoFunction, IntoObject, IntoValue,
            Number, OrdinaryObject, PropertyKey, String, TryBreak, TryGetContinue, Value,
            handle_try_get_result,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::{
        IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, ObjectEntry,
        ObjectEntryPropertyDescriptor, WellKnownSymbolIndexes,
    },
};

pub(crate) struct FunctionPrototype;
impl Builtin for FunctionPrototype {
    const NAME: String<'static> = String::EMPTY_STRING;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(Self::behaviour);
}
impl BuiltinIntrinsicConstructor for FunctionPrototype {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::FunctionPrototype;
}

struct FunctionPrototypeApply;
impl Builtin for FunctionPrototypeApply {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.apply;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(FunctionPrototype::apply);
}

struct FunctionPrototypeBind;
impl Builtin for FunctionPrototypeBind {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.bind;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(FunctionPrototype::bind);
}

struct FunctionPrototypeCall;
impl Builtin for FunctionPrototypeCall {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.call;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(FunctionPrototype::call);
}

struct FunctionPrototypeToString;
impl Builtin for FunctionPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(FunctionPrototype::to_string);
}

struct FunctionPrototypeHasInstance;
impl Builtin for FunctionPrototypeHasInstance {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_hasInstance_;

    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::HasInstance.to_property_key());

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(FunctionPrototype::has_instance);

    const WRITABLE: bool = false;
    const CONFIGURABLE: bool = false;
}

impl FunctionPrototype {
    fn behaviour(
        _: &mut Agent,
        _: Value,
        _: ArgumentsList,
        _: GcScope,
    ) -> JsResult<'static, Value<'static>> {
        Ok(Value::Undefined)
    }

    /// ### [20.2.3.1 Function.prototype.apply ( thisArg, argArray )](https://tc39.es/ecma262/#sec-function.prototype.apply)
    fn apply<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let this_arg = args.get(0).bind(gc.nogc());
        let arg_array = args.get(1).bind(gc.nogc());
        // 1. Let func be the this value.
        let Some(func) = is_callable(this_value, gc.nogc()) else {
            // 2. If IsCallable(func) is false, throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a callable value",
                gc.into_nogc(),
            ));
        };
        if arg_array.is_undefined() || arg_array.is_null() {
            // 3. If argArray is either undefined or null, then
            //   a. TODO: Perform PrepareForTailCall().
            //   b. Return ? Call(func, thisArg).
            return call_function(agent, func.unbind(), this_arg.unbind(), None, gc);
        }
        let func = func.scope(agent, gc.nogc());
        let this_arg = this_arg.scope(agent, gc.nogc());
        // 4. Let argList be ? CreateListFromArrayLike(argArray).
        let args_list = create_list_from_array_like(agent, arg_array.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 5. TODO: Perform PrepareForTailCall().
        // 6.Return ? Call(func, thisArg, argList).
        call_function(
            agent,
            func.get(agent),
            this_arg.get(agent),
            Some(ArgumentsList::from_mut_slice(&mut args_list.unbind())),
            gc,
        )
    }

    /// ### [20.2.3.2 Function.prototype.bind ( thisArg, ...args )](https://tc39.es/ecma262/#sec-function.prototype.bind)
    ///
    /// > #### Note 1
    /// >
    /// > Function objects created using **`Function.prototype.bind`** are
    /// > exotic objects. They also do not have a **"prototype"** property.
    ///
    /// > #### Note 2
    /// >
    /// > If `Target` is either an arrow function or a bound function exotic
    /// > object, then the `thisArg` passed to this method will not be used by
    /// > subsequent calls to `F`.
    fn bind<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let this_arg = args.get(0).bind(gc.nogc());
        let args = if args.len() > 1 { &args[1..] } else { &[] };
        let args_len = args.len();
        // 1. Let Target be the this value.
        let target = this_value;
        // 2. If IsCallable(Target) is false, throw a TypeError exception.
        let Some(mut target) = is_callable(target, gc.nogc()) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot bind a non-callable object",
                gc.into_nogc(),
            ));
        };
        let scoped_target = target.scope(agent, gc.nogc());
        // 3. Let F be ? BoundFunctionCreate(Target, thisArg, args).
        let mut f = bound_function_create(
            agent,
            target.unbind(),
            this_arg.unbind(),
            args,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        target = scoped_target.get(agent);
        let mut scoped_f = None;
        // 4. Let L be 0.
        let mut l = 0;
        // 5. Let targetHasLength be ? HasOwnProperty(Target, "length").
        let target_has_length = if let TryResult::Continue(result) = try_has_own_property(
            agent,
            scoped_target.get(agent).into_object(),
            BUILTIN_STRING_MEMORY.length.into(),
            gc.nogc(),
        ) {
            result
        } else {
            scoped_f = Some(f.scope(agent, gc.nogc()));
            let result = has_own_property(
                agent,
                target.into_object(),
                BUILTIN_STRING_MEMORY.length.into(),
                gc.reborrow(),
            )
            .unbind()?;
            f = scoped_f.as_ref().unwrap().get(agent).bind(gc.nogc());
            target = scoped_target.get(agent);
            result
        };
        // 6. If targetHasLength is true, then
        if target_has_length {
            // a. Let targetLen be ? Get(Target, "length").
            let target_len = try_get(
                agent,
                target,
                BUILTIN_STRING_MEMORY.length.to_property_key(),
                None,
                gc.nogc(),
            );
            let target_len = match target_len {
                ControlFlow::Continue(TryGetContinue::Unset) => Value::Undefined,
                ControlFlow::Continue(TryGetContinue::Value(v)) => v,
                ControlFlow::Break(TryBreak::Error(e)) => {
                    return Err(e.unbind().bind(gc.into_nogc()));
                }
                _ => {
                    if scoped_f.is_none() {
                        scoped_f = Some(f.scope(agent, gc.nogc()));
                    }
                    let result = handle_try_get_result(
                        agent,
                        target.unbind(),
                        BUILTIN_STRING_MEMORY.length.to_property_key(),
                        target_len.unbind(),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    f = scoped_f.as_ref().unwrap().get(agent).bind(gc.nogc());
                    target = scoped_target.get(agent);
                    result
                }
            };

            // b. If targetLen is a Number, then
            if let Ok(target_len) = Number::try_from(target_len) {
                match target_len {
                    Number::Integer(target_len) => {
                        // 3. Let argCount be the number of elements in args.
                        let arg_count = args_len;
                        // 4. Set L to max(targetLenAsInt - argCount, 0).
                        l = 0.max(target_len.into_i64() - arg_count as i64) as usize;
                    }
                    _ => {
                        // i. If targetLen is +∞𝔽, then
                        if target_len.is_pos_infinity(agent) {
                            // 1. Set L to +∞.
                            l = usize::MAX;
                        } else if target_len.is_neg_infinity(agent) {
                            // ii. Else if targetLen is -∞𝔽, then
                            // 1. Set L to 0.
                            l = 0;
                        } else {
                            // iii. Else,
                            // 1. Let targetLenAsInt be ! ToIntegerOrInfinity(targetLen).
                            let target_len_as_int =
                                to_integer_or_infinity_number(agent, target_len).into_i64();
                            // 2. Assert: targetLenAsInt is finite.
                            // 3. Let argCount be the number of elements in args.
                            let arg_count = args_len;
                            // 4. Set L to max(targetLenAsInt - argCount, 0).
                            l = 0.max(target_len_as_int - arg_count as i64) as usize;
                        }
                    }
                }
            }
        }
        // 7. Perform SetFunctionLength(F, L).
        agent[f].length = u8::try_from(l).unwrap_or(u8::MAX);
        // 8. Let targetName be ? Get(Target, "name").
        let target_name = try_get(
            agent,
            target,
            BUILTIN_STRING_MEMORY.name.to_property_key(),
            None,
            gc.nogc(),
        );
        let target_name = match target_name {
            ControlFlow::Continue(TryGetContinue::Unset) => Value::Undefined,
            ControlFlow::Continue(TryGetContinue::Value(v)) => v,
            ControlFlow::Break(TryBreak::Error(e)) => return Err(e.unbind().bind(gc.into_nogc())),
            _ => {
                if scoped_f.is_none() {
                    scoped_f = Some(f.scope(agent, gc.nogc()));
                }
                let result = handle_try_get_result(
                    agent,
                    target.unbind(),
                    BUILTIN_STRING_MEMORY.length.to_property_key(),
                    target_name.unbind(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                f = scoped_f.as_ref().unwrap().get(agent).bind(gc.nogc());
                result
            }
        };
        // 9. If targetName is not a String, set targetName to the empty String.
        let target_name = String::try_from(target_name).unwrap_or(String::EMPTY_STRING);
        // 10. Perform SetFunctionName(F, targetName, "bound").
        set_function_name(
            agent,
            f,
            target_name.into(),
            Some(SetFunctionNamePrefix::Bound),
            gc.nogc(),
        );
        // 11. Return F.

        Ok(f.into_value().unbind())
    }

    fn call<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let this_arg = args.get(0).bind(nogc);
        let Some(func) = is_callable(this_value, nogc) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a callable value",
                gc.into_nogc(),
            ));
        };
        // TODO: PrepareForTailCall
        let args = if !args.is_empty() {
            args.slice_from(1)
        } else {
            args
        };
        call_function(agent, func.unbind(), this_arg.unbind(), Some(args), gc)
    }

    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        // Let func be the this value.
        let Ok(func) = Function::try_from(this_value) else {
            // 5. Throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a callable value",
                gc.into_nogc(),
            ));
        };

        match func {
            // 2. If func is an Object, func has a [[SourceText]] internal slot,
            // func.[[SourceText]] is a sequence of Unicode code points, and
            // HostHasSourceTextAvailable(func) is true, then
            Function::ECMAScriptFunction(idx) => {
                // a. Return CodePointsToString(func.[[SourceText]]).
                let data = &agent[idx].ecmascript_function;
                let span = data.source_text;
                let source_text = data.source_code.get_source_text(agent)
                    [(span.start as usize)..(span.end as usize)]
                    .to_string();
                Ok(Value::from_string(agent, source_text, gc.nogc()).unbind())
            }
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
                            format!(
                                "function {}() {{ [ native code ] }}",
                                agent[idx].to_string_lossy()
                            )
                        }
                        crate::ecmascript::types::String::SmallString(string) => {
                            format!(
                                "function {}() {{ [ native code ] }}",
                                string.to_string_lossy()
                            )
                        }
                    },
                );
                Ok(Value::from_string(agent, initial_name, gc.nogc()).unbind())
            }
            Function::BuiltinConstructorFunction(_) => {
                Ok(Value::from_static_str(agent, "class { [ native code ] }", gc.nogc()).unbind())
            }
            // 4. If func is an Object and IsCallable(func) is true, return an
            // implementation-defined String source code representation of func.
            // The representation must have the syntax of a NativeFunction.
            Function::BoundFunction(_) | Function::BuiltinPromiseResolvingFunction(_) => {
                // Promise resolving functions have no initial name.
                Ok(
                    Value::from_static_str(agent, "function () { [ native code ] }", gc.nogc())
                        .unbind(),
                )
            }
            Function::BuiltinGeneratorFunction
            | Function::BuiltinPromiseCollectorFunction
            | Function::BuiltinProxyRevokerFunction => unreachable!(),
        }

        // NOTE: NativeFunction means the following string:
        // `function <?:"get"/"set"> <?:name> (<?:parameters>) { [ native code ] }``
        // <?:...> is an optional template part.
    }

    fn has_instance<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let v = args.get(0);
        let f = this_value;
        ordinary_has_instance(agent, f, v, gc).map(|result| result.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        ThrowTypeError::create_intrinsic(agent, realm);

        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
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
            .with_builtin_function_property::<FunctionPrototypeHasInstance>()
            .build();
    }
}

struct ThrowTypeError;
impl Builtin for ThrowTypeError {
    const NAME: String<'static> = String::EMPTY_STRING;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(Self::behaviour);
}
impl BuiltinIntrinsic for ThrowTypeError {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ThrowTypeError;
}

impl ThrowTypeError {
    fn behaviour<'gc>(
        agent: &mut Agent,
        _: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.throw_exception_with_static_message(ExceptionType::TypeError, "'caller', 'callee', and 'arguments' properties may not be accessed on strict mode functions or the arguments objects for calls to them", gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let throw_type_error =
            BuiltinFunctionBuilder::new_intrinsic_function::<ThrowTypeError>(agent, realm).build();
        let backing_object = create_throw_type_error_backing_object(agent, realm);
        agent[throw_type_error].object_index = Some(backing_object);
    }
}

fn create_throw_type_error_backing_object(
    agent: &mut Agent,
    realm: Realm,
) -> OrdinaryObject<'static> {
    let prototype = agent
        .get_realm_record_by_id(realm)
        .intrinsics()
        .get_intrinsic_default_proto(BuiltinFunction::DEFAULT_PROTOTYPE);

    let length_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
        // The "length" property of this function has the attributes { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }.
        value: ObjectEntryPropertyDescriptor::Data {
            value: ThrowTypeError::LENGTH.into(),
            writable: false,
            enumerable: false,
            configurable: false,
        },
    };
    let name_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
        // The "name" property of this function has the attributes { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }.
        value: ObjectEntryPropertyDescriptor::Data {
            value: ThrowTypeError::NAME.into_value(),
            writable: false,
            enumerable: false,
            configurable: false,
        },
    };
    let object = OrdinaryObject::create_object(agent, Some(prototype), &[length_entry, name_entry]);
    // The value of the [[Extensible]] internal slot of this function is false.
    object.internal_set_extensible(agent, false);
    object
}
