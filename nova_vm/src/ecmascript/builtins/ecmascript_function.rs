use oxc_ast::{
    ast::{FormalParameters, FunctionBody},
    syntax_directed_operations::{BoundNames, IsSimpleParameterList},
};
use oxc_span::{Atom, Span};

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::define_property_or_throw, type_conversion::to_object,
        },
        execution::{
            agent::{
                get_active_script_or_module,
                ExceptionType::{self, SyntaxError},
            },
            new_declarative_environment, new_function_environment, Agent,
            ECMAScriptCodeEvaluationState, EnvironmentIndex, ExecutionContext, JsResult,
            PrivateEnvironmentIndex, ProtoIntrinsics, RealmIdentifier, ThisBindingStatus,
        },
        scripts_and_modules::ScriptOrModule,
        syntax_directed_operations::{
            function_definitions::evaluate_function_body,
            miscellaneous::instantiate_function_object,
            scope_analysis::{
                function_body_lexically_declared_names, function_body_lexically_scoped_decarations,
                function_body_var_declared_names, function_body_var_scoped_declarations,
                LexicallyScopedDeclaration, VarScopedDeclaration,
            },
        },
        types::{
            ECMAScriptFunctionHeapData, Function, InternalMethods, IntoFunction, IntoObject,
            IntoValue, Object, PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{indexes::ECMAScriptFunctionIndex, CreateHeapData, GetHeapData},
};

