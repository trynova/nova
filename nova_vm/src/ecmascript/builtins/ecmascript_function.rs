// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{
    ops::{Index, IndexMut},
    ptr::NonNull,
};

use oxc_ast::ast::{FormalParameters, FunctionBody};
use oxc_ecmascript::IsSimpleParameterList;
use oxc_span::Span;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_object,
        execution::{
            Agent, ECMAScriptCodeEvaluationState, Environment, ExecutionContext,
            FunctionEnvironment, JsResult, PrivateEnvironment, ProtoIntrinsics, Realm,
            ThisBindingStatus,
            agent::{
                ExceptionType::{self, SyntaxError},
                get_active_script_or_module,
            },
            new_function_environment,
        },
        scripts_and_modules::{ScriptOrModule, source_code::SourceCode},
        syntax_directed_operations::function_definitions::{
            evaluate_async_function_body, evaluate_function_body, evaluate_generator_body,
        },
        types::{
            BUILTIN_STRING_MEMORY, ECMAScriptFunctionHeapData, Function,
            FunctionInternalProperties, InternalMethods, InternalSlots, IntoFunction, IntoObject,
            IntoValue, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value,
            function_create_backing_object, function_internal_define_own_property,
            function_internal_delete, function_internal_get, function_internal_get_own_property,
            function_internal_has_property, function_internal_own_property_keys,
            function_internal_set,
        },
    },
    engine::{Executable, rootable::Scopable},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::ECMAScriptFunctionIndex,
    },
};
use crate::{
    ecmascript::{
        syntax_directed_operations::function_definitions::evaluate_async_generator_body,
        types::{function_try_get, function_try_has_property, function_try_set},
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
};

use super::{
    ArgumentsList,
    ordinary::{ordinary_create_from_constructor, ordinary_object_create_with_intrinsics},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ECMAScriptFunction<'a>(ECMAScriptFunctionIndex<'a>);

impl<'a> From<ECMAScriptFunction<'a>> for ECMAScriptFunctionIndex<'a> {
    fn from(val: ECMAScriptFunction<'a>) -> Self {
        val.0
    }
}

impl<'a> From<ECMAScriptFunctionIndex<'a>> for ECMAScriptFunction<'a> {
    fn from(value: ECMAScriptFunctionIndex<'a>) -> Self {
        Self(value)
    }
}

impl<'a> TryFrom<Value<'a>> for ECMAScriptFunction<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        if let Value::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Object<'a>> for ECMAScriptFunction<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        if let Object::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Function<'a>> for ECMAScriptFunction<'a> {
    type Error = ();

    fn try_from(value: Function<'a>) -> Result<Self, Self::Error> {
        if let Function::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl<'a> From<ECMAScriptFunction<'a>> for Value<'a> {
    fn from(value: ECMAScriptFunction<'a>) -> Self {
        Value::ECMAScriptFunction(value)
    }
}

impl<'a> From<ECMAScriptFunction<'a>> for Object<'a> {
    fn from(value: ECMAScriptFunction<'a>) -> Self {
        Object::ECMAScriptFunction(value)
    }
}

impl<'a> From<ECMAScriptFunction<'a>> for Function<'a> {
    fn from(val: ECMAScriptFunction<'a>) -> Self {
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
pub(crate) struct ECMAScriptFunctionObjectHeapData<'a> {
    /// \[\[Environment]]
    pub environment: Environment<'a>,

    /// \[\[PrivateEnvironment]]
    pub private_environment: Option<PrivateEnvironment<'a>>,

    /// \[\[FormalParameters]]
    ///
    /// SAFETY: SourceCode owns the Allocator into which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub formal_parameters: NonNull<FormalParameters<'a>>,

    /// \[\[ECMAScriptCode]]
    ///
    /// SAFETY: SourceCode owns the Allocator into which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub ecmascript_code: NonNull<FunctionBody<'a>>,

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
    pub realm: Realm<'a>,

    /// \[\[ScriptOrModule]]
    pub script_or_module: ScriptOrModule<'a>,

    /// \[\[ThisMode]]
    pub this_mode: ThisMode,

    /// \[\[Strict]]
    pub strict: bool,

    /// \[\[HomeObject]]
    pub home_object: Option<Object<'a>>,

    ///  \[\[SourceText]]
    pub source_text: Span,

    /// \[\[SourceCode]]
    ///
    /// Nova specific addition: This SourceCode is where \[\[SourceText]]
    /// refers to.
    pub source_code: SourceCode<'a>,
    // TODO: [[Fields]],  [[PrivateMethods]], [[ClassFieldInitializerName]]
}

