use std::ptr::NonNull;

use oxc_ast::ast::{FormalParameters, FunctionBody};
use oxc_span::Span;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_object,
        execution::{
            agent::{
                get_active_script_or_module,
                ExceptionType::{self, SyntaxError},
            },
            new_function_environment, Agent, ECMAScriptCodeEvaluationState, EnvironmentIndex,
            ExecutionContext, JsResult, PrivateEnvironmentIndex, RealmIdentifier,
            ThisBindingStatus,
        },
        scripts_and_modules::ScriptOrModule,
        types::{ECMAScriptFunctionHeapData, Function, Object, Value},
    },
    heap::{indexes::ECMAScriptFunctionIndex, CreateHeapData, GetHeapData},
};

use super::ArgumentsList;

#[derive(Debug, Clone, Copy)]
pub enum ConstructorKind {
    Base,
    Derived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThisMode {
    Lexical,
    Strict,
    Global,
}

/// ### [10.2 ECMAScript Function Objects](https://tc39.es/ecma262/#sec-ecmascript-function-objects)
#[derive(Debug)]
pub(crate) struct ECMAScriptFunction {
    /// \[\[Environment]]
    pub environment: EnvironmentIndex,

    /// \[\[PrivateEnvironment]]
    pub private_environment: Option<PrivateEnvironmentIndex>,

    /// \[\[FormalParameters]]
    ///
    /// SAFETY: ScriptOrModule owns the Program which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub formal_parameters: NonNull<FormalParameters<'static>>,

    /// \[\[ECMAScriptCode]]
    ///
    /// SAFETY: ScriptOrModule owns the Program which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub ecmascript_code: NonNull<FunctionBody<'static>>,

    /// \[\[ConstructorKind]]
    pub constructor_kind: ConstructorKind,

    /// \[\[Realm]]
    pub realm: RealmIdentifier,

    /// \[\[ScriptOrModule]]
    pub script_or_module: ScriptOrModule,

    /// \[\[ThisMode]]
    pub this_mode: ThisMode,

    /// \[\[Strict]]
    pub strict: bool,

    /// \[\[HomeObject]]
    pub home_object: Option<Object>,

    ///  [[SourceText]]
    pub source_text: Span,

    // TODO: [[Fields]],  [[PrivateMethods]], [[ClassFieldInitializerName]]
    /// \[\[IsClassConstructor]]
    pub is_class_constructor: bool,
}

pub(crate) struct OrdinaryFunctionCreateParams<'program> {
    pub function_prototype: Option<Object>,
    pub source_text: Span,
    pub parameters_list: &'program FormalParameters<'program>,
    pub body: &'program FunctionBody<'program>,
    pub this_mode: ThisMode,
    pub env: EnvironmentIndex,
    pub private_env: Option<PrivateEnvironmentIndex>,
}

impl ECMAScriptFunctionIndex {
    pub(crate) fn get(self, agent: &Agent) -> &ECMAScriptFunction {
        &agent.heap.get(self).ecmascript_function
    }

    /// ### [10.2.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-call)
    ///
    /// The \[\[Call]] internal method of an ECMAScript function object `F`
    /// takes arguments `thisArgument` (an ECMAScript language value) and
    /// `argumentsList` (a List of ECMAScript language values) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion.
    pub(crate) fn call(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList<'_>,
    ) -> JsResult<Value> {
        // 1. Let callerContext be the running execution context.
        let _ = agent.running_execution_context();
        // 2. Let calleeContext be PrepareForOrdinaryCall(F, undefined).
        let callee_context = prepare_for_ordinary_call(agent, self, None);
        // This is step 4. or OrdinaryCallBindThis:
        // "Let localEnv be the LexicalEnvironment of calleeContext."
        let local_env = callee_context
            .ecmascript_code
            .as_ref()
            .unwrap()
            .lexical_environment;
        // 3. Assert: calleeContext is now the running execution context.
        // assert!(std::ptr::eq(agent.running_execution_context(), callee_context));
        // 4. If F.[[IsClassConstructor]] is true, then
        if self.get(agent).is_class_constructor {
            // a. Let error be a newly created TypeError object.
            // b. NOTE: error is created in calleeContext with F's associated Realm Record.
            let error = agent.throw_exception(ExceptionType::TypeError, "fail");
            // c. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
            agent.execution_context_stack.pop();
            // d. Return ThrowCompletion(error).
            return Err(error);
        }
        // 5. Perform OrdinaryCallBindThis(F, calleeContext, thisArgument).
        // Note: We pass in the localEnv directly to avoid borrow issues.
        ordinary_call_bind_this(agent, self, local_env, this_argument);
        // 6. Let result be Completion(OrdinaryCallEvaluateBody(F, argumentsList)).
        let result = ordinary_call_evaluate_body(agent, self, arguments_list);
        // 7. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
        // NOTE: calleeContext must not be destroyed if it is suspended and retained for later resumption by an accessible Generator.
        agent.execution_context_stack.pop();
        // 8. If result is a return completion, return result.[[Value]].
        // 9. ReturnIfAbrupt(result).
        // 10. Return undefined.
        result
    }
}

