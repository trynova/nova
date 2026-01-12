// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast;
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_string, to_string_primitive},
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor, ECMAScriptFunction,
            FunctionAstRef, OrdinaryFunctionCreateParams, ecmascript_function::make_constructor,
            ecmascript_function::ordinary_function_create, ecmascript_function::set_function_name,
            ordinary::get_prototype_from_constructor,
        },
        execution::{Agent, Environment, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        scripts_and_modules::source_code::{ParseResult, SourceCode, SourceCodeType},
        types::{BUILTIN_STRING_MEMORY, Function, Object, Primitive, String, Value},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct FunctionConstructor;

impl Builtin for FunctionConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Function;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for FunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Function;
}

impl FunctionConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
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

        // 3. Return ? CreateDynamicFunction(C, NewTarget, normal, parameterArgs, bodyArg).
        let f = create_dynamic_function(
            agent,
            constructor,
            DynamicFunctionKind::Normal,
            parameter_args,
            body_arg,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 20.2.1.1.1 CreateDynamicFunction ( constructor, newTarget, kind, parameterArgs, bodyArg )
        // 32. Else if kind is normal, then
        //   a. Perform MakeConstructor(F).
        make_constructor(agent, f.unbind(), None, None, gc.nogc());

        Ok(f.unbind().into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let function_prototype = intrinsics.function_prototype().into();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<FunctionConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(function_prototype)
            .with_prototype_property(function_prototype)
            .build();
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DynamicFunctionKind {
    Normal,
    Generator,
    Async,
    AsyncGenerator,
}
impl DynamicFunctionKind {
    fn prefix(&self) -> &'static str {
        match self {
            DynamicFunctionKind::Normal => "function",
            DynamicFunctionKind::Generator => "function*",
            DynamicFunctionKind::Async => "async function",
            DynamicFunctionKind::AsyncGenerator => "async function*",
        }
    }
    fn function_matches_kind(&self, function: &oxc_ast::ast::Function) -> bool {
        let (is_async, is_generator) = match self {
            DynamicFunctionKind::Normal => (false, false),
            DynamicFunctionKind::Generator => (false, true),
            DynamicFunctionKind::Async => (true, false),
            DynamicFunctionKind::AsyncGenerator => (true, true),
        };
        function.r#async == is_async && function.generator == is_generator
    }
    fn intrinsic_prototype(&self) -> ProtoIntrinsics {
        match self {
            DynamicFunctionKind::Normal => ProtoIntrinsics::Function,
            DynamicFunctionKind::Generator => ProtoIntrinsics::GeneratorFunction,
            DynamicFunctionKind::Async => ProtoIntrinsics::AsyncFunction,
            DynamicFunctionKind::AsyncGenerator => ProtoIntrinsics::AsyncGeneratorFunction,
        }
    }
}

/// ### [20.2.1.1.1 CreateDynamicFunction ( constructor, newTarget, kind, parameterArgs, bodyArg )](https://tc39.es/ecma262/#sec-createdynamicfunction)
///
/// NOTE: This implementation doesn't cover steps 30-32, those should be handled by the caller.
pub(crate) fn create_dynamic_function<'a>(
    agent: &mut Agent,
    constructor: Function,
    kind: DynamicFunctionKind,
    parameter_args: &[Value],
    body_arg: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ECMAScriptFunction<'a>> {
    let mut constructor = constructor.bind(gc.nogc());
    // 11. Perform ? HostEnsureCanCompileStrings(currentRealm, parameterStrings, bodyString, false).
    agent
        .host_hooks
        .ensure_can_compile_strings(agent.current_realm(gc.nogc()), gc.nogc())
        .unbind()?;

    let source_string = {
        let parameter_strings_vec;
        let parameter_strings_slice;
        let body_string;
        if body_arg.is_string() && parameter_args.iter().all(|arg| arg.is_string()) {
            body_string = String::try_from(body_arg).unwrap().bind(gc.nogc());
            parameter_strings_slice =
                // Safety: All the strings were checked to be strings.
                unsafe { core::mem::transmute::<&[Value], &[String<'_>]>(parameter_args) };
        } else if body_arg.is_primitive() && parameter_args.iter().all(|arg| arg.is_primitive()) {
            // We don't need to call JavaScript here. Nice.
            let gc = gc.nogc();
            let mut parameter_strings = Vec::with_capacity(parameter_args.len());
            for param in parameter_args {
                parameter_strings.push(
                    to_string_primitive(agent, Primitive::try_from(*param).unwrap(), gc)
                        .unbind()?
                        .bind(gc),
                );
            }
            parameter_strings_vec = parameter_strings;
            parameter_strings_slice = &parameter_strings_vec;
            body_string = to_string_primitive(agent, Primitive::try_from(body_arg).unwrap(), gc)
                .unbind()?
                .bind(gc);
        } else {
            // Some of the parameters are non-primitives. This means we'll be
            // calling into JavaScript during this work.
            let scoped_constructor = constructor.scope(agent, gc.nogc());
            let mut parameter_string_roots = Vec::with_capacity(parameter_args.len());
            for param in parameter_args {
                // Each parameter has to be rooted in case the next parameter
                // or the body argument is the one that calls to JavaScript.
                parameter_string_roots.push(
                    to_string(agent, *param, gc.reborrow())
                        .unbind()?
                        .scope(agent, gc.nogc()),
                );
            }
            let body_string_unbound = body_arg.to_string(agent, gc.reborrow()).unbind()?;
            // We've done all our potential JavaScript calling: Now we rest.
            let gc = gc.nogc();
            body_string = body_string_unbound.bind(gc);
            let parameter_strings = parameter_string_roots
                .into_iter()
                .map(|param_root| param_root.get(agent).bind(gc))
                .collect::<Vec<_>>();

            constructor = scoped_constructor.get(agent).bind(gc);
            parameter_strings_vec = parameter_strings;
            parameter_strings_slice = &parameter_strings_vec;
        }

        // format!("{} anonymous({}\n) {{\n{}\n}}", kind.prefix(), parameters, body_arg)
        let str_len = kind.prefix().len()
            + 18
            + body_string.len_(agent)
            + if !parameter_strings_slice.is_empty() {
                // Separated by a single comma character
                parameter_strings_slice
                    .iter()
                    .map(|str| str.len_(agent) + 1)
                    .sum::<usize>()
                    - 1
            } else {
                0
            };
        let mut string = Wtf8Buf::with_capacity(str_len);
        string.push_str(kind.prefix());
        string.push_str(" anonymous(");
        for (i, parameter) in parameter_strings_slice.iter().enumerate() {
            if i != 0 {
                string.push_char(',');
            }
            string.push_wtf8(parameter.as_wtf8_(agent));
        }
        string.push_str("\n) {\n");
        string.push_wtf8(body_string.as_wtf8_(agent));
        string.push_str("\n}");

        debug_assert_eq!(string.len(), str_len);

        String::from_wtf8_buf(agent, string, gc.nogc())
    };

    // The spec says to parse the parameters and the function body separately to
    // avoid code injection, but oxc doesn't have a public API to do that.
    // Instead, we parse the source string as a script, and throw unless it has
    // exactly one statement which is a function declaration of the right kind.

    // SAFETY: The safety requirements are that the SourceCode cannot be
    // GC'd before the program is dropped. If this function returns
    // successfully, then the program's AST and the SourceCode will both be
    // kept alive in the returned function object.
    let parse_result = unsafe {
        SourceCode::parse_source(
            agent,
            source_string,
            SourceCodeType::Script,
            #[cfg(feature = "typescript")]
            false,
            gc.nogc(),
        )
    };

    let ParseResult {
        source_code,
        body,
        directives,
        is_strict,
    } = match parse_result {
        Ok(r) => r,
        Err(err) => {
            let gc = gc.into_nogc();
            let message = String::from_string(agent, err.first().unwrap().message.to_string(), gc);
            return Err(agent.throw_exception_with_message(
                ExceptionType::SyntaxError,
                message,
                gc,
            ));
        }
    };

    let function = if let oxc_ast::ast::Statement::FunctionDeclaration(func) = &body[0]
        // Note: we didn't add a directives so there should be none...
        && directives.is_empty()
        // ... and as a result, is_strict should be false!
        && !is_strict
        && body.len() == 1
        && kind.function_matches_kind(func)
    {
        func.as_ref()
    } else {
        // SAFETY: source text was parsed but didn't match our expectations; no
        // one has seen it yet and thus it can never be executed in this branch.
        unsafe { source_code.manually_drop(agent) };
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::SyntaxError,
            "Invalid function source text.",
            gc.into_nogc(),
        ));
    };

    let source_code = source_code.scope(agent, gc.nogc());
    // SAFETY: SourceCode keeps the result Function's backing allocation from
    // being dropped, and SourceCode is currently rooted. Hence, we can detach
    // the Function from the garbage collector lifetime temporarily.
    let function = unsafe { core::mem::transmute::<&ast::Function, &ast::Function>(function) };
    let params = OrdinaryFunctionCreateParams {
        function_prototype: get_prototype_from_constructor(
            agent,
            constructor.unbind(),
            kind.intrinsic_prototype(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc()),
        // SAFETY: source_code was not shared.
        source_code: Some(unsafe { source_code.take(agent) }),
        source_text: function.span,
        ast: FunctionAstRef::from(function),
        lexical_this: false,
        env: Environment::Global(
            agent
                .current_realm_record()
                .global_env
                .unwrap()
                .unbind()
                .bind(gc.nogc()),
        ),
        private_env: None,
    };
    let f = ordinary_function_create(agent, params, gc.nogc()).unbind();
    let gc = gc.into_nogc();
    let f = f.bind(gc);

    set_function_name(
        agent,
        f,
        BUILTIN_STRING_MEMORY.anonymous.to_property_key(),
        None,
        gc,
    );
    // NOTE: Skipping steps 30-32.

    Ok(f)
}