pub(crate) struct OrdinaryFunctionCreateParams<'agent, 'program, 'gc> {
    pub function_prototype: Option<Object<'gc>>,
    pub source_code: Option<SourceCode<'gc>>,
    pub source_text: Span,
    pub parameters_list: &'agent FormalParameters<'program>,
    pub body: &'agent FunctionBody<'program>,
    pub is_concise_arrow_function: bool,
    pub is_async: bool,
    pub is_generator: bool,
    pub lexical_this: bool,
    pub env: Environment<'gc>,
    pub private_env: Option<PrivateEnvironment<'gc>>,
}

impl Index<ECMAScriptFunction<'_>> for Agent {
    type Output = ECMAScriptFunctionHeapData<'static>;

    fn index(&self, index: ECMAScriptFunction) -> &Self::Output {
        &self.heap.ecmascript_functions[index]
    }
}

impl IndexMut<ECMAScriptFunction<'_>> for Agent {
    fn index_mut(&mut self, index: ECMAScriptFunction) -> &mut Self::Output {
        &mut self.heap.ecmascript_functions[index]
    }
}

impl Index<ECMAScriptFunction<'_>> for Vec<Option<ECMAScriptFunctionHeapData<'static>>> {
    type Output = ECMAScriptFunctionHeapData<'static>;

    fn index(&self, index: ECMAScriptFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("ECMAScriptFunction out of bounds")
            .as_ref()
            .expect("ECMAScriptFunction slot empty")
    }
}

impl IndexMut<ECMAScriptFunction<'_>> for Vec<Option<ECMAScriptFunctionHeapData<'static>>> {
    fn index_mut(&mut self, index: ECMAScriptFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ECMAScriptFunction out of bounds")
            .as_mut()
            .expect("ECMAScriptFunction slot empty")
    }
}