use super::{
    create_unmapped_arguments_object, ordinary::ordinary_object_create_with_intrinsics,
    ArgumentsList,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ECMAScriptFunction(ECMAScriptFunctionIndex);

impl From<ECMAScriptFunction> for ECMAScriptFunctionIndex {
    fn from(val: ECMAScriptFunction) -> Self {
        val.0
    }
}

impl From<ECMAScriptFunctionIndex> for ECMAScriptFunction {
    fn from(value: ECMAScriptFunctionIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for ECMAScriptFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for ECMAScriptFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoFunction for ECMAScriptFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<ECMAScriptFunction> for Value {
    fn from(val: ECMAScriptFunction) -> Self {
        Value::ECMAScriptFunction(val.0)
    }
}

impl From<ECMAScriptFunction> for Object {
    fn from(val: ECMAScriptFunction) -> Self {
        Object::ECMAScriptFunction(val.0)
    }
}

impl From<ECMAScriptFunction> for Function {
    fn from(val: ECMAScriptFunction) -> Self {
        Function::ECMAScriptFunction(val.0)
    }
}

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
pub(crate) struct ECMAScriptFunctionObjectHeapData {
    /// \[\[Environment]]
    pub environment: EnvironmentIndex,

    /// \[\[PrivateEnvironment]]
    pub private_environment: Option<PrivateEnvironmentIndex>,

    /// \[\[FormalParameters]]
    ///
    /// SAFETY: ScriptOrModule owns the Program which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub formal_parameters: &'static FormalParameters<'static>,

    /// \[\[ECMAScriptCode]]
    ///
    /// SAFETY: ScriptOrModule owns the Program which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub ecmascript_code: &'static FunctionBody<'static>,

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

pub(crate) struct OrdinaryFunctionCreateParams<'agent, 'program> {
    pub function_prototype: Option<Object>,
    pub source_text: Span,
    pub parameters_list: &'agent FormalParameters<'program>,
    pub body: &'agent FunctionBody<'program>,
    pub this_mode: ThisMode,
    pub env: EnvironmentIndex,
    pub private_env: Option<PrivateEnvironmentIndex>,
}

impl InternalMethods for ECMAScriptFunction {
    fn get_prototype_of(self, _agent: &mut Agent) -> JsResult<Option<Object>> {
        todo!()
    }

    fn set_prototype_of(self, _agent: &mut Agent, _prototype: Option<Object>) -> JsResult<bool> {
        todo!()
    }

    fn is_extensible(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn prevent_extensions(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        todo!()
    }

    fn define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!()
    }

    fn has_property(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn get(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _receiver: Value,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!()
    }

    fn delete(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn own_property_keys(self, _agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        todo!()
    }

    /// ### [10.2.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-call)
    ///
    /// The \[\[Call]] internal method of an ECMAScript function object `F`
    /// takes arguments `thisArgument` (an ECMAScript language value) and
    /// `argumentsList` (a List of ECMAScript language values) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion.
    fn call(
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
        if self.heap_data(agent).is_class_constructor {
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

    fn construct(
        self,
        _agent: &mut Agent,
        _arguments_list: ArgumentsList,
        _new_target: Function,
    ) -> JsResult<Object> {
        todo!()
    }
}

impl ECMAScriptFunction {
    pub(crate) fn heap_data(self, agent: &Agent) -> &ECMAScriptFunctionObjectHeapData {
        &agent.heap.get(self.0).ecmascript_function
    }
}

/// ### [10.2.1.1 PrepareForOrdinaryCall ( F, newTarget )](https://tc39.es/ecma262/#sec-prepareforordinarycall)
///
/// The abstract operation PrepareForOrdinaryCall takes arguments `F` (an
/// ECMAScript function object) and newTarget (an Object or undefined) and
/// returns an execution context.
pub(crate) fn prepare_for_ordinary_call(
    agent: &mut Agent,
    f: ECMAScriptFunction,
    new_target: Option<Object>,
) -> &ExecutionContext {
    let ecmascript_function_object = f.heap_data(agent);
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
        function: Some(f.into()),
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
    f: ECMAScriptFunction,
    local_env: EnvironmentIndex,
    this_argument: Value,
) {
    let function_heap_data = f.heap_data(agent);
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
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
) -> JsResult<Value> {
    let function_heap_data = function_object.heap_data(agent);
    // SAFETY: Heap is self-referential: This
    let heap_data = function_heap_data;
    if heap_data.ecmascript_code.statements.is_empty()
        && heap_data.formal_parameters.is_simple_parameter_list()
    {
        // Optimisation: Empty body and only simple parameters means no code will effectively run.
        return Ok(Value::Undefined);
    }
    // FunctionBody : FunctionStatementList
    // 1. Return ? EvaluateFunctionBody of FunctionBody with arguments functionObject and argumentsList.
    evaluate_function_body(agent, function_object, arguments_list)
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
    f: ECMAScriptFunction,
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
pub(crate) fn ordinary_function_create<'agent, 'program>(
    agent: &'agent mut Agent,
    params: OrdinaryFunctionCreateParams<'agent, 'program>,
) -> Function {
    // 1. Let internalSlotsList be the internal slots listed in Table 30.
    // 2. Let F be OrdinaryObjectCreate(functionPrototype, internalSlotsList).
    // 3. Set F.[[Call]] to the definition specified in 10.2.1.
    let ecmascript_function = ECMAScriptFunctionObjectHeapData {
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
                &'agent FormalParameters<'program>,
                &'static FormalParameters<'static>,
            >(params.parameters_list)
        },
        // 6. Set F.[[ECMAScriptCode]] to Body.
        // SAFETY: Same as above: Self-referential reference to ScriptOrModule.
        ecmascript_code: unsafe {
            std::mem::transmute::<&'agent FunctionBody<'program>, &FunctionBody<'static>>(
                params.body,
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
        ecmascript_function,
        name: None,
    };
    if let Some(function_prototype) = params.function_prototype {
        if function_prototype
            != agent
                .current_realm()
                .intrinsics()
                .function_prototype()
                .into_object()
        {
            function.object_index = Some(
                agent
                    .heap
                    .create_object_with_prototype(function_prototype, vec![]),
            );
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

/// ### [10.2.5 MakeConstructor ( F \[ , writablePrototype \[ , prototype \] \] )](https://tc39.es/ecma262/#sec-makeconstructor)
/// The abstract operation MakeConstructor takes argument F (an ECMAScript
/// function object or a built-in function object) and optional arguments
/// writablePrototype (a Boolean) and prototype (an Object) and returns
/// UNUSED. It converts F into a constructor.
pub(crate) fn make_constructor(
    agent: &mut Agent,
    function: Function,
    writable_prototype: Option<bool>,
    prototype: Option<Object>,
) {
    // 4. If writablePrototype is not present, set writablePrototype to true.
    let writable_prototype = writable_prototype.unwrap_or(true);
    match function {
        Function::BoundFunction(_) => unreachable!(),
        // 1. If F is an ECMAScript function object, then
        Function::ECMAScriptFunction(idx) => {
            // a. Assert: IsConstructor(F) is false.
            // TODO: How do we separate constructors and non-constructors?
            let data = agent.heap.get_mut(idx);
            // b. Assert: F is an extensible object that does not have a "prototype" own property.
            // TODO: Handle Some() object indexes?
            assert!(data.object_index.is_none());
            // c. Set F.[[Construct]] to the definition specified in 10.2.2.
            // 3. Set F.[[ConstructorKind]] to BASE.
            data.ecmascript_function.constructor_kind = ConstructorKind::Base;
        }
        Function::BuiltinFunction(_) => {
            // 2. Else,
            // a. Set F.[[Construct]] to the definition specified in 10.3.2.
        }
    }
    // 5. If prototype is not present, then
    let prototype = prototype.unwrap_or_else(|| {
        // a. Set prototype to OrdinaryObjectCreate(%Object.prototype%).
        let prototype =
            ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object));
        // b. Perform ! DefinePropertyOrThrow(prototype, "constructor", PropertyDescriptor { [[Value]]: F, [[Writable]]: writablePrototype, [[Enumerable]]: false, [[Configurable]]: true }).
        let key = PropertyKey::from(BUILTIN_STRING_MEMORY.constructor);
        define_property_or_throw(
            agent,
            prototype,
            key,
            PropertyDescriptor {
                value: Some(function.into_value()),
                writable: Some(writable_prototype),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        prototype
    });
    // 6. Perform ! DefinePropertyOrThrow(F, "prototype", PropertyDescriptor { [[Value]]: prototype, [[Writable]]: writablePrototype, [[Enumerable]]: false, [[Configurable]]: false }).
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.prototype);
    define_property_or_throw(
        agent,
        prototype,
        key,
        PropertyDescriptor {
            value: Some(prototype.into_value()),
            writable: Some(writable_prototype),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    // 7. Return UNUSED.
}

/// ### [10.2.9 SetFunctionName ( F, name \[ , prefix \] )](https://tc39.es/ecma262/#sec-setfunctionname)
/// The abstract operation SetFunctionName takes arguments F (a function
/// object) and name (a property key or Private Name) and optional argument
/// prefix (a String) and returns UNUSED. It adds a "name" property to F.
pub(crate) fn set_function_name(
    agent: &mut Agent,
    function: Function,
    name: PropertyKey,
    _prefix: Option<String>,
) {
    // 2. If name is a Symbol, then
    let name: String = match name {
        PropertyKey::Symbol(idx) => {
            // a. Let description be name's [[Description]] value.
            // b. If description is undefined, set name to the empty String.
            // c. Else, set name to the string-concatenation of "[", description, and "]".
            let symbol_data = agent.heap.get(idx);
            symbol_data
                .descriptor
                .map_or(String::EMPTY_STRING, |descriptor| {
                    let descriptor = descriptor.as_str(agent).unwrap();
                    String::from_str(agent, &format!("[{}]", descriptor))
                })
        }
        // TODO: Private Name
        // 3. Else if name is a Private Name, then
        // a. Set name to name.[[Description]].
        PropertyKey::Integer(_integer) => todo!(),
        PropertyKey::SmallString(str) => str.into(),
        PropertyKey::String(str) => str.into(),
    };
    // 5. If prefix is present, then
    // a. Set name to the string-concatenation of prefix, the code unit 0x0020 (SPACE), and name.
    // TODO: Handle prefixing

    match function {
        Function::BoundFunction(_idx) => todo!(),
        Function::BuiltinFunction(_idx) => todo!(),
        Function::ECMAScriptFunction(idx) => {
            let function = agent.heap.get_mut(idx);
            // 1. Assert: F is an extensible object that does not have a "name" own property.
            // TODO: Also potentially allow running this function with Some() object index if needed.
            assert!(function.object_index.is_none() && function.name.is_none());
            // 6. Perform ! DefinePropertyOrThrow(F, "name", PropertyDescriptor { [[Value]]: name, [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
            function.name = Some(name);
            // 7. Return UNUSED.
        }
    }
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

/// ### [10.2.11 FunctionDeclarationInstantiation ( func, argumentsList )](https://tc39.es/ecma262/#sec-functiondeclarationinstantiation)
///
/// The abstract operation FunctionDeclarationInstantiation takes arguments
/// func (an ECMAScript function object) and argumentsList (a List of
/// ECMAScript language values) and returns either a normal completion
/// containing unused or an abrupt completion. func is the function object for
/// which the execution context is being established.
///
/// #### Note 1:
/// When an execution context is established for evaluating an ECMAScript
/// function a new Function Environment Record is created and bindings for each
/// formal parameter are instantiated in that Environment Record. Each
/// declaration in the function body is also instantiated. If the function's
/// formal parameters do not include any default value initializers then the
/// body declarations are instantiated in the same Environment Record as the
/// parameters. If default value parameter initializers exist, a second
/// Environment Record is created for the body declarations. Formal parameters
/// and functions are initialized as part of FunctionDeclarationInstantiation.
/// All other bindings are initialized during evaluation of the function body.
pub(crate) fn function_declaration_instantiation(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
) -> JsResult<()> {
    // 1. Let calleeContext be the running execution context.
    let callee_context = agent.running_execution_context();
    // 35. Let privateEnv be the PrivateEnvironment of calleeContext.
    let ECMAScriptCodeEvaluationState {
        lexical_environment: callee_lex_env,
        variable_environment: callee_var_env,
        private_environment: callee_private_env,
    } = *callee_context.ecmascript_code.as_ref().unwrap();
    // 2. Let code be func.[[ECMAScriptCode]].
    let heap_data = agent.heap.get(function_object.0);
    let code = heap_data.ecmascript_function.ecmascript_code;
    // 3. Let strict be func.[[Strict]].
    let strict = heap_data.ecmascript_function.strict;
    // 4. Let formals be func.[[FormalParameters]].
    let formals = heap_data.ecmascript_function.formal_parameters;
    let this_mode = heap_data.ecmascript_function.this_mode;
    // 5. Let parameterNames be the BoundNames of formals.
    let mut parameter_names = Vec::with_capacity(
        heap_data
            .ecmascript_function
            .formal_parameters
            .parameters_count(),
    );
    heap_data
        .ecmascript_function
        .formal_parameters
        .bound_names(&mut |identifier| {
            parameter_names.push(identifier.name.clone());
        });
    // 6. If parameterNames has any duplicate entries, let hasDuplicates be true. Otherwise, let hasDuplicates be false.
    // TODO: Check duplicates
    let has_duplicates = false;

    // 7. Let simpleParameterList be IsSimpleParameterList of formals.
    let simple_parameter_list = formals.is_simple_parameter_list();
    // 8. Let hasParameterExpressions be ContainsExpression of formals.
    // TODO: impl ContainsExpression
    let has_parameter_expression = false;

    // 9. Let varNames be the VarDeclaredNames of code.
    let var_names = function_body_var_declared_names(code);
    // 10. Let varDeclarations be the VarScopedDeclarations of code.
    let var_declarations = function_body_var_scoped_declarations(code);
    // 11. Let lexicalNames be the LexicallyDeclaredNames of code.
    let lexical_names = function_body_lexically_declared_names(code);
    // 33. Let lexDeclarations be the LexicallyScopedDeclarations of code.
    let lex_declarations = function_body_lexically_scoped_decarations(code);
    // 12. Let functionNames be a new empty List.
    let mut function_names = vec![];
    // 13. Let functionsToInitialize be a new empty List.
    let mut functions_to_initialize: Vec<&oxc_ast::ast::Function<'_>> = vec![];
    // 14. For each element d of varDeclarations, in reverse List order, do
    for d in var_declarations.iter().rev() {
        // a. If d is neither a VariableDeclaration nor a ForBinding nor a BindingIdentifier, then
        if let VarScopedDeclaration::Function(d) = d {
            // i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
            // ii. Let fn be the sole element of the BoundNames of d.
            let f_name = d.id.clone().unwrap().name;
            // iii. If functionNames does not contain fn, then
            if !function_names.contains(&f_name) {
                // 1. Insert fn as the first element of functionNames.
                function_names.push(f_name);
                // 2. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
                // 3. Insert d as the first element of functionsToInitialize.
                functions_to_initialize.push(d);
            }
        }
    }
    // 15. Let argumentsObjectNeeded be true.
    let arguments_name = Atom::new_inline("arguments");
    let mut arguments_object_needed = true;
    // 16. If func.[[ThisMode]] is lexical, then
    if this_mode == ThisMode::Lexical {
        // a. NOTE: Arrow functions never have an arguments object.
        // b. Set argumentsObjectNeeded to false.
        arguments_object_needed = false;
    } else {
        // 17. Else if parameterNames contains "arguments", then
        if parameter_names.contains(&arguments_name) {
            // a. Set argumentsObjectNeeded to false.
            arguments_object_needed = false;
        } else if !has_parameter_expression {
            // 18. Else if hasParameterExpressions is false, then
            // a. If functionNames contains "arguments" or lexicalNames contains "arguments", then
            if function_names.contains(&arguments_name) || lexical_names.contains(&arguments_name) {
                // i. Set argumentsObjectNeeded to false.
                arguments_object_needed = false;
            }
        }
    }
    // 19. If strict is true or hasParameterExpressions is false, then
    let env = if strict || !has_parameter_expression {
        // a. NOTE: Only a single Environment Record is needed for the parameters,
        // since calls to eval in strict mode code cannot create new bindings which are visible outside of the eval.
        // b. Let env be the LexicalEnvironment of calleeContext.
        callee_lex_env
    } else {
        // 20. Else,
        // a. NOTE: A separate Environment Record is needed to ensure that bindings created by direct eval calls in
        // the formal parameter list are outside the environment where parameters are declared.
        // b. Let calleeEnv be the LexicalEnvironment of calleeContext.
        let callee_env = callee_lex_env;
        // c. Let env be NewDeclarativeEnvironment(calleeEnv).
        let env =
            EnvironmentIndex::Declarative(new_declarative_environment(agent, Some(callee_env)));
        // d. Assert: The VariableEnvironment of calleeContext is calleeEnv.
        assert_eq!(callee_var_env, callee_env);
        // e. Set the LexicalEnvironment of calleeContext to env.
        agent
            .running_execution_context_mut()
            .ecmascript_code
            .as_mut()
            .unwrap()
            .lexical_environment = env;
        env
    };

    // 21. For each String paramName of parameterNames, do
    for param_name in &parameter_names {
        // a. Let alreadyDeclared be ! env.HasBinding(paramName).
        let already_declared = env.has_binding(agent, param_name).unwrap();
        // b. NOTE: Early errors ensure that duplicate parameter names can only occur
        // in non-strict functions that do not have parameter default values or rest parameters.
        // c. If alreadyDeclared is false, then
        if !already_declared {
            // i. Perform ! env.CreateMutableBinding(paramName, false).
            env.create_mutable_binding(agent, param_name, false)
                .unwrap();
        }
        // ii. If hasDuplicates is true, then
        if has_duplicates {
            // 1. Perform ! env.InitializeBinding(paramName, undefined).
            env.initialize_binding(agent, param_name, Value::Undefined)
                .unwrap();
        }
    }

    // 22. If argumentsObjectNeeded is true, then
    let parameter_bindings = if arguments_object_needed {
        // a. If strict is true or simpleParameterList is false, then
        let ao: Value = if strict || !simple_parameter_list {
            // i. Let ao be CreateUnmappedArgumentsObject(argumentsList).
            create_unmapped_arguments_object(agent, arguments_list).into_value()
        } else {
            // b. Else,
            todo!("Handle arguments object creation");
            // i. NOTE: A mapped argument object is only provided for non-strict functions
            // that don't have a rest parameter, any parameter default value initializers,
            // or any destructured parameters.
            // ii. Let ao be CreateMappedArgumentsObject(func, formals, argumentsList, env).
        };
        // c. If strict is true, then
        if strict {
            // i. Perform ! env.CreateImmutableBinding("arguments", false).
            env.create_immutable_binding(agent, &arguments_name, false)
                .unwrap();
        } else {
            // ii. NOTE: In strict mode code early errors prevent attempting to assign to this binding, so its mutability is not observable.
            // d. Else,
            // i. Perform ! env.CreateMutableBinding("arguments", false).
            env.create_mutable_binding(agent, &arguments_name, false)
                .unwrap();
            // e. Perform ! env.InitializeBinding("arguments", ao).
            env.initialize_binding(agent, &arguments_name, ao).unwrap();
            // f. Let parameterBindings be the list-concatenation of parameterNames and « "arguments" ».
        }
        parameter_names.push(arguments_name);
        parameter_names
    } else {
        // 23. Else,
        // a. Let parameterBindings be parameterNames.
        parameter_names
    };

    // 24. Let iteratorRecord be CreateListIteratorRecord(argumentsList).
    // TODO: Spread arguments.
    // let iterator_record = create_list_iterator_record(arguments_list);
    // 25. If hasDuplicates is true, then
    // if has_duplicates {
    // a. Perform ? IteratorBindingInitialization of formals with arguments iteratorRecord and undefined.
    // iterator_binding_initialization(agent, formals, iterator_record, None);
    // } else {
    // 26. Else,
    // a. Perform ? IteratorBindingInitialization of formals with arguments iteratorRecord and env.
    // iterator_binding_initialization(agent, formals, iterator_record, env);
    // }

    // 27. If hasParameterExpressions is false, then
    let var_env = if !has_parameter_expression {
        // a. NOTE: Only a single Environment Record is needed for the parameters and top-level vars.
        // b. Let instantiatedVarNames be a copy of the List parameterBindings.
        let mut instantiated_var_names = parameter_bindings.clone();
        // c. For each element n of varNames, do
        for n in &var_names {
            // i. If instantiatedVarNames does not contain n, then
            if !instantiated_var_names.contains(n) {
                // 1. Append n to instantiatedVarNames.
                instantiated_var_names.push(n.clone());
                // 2. Perform ! env.CreateMutableBinding(n, false).
                env.create_mutable_binding(agent, n, false).unwrap();
                // 3. Perform ! env.InitializeBinding(n, undefined).
                env.initialize_binding(agent, n, Value::Undefined).unwrap();
            }
        }
        // d. Let varEnv be env.
        env
    } else {
        // 28. Else,
        // a. NOTE: A separate Environment Record is needed to ensure that closures
        // created by expressions in the formal parameter list do not have visibility
        // of declarations in the function body.
        // b. Let varEnv be NewDeclarativeEnvironment(env).
        let var_env = EnvironmentIndex::Declarative(new_declarative_environment(agent, Some(env)));
        // c. Set the VariableEnvironment of calleeContext to varEnv.
        agent
            .running_execution_context_mut()
            .ecmascript_code
            .as_mut()
            .unwrap()
            .variable_environment = var_env;
        // d. Let instantiatedVarNames be a new empty List.
        let mut instantiated_var_names = vec![];
        // e. For each element n of varNames, do
        for n in &var_names {
            // i. If instantiatedVarNames does not contain n, then
            if !instantiated_var_names.contains(&n) {
                // 1. Append n to instantiatedVarNames.
                instantiated_var_names.push(n);
                // 2. Perform ! varEnv.CreateMutableBinding(n, false).
                var_env.create_mutable_binding(agent, n, false).unwrap();
                // 3. If parameterBindings does not contain n, or if functionNames contains n, then
                let initial_value = if !parameter_bindings.contains(n) || function_names.contains(n)
                {
                    // a. Let initialValue be undefined.
                    Value::Undefined
                } else {
                    // 4. Else,
                    // a. Let initialValue be ! env.GetBindingValue(n, false).
                    env.get_binding_value(agent, n, false).unwrap()
                };
                // 5. Perform ! varEnv.InitializeBinding(n, initialValue).
                var_env.initialize_binding(agent, n, initial_value).unwrap();
                // 6. NOTE: A var with the same name as a formal parameter initially has
                // the same value as the corresponding initialized parameter.
            }
        }
        var_env
    };

    // 29. NOTE: Annex B.3.2.1 adds additional steps at this point.
    // 30. If strict is false, then
    let lex_env = if !strict {
        // a. Let lexEnv be NewDeclarativeEnvironment(varEnv).
        // b. NOTE: Non-strict functions use a separate Environment Record for top-level
        // lexical declarations so that a direct eval can determine whether any var scoped
        // declarations introduced by the eval code conflict with pre-existing top-level
        // lexically scoped declarations. This is not needed for strict functions because
        // a strict direct eval always places all declarations into a new Environment Record.
        EnvironmentIndex::Declarative(new_declarative_environment(agent, Some(var_env)))
    } else {
        // 31. Else,
        // a. Let lexEnv be varEnv.
        var_env
    };
    // 32. Set the LexicalEnvironment of calleeContext to lexEnv.
    agent
        .running_execution_context_mut()
        .ecmascript_code
        .as_mut()
        .unwrap()
        .lexical_environment = lex_env;
    // 34. For each element d of lexDeclarations, do
    for d in lex_declarations {
        // a. NOTE: A lexically declared name cannot be the same as a function/generator
        // declaration, formal parameter, or a var name. Lexically declared names are
        // only instantiated here but not initialized.
        // b. For each element dn of the BoundNames of d, do
        match d {
            LexicallyScopedDeclaration::Variable(decl) => {
                // i. If IsConstantDeclaration of d is true, then
                if decl.kind.is_const() {
                    // 1. Perform ! lexEnv.CreateImmutableBinding(dn, true).
                    decl.id.bound_names(&mut |identifier| {
                        lex_env
                            .create_immutable_binding(agent, &identifier.name, true)
                            .unwrap();
                    });
                } else {
                    decl.id.bound_names(&mut |identifier| {
                        // ii. Else,
                        // 1. Perform ! lexEnv.CreateMutableBinding(dn, false).
                        lex_env
                            .create_mutable_binding(agent, &identifier.name, false)
                            .unwrap();
                    });
                }
            }
            LexicallyScopedDeclaration::Function(decl) => {
                lex_env
                    .create_mutable_binding(agent, &decl.id.as_ref().unwrap().name, false)
                    .unwrap();
            }
            LexicallyScopedDeclaration::Class(decl) => {
                lex_env
                    .create_mutable_binding(agent, &decl.id.as_ref().unwrap().name, false)
                    .unwrap();
            }
            LexicallyScopedDeclaration::DefaultExport => {
                lex_env
                    .create_mutable_binding(agent, &Atom::new_inline("*default*"), false)
                    .unwrap();
            }
        }
    }
    // 35. Let privateEnv be the PrivateEnvironment of calleeContext.
    let private_env = callee_private_env;
    // 36. For each Parse Node f of functionsToInitialize, do
    for f in functions_to_initialize {
        // a. Let fn be the sole element of the BoundNames of f.
        let f_name = &f.id.as_ref().unwrap().name;
        // b. Let fo be InstantiateFunctionObject of f with arguments lexEnv and privateEnv.
        let fo = instantiate_function_object(agent, f, lex_env, private_env);
        // c. Perform ! varEnv.SetMutableBinding(fn, fo, false).
        var_env
            .set_mutable_binding(agent, f_name, fo.into_value(), false)
            .unwrap();
    }
    // 37. Return unused.
    Ok(())

    // Note 2

    // B.3.2 provides an extension to the above algorithm that is necessary for backwards
    // compatibility with web browser implementations of ECMAScript that predate ECMAScript 2015.
}
