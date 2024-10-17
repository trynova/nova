// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    ops::{Index, IndexMut},
    ptr::NonNull,
};

use oxc_ast::{
    ast::{FormalParameters, FunctionBody},
    syntax_directed_operations::IsSimpleParameterList,
};
use oxc_span::Span;

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
            new_function_environment, Agent, ECMAScriptCodeEvaluationState, EnvironmentIndex,
            ExecutionContext, FunctionEnvironmentIndex, JsResult, PrivateEnvironmentIndex,
            ProtoIntrinsics, RealmIdentifier, ThisBindingStatus,
        },
        scripts_and_modules::{source_code::SourceCode, ScriptOrModule},
        syntax_directed_operations::function_definitions::{
            evaluate_async_function_body, evaluate_function_body, evaluate_generator_body,
        },
        types::{
            function_create_backing_object, function_internal_define_own_property,
            function_internal_delete, function_internal_get, function_internal_get_own_property,
            function_internal_has_property, function_internal_own_property_keys,
            function_internal_set, ECMAScriptFunctionHeapData, Function,
            FunctionInternalProperties, InternalMethods, InternalSlots, IntoFunction, IntoObject,
            IntoValue, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::ECMAScriptFunctionIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

use super::{
    ordinary::{ordinary_create_from_constructor, ordinary_object_create_with_intrinsics},
    ArgumentsList,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

impl TryFrom<Value> for ECMAScriptFunction {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Object> for ECMAScriptFunction {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        if let Object::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Function> for ECMAScriptFunction {
    type Error = ();

    fn try_from(value: Function) -> Result<Self, Self::Error> {
        if let Function::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl From<ECMAScriptFunction> for Value {
    fn from(val: ECMAScriptFunction) -> Self {
        Value::ECMAScriptFunction(val)
    }
}

impl From<ECMAScriptFunction> for Object {
    fn from(val: ECMAScriptFunction) -> Self {
        Object::ECMAScriptFunction(val)
    }
}

impl From<ECMAScriptFunction> for Function {
    fn from(val: ECMAScriptFunction) -> Self {
        Function::ECMAScriptFunction(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorStatus {
    NonConstructor,
    ConstructorFunction,
    BaseClass,
    DerivedClass,
}

impl ConstructorStatus {
    pub fn is_constructor(self) -> bool {
        self != ConstructorStatus::NonConstructor
    }
    pub fn is_class_constructor(self) -> bool {
        matches!(
            self,
            ConstructorStatus::BaseClass | ConstructorStatus::DerivedClass
        )
    }
    pub fn is_derived_class(self) -> bool {
        self == ConstructorStatus::DerivedClass
    }
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
    /// SAFETY: SourceCode owns the Allocator into which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub formal_parameters: NonNull<FormalParameters<'static>>,

    /// \[\[ECMAScriptCode]]
    ///
    /// SAFETY: SourceCode owns the Allocator into which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub ecmascript_code: NonNull<FunctionBody<'static>>,

    /// True if the function body is a ConciseBody (can only be true for arrow
    /// functions).
    ///
    /// This is used to know whether to treat the function as having an implicit
    /// return or not.
    pub is_concise_arrow_function: bool,

    pub is_async: bool,

    pub is_generator: bool,

    /// \[\[ConstructorKind]]
    /// \[\[IsClassConstructor]]
    pub constructor_status: ConstructorStatus,

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

    ///  \[\[SourceText]]
    pub source_text: Span,

    /// \[\[SourceCode]]
    ///
    /// Nova specific addition: This SourceCode is where \[\[SourceText]]
    /// refers to.
    pub source_code: SourceCode,
    // TODO: [[Fields]],  [[PrivateMethods]], [[ClassFieldInitializerName]]
}

pub(crate) struct OrdinaryFunctionCreateParams<'agent, 'program> {
    pub function_prototype: Option<Object>,
    pub source_code: Option<SourceCode>,
    pub source_text: Span,
    pub parameters_list: &'agent FormalParameters<'program>,
    pub body: &'agent FunctionBody<'program>,
    pub is_concise_arrow_function: bool,
    pub is_async: bool,
    pub is_generator: bool,
    pub lexical_this: bool,
    pub env: EnvironmentIndex,
    pub private_env: Option<PrivateEnvironmentIndex>,
}

impl Index<ECMAScriptFunction> for Agent {
    type Output = ECMAScriptFunctionHeapData;

    fn index(&self, index: ECMAScriptFunction) -> &Self::Output {
        &self.heap.ecmascript_functions[index]
    }
}

impl IndexMut<ECMAScriptFunction> for Agent {
    fn index_mut(&mut self, index: ECMAScriptFunction) -> &mut Self::Output {
        &mut self.heap.ecmascript_functions[index]
    }
}

impl Index<ECMAScriptFunction> for Vec<Option<ECMAScriptFunctionHeapData>> {
    type Output = ECMAScriptFunctionHeapData;

    fn index(&self, index: ECMAScriptFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("ECMAScriptFunction out of bounds")
            .as_ref()
            .expect("ECMAScriptFunction slot empty")
    }
}

impl IndexMut<ECMAScriptFunction> for Vec<Option<ECMAScriptFunctionHeapData>> {
    fn index_mut(&mut self, index: ECMAScriptFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ECMAScriptFunction out of bounds")
            .as_mut()
            .expect("ECMAScriptFunction slot empty")
    }
}

impl ECMAScriptFunction {
    pub(crate) const fn _def() -> Self {
        ECMAScriptFunction(ECMAScriptFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn is_constructor(self, agent: &Agent) -> bool {
        // An ECMAScript function has the [[Construct]] slot if its constructor
        // status is something other than non-constructor.
        agent[self].ecmascript_function.constructor_status != ConstructorStatus::NonConstructor
    }
}

impl InternalSlots for ECMAScriptFunction {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject {
        function_create_backing_object(self, agent)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_prototype(agent)
        } else {
            let realm = agent[self].ecmascript_function.realm;
            let intrinsics = agent[realm].intrinsics();
            let proto = match (
                agent[self].ecmascript_function.is_async,
                agent[self].ecmascript_function.is_generator,
            ) {
                (false, false) => intrinsics.function_prototype().into_object(),
                (false, true) => intrinsics.generator_function_prototype().into_object(),
                (true, false) => intrinsics.async_function_prototype().into_object(),
                (true, true) => intrinsics
                    .async_generator_function_prototype()
                    .into_object(),
            };
            Some(proto)
        }
    }
}

impl FunctionInternalProperties for ECMAScriptFunction {
    fn get_name(self, agent: &Agent) -> String {
        agent[self].name.unwrap_or(String::EMPTY_STRING)
    }

    fn get_length(self, agent: &Agent) -> u8 {
        agent[self].length
    }
}

impl InternalMethods for ECMAScriptFunction {
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        function_internal_get_own_property(self, agent, property_key)
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        function_internal_define_own_property(self, agent, property_key, property_descriptor)
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        function_internal_has_property(self, agent, property_key)
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        function_internal_get(self, agent, property_key, receiver)
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        function_internal_set(self, agent, property_key, value, receiver)
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        function_internal_delete(self, agent, property_key)
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        function_internal_own_property_keys(self, agent)
    }

    /// ### [10.2.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-call)
    ///
    /// The \[\[Call]] internal method of an ECMAScript function object `F`
    /// takes arguments `thisArgument` (an ECMAScript language value) and
    /// `argumentsList` (a List of ECMAScript language values) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion.
    fn internal_call(
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
        if agent[self]
            .ecmascript_function
            .constructor_status
            .is_class_constructor()
        {
            // a. Let error be a newly created TypeError object.
            // b. NOTE: error is created in calleeContext with F's associated Realm Record.
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "class constructors must be invoked with 'new'",
            );
            // c. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
            agent.execution_context_stack.pop();
            // d. Return ThrowCompletion(error).
            return Err(error);
        }
        // 5. Perform OrdinaryCallBindThis(F, calleeContext, thisArgument).
        let EnvironmentIndex::Function(local_env) = local_env else {
            panic!("localEnv is not a Function Environment Record");
        };
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

    fn internal_construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        // 2. Let kind be F.[[ConstructorKind]].
        let is_base = !agent[self]
            .ecmascript_function
            .constructor_status
            .is_derived_class();
        // 3. If kind is BASE, then
        let this_argument = if is_base {
            // a. Let thisArgument be ? OrdinaryCreateFromConstructor(newTarget, "%Object.prototype%").
            Some(ordinary_create_from_constructor(
                agent,
                new_target,
                ProtoIntrinsics::Object,
            )?)
        } else {
            None
        };

        // 4. Let calleeContext be PrepareForOrdinaryCall(F, newTarget).
        let callee_context = prepare_for_ordinary_call(agent, self, Some(new_target.into_object()));
        // 7. Let constructorEnv be the LexicalEnvironment of calleeContext.
        let constructor_env = callee_context
            .ecmascript_code
            .as_ref()
            .unwrap()
            .lexical_environment;
        let EnvironmentIndex::Function(constructor_env) = constructor_env else {
            panic!("constructorEnv is not a Function Environment Record");
        };
        // 5. Assert: calleeContext is now the running execution context.
        // assert!(std::ptr::eq(agent.running_execution_context(), callee_context));

        // 6. If kind is base, then
        if is_base {
            // a. Perform OrdinaryCallBindThis(F, calleeContext, thisArgument).
            ordinary_call_bind_this(
                agent,
                self,
                constructor_env,
                this_argument.unwrap().into_value(),
            );
            // b. Let initializeResult be Completion(InitializeInstanceElements(thisArgument, F)).
            // c. If initializeResult is an abrupt completion, then
            //    i. Remove calleeContext from the execution context stack and
            //       restore callerContext as the running execution context.
            //    ii. Return ? initializeResult.
            // TODO: Classes.
        }

        // 8. Let result be Completion(OrdinaryCallEvaluateBody(F, argumentsList)).
        let result = ordinary_call_evaluate_body(agent, self, arguments_list);
        // 9. Remove calleeContext from the execution context stack and restore
        //    callerContext as the running execution context.
        agent.execution_context_stack.pop();
        // 10. If result is a return completion, then
        // 11. Else,
        //   a. ReturnIfAbrupt(result).
        let value = result?;
        // 10. If result is a return completion, then
        //   a. If result.[[Value]] is an Object, return result.[[Value]].
        if let Ok(value) = Object::try_from(value) {
            Ok(value)
        } else
        //   b. If kind is base, return thisArgument.
        if is_base {
            Ok(this_argument.unwrap())
        } else
        //   c. If result.[[Value]] is not undefined, throw a TypeError exception.
        if !value.is_undefined() {
            let message = format!(
                "derived class constructor returned invalid value {}",
                value.string_repr(agent).as_str(agent)
            );
            let message = String::from_string(agent, message);
            Err(agent.throw_exception_with_message(ExceptionType::TypeError, message))
        } else {
            // 12. Let thisBinding be ? constructorEnv.GetThisBinding().
            // 13. Assert: thisBinding is an Object.
            let Ok(this_binding) = Object::try_from(constructor_env.get_this_binding(agent)?)
            else {
                unreachable!();
            };

            // 14. Return thisBinding.
            Ok(this_binding)
        }
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
    let ecmascript_function_object = &agent[f].ecmascript_function;
    let private_environment = ecmascript_function_object.private_environment;
    let is_strict_mode = ecmascript_function_object.strict;
    let script_or_module = ecmascript_function_object.script_or_module;
    let source_code = ecmascript_function_object.source_code;
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
            is_strict_mode,
            source_code,
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
    local_env: FunctionEnvironmentIndex,
    this_argument: Value,
) {
    let function_heap_data = &agent[f].ecmascript_function;
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
    let function_heap_data = &agent[function_object].ecmascript_function;
    let heap_data = function_heap_data;
    match (heap_data.is_generator, heap_data.is_async) {
        (false, true) => {
            // AsyncFunctionBody : FunctionBody
            // 1. Return ? EvaluateAsyncFunctionBody of AsyncFunctionBody with arguments functionObject and argumentsList.
            // AsyncConciseBody : ExpressionBody
            // 1. Return ? EvaluateAsyncConciseBody of AsyncConciseBody with arguments functionObject and argumentsList.
            Ok(evaluate_async_function_body(agent, function_object, arguments_list).into_value())
        }
        (false, false) => {
            // SAFETY: AS the ECMAScriptFunction is alive in the heap, its referred
            // SourceCode must be as well. Thus the Allocator is live as well, and no
            // other references to it can exist.
            if unsafe { heap_data.ecmascript_code.as_ref() }
                .statements
                .is_empty()
                && unsafe { heap_data.formal_parameters.as_ref() }.is_simple_parameter_list()
            {
                // Optimisation: Empty body and only simple parameters means no code will effectively run.
                return Ok(Value::Undefined);
            }
            // FunctionBody : FunctionStatementList
            // 1. Return ? EvaluateFunctionBody of FunctionBody with arguments functionObject and argumentsList.
            // ConciseBody : ExpressionBody
            // 1. Return ? EvaluateConciseBody of ConciseBody with arguments functionObject and argumentsList.
            evaluate_function_body(agent, function_object, arguments_list)
        }
        (true, false) => {
            // GeneratorBody : FunctionBody
            // 1. Return ? EvaluateGeneratorBody of GeneratorBody with arguments functionObject and argumentsList.
            evaluate_generator_body(agent, function_object, arguments_list)
        }
        // AsyncGeneratorBody : FunctionBody
        // 1. Return ? EvaluateAsyncGeneratorBody of AsyncGeneratorBody with arguments functionObject and argumentsList.
        _ => todo!(),
    }

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
) -> ECMAScriptFunction {
    let (source_code, outer_env_is_strict) = if let Some(source_code) = params.source_code {
        (source_code, false)
    } else {
        let running_ecmascript_code = &agent.running_execution_context().ecmascript_code.unwrap();
        (
            running_ecmascript_code.source_code,
            running_ecmascript_code.is_strict_mode,
        )
    };
    // 7. If the source text matched by Body is strict mode code, let Strict be true; else let Strict be false.
    let strict = outer_env_is_strict || params.body.has_use_strict_directive();

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
        formal_parameters: NonNull::from(unsafe {
            std::mem::transmute::<&FormalParameters<'program>, &FormalParameters<'static>>(
                params.parameters_list,
            )
        }),
        // 6. Set F.[[ECMAScriptCode]] to Body.
        // SAFETY: Same as above: Self-referential reference to ScriptOrModule.
        ecmascript_code: NonNull::from(unsafe {
            std::mem::transmute::<&FunctionBody<'program>, &FunctionBody<'static>>(params.body)
        }),
        is_concise_arrow_function: params.is_concise_arrow_function,
        is_async: params.is_async,
        is_generator: params.is_generator,
        // 12. Set F.[[IsClassConstructor]] to false.
        constructor_status: ConstructorStatus::NonConstructor,
        // 16. Set F.[[Realm]] to the current Realm Record.
        realm: agent.current_realm_id(),
        // 15. Set F.[[ScriptOrModule]] to GetActiveScriptOrModule().
        script_or_module: get_active_script_or_module(agent).unwrap(),
        // 9. If thisMode is LEXICAL-THIS, set F.[[ThisMode]] to LEXICAL.
        // 10. Else if Strict is true, set F.[[ThisMode]] to STRICT.
        // 11. Else, set F.[[ThisMode]] to GLOBAL.
        this_mode: if params.lexical_this {
            ThisMode::Lexical
        } else if strict {
            ThisMode::Strict
        } else {
            ThisMode::Global
        },
        // 8. Set F.[[Strict]] to Strict.
        strict,
        // 17. Set F.[[HomeObject]] to undefined.
        home_object: None,
        // 4. Set F.[[SourceText]] to sourceText.
        source_text: params.source_text,
        source_code,
    };

    let mut function = ECMAScriptFunctionHeapData {
        object_index: None,
        length: 0,
        ecmascript_function,
        compiled_bytecode: None,
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
                    .create_object_with_prototype(function_prototype, &[]),
            );
        }
    }

    // 18. Set F.[[Fields]] to a new empty List.
    // 19. Set F.[[PrivateMethods]] to a new empty List.
    // 20. Set F.[[ClassFieldInitializerName]] to EMPTY.
    // 21. Let len be the ExpectedArgumentCount of ParameterList.
    let len = params
        .parameters_list
        .items
        .iter()
        .filter(|par| !par.pattern.kind.is_assignment_pattern())
        .count();
    // 22. Perform SetFunctionLength(F, len).
    set_ecmascript_function_length(agent, &mut function, len).unwrap();
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
    function: impl IntoFunction,
    writable_prototype: Option<bool>,
    prototype: Option<Object>,
) {
    // 4. If writablePrototype is not present, set writablePrototype to true.
    let writable_prototype = writable_prototype.unwrap_or(true);
    match function.into_function() {
        Function::BoundFunction(_) => unreachable!(),
        // 1. If F is an ECMAScript function object, then
        Function::ECMAScriptFunction(idx) => {
            let data = &mut agent[idx];
            // a. Assert: IsConstructor(F) is false.
            debug_assert!(!data.ecmascript_function.constructor_status.is_constructor());
            // b. Assert: F is an extensible object that does not have a "prototype" own property.
            // c. Set F.[[Construct]] to the definition specified in 10.2.2.
            // 3. Set F.[[ConstructorKind]] to BASE.
            data.ecmascript_function.constructor_status = ConstructorStatus::ConstructorFunction;
        }
        Function::BuiltinFunction(_) => {
            // 2. Else,
            // a. Set F.[[Construct]] to the definition specified in 10.3.2.
        }
        Function::BuiltinGeneratorFunction => todo!(),
        Function::BuiltinConstructorFunction(_) => unreachable!(),
        Function::BuiltinPromiseResolvingFunction(_) => todo!(),
        Function::BuiltinPromiseCollectorFunction => todo!(),
        Function::BuiltinProxyRevokerFunction => todo!(),
    }
    // 5. If prototype is not present, then
    let prototype = prototype.unwrap_or_else(|| {
        // a. Set prototype to OrdinaryObjectCreate(%Object.prototype%).
        let prototype =
            ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None);
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
        function.into_object(),
        key,
        PropertyDescriptor {
            value: Some(prototype.into_value()),
            writable: Some(writable_prototype),
            enumerable: Some(false),
            configurable: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    // 7. Return UNUSED.
}

/// ### [10.2.7 MakeMethod ( F, homeObject )](https://tc39.es/ecma262/#sec-makemethod)
///
/// The abstract operation MakeMethod takes arguments F (an ECMAScript function
/// object) and homeObject (an Object) and returns unused. It configures F as a
/// method.
#[inline]
pub(crate) fn make_method(agent: &mut Agent, f: ECMAScriptFunction, home_object: Object) {
    // 1. Set F.[[HomeObject]] to homeObject.
    agent[f].ecmascript_function.home_object = Some(home_object);
    // 2. Return unused.
}

/// ### [10.2.9 SetFunctionName ( F, name \[ , prefix \] )](https://tc39.es/ecma262/#sec-setfunctionname)
/// The abstract operation SetFunctionName takes arguments F (a function
/// object) and name (a property key or Private Name) and optional argument
/// prefix (a String) and returns UNUSED. It adds a "name" property to F.
pub(crate) fn set_function_name(
    agent: &mut Agent,
    function: impl IntoFunction,
    name: PropertyKey,
    _prefix: Option<String>,
) {
    // 2. If name is a Symbol, then
    let name: String = match name {
        PropertyKey::Symbol(idx) => {
            // a. Let description be name's [[Description]] value.
            // b. If description is undefined, set name to the empty String.
            // c. Else, set name to the string-concatenation of "[", description, and "]".
            let symbol_data = &agent[idx];
            symbol_data
                .descriptor
                .map_or(String::EMPTY_STRING, |descriptor| {
                    let descriptor = descriptor.as_str(agent);
                    String::from_string(agent, format!("[{}]", descriptor))
                })
        }
        // TODO: Private Name
        // 3. Else if name is a Private Name, then
        // a. Set name to name.[[Description]].
        PropertyKey::Integer(integer) => {
            String::from_string(agent, format!("{}", integer.into_i64()))
        }
        PropertyKey::SmallString(str) => str.into(),
        PropertyKey::String(str) => str.into(),
    };
    // 5. If prefix is present, then
    // a. Set name to the string-concatenation of prefix, the code unit 0x0020 (SPACE), and name.
    // TODO: Handle prefixing

    match function.into_function() {
        Function::BoundFunction(idx) => {
            let function = &mut agent[idx];
            // Note: It's possible that the bound function targeted a function
            // with a non-default prototype. In that case, object_index is
            // already set.
            assert!(function.name.is_none());
            function.name = Some(name);
        }
        Function::BuiltinFunction(_idx) => unreachable!(),
        Function::ECMAScriptFunction(idx) => {
            let function = &mut agent[idx];
            // 1. Assert: F is an extensible object that does not have a "name" own property.
            assert!(function.name.is_none());
            // 6. Perform ! DefinePropertyOrThrow(F, "name", PropertyDescriptor { [[Value]]: name, [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
            function.name = Some(name);
            // 7. Return UNUSED.
        }
        Function::BuiltinGeneratorFunction => todo!(),
        Function::BuiltinConstructorFunction(_) => unreachable!(),
        Function::BuiltinPromiseResolvingFunction(_) => todo!(),
        Function::BuiltinPromiseCollectorFunction => todo!(),
        Function::BuiltinProxyRevokerFunction => todo!(),
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
        return Err(agent.throw_exception_with_static_message(
            SyntaxError,
            "Too many arguments in function call (only 255 allowed)",
        ));
    }
    function.length = length as u8;

    // 3. Return unused.
    Ok(())
}

impl HeapMarkAndSweep for ECMAScriptFunction {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.ecmascript_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.ecmascript_functions.shift_index(&mut self.0);
    }
}

impl CreateHeapData<ECMAScriptFunctionHeapData, ECMAScriptFunction> for Heap {
    fn create(&mut self, data: ECMAScriptFunctionHeapData) -> ECMAScriptFunction {
        self.ecmascript_functions.push(Some(data));
        ECMAScriptFunction(ECMAScriptFunctionIndex::last(&self.ecmascript_functions))
    }
}

impl HeapMarkAndSweep for ECMAScriptFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            length: _,
            ecmascript_function,
            compiled_bytecode,
            name,
        } = self;
        let ECMAScriptFunctionObjectHeapData {
            environment,
            private_environment,
            formal_parameters: _,
            ecmascript_code: _,
            is_concise_arrow_function: _,
            is_async: _,
            is_generator: _,
            constructor_status: _,
            realm,
            script_or_module,
            this_mode: _,
            strict: _,
            home_object,
            source_text: _,
            source_code,
        } = ecmascript_function;
        object_index.mark_values(queues);
        compiled_bytecode.mark_values(queues);
        name.mark_values(queues);
        environment.mark_values(queues);
        private_environment.mark_values(queues);
        realm.mark_values(queues);
        script_or_module.mark_values(queues);
        home_object.mark_values(queues);
        source_code.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            length: _,
            ecmascript_function,
            compiled_bytecode,
            name,
        } = self;
        let ECMAScriptFunctionObjectHeapData {
            environment,
            private_environment,
            formal_parameters: _,
            ecmascript_code: _,
            is_concise_arrow_function: _,
            is_async: _,
            is_generator: _,
            constructor_status: _,
            realm,
            script_or_module,
            this_mode: _,
            strict: _,
            home_object,
            source_text: _,
            source_code,
        } = ecmascript_function;
        object_index.sweep_values(compactions);
        compiled_bytecode.sweep_values(compactions);
        name.sweep_values(compactions);
        environment.sweep_values(compactions);
        private_environment.sweep_values(compactions);
        realm.sweep_values(compactions);
        script_or_module.sweep_values(compactions);
        home_object.sweep_values(compactions);
        source_code.sweep_values(compactions);
    }
}