impl<'a> ECMAScriptFunction<'a> {
    pub(crate) const fn _def() -> Self {
        ECMAScriptFunction(ECMAScriptFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn get_executable(self, agent: &Agent) -> Executable<'a> {
        agent[self].compiled_bytecode.unwrap().unbind()
    }

    pub fn is_constructor(self, agent: &Agent) -> bool {
        // An ECMAScript function has the [[Construct]] slot if its constructor
        // status is something other than non-constructor.
        agent[self].ecmascript_function.constructor_status != ConstructorStatus::NonConstructor
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ECMAScriptFunction<'_> {
    type Of<'a> = ECMAScriptFunction<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> InternalSlots<'a> for ECMAScriptFunction<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        function_create_backing_object(self, agent)
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
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

impl<'a> FunctionInternalProperties<'a> for ECMAScriptFunction<'a> {
    fn get_name(self, agent: &Agent) -> String<'static> {
        agent[self].name.unwrap_or(String::EMPTY_STRING)
    }

    fn get_length(self, agent: &Agent) -> u8 {
        agent[self].length
    }
}

impl<'a> InternalMethods<'a> for ECMAScriptFunction<'a> {
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        TryResult::Continue(function_internal_get_own_property(
            self,
            agent,
            property_key,
            gc,
        ))
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        TryResult::Continue(function_internal_define_own_property(
            self,
            agent,
            property_key,
            property_descriptor,
            gc,
        ))
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        function_try_has_property(self, agent, property_key, gc)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        function_internal_has_property(self, agent, property_key, gc)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
        function_try_get(self, agent, property_key, receiver, gc)
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        function_internal_get(self, agent, property_key, receiver, gc)
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        function_try_set(self, agent, property_key, value, receiver, gc)
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        function_internal_set(self, agent, property_key, value, receiver, gc)
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        TryResult::Continue(function_internal_delete(self, agent, property_key, gc))
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        TryResult::Continue(function_internal_own_property_keys(self, agent, gc))
    }

    /// ### [10.2.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-call)
    ///
    /// The \[\[Call]] internal method of an ECMAScript function object `F`
    /// takes arguments `thisArgument` (an ECMAScript language value) and
    /// `argumentsList` (a List of ECMAScript language values) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion.
    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let f = self.bind(gc.nogc());
        let arguments_list = arguments_list.bind(gc.nogc());

        // 1. Let callerContext be the running execution context.
        let _ = agent.running_execution_context();
        // 2. Let calleeContext be PrepareForOrdinaryCall(F, undefined).
        let callee_context = prepare_for_ordinary_call(agent, f, None, gc.nogc());
        // This is step 4. or OrdinaryCallBindThis:
        // "Let localEnv be the LexicalEnvironment of calleeContext."
        let local_env = callee_context
            .ecmascript_code
            .as_ref()
            .unwrap()
            .lexical_environment
            .bind(gc.nogc());
        // 3. Assert: calleeContext is now the running execution context.
        // assert!(core::ptr::eq(agent.running_execution_context(), callee_context));
        // 4. If F.[[IsClassConstructor]] is true, then
        if agent[f]
            .ecmascript_function
            .constructor_status
            .is_class_constructor()
        {
            // a. Let error be a newly created TypeError object.
            // b. NOTE: error is created in calleeContext with F's associated Realm Record.
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "class constructors must be invoked with 'new'",
                gc.nogc(),
            );
            // c. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
            agent.pop_execution_context();
            // d. Return ThrowCompletion(error).
            return Err(error.unbind());
        }
        let Environment::Function(local_env) = local_env else {
            panic!("localEnv is not a Function Environment Record");
        };
        // 5. Perform OrdinaryCallBindThis(F, calleeContext, thisArgument).
        ordinary_call_bind_this(agent, f, local_env, this_argument, gc.nogc());
        // 6. Let result be Completion(OrdinaryCallEvaluateBody(F, argumentsList)).
        let result = ordinary_call_evaluate_body(agent, f.unbind(), arguments_list.unbind(), gc);
        // 7. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
        // NOTE: calleeContext must not be destroyed if it is suspended and retained for later resumption by an accessible Generator.
        agent.pop_execution_context();
        // 8. If result is a return completion, return result.[[Value]].
        // 9. ReturnIfAbrupt(result).
        // 10. Return undefined.
        result
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments: ArgumentsList,
        new_target: Function,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        let mut self_fn = self.bind(gc.nogc());
        let mut new_target = new_target.bind(gc.nogc());
        let mut arguments_list = arguments.bind(gc.nogc());
        // 2. Let kind be F.[[ConstructorKind]].
        let is_base = !agent[self_fn]
            .ecmascript_function
            .constructor_status
            .is_derived_class();
        // 3. If kind is BASE, then
        let this_argument = if is_base {
            let scoped_self_fn = self_fn.scope(agent, gc.nogc());
            let scoped_new_target = new_target.scope(agent, gc.nogc());
            // a. Let thisArgument be ? OrdinaryCreateFromConstructor(newTarget, "%Object.prototype%").
            let unbound_new_target = new_target.unbind();
            let mut args = arguments_list.unbind();
            let this_argument = args
                .with_scoped(
                    agent,
                    |agent, _, gc| {
                        ordinary_create_from_constructor(
                            agent,
                            unbound_new_target,
                            ProtoIntrinsics::Object,
                            gc,
                        )
                    },
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
            self_fn = scoped_self_fn.get(agent).bind(gc.nogc());
            new_target = scoped_new_target.get(agent).bind(gc.nogc());
            arguments_list = args.bind(gc.nogc());
            Some(this_argument)
        } else {
            None
        };

        // 4. Let calleeContext be PrepareForOrdinaryCall(F, newTarget).
        let callee_context =
            prepare_for_ordinary_call(agent, self_fn, Some(new_target.into_object()), gc.nogc());
        // 7. Let constructorEnv be the LexicalEnvironment of calleeContext.
        let constructor_env = callee_context
            .ecmascript_code
            .as_ref()
            .unwrap()
            .lexical_environment
            .bind(gc.nogc());
        let Environment::Function(constructor_env) = constructor_env else {
            panic!("constructorEnv is not a Function Environment Record");
        };
        // 5. Assert: calleeContext is now the running execution context.
        // assert!(core::ptr::eq(agent.running_execution_context(), callee_context));

        // 6. If kind is base, then
        if is_base {
            // a. Perform OrdinaryCallBindThis(F, calleeContext, thisArgument).
            ordinary_call_bind_this(
                agent,
                self,
                constructor_env,
                this_argument.unwrap().into_value(),
                gc.nogc(),
            );
            // b. Let initializeResult be Completion(InitializeInstanceElements(thisArgument, F)).
            // c. If initializeResult is an abrupt completion, then
            //    i. Remove calleeContext from the execution context stack and
            //       restore callerContext as the running execution context.
            //    ii. Return ? initializeResult.
            // TODO: Classes.
        }

        let scoped_constructor_env = constructor_env.scope(agent, gc.nogc());
        let scoped_this_argument = this_argument.map(|f| f.scope(agent, gc.nogc()));

        // 8. Let result be Completion(OrdinaryCallEvaluateBody(F, argumentsList)).
        let result = ordinary_call_evaluate_body(
            agent,
            self_fn.unbind(),
            arguments_list.unbind(),
            gc.reborrow(),
        );
        // 9. Remove calleeContext from the execution context stack and restore
        //    callerContext as the running execution context.
        agent.pop_execution_context();
        // 10. If result is a return completion, then
        // 11. Else,
        //   a. ReturnIfAbrupt(result).
        let value = result.unbind()?.bind(gc.nogc());
        // 10. If result is a return completion, then
        //   a. If result.[[Value]] is an Object, return result.[[Value]].
        if let Ok(value) = Object::try_from(value) {
            Ok(value.unbind())
        } else
        //   b. If kind is base, return thisArgument.
        if is_base {
            Ok(scoped_this_argument.unwrap().get(agent))
        } else
        //   c. If result.[[Value]] is not undefined, throw a TypeError exception.
        if !value.is_undefined() {
            let message = format!(
                "derived class constructor returned invalid value {}",
                value
                    .unbind()
                    .string_repr(agent, gc.reborrow())
                    .to_string_lossy(agent)
            );
            let message = String::from_string(agent, message, gc.nogc());
            Err(agent.throw_exception_with_message(
                ExceptionType::TypeError,
                message.unbind(),
                gc.into_nogc(),
            ))
        } else {
            // 12. Let thisBinding be ? constructorEnv.GetThisBinding().
            // 13. Assert: thisBinding is an Object.
            let Ok(this_binding) = Object::try_from(
                scoped_constructor_env
                    .get(agent)
                    .get_this_binding(agent, gc.into_nogc())?,
            ) else {
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
pub(crate) fn prepare_for_ordinary_call<'a>(
    agent: &'a mut Agent,
    f: ECMAScriptFunction,
    new_target: Option<Object>,
    gc: NoGcScope,
) -> &'a ExecutionContext {
    let f = f.bind(gc);
    let new_target = new_target.bind(gc);
    let ecmascript_function_object = &agent[f].ecmascript_function;
    let private_environment = ecmascript_function_object.private_environment.bind(gc);
    let is_strict_mode = ecmascript_function_object.strict;
    let script_or_module = ecmascript_function_object.script_or_module;
    let source_code = ecmascript_function_object.source_code;
    // 1. Let callerContext be the running execution context.
    let _caller_context = agent.running_execution_context();
    // 4. Let calleeRealm be F.[[Realm]].
    let callee_realm = ecmascript_function_object.realm;
    // 7. Let localEnv be NewFunctionEnvironment(F, newTarget).
    let local_env = new_function_environment(agent, f, new_target, gc);
    // 2. Let calleeContext be a new ECMAScript code execution context.
    let callee_context = ExecutionContext {
        // 8. Set the LexicalEnvironment of calleeContext to localEnv.
        // 9. Set the VariableEnvironment of calleeContext to localEnv.
        // 10. Set the PrivateEnvironment of calleeContext to F.[[PrivateEnvironment]].
        ecmascript_code: Some(ECMAScriptCodeEvaluationState {
            lexical_environment: Environment::Function(local_env.unbind()),
            variable_environment: Environment::Function(local_env.unbind()),
            private_environment: private_environment.unbind(),
            is_strict_mode,
            source_code,
        }),
        // 3. Set the Function of calleeContext to F.
        function: Some(f.into_function().unbind()),
        // 5. Set the Realm of calleeContext to calleeRealm.
        realm: callee_realm,
        // 6. Set the ScriptOrModule of calleeContext to F.[[ScriptOrModule]].
        script_or_module: Some(script_or_module),
    };
    // 11. If callerContext is not already suspended, suspend callerContext.
    // 12. Push calleeContext onto the execution context stack; calleeContext is now the running execution context.
    agent.push_execution_context(callee_context);
    // 13. NOTE: Any exception objects produced after this point are associated with calleeRealm.
    // 14. Return calleeContext.
    agent.running_execution_context()
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
    local_env: FunctionEnvironment,
    this_argument: Value,
    gc: NoGcScope,
) {
    let function_heap_data = &agent[f].ecmascript_function;
    // 1. Let thisMode be F.[[ThisMode]].
    let this_mode = function_heap_data.this_mode;
    // 2. If thisMode is LEXICAL, return UNUSED.
    if this_mode == ThisMode::Lexical {
        return;
    }
    // 3. Let calleeRealm be F.[[Realm]].
    let callee_realm = function_heap_data.realm.bind(gc);
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
            let global_env = agent.get_realm_record_by_id(callee_realm).global_env;
            // ii. Assert: globalEnv is a Global Environment Record.
            let global_env = global_env.unwrap();
            // iii. Let thisValue be globalEnv.[[GlobalThisValue]].
            global_env.get_this_binding(agent).into_value()
        } else {
            // b. Else,
            // i. Let thisValue be ! ToObject(thisArgument).
            to_object(agent, this_argument, gc).unwrap().into_value()
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
    local_env.bind_this_value(agent, this_value, gc).unwrap();
    // 10. Return UNUSED.
}

/// ### [10.2.1.3 Runtime Semantics: EvaluateBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluatebody)
///
/// The syntax-directed operation EvaluateBody takes arguments `functionObject`
/// (an ECMAScript function object) and `argumentsList` (a List of ECMAScript
/// language values) and returns either a normal completion containing an
/// ECMAScript language value or an abrupt completion.
pub(crate) fn evaluate_body<'gc>(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let function_object = function_object.bind(gc.nogc());
    let function_heap_data = &agent[function_object];
    let ecmascript_function_object = &function_heap_data.ecmascript_function;
    match (
        ecmascript_function_object.is_generator,
        ecmascript_function_object.is_async,
    ) {
        (false, true) => {
            // AsyncFunctionBody : FunctionBody
            // 1. Return ? EvaluateAsyncFunctionBody of AsyncFunctionBody with arguments functionObject and argumentsList.
            // AsyncConciseBody : ExpressionBody
            // 1. Return ? EvaluateAsyncConciseBody of AsyncConciseBody with arguments functionObject and argumentsList.
            Ok(
                evaluate_async_function_body(agent, function_object.unbind(), arguments_list, gc)
                    .into_value(),
            )
        }
        (false, false) => {
            // SAFETY: AS the ECMAScriptFunction is alive in the heap, its referred
            // SourceCode must be as well. Thus the Allocator is live as well, and no
            // other references to it can exist.
            if function_heap_data.compiled_bytecode.is_none()
                && unsafe { ecmascript_function_object.ecmascript_code.as_ref() }
                    .statements
                    .is_empty()
                && unsafe { ecmascript_function_object.formal_parameters.as_ref() }
                    .is_simple_parameter_list()
            {
                // Optimisation: Empty body and only simple parameters means no code will effectively run.
                return Ok(Value::Undefined);
            }
            // FunctionBody : FunctionStatementList
            // 1. Return ? EvaluateFunctionBody of FunctionBody with arguments functionObject and argumentsList.
            // ConciseBody : ExpressionBody
            // 1. Return ? EvaluateConciseBody of ConciseBody with arguments functionObject and argumentsList.
            evaluate_function_body(agent, function_object.unbind(), arguments_list, gc)
        }
        (true, false) => {
            // GeneratorBody : FunctionBody
            // 1. Return ? EvaluateGeneratorBody of GeneratorBody with arguments functionObject and argumentsList.
            evaluate_generator_body(agent, function_object.unbind(), arguments_list, gc)
        }
        // AsyncGeneratorBody : FunctionBody
        // 1. Return ? EvaluateAsyncGeneratorBody of AsyncGeneratorBody with arguments functionObject and argumentsList.
        _ => evaluate_async_generator_body(agent, function_object.unbind(), arguments_list, gc),
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
pub(crate) fn ordinary_call_evaluate_body<'gc>(
    agent: &mut Agent,
    f: ECMAScriptFunction,
    arguments_list: ArgumentsList,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    // 1. Return ? EvaluateBody of F.[[ECMAScriptCode]] with arguments F and argumentsList.
    evaluate_body(agent, f, arguments_list, gc)
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
pub(crate) fn ordinary_function_create<'agent, 'program, 'gc>(
    agent: &'agent mut Agent,
    params: OrdinaryFunctionCreateParams<'agent, 'program, 'gc>,
    gc: NoGcScope<'gc, '_>,
) -> ECMAScriptFunction<'gc> {
    let (source_code, outer_env_is_strict) = if let Some(source_code) = params.source_code {
        (source_code, false)
    } else {
        let running_ecmascript_code = &agent.running_execution_context().ecmascript_code.unwrap();
        (
            running_ecmascript_code.source_code.bind(gc),
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
        environment: params.env.unbind(),
        // 14. Set F.[[PrivateEnvironment]] to privateEnv.
        private_environment: params.private_env.unbind(),
        // 5. Set F.[[FormalParameters]] to ParameterList.
        // SAFETY: The reference to FormalParameters points to ScriptOrModule
        // and is valid until it gets dropped. Our GC keeps ScriptOrModule
        // alive until this ECMAScriptFunction gets dropped, hence the 'static
        // lifetime here is justified.
        formal_parameters: NonNull::from(unsafe {
            core::mem::transmute::<&FormalParameters<'program>, &FormalParameters<'static>>(
                params.parameters_list,
            )
        }),
        // 6. Set F.[[ECMAScriptCode]] to Body.
        // SAFETY: Same as above: Self-referential reference to ScriptOrModule.
        ecmascript_code: NonNull::from(unsafe {
            core::mem::transmute::<&FunctionBody<'program>, &FunctionBody<'static>>(params.body)
        }),
        is_concise_arrow_function: params.is_concise_arrow_function,
        is_async: params.is_async,
        is_generator: params.is_generator,
        // 12. Set F.[[IsClassConstructor]] to false.
        constructor_status: ConstructorStatus::NonConstructor,
        // 16. Set F.[[Realm]] to the current Realm Record.
        realm: agent.current_realm(gc),
        // 15. Set F.[[ScriptOrModule]] to GetActiveScriptOrModule().
        script_or_module: get_active_script_or_module(agent, gc).unwrap().unbind(),
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
        source_code: source_code.unbind(),
    };

    let mut function = ECMAScriptFunctionHeapData {
        object_index: None,
        length: 0,
        ecmascript_function,
        compiled_bytecode: None,
        name: None,
    };
    if let Some(function_prototype) = params.function_prototype
        && function_prototype
            != agent
                .current_realm_record()
                .intrinsics()
                .function_prototype()
                .into_object()
    {
        function.object_index = Some(OrdinaryObject::create_object(
            agent,
            Some(function_prototype),
            &[],
        ));
    }

    // 18. Set F.[[Fields]] to a new empty List.
    // 19. Set F.[[PrivateMethods]] to a new empty List.
    // 20. Set F.[[ClassFieldInitializerName]] to EMPTY.
    // 21. Let len be the ExpectedArgumentCount of ParameterList.
    let len = expected_arguments_count(params.parameters_list);
    // 22. Perform SetFunctionLength(F, len).
    set_ecmascript_function_length(agent, &mut function, len, gc).unwrap();
    // 23. Return F.
    agent.heap.create(function)
}

/// ### [15.1.5 Static Semantics: ExpectedArgumentCount](https://tc39.es/ecma262/#sec-static-semantics-expectedargumentcount)
fn expected_arguments_count(params: &FormalParameters) -> usize {
    // FormalParameterList : FormalParameterList , FormalParameter

    // 1. Let count be the ExpectedArgumentCount of FormalParameterList.
    let mut count = 0;
    // 2. If HasInitializer of FormalParameterList is true or HasInitializer of
    //    FormalParameter is true, return count.
    // 3. Return count + 1.
    for param in params.items.iter() {
        if param.pattern.kind.is_assignment_pattern() {
            // FormalParameterList : FormalParameter
            // 1. If HasInitializer of FormalParameter is true, return 0.
            break;
        }
        count += 1;
    }
    count
}

/// ### [10.2.5 MakeConstructor ( F \[ , writablePrototype \[ , prototype \] \] )](https://tc39.es/ecma262/#sec-makeconstructor)
/// The abstract operation MakeConstructor takes argument F (an ECMAScript
/// function object or a built-in function object) and optional arguments
/// writablePrototype (a Boolean) and prototype (an Object) and returns
/// UNUSED. It converts F into a constructor.
pub(crate) fn make_constructor<'a>(
    agent: &mut Agent,
    function: impl IntoFunction<'a> + InternalMethods<'a>,
    writable_prototype: Option<bool>,
    prototype: Option<OrdinaryObject>,
    gc: NoGcScope,
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
        Function::BuiltinGeneratorFunction
        | Function::BuiltinConstructorFunction(_)
        | Function::BuiltinPromiseResolvingFunction(_)
        | Function::BuiltinPromiseCollectorFunction
        | Function::BuiltinProxyRevokerFunction => unreachable!(),
    }
    // 5. If prototype is not present, then
    let prototype = prototype.unwrap_or_else(|| {
        // a. Set prototype to OrdinaryObjectCreate(%Object.prototype%).
        let prototype = OrdinaryObject::try_from(ordinary_object_create_with_intrinsics(
            agent,
            Some(ProtoIntrinsics::Object),
            None,
            gc,
        ))
        .unwrap();
        // b. Perform ! DefinePropertyOrThrow(prototype, "constructor", PropertyDescriptor { [[Value]]: F, [[Writable]]: writablePrototype, [[Enumerable]]: false, [[Configurable]]: true }).
        let key = PropertyKey::from(BUILTIN_STRING_MEMORY.constructor);
        prototype.property_storage().set(
            agent,
            prototype.into_object(),
            key,
            PropertyDescriptor {
                value: Some(function.into_value().unbind()),
                writable: Some(writable_prototype),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            },
            gc,
        );
        prototype
    });
    // 6. Perform ! DefinePropertyOrThrow(F, "prototype", PropertyDescriptor { [[Value]]: prototype, [[Writable]]: writablePrototype, [[Enumerable]]: false, [[Configurable]]: false }).
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.prototype);
    let backing_object = function
        .get_backing_object(agent)
        .unwrap_or_else(|| function.create_backing_object(agent));
    backing_object.property_storage().set(
        agent,
        function.into_object(),
        key,
        PropertyDescriptor {
            value: Some(prototype.into_value().unbind()),
            writable: Some(writable_prototype),
            enumerable: Some(false),
            configurable: Some(false),
            ..Default::default()
        },
        gc,
    );
    // 7. Return UNUSED.
}