/// ### [10.2.1.1 PrepareForOrdinaryCall ( F, newTarget )](https://tc39.es/ecma262/#sec-prepareforordinarycall)
///
/// The abstract operation PrepareForOrdinaryCall takes arguments `F` (an
/// ECMAScript function object) and newTarget (an Object or undefined) and
/// returns an execution context.
pub(crate) fn prepare_for_ordinary_call(
    agent: &mut Agent,
    f: ECMAScriptFunctionIndex,
    new_target: Option<Object>,
) -> &ExecutionContext {
    let ecmascript_function_object = f.get(agent);
    let private_environment = ecmascript_function_object.private_environment;
    let script_or_module = ecmascript_function_object.script_or_module;
    // 1. Let callerContext be the running execution context.
    let _caller_context = agent.running_execution_context();
    // 4. Let calleeRealm be F.[[Realm]].
    let callee_realm = ecmascript_function_object.realm;
    // 7. Let localEnv be NewFunctionEnvironment(F, newTarget).
    let local_env = new_function_environment(agent, f, new_target);
    // 2. Let calleeContext be a new ECMAScript code execution context.
    let callee_context = ExecutionContext {
        // 8. Set the LexicalEnvironment of calleeContext to localEnv.
        // 9. Set the VariableEnvironment of calleeContext to localEnv.
        // 10. Set the PrivateEnvironment of calleeContext to F.[[PrivateEnvironment]].
        ecmascript_code: Some(ECMAScriptCodeEvaluationState {
            lexical_environment: EnvironmentIndex::Function(local_env),
            variable_environment: EnvironmentIndex::Function(local_env),
            private_environment,
        }),
        // 3. Set the Function of calleeContext to F.
        function: Some(Function::ECMAScriptFunction(f)),
        // 5. Set the Realm of calleeContext to calleeRealm.
        realm: callee_realm,
        // 6. Set the ScriptOrModule of calleeContext to F.[[ScriptOrModule]].
        script_or_module: Some(script_or_module),
    };
    // 11. If callerContext is not already suspended, suspend callerContext.
    // 12. Push calleeContext onto the execution context stack; calleeContext is now the running execution context.
    agent.execution_context_stack.push(callee_context);
    // 13. NOTE: Any exception objects produced after this point are associated with calleeRealm.
    // 14. Return calleeContext.
    agent.execution_context_stack.last().unwrap()
}

/// ### [10.2.1.2 OrdinaryCallBindThis ( F, calleeContext, thisArgument )](https://tc39.es/ecma262/#sec-ordinarycallbindthis)
///
/// The abstract operation OrdinaryCallBindThis takes arguments `F` (an
/// ECMAScript function object), calleeContext (an execution context), and
/// `thisArgument` (an ECMAScript language value) and returns UNUSED.
///
/// Note: calleeContext is replaced by localEnv which is the only thing it is
/// truly used for.
pub(crate) fn ordinary_call_bind_this(
    agent: &mut Agent,
    f: ECMAScriptFunctionIndex,
    local_env: EnvironmentIndex,
    this_argument: Value,
) {
    let function_heap_data = f.get(agent);
    // 1. Let thisMode be F.[[ThisMode]].
    let this_mode = function_heap_data.this_mode;
    // 2. If thisMode is LEXICAL, return UNUSED.
    if this_mode == ThisMode::Lexical {
        return;
    }
    // 3. Let calleeRealm be F.[[Realm]].
    let callee_realm = function_heap_data.realm;
    // 4. Let localEnv be the LexicalEnvironment of calleeContext.
    // 5. If thisMode is STRICT, then
    let this_value = if this_mode == ThisMode::Strict {
        // a. Let thisValue be thisArgument.
        this_argument
    } else {
        // 6. Else,
        // a. If thisArgument is either undefined or null, then
        if this_argument == Value::Undefined || this_argument == Value::Null {
            // i. Let globalEnv be calleeRealm.[[GlobalEnv]].
            let global_env = agent.get_realm(callee_realm).global_env;
            // ii. Assert: globalEnv is a Global Environment Record.
            let global_env = global_env.unwrap();
            // iii. Let thisValue be globalEnv.[[GlobalThisValue]].
            global_env.get_this_binding(agent).into_value()
        } else {
            // b. Else,
            // i. Let thisValue be ! ToObject(thisArgument).
            to_object(agent, this_argument).unwrap().into_value()
            // ii. NOTE: ToObject produces wrapper objects using calleeRealm.
        }
    };
    // 7. Assert: localEnv is a Function Environment Record.
    let EnvironmentIndex::Function(local_env) = local_env else {
        panic!("localEnv is not a Function Environment Record");
    };
    // 8. Assert: The next step never returns an abrupt completion because localEnv.[[ThisBindingStatus]] is not INITIALIZED.
    assert_ne!(
        local_env.get_this_binding_status(agent),
        ThisBindingStatus::Initialized
    );
    // 9. Perform ! localEnv.BindThisValue(thisValue).
    local_env.bind_this_value(agent, this_value).unwrap();
    // 10. Return UNUSED.
}

