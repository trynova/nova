// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_span::SourceType;

use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{
        make_constructor, ordinary::get_prototype_from_constructor, ordinary_function_create,
        set_function_name, ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
        ECMAScriptFunction, OrdinaryFunctionCreateParams,
    },
    execution::{
        agent::ExceptionType, Agent, EnvironmentIndex, JsResult, ProtoIntrinsics, RealmIdentifier,
    },
    scripts_and_modules::source_code::SourceCode,
    types::{Function, IntoObject, IntoValue, Object, String, Value, BUILTIN_STRING_MEMORY},
};
use crate::heap::IntrinsicConstructorIndexes;

pub(crate) struct FunctionConstructor;

impl Builtin for FunctionConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Function;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}
impl BuiltinIntrinsicConstructor for FunctionConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Function;
}

impl FunctionConstructor {
    fn behaviour(
        agent: &mut Agent,
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

        // 3. Return ? CreateDynamicFunction(C, NewTarget, normal, parameterArgs, bodyArg).
        let f = create_dynamic_function(
            agent,
            constructor,
            DynamicFunctionKind::Normal,
            parameter_args,
            body_arg,
        )?;
        // 20.2.1.1.1 CreateDynamicFunction ( constructor, newTarget, kind, parameterArgs, bodyArg )
        // 32. Else if kind is normal, then
        //   a. Perform MakeConstructor(F).
        make_constructor(agent, f, None, None);

        Ok(f.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let function_prototype = intrinsics.function_prototype().into_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<FunctionConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(function_prototype)
            .with_prototype_property(function_prototype)
            .build();
    }
}

pub(crate) enum DynamicFunctionKind {
    Normal,
    Generator,
    Async,
    // AsyncGenerator
}
impl DynamicFunctionKind {
    fn prefix(&self) -> &'static str {
        match self {
            DynamicFunctionKind::Normal => "function",
            DynamicFunctionKind::Generator => "function*",
            DynamicFunctionKind::Async => "async function",
        }
    }
    fn function_matches_kind(&self, function: &oxc_ast::ast::Function) -> bool {
        let (is_async, is_generator) = match self {
            DynamicFunctionKind::Normal => (false, false),
            DynamicFunctionKind::Generator => (false, true),
            DynamicFunctionKind::Async => (true, false),
        };
        function.r#async == is_async && function.generator == is_generator
    }
    fn intrinsic_prototype(&self) -> ProtoIntrinsics {
        match self {
            DynamicFunctionKind::Normal => ProtoIntrinsics::Function,
            DynamicFunctionKind::Generator => ProtoIntrinsics::GeneratorFunction,
            DynamicFunctionKind::Async => ProtoIntrinsics::AsyncFunction,
        }
    }
}

/// ### [20.2.1.1.1 CreateDynamicFunction ( constructor, newTarget, kind, parameterArgs, bodyArg )](https://tc39.es/ecma262/#sec-createdynamicfunction)
///
/// NOTE: This implementation doesn't cover steps 30-32, those should be handled by the caller.
pub(crate) fn create_dynamic_function(
    agent: &mut Agent,
    constructor: Function,
    kind: DynamicFunctionKind,
    parameter_args: &[Value],
    body_arg: Value,
) -> JsResult<ECMAScriptFunction> {
    // 11. Perform ? HostEnsureCanCompileStrings(currentRealm, parameterStrings, bodyString, false).
    agent
        .host_hooks
        .host_ensure_can_compile_strings(agent.current_realm_mut())?;

    let source_string = {
        // format!("{} anonymous({}\n) {{\n{}\n}}", kind.prefix(), parameters, body_arg)
        let parameter_strings = parameter_args
            .iter()
            .map(|param| param.to_string(agent))
            .collect::<JsResult<Vec<_>>>()?;
        let body_string = body_arg.to_string(agent)?;

        let mut str_len = kind.prefix().len() + body_string.len(agent) + 18;
        if !parameter_strings.is_empty() {
            // Separated by a single comma character
            str_len += parameter_strings
                .iter()
                .map(|str| str.len(agent) + 1)
                .sum::<usize>()
                - 1;
        }

        let mut string = std::string::String::with_capacity(str_len);
        string.push_str(kind.prefix());
        string.push_str(" anonymous(");
        for (i, parameter) in parameter_strings.iter().enumerate() {
            if i != 0 {
                string.push(',');
            }
            string.push_str(parameter.as_str(agent));
        }
        string.push_str("\n) {\n");
        string.push_str(body_string.as_str(agent));
        string.push_str("\n}");
        debug_assert_eq!(string.len(), str_len);

        String::from_string(agent, string)
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
        let parsed_result = unsafe { SourceCode::parse_source(agent, source_string, source_type) };

        if let Ok((program, sc)) = parsed_result {
            source_code = Some(sc);
            if program.hashbang.is_none()
                && program.directives.is_empty()
                && program.body.len() == 1
            {
                if let oxc_ast::ast::Statement::FunctionDeclaration(funct) = &program.body[0] {
                    if kind.function_matches_kind(funct) {
                        // SAFETY: the Function is inside a oxc_allocator::Box, which will remain
                        // alive as long as `source_code` is kept alive. Similarly, the inner
                        // lifetime of Function is also kept alive by `source_code`.`
                        function = Some(unsafe {
                            std::mem::transmute::<
                                &oxc_ast::ast::Function,
                                &'static oxc_ast::ast::Function,
                            >(funct)
                        });
                    }
                }
            }
        }

        if let Some(function) = function {
            (function, source_code.unwrap())
        } else {
            if source_code.is_some() {
                // In this branch, since we're not returning the function, we
                // know `source_code` won't be reachable from any heap object,
                // so we pop it off the heap to help GC along.
                agent.heap.source_codes.pop();
                debug_assert_eq!(
                    source_code.unwrap().get_index(),
                    agent.heap.source_codes.len()
                );
            }
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::SyntaxError,
                "Invalid function source text.",
            ));
        }
    };

    let params = OrdinaryFunctionCreateParams {
        function_prototype: get_prototype_from_constructor(
            agent,
            constructor,
            kind.intrinsic_prototype(),
        )?,
        source_code: Some(source_code),
        source_text: function.span,
        parameters_list: &function.params,
        body: function.body.as_ref().unwrap(),
        is_concise_arrow_function: false,
        is_async: function.r#async,
        is_generator: function.generator,
        lexical_this: false,
        env: EnvironmentIndex::Global(agent.current_realm().global_env.unwrap()),
        private_env: None,
    };
    let f = ordinary_function_create(agent, params);

    set_function_name(
        agent,
        f,
        BUILTIN_STRING_MEMORY.anonymous.to_property_key(),
        None,
    );
    // NOTE: Skipping steps 30-32.

    Ok(f)
}