/// ### [10.2.7 MakeMethod ( F, homeObject )](https://tc39.es/ecma262/#sec-makemethod)
///
/// The abstract operation MakeMethod takes arguments F (an ECMAScript function
/// object) and homeObject (an Object) and returns unused. It configures F as a
/// method.
#[inline]
pub(crate) fn make_method(agent: &mut Agent, f: ECMAScriptFunction, home_object: Object) {
    // 1. Assert: homeObject is an ordinary object.
    // 2. Set F.[[HomeObject]] to homeObject.
    agent[f].ecmascript_function.home_object = Some(home_object.unbind());
    // 3. Return unused.
}

pub(crate) enum SetFunctionNamePrefix {
    Get,
    Set,
    Bound,
}

impl SetFunctionNamePrefix {
    fn into_str(self) -> &'static str {
        match self {
            SetFunctionNamePrefix::Get => "get ",
            SetFunctionNamePrefix::Set => "set ",
            SetFunctionNamePrefix::Bound => "bound ",
        }
    }
}

fn prefix_into_str(prefix: Option<SetFunctionNamePrefix>) -> &'static str {
    match prefix {
        Some(p) => p.into_str(),
        None => "",
    }
}

/// ### [10.2.9 SetFunctionName ( F, name \[ , prefix \] )](https://tc39.es/ecma262/#sec-setfunctionname)
/// The abstract operation SetFunctionName takes arguments F (a function
/// object) and name (a property key or Private Name) and optional argument
/// prefix (a String) and returns UNUSED. It adds a "name" property to F.
pub(crate) fn set_function_name<'a>(
    agent: &mut Agent,
    function: impl IntoFunction<'a>,
    name: PropertyKey,
    prefix: Option<SetFunctionNamePrefix>,
    gc: NoGcScope,
) {
    // 2. If name is a Symbol, then
    // 5. If prefix is present, then
    // a. Set name to the string-concatenation of prefix, the code unit 0x0020 (SPACE), and name.
    let name: String = match name {
        PropertyKey::Symbol(idx) => {
            // a. Let description be name's [[Description]] value.
            // b. If description is undefined, set name to the empty String.
            // c. Else, set name to the string-concatenation of "[", description, and "]".
            let symbol_data = &agent[idx];
            symbol_data
                .descriptor
                .map_or(String::EMPTY_STRING, |descriptor| {
                    let descriptor = descriptor.to_string_lossy(agent);
                    String::from_string(
                        agent,
                        format!("{}[{descriptor}]", prefix_into_str(prefix)),
                        gc,
                    )
                })
        }

        PropertyKey::Integer(integer) => String::from_string(
            agent,
            format!("{}{}", prefix_into_str(prefix), integer.into_i64()),
            gc,
        ),
        PropertyKey::SmallString(str) => {
            if let Some(prefix) = prefix {
                String::from_string(
                    agent,
                    format!("{}{}", prefix.into_str(), str.to_string_lossy()),
                    gc,
                )
            } else {
                str.into()
            }
        }
        PropertyKey::String(str) => {
            if let Some(prefix) = prefix {
                String::from_string(
                    agent,
                    format!("{}{}", prefix.into_str(), str.to_string_lossy(agent)),
                    gc,
                )
            } else {
                str.into()
            }
        }
        // 3. Else if name is a Private Name, then
        // a. Set name to name.[[Description]].
        PropertyKey::PrivateName(p) => p
            .get_description(agent, gc)
            .expect("Should always find PrivateName in scope when calling SetFunctionName"),
    };

    match function.into_function() {
        Function::BoundFunction(idx) => {
            let function = &mut agent[idx];
            // Note: It's possible that the bound function targeted a function
            // with a non-default prototype. In that case, object_index is
            // already set.
            assert!(function.name.is_none());
            function.name = Some(name.unbind());
        }
        Function::BuiltinFunction(_idx) => unreachable!(),
        Function::ECMAScriptFunction(idx) => {
            let function = &mut agent[idx];
            // 1. Assert: F is an extensible object that does not have a "name" own property.
            assert!(function.name.is_none());
            // 6. Perform ! DefinePropertyOrThrow(F, "name", PropertyDescriptor { [[Value]]: name, [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
            function.name = Some(name.unbind());
            // 7. Return UNUSED.
        }
        Function::BuiltinGeneratorFunction
        | Function::BuiltinConstructorFunction(_)
        | Function::BuiltinPromiseResolvingFunction(_)
        | Function::BuiltinPromiseCollectorFunction
        | Function::BuiltinProxyRevokerFunction => unreachable!(),
    }
}