/// ### [10.2.1.3 Runtime Semantics: EvaluateBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluatebody)
///
/// The syntax-directed operation EvaluateBody takes arguments `functionObject`
/// (an ECMAScript function object) and `argumentsList` (a List of ECMAScript
/// language values) and returns either a normal completion containing an
/// ECMAScript language value or an abrupt completion.
pub(crate) fn evaluate_body(
    agent: &mut Agent,
    function_object: ECMAScriptFunctionIndex,
    _arguments_list: ArgumentsList,
) -> JsResult<Value> {
    let function_heap_data = function_object.get(agent);
    let body = unsafe { function_heap_data.ecmascript_code.as_ref() };
    if body.statements.is_empty() {
        return Ok(Value::Undefined);
    }
    todo!()
    // FunctionBody : FunctionStatementList
    // 1. Return ? EvaluateFunctionBody of FunctionBody with arguments functionObject and argumentsList.
    // ConciseBody : ExpressionBody
    // 1. Return ? EvaluateConciseBody of ConciseBody with arguments functionObject and argumentsList.
    // GeneratorBody : FunctionBody
    // 1. Return ? EvaluateGeneratorBody of GeneratorBody with arguments functionObject and argumentsList.
    // AsyncGeneratorBody : FunctionBody
    // 1. Return ? EvaluateAsyncGeneratorBody of AsyncGeneratorBody with arguments functionObject and argumentsList.
    // AsyncFunctionBody : FunctionBody
    // 1. Return ? EvaluateAsyncFunctionBody of AsyncFunctionBody with arguments functionObject and argumentsList.
    // AsyncConciseBody : ExpressionBody
    // 1. Return ? EvaluateAsyncConciseBody of AsyncConciseBody with arguments functionObject and argumentsList.
    // Initializer :
    // = AssignmentExpression
    // 1. Assert: argumentsList is empty.
    // 2. Assert: functionObject.[[ClassFieldInitializerName]] is not EMPTY.
    // 3. If IsAnonymousFunctionDefinition(AssignmentExpression) is true, then
    // a. Let value be ? NamedEvaluation of Initializer with argument functionObject.[[ClassFieldInitializerName]].
    // 4. Else,
    // a. Let rhs be ? Evaluation of AssignmentExpression.
    // b. Let value be ? GetValue(rhs).
    // 5. Return Completion Record { [[Type]]: RETURN, [[Value]]: value, [[Target]]: EMPTY }.
    // NOTE
    // Even though field initializers constitute a function boundary, calling FunctionDeclarationInstantiation does not have any observable effect and so is omitted.
    // ClassStaticBlockBody : ClassStaticBlockStatementList
    // 1. Assert: argumentsList is empty.
    // 2. Return ? EvaluateClassStaticBlockBody of ClassStaticBlockBody with argument functionObject.
}

/// ### [10.2.1.4 OrdinaryCallEvaluateBody ( F, argumentsList )](https://tc39.es/ecma262/#sec-ordinarycallevaluatebody)
///
/// The abstract operation OrdinaryCallEvaluateBody takes arguments `F` (an
/// ECMAScript function object) and `argumentsList` (a List of ECMAScript
/// language values) and returns either a normal completion containing an
/// ECMAScript language value or an abrupt completion.
pub(crate) fn ordinary_call_evaluate_body(
    agent: &mut Agent,
    f: ECMAScriptFunctionIndex,
    arguments_list: ArgumentsList,
) -> JsResult<Value> {
    // 1. Return ? EvaluateBody of F.[[ECMAScriptCode]] with arguments F and argumentsList.
    evaluate_body(agent, f, arguments_list)
}

