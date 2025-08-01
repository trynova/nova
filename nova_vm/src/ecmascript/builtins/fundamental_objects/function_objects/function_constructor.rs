// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_span::SourceType;
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_string, to_string_primitive},
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor, ECMAScriptFunction,
            OrdinaryFunctionCreateParams, make_constructor,
            ordinary::get_prototype_from_constructor, ordinary_function_create, set_function_name,
        },
        execution::{Agent, Environment, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        scripts_and_modules::source_code::{SourceCode, SourceCodeHeapData},
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, Primitive, String,
            Value,
        },
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

        Ok(f.into_value().unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let function_prototype = intrinsics.function_prototype().into_object();

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
        .ensure_can_compile_strings(agent.current_realm_record_mut(), gc.nogc())
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
            + body_string.len(agent)
            + if !parameter_strings_slice.is_empty() {
                // Separated by a single comma character
                parameter_strings_slice
                    .iter()
                    .map(|str| str.len(agent) + 1)
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
            string.push_wtf8(parameter.as_wtf8(agent));
        }
        string.push_str("\n) {\n");
        string.push_wtf8(body_string.as_wtf8(agent));
        string.push_str("\n}");

        debug_assert_eq!(string.len(), str_len);

        String::from_wtf8_buf(agent, string, gc.nogc())
    };

    // The spec says to parse the parameters and the function body separately to
    // avoid code injection, but oxc doesn't have a public API to do that.
    // Instead, we parse the source string as a script, and throw unless it has
    // exactly one statement which is a function declaration of the right kind.
    let (function, source_code) = {
        let mut function = None;
        let mut source_code = None;

        let source_type = SourceType::default().with_script(true);
        // SAFETY: The safety requirements are that the SourceCode cannot be
        // GC'd before the program is dropped. If this function returns
        // successfully, then the program's AST and the SourceCode will both be
        // kept alive in the returned function object.
        let parsed_result =
            unsafe { SourceCode::parse_source(agent, source_string, source_type, gc.nogc()) };

        if let Ok((program, sc)) = parsed_result {
            source_code = Some(sc);
            if program.hashbang.is_none()
                && program.directives.is_empty()
                && program.body.len() == 1
                && let oxc_ast::ast::Statement::FunctionDeclaration(funct) = &program.body[0]
                && kind.function_matches_kind(funct)
            {
                // SAFETY: the Function is inside a oxc_allocator::Box, which will remain
                // alive as long as `source_code` is kept alive. Similarly, the inner
                // lifetime of Function is also kept alive by `source_code`.`
                function = Some(unsafe {
                    core::mem::transmute::<&oxc_ast::ast::Function, &'static oxc_ast::ast::Function>(
                        funct,
                    )
                });
            }
        }

        if let Some(function) = function {
            (function, source_code.unwrap())
        } else {
            if source_code.is_some() {
                // In this branch, since we're not returning the function, we
                // know `source_code` won't be reachable from any heap object,
                // so we pop it off the heap to help GC along.
                agent.heap.alloc_counter = agent
                    .heap
                    .alloc_counter
                    .saturating_sub(core::mem::size_of::<Option<SourceCodeHeapData<'static>>>());
                agent.heap.source_codes.pop();
                debug_assert_eq!(
                    source_code.unwrap().get_index(),
                    agent.heap.source_codes.len()
                );
            }
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::SyntaxError,
                "Invalid function source text.",
                gc.into_nogc(),
            ));
        }
    };

    let source_code = source_code.scope(agent, gc.nogc());
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
        parameters_list: &function.params,
        body: function.body.as_ref().unwrap(),
        is_concise_arrow_function: false,
        is_async: function.r#async,
        is_generator: function.generator,
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