/// ### [10.2.10 SetFunctionLength ( F, length )](https://tc39.es/ecma262/#sec-setfunctionlength)
fn set_ecmascript_function_length<'a>(
    agent: &mut Agent,
    function: &mut ECMAScriptFunctionHeapData,
    length: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // TODO: 1. Assert: F is an extensible object that does not have a "length" own property.

    // 2. Perform ! DefinePropertyOrThrow(F, "length", PropertyDescriptor { [[Value]]: 𝔽(length), [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
    if length > u8::MAX as usize {
        return Err(agent.throw_exception_with_static_message(
            SyntaxError,
            "Too many arguments in function call (only 255 allowed)",
            gc,
        ));
    }
    function.length = length as u8;

    // 3. Return unused.
    Ok(())
}

impl Rootable for ECMAScriptFunction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::ECMAScriptFunction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::ECMAScriptFunction(d) => Some(d),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for ECMAScriptFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.ecmascript_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.ecmascript_functions.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for ECMAScriptFunction<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .ecmascript_functions
            .shift_weak_index(self.0)
            .map(Self)
    }
}

impl<'a> CreateHeapData<ECMAScriptFunctionHeapData<'a>, ECMAScriptFunction<'a>> for Heap {
    fn create(&mut self, data: ECMAScriptFunctionHeapData<'a>) -> ECMAScriptFunction<'a> {
        self.ecmascript_functions.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<ECMAScriptFunctionHeapData<'static>>>();

        ECMAScriptFunction(ECMAScriptFunctionIndex::last(&self.ecmascript_functions))
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ECMAScriptFunctionHeapData<'_> {
    type Of<'a> = ECMAScriptFunctionHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for ECMAScriptFunctionHeapData<'static> {
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