/// ### [10.2.3 OrdinaryFunctionCreate ( functionPrototype, sourceText, ParameterList, Body, thisMode, env, privateEnv )](https://tc39.es/ecma262/#sec-ordinaryfunctioncreate)
///
/// The abstract operation OrdinaryFunctionCreate takes arguments
/// functionPrototype (an Object), sourceText (a sequence of Unicode code
/// points), ParameterList (a Parse Node), Body (a Parse Node), thisMode
/// (LEXICAL-THIS or NON-LEXICAL-THIS), env (an Environment Record), and
/// privateEnv (a PrivateEnvironment Record or null) and returns an ECMAScript
/// function object. It is used to specify the runtime creation of a new
/// function with a default \[\[Call\]\] internal method and no
/// \[\[Construct\]\] internal method (although one may be subsequently added
/// by an operation such as MakeConstructor). sourceText is the source text of
/// the syntactic definition of the function to be created.
pub(crate) fn ordinary_function_create<'program>(
    agent: &mut Agent,
    params: OrdinaryFunctionCreateParams<'program>,
) -> Function {
    // 1. Let internalSlotsList be the internal slots listed in Table 30.
    // 2. Let F be OrdinaryObjectCreate(functionPrototype, internalSlotsList).
    // 3. Set F.[[Call]] to the definition specified in 10.2.1.
    let ecmascript_function = ECMAScriptFunction {
        // 13. Set F.[[Environment]] to env.
        environment: params.env,
        // 14. Set F.[[PrivateEnvironment]] to privateEnv.
        private_environment: params.private_env,
        // 5. Set F.[[FormalParameters]] to ParameterList.
        // SAFETY: The reference to FormalParameters points to ScriptOrModule
        // and is valid until it gets dropped. Our GC keeps ScriptOrModule
        // alive until this ECMAScriptFunction gets dropped, hence the 'static
        // lifetime here is justified.
        formal_parameters: unsafe {
            std::mem::transmute::<
                NonNull<FormalParameters<'program>>,
                NonNull<FormalParameters<'static>>,
            >(params.parameters_list.into())
        },
        // 6. Set F.[[ECMAScriptCode]] to Body.
        // SAFETY: Same as above: Self-referential reference to ScriptOrModule.
        ecmascript_code: unsafe {
            std::mem::transmute::<NonNull<FunctionBody<'program>>, NonNull<FunctionBody<'static>>>(
                params.body.into(),
            )
        },
        constructor_kind: ConstructorKind::Base,
        // 16. Set F.[[Realm]] to the current Realm Record.
        realm: agent.current_realm_id(),
        // 15. Set F.[[ScriptOrModule]] to GetActiveScriptOrModule().
        script_or_module: get_active_script_or_module(agent).unwrap(),
        // 9. If thisMode is LEXICAL-THIS, set F.[[ThisMode]] to LEXICAL.
        // 10. Else if Strict is true, set F.[[ThisMode]] to STRICT.
        // 11. Else, set F.[[ThisMode]] to GLOBAL.
        this_mode: params.this_mode,
        // 7. If the source text matched by Body is strict mode code, let Strict be true; else let Strict be false.
        // 8. Set F.[[Strict]] to Strict.
        strict: true,
        // 17. Set F.[[HomeObject]] to undefined.
        home_object: None,
        // 4. Set F.[[SourceText]] to sourceText.
        source_text: params.source_text,
        // 12. Set F.[[IsClassConstructor]] to false.
        is_class_constructor: false,
    };

    let mut function = ECMAScriptFunctionHeapData {
        object_index: None,
        length: 0,
        initial_name: Value::Undefined,
        ecmascript_function,
    };
    if let Some(function_prototype) = params.function_prototype {
        if function_prototype != agent.current_realm().intrinsics().function_prototype() {
            function.object_index =
                Some(agent.heap.create_object_with_prototype(function_prototype));
        }
    }

    // 18. Set F.[[Fields]] to a new empty List.
    // 19. Set F.[[PrivateMethods]] to a new empty List.
    // 20. Set F.[[ClassFieldInitializerName]] to EMPTY.
    // 21. Let len be the ExpectedArgumentCount of ParameterList.
    // 22. Perform SetFunctionLength(F, len).
    set_ecmascript_function_length(
        agent,
        &mut function,
        params
            .parameters_list
            .items
            .iter()
            .filter(|par| !par.pattern.kind.is_binding_identifier())
            .count(),
    )
    .unwrap();
    // 23. Return F.
    agent.heap.create(function)
}

/// ### [10.2.10 SetFunctionLength ( F, length )](https://tc39.es/ecma262/#sec-setfunctionlength)
fn set_ecmascript_function_length(
    agent: &mut Agent,
    function: &mut ECMAScriptFunctionHeapData,
    length: usize,
) -> JsResult<()> {
    // TODO: 1. Assert: F is an extensible object that does not have a "length" own property.

    // 2. Perform ! DefinePropertyOrThrow(F, "length", PropertyDescriptor { [[Value]]: 𝔽(length), [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
    if length > u8::MAX as usize {
        return Err(agent.throw_exception(
            SyntaxError,
            "Too many arguments in function call (only 255 allowed)",
        ));
    }
    function.length = length as u8;

    // 3. Return unused.
    Ok(())
}
