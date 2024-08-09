// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    ops::{Index, IndexMut},
    ptr::NonNull,
};

use oxc_ast::{
    ast::{FormalParameters, FunctionBody},
    syntax_directed_operations::{BoundNames, IsSimpleParameterList},
};
use oxc_span::Span;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::define_property_or_throw, type_conversion::to_object,
        },
        execution::{
            agent::{
                get_active_script_or_module, resolve_binding,
                ExceptionType::{self, SyntaxError},
            },
            new_declarative_environment, new_function_environment, Agent,
            ECMAScriptCodeEvaluationState, EnvironmentIndex, ExecutionContext, JsResult,
            PrivateEnvironmentIndex, ProtoIntrinsics, RealmIdentifier, ThisBindingStatus,
        },
        scripts_and_modules::{source_code::SourceCode, ScriptOrModule},
        syntax_directed_operations::{
            function_definitions::{
                evaluate_async_function_body, evaluate_function_body, evaluate_generator_body,
            },
            miscellaneous::instantiate_function_object,
            scope_analysis::{
                function_body_lexically_declared_names, function_body_lexically_scoped_decarations,
                function_body_var_declared_names, function_body_var_scoped_declarations,
                LexicallyScopedDeclaration, VarScopedDeclaration,
            },
        },
        types::{
            initialize_referenced_binding, put_value, ECMAScriptFunctionHeapData, Function,
            InternalMethods, InternalSlots, IntoFunction, IntoObject, IntoValue, Object,
            ObjectHeapData, PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::ECMAScriptFunctionIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues,
    },
};

use super::{
    create_unmapped_arguments_object,
    ordinary::{ordinary_create_from_constructor, ordinary_object_create_with_intrinsics},
    ArgumentsList,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ECMAScriptFunction<'gen>(ECMAScriptFunctionIndex<'gen>);

impl<'gen> From<ECMAScriptFunction<'gen>> for ECMAScriptFunctionIndex<'gen> {
    fn from(val: ECMAScriptFunction<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<ECMAScriptFunctionIndex<'gen>> for ECMAScriptFunction<'gen> {
    fn from(value: ECMAScriptFunctionIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for ECMAScriptFunction<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for ECMAScriptFunction<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> IntoFunction<'gen> for ECMAScriptFunction<'gen> {
    fn into_function(self) -> Function<'gen> {
        self.into()
    }
}

impl<'gen> TryFrom<Value<'gen>> for ECMAScriptFunction<'gen> {
    type Error = ();

    fn try_from(value: Value<'gen>) -> Result<Self, Self::Error> {
        if let Value::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl<'gen> TryFrom<Object<'gen>> for ECMAScriptFunction<'gen> {
    type Error = ();

    fn try_from(value: Object<'gen>) -> Result<Self, Self::Error> {
        if let Object::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl<'gen> TryFrom<Function<'gen>> for ECMAScriptFunction<'gen> {
    type Error = ();

    fn try_from(value: Function<'gen>) -> Result<Self, Self::Error> {
        if let Function::ECMAScriptFunction(function) = value {
            Ok(function)
        } else {
            Err(())
        }
    }
}

impl<'gen> From<ECMAScriptFunction<'gen>> for Value<'gen> {
    fn from(val: ECMAScriptFunction<'gen>) -> Self {
        Value::ECMAScriptFunction(val)
    }
}

impl<'gen> From<ECMAScriptFunction<'gen>> for Object<'gen> {
    fn from(val: ECMAScriptFunction<'gen>) -> Self {
        Object::ECMAScriptFunction(val)
    }
}

impl<'gen> From<ECMAScriptFunction<'gen>> for Function<'gen> {
    fn from(val: ECMAScriptFunction<'gen>) -> Self {
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
pub(crate) struct ECMAScriptFunctionObjectHeapData<'gen> {
    /// \[\[Environment]]
    pub environment: EnvironmentIndex<'gen>,

    /// \[\[PrivateEnvironment]]
    pub private_environment: Option<PrivateEnvironmentIndex<'gen>>,

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
    pub realm: RealmIdentifier<'gen>,

    /// \[\[ScriptOrModule]]
    pub script_or_module: ScriptOrModule<'gen>,

    /// \[\[ThisMode]]
    pub this_mode: ThisMode,

    /// \[\[Strict]]
    pub strict: bool,

    /// \[\[HomeObject]]
    pub home_object: Option<Object<'gen>>,

    ///  \[\[SourceText]]
    pub source_text: Span,

    /// \[\[SourceCode]]
    ///
    /// Nova specific addition: This SourceCode is where \[\[SourceText]]
    /// refers to.
    pub source_code: SourceCode<'gen>,
    // TODO: [[Fields]],  [[PrivateMethods]], [[ClassFieldInitializerName]]
}

pub(crate) struct OrdinaryFunctionCreateParams<'gen, 'agent, 'program> {
    pub function_prototype: Option<Object<'gen>>,
    pub source_text: Span,
    pub parameters_list: &'agent FormalParameters<'program>,
    pub body: &'agent FunctionBody<'program>,
    pub is_concise_arrow_function: bool,
    pub is_async: bool,
    pub is_generator: bool,
    pub lexical_this: bool,
    pub env: EnvironmentIndex<'gen>,
    pub private_env: Option<PrivateEnvironmentIndex<'gen>>,
}

impl<'gen> Index<ECMAScriptFunction<'gen>> for Agent<'gen> {
    type Output = ECMAScriptFunctionHeapData<'gen>;

    fn index(&self, index: ECMAScriptFunction<'gen>) -> &Self::Output {
        &self.heap.ecmascript_functions[index]
    }
}

impl<'gen> IndexMut<ECMAScriptFunction<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: ECMAScriptFunction<'gen>) -> &mut Self::Output {
        &mut self.heap.ecmascript_functions[index]
    }
}

impl<'gen> Index<ECMAScriptFunction<'gen>> for Vec<Option<ECMAScriptFunctionHeapData<'gen>>> {
    type Output = ECMAScriptFunctionHeapData<'gen>;

    fn index(&self, index: ECMAScriptFunction<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("ECMAScriptFunction out of bounds")
            .as_ref()
            .expect("ECMAScriptFunction slot empty")
    }
}

impl<'gen> IndexMut<ECMAScriptFunction<'gen>> for Vec<Option<ECMAScriptFunctionHeapData<'gen>>> {
    fn index_mut(&mut self, index: ECMAScriptFunction<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ECMAScriptFunction out of bounds")
            .as_mut()
            .expect("ECMAScriptFunction slot empty")
    }
}

impl ECMAScriptFunction<'_> {
    pub(crate) const fn _def() -> Self {
        ECMAScriptFunction(ECMAScriptFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn is_constructor(self, agent: &Agent<'_>) -> bool {
        // An ECMAScript function has the [[Construct]] slot if its constructor
        // status is something other than non-constructor.
        agent[self].ecmascript_function.constructor_status != ConstructorStatus::NonConstructor
    }
}

impl<'gen> InternalSlots<'gen> for ECMAScriptFunction<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
        debug_assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent);
        let length_entry = ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            value: ObjectEntryPropertyDescriptor::Data {
                value: agent[self].length.into(),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        };
        let name_entry = ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            value: ObjectEntryPropertyDescriptor::Data {
                value: agent[self]
                    .name
                    .unwrap_or(String::EMPTY_STRING)
                    .into_value(),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        };
        let (keys, values) = agent
            .heap
            .elements
            .create_object_entries(&[length_entry, name_entry]);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype,
            keys,
            values,
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }

    fn internal_prototype(self, agent: &Agent<'gen>) -> Option<Object<'gen>> {
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

impl<'gen> InternalMethods<'gen> for ECMAScriptFunction<'gen> {
    fn internal_get_own_property(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
    ) -> JsResult<'gen, Option<PropertyDescriptor<'gen>>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get_own_property(agent, property_key)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(Some(PropertyDescriptor {
                value: Some(agent[self].length.into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
            Ok(Some(PropertyDescriptor {
                value: Some(agent[self].name.unwrap_or(String::EMPTY_STRING).into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
        } else {
            Ok(None)
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        property_descriptor: PropertyDescriptor<'gen>,
    ) -> JsResult<'gen, bool> {
        let object_index = agent[self]
            .object_index
            .unwrap_or_else(|| self.create_backing_object(agent));
        object_index.internal_define_own_property(agent, property_key, property_descriptor)
    }

    fn internal_has_property(self, agent: &mut Agent<'gen>, property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            Ok(true)
        } else {
            let parent = self.internal_get_prototype_of(agent)?;
            parent.map_or(Ok(false), |parent| {
                parent.internal_has_property(agent, property_key)
            })
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        receiver: Value<'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get(agent, property_key, receiver)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(agent[self].length.into())
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
            Ok(agent[self].name.unwrap_or(String::EMPTY_STRING).into())
        } else {
            let parent = self.internal_get_prototype_of(agent)?;
            parent.map_or(Ok(Value::Undefined), |parent| {
                parent.internal_get(agent, property_key, receiver)
            })
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        value: Value<'gen>,
        receiver: Value<'gen>,
    ) -> JsResult<'gen, bool> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_set(agent, property_key, value, receiver)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            // length and name are not writable
            Ok(false)
        } else {
            self.create_backing_object(agent)
                .internal_set(agent, property_key, value, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent<'gen>, property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_delete(agent, property_key)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            let object_index = self.create_backing_object(agent);
            object_index.internal_delete(agent, property_key)
        } else {
            // Non-existing property
            Ok(true)
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent<'gen>) -> JsResult<'gen, Vec<PropertyKey<'gen>>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_own_property_keys(agent)
        } else {
            Ok(vec![
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            ])
        }
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
        agent: &mut Agent<'gen>,
        this_argument: Value<'gen>,
        arguments_list: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
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

    fn internal_construct(
        self,
        agent: &mut Agent<'gen>,
        arguments_list: ArgumentsList<'_, 'gen>,
        new_target: Function<'gen>,
    ) -> JsResult<'gen, Object<'gen>> {
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
            //    i. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
            //    ii. Return ? initializeResult.
            // TODO: Classes.
        }

        // 8. Let result be Completion(OrdinaryCallEvaluateBody(F, argumentsList)).
        let result = ordinary_call_evaluate_body(agent, self, arguments_list);
        // 9. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
        agent.execution_context_stack.pop();
        // 10. If result is a return completion, then
        // 11. Else,
        //   a. ReturnIfAbrupt(result).
        let value = result?;
        // 10. If result is a return completion, then
        //   a. If result.[[Value]] is an Object, return result.[[Value]].
        if let Ok(value) = Object::try_from(value) {
            return Ok(value);
        }
        //   b. If kind is base, return thisArgument.
        if is_base {
            return Ok(this_argument.unwrap());
        }
        todo!("Derived classes");
        //   c. If result.[[Value]] is not undefined, throw a TypeError exception.
        // 12. Let thisBinding be ? constructorEnv.GetThisBinding().
        // 13. Assert: thisBinding is an Object.
        // 14. Return thisBinding.
    }
}

/// ### [10.2.1.1 PrepareForOrdinaryCall ( F, newTarget )](https://tc39.es/ecma262/#sec-prepareforordinarycall)
///
/// The abstract operation PrepareForOrdinaryCall takes arguments `F` (an
/// ECMAScript function object) and newTarget (an Object or undefined) and
/// returns an execution context.
pub(crate) fn prepare_for_ordinary_call<'a, 'gen>(
    agent: &'a mut Agent<'gen>,
    f: ECMAScriptFunction<'gen>,
    new_target: Option<Object<'gen>>,
) -> &'a ExecutionContext<'gen> {
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
pub(crate) fn ordinary_call_bind_this<'gen>(
    agent: &mut Agent<'gen>,
    f: ECMAScriptFunction<'gen>,
    local_env: EnvironmentIndex<'gen>,
    this_argument: Value<'gen>,
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
pub(crate) fn evaluate_body<'gen>(
    agent: &mut Agent<'gen>,
    function_object: ECMAScriptFunction<'gen>,
    arguments_list: ArgumentsList<'_, 'gen>,
) -> JsResult<'gen, Value<'gen>> {
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
pub(crate) fn ordinary_call_evaluate_body<'gen>(
    agent: &mut Agent<'gen>,
    f: ECMAScriptFunction<'gen>,
    arguments_list: ArgumentsList<'_, 'gen>,
) -> JsResult<'gen, Value<'gen>> {
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
pub(crate) fn ordinary_function_create<'gen, 'agent, 'program>(
    agent: &'agent mut Agent,
    params: OrdinaryFunctionCreateParams<'gen, 'agent, 'program>,
) -> ECMAScriptFunction<'gen> {
    let running_ecmascript_code = &agent.running_execution_context().ecmascript_code.unwrap();
    let source_code = running_ecmascript_code.source_code;
    // 7. If the source text matched by Body is strict mode code, let Strict be true; else let Strict be false.
    let strict = params.body.has_use_strict_directive() || running_ecmascript_code.is_strict_mode;

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
pub(crate) fn make_constructor<'gen>(
    agent: &mut Agent<'gen>,
    function: impl IntoFunction<'gen>,
    writable_prototype: Option<bool>,
    prototype: Option<Object<'gen>>,
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
            // TODO: Handle Some() object indexes?
            assert!(data.object_index.is_none());
            // c. Set F.[[Construct]] to the definition specified in 10.2.2.
            // 3. Set F.[[ConstructorKind]] to BASE.
            data.ecmascript_function.constructor_status = ConstructorStatus::ConstructorFunction;
        }
        Function::BuiltinFunction(_) => {
            // 2. Else,
            // a. Set F.[[Construct]] to the definition specified in 10.3.2.
        }
        Function::BuiltinGeneratorFunction => todo!(),
        Function::BuiltinConstructorFunction => todo!(),
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
pub(crate) fn make_method<'gen>(agent: &mut Agent<'gen>, f: ECMAScriptFunction<'gen>, home_object: Object<'gen>) {
    // 1. Set F.[[HomeObject]] to homeObject.
    agent[f].ecmascript_function.home_object = Some(home_object);
    // 2. Return unused.
}

/// ### [10.2.9 SetFunctionName ( F, name \[ , prefix \] )](https://tc39.es/ecma262/#sec-setfunctionname)
/// The abstract operation SetFunctionName takes arguments F (a function
/// object) and name (a property key or Private Name) and optional argument
/// prefix (a String) and returns UNUSED. It adds a "name" property to F.
pub(crate) fn set_function_name<'gen>(
    agent: &mut Agent<'gen>,
    function: impl IntoFunction<'gen>,
    name: PropertyKey<'gen>,
    _prefix: Option<String<'gen>>,
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
        Function::BoundFunction(_idx) => todo!(),
        Function::BuiltinFunction(_idx) => todo!(),
        Function::ECMAScriptFunction(idx) => {
            let function = &mut agent[idx];
            // 1. Assert: F is an extensible object that does not have a "name" own property.
            // TODO: Also potentially allow running this function with Some() object index if needed.
            assert!(function.object_index.is_none() && function.name.is_none());
            // 6. Perform ! DefinePropertyOrThrow(F, "name", PropertyDescriptor { [[Value]]: name, [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
            function.name = Some(name);
            // 7. Return UNUSED.
        }
        Function::BuiltinGeneratorFunction => todo!(),
        Function::BuiltinConstructorFunction => todo!(),
        Function::BuiltinPromiseResolvingFunction(_) => todo!(),
        Function::BuiltinPromiseCollectorFunction => todo!(),
        Function::BuiltinProxyRevokerFunction => todo!(),
    }
}

/// ### [10.2.10 SetFunctionLength ( F, length )](https://tc39.es/ecma262/#sec-setfunctionlength)
fn set_ecmascript_function_length<'gen>(
    agent: &mut Agent<'gen>,
    function: &mut ECMAScriptFunctionHeapData<'gen>,
    length: usize,
) -> JsResult<'gen, ()> {
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
pub(crate) fn function_declaration_instantiation<'gen>(
    agent: &mut Agent<'gen>,
    function_object: ECMAScriptFunction<'gen>,
    arguments_list: ArgumentsList<'_, 'gen>,
) -> JsResult<'gen, ()> {
    // 1. Let calleeContext be the running execution context.
    let callee_context = agent.running_execution_context();
    // 35. Let privateEnv be the PrivateEnvironment of calleeContext.
    let ECMAScriptCodeEvaluationState {
        lexical_environment: callee_lex_env,
        variable_environment: callee_var_env,
        private_environment: callee_private_env,
        ..
    } = *callee_context.ecmascript_code.as_ref().unwrap();
    // 2. Let code be func.[[ECMAScriptCode]].
    let (var_names, var_declarations, lexical_names, lex_declarations) = {
        // SAFETY: We're alive so SourceCode must be too.
        let code = unsafe {
            agent[function_object]
                .ecmascript_function
                .ecmascript_code
                .as_ref()
        };
        // 9. Let varNames be the VarDeclaredNames of code.
        let var_names = function_body_var_declared_names(code)
            .iter()
            .map(|atom| String::from_str(agent, atom.as_str()))
            .collect::<Vec<_>>();
        // 10. Let varDeclarations be the VarScopedDeclarations of code.
        let var_declarations = function_body_var_scoped_declarations(code);
        // 11. Let lexicalNames be the LexicallyDeclaredNames of code.
        let lexical_names = function_body_lexically_declared_names(code)
            .iter()
            .map(|atom| String::from_str(agent, atom.as_str()))
            .collect::<Vec<_>>();
        // 33. Let lexDeclarations be the LexicallyScopedDeclarations of code.
        let lex_declarations = function_body_lexically_scoped_decarations(code);
        (var_names, var_declarations, lexical_names, lex_declarations)
    };
    let heap_data = &agent[function_object];
    // 3. Let strict be func.[[Strict]].
    let strict = heap_data.ecmascript_function.strict;
    // 4. Let formals be func.[[FormalParameters]].
    // SAFETY: We're alive, thus SourceCode must be alive as well.
    let formals = unsafe { heap_data.ecmascript_function.formal_parameters.as_ref() };
    let this_mode = heap_data.ecmascript_function.this_mode;
    // 5. Let parameterNames be the BoundNames of formals.
    let mut parameter_names = Vec::with_capacity(formals.parameters_count());
    formals.bound_names(&mut |identifier| {
        parameter_names.push(String::from_str(agent, identifier.name.as_str()));
    });
    // 6. If parameterNames has any duplicate entries, let hasDuplicates be true. Otherwise, let hasDuplicates be false.
    // TODO: Check duplicates
    let has_duplicates = false;

    // 7. Let simpleParameterList be IsSimpleParameterList of formals.
    let simple_parameter_list = formals.is_simple_parameter_list();
    // 8. Let hasParameterExpressions be ContainsExpression of formals.
    // TODO: impl ContainsExpression
    let has_parameter_expression = false;

    // 12. Let functionNames be a new empty List.
    let mut function_names = vec![];
    // 13. Let functionsToInitialize be a new empty List.
    let mut functions_to_initialize: Vec<(String, &oxc_ast::ast::Function<'_>)> = vec![];
    // 14. For each element d of varDeclarations, in reverse List order, do
    for d in var_declarations.iter().rev() {
        // a. If d is neither a VariableDeclaration nor a ForBinding nor a BindingIdentifier, then
        if let VarScopedDeclaration::Function(d) = d {
            // i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
            // ii. Let fn be the sole element of the BoundNames of d.
            let f_name = String::from_str(agent, d.id.as_ref().unwrap().name.as_str());
            // iii. If functionNames does not contain fn, then
            if !function_names.contains(&f_name) {
                // 1. Insert fn as the first element of functionNames.
                function_names.push(f_name);
                // 2. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
                // 3. Insert d as the first element of functionsToInitialize.
                functions_to_initialize.push((f_name, d));
            }
        }
    }
    // 15. Let argumentsObjectNeeded be true.
    let arguments_name = BUILTIN_STRING_MEMORY.arguments;
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
        let already_declared = env.has_binding(agent, *param_name).unwrap();
        // b. NOTE: Early errors ensure that duplicate parameter names can only occur
        // in non-strict functions that do not have parameter default values or rest parameters.
        // c. If alreadyDeclared is false, then
        if !already_declared {
            // i. Perform ! env.CreateMutableBinding(paramName, false).
            env.create_mutable_binding(agent, *param_name, false)
                .unwrap();
        }
        // ii. If hasDuplicates is true, then
        if has_duplicates {
            // 1. Perform ! env.InitializeBinding(paramName, undefined).
            env.initialize_binding(agent, *param_name, Value::Undefined)
                .unwrap();
        }
    }

    // 22. If argumentsObjectNeeded is true, then
    // Note: parameter_names is a slice of parameter_bindings.
    let parameter_bindings;
    let parameter_names = if arguments_object_needed {
        // TODO: For now we use an unmapped arguments object even in non-strict mode.
        // // a. If strict is true or simpleParameterList is false, then
        // let ao: Value = if strict || !simple_parameter_list {
        //     // i. Let ao be CreateUnmappedArgumentsObject(argumentsList).
        //     create_unmapped_arguments_object(agent, arguments_list).into_value()
        // } else {
        //     // b. Else,
        //     todo!("Handle arguments object creation");
        //     // i. NOTE: A mapped argument object is only provided for non-strict functions
        //     // that don't have a rest parameter, any parameter default value initializers,
        //     // or any destructured parameters.
        //     // ii. Let ao be CreateMappedArgumentsObject(func, formals, argumentsList, env).
        // };
        let ao = create_unmapped_arguments_object(agent, arguments_list).into_value();
        // c. If strict is true, then
        if strict {
            // i. Perform ! env.CreateImmutableBinding("arguments", false).
            env.create_immutable_binding(agent, arguments_name, false)
                .unwrap();
        } else {
            // ii. NOTE: In strict mode code early errors prevent attempting to assign to this binding, so its mutability is not observable.
            // d. Else,
            // i. Perform ! env.CreateMutableBinding("arguments", false).
            env.create_mutable_binding(agent, arguments_name, false)
                .unwrap();
        }
        // e. Perform ! env.InitializeBinding("arguments", ao).
        env.initialize_binding(agent, arguments_name, ao).unwrap();
        // f. Let parameterBindings be the list-concatenation of parameterNames and « "arguments" ».
        parameter_names.push(arguments_name);
        parameter_bindings = parameter_names;
        &parameter_bindings[..(parameter_bindings.len() - 1)]
    } else {
        // 23. Else,
        // a. Let parameterBindings be parameterNames.
        parameter_bindings = parameter_names;
        &parameter_bindings[..]
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
    if simple_parameter_list {
        for (i, param_name) in parameter_names.iter().enumerate() {
            // Runtime Semantics: IteratorBindingInitialization
            // SingleNameBinding : BindingIdentifier Initializer[opt]
            // 1. Let bindingId be StringValue of BindingIdentifier.
            // 2. Let lhs be ? ResolveBinding(bindingId, environment).
            let lhs = if has_duplicates {
                resolve_binding(agent, *param_name, None)?
            } else {
                resolve_binding(agent, *param_name, Some(env))?
            };
            // 3. Let v be undefined.
            // 4. If iteratorRecord.[[Done]] is false, then
            //   a. Let next be ? IteratorStepValue(iteratorRecord).
            //   b. If next is not DONE, then
            //     i. Set v to next.
            let v = arguments_list.get(i);
            // 5. If initializer is present and v is undefined, then ...
            // TODO: Support function argument initializers.
            // 6. If environment is undefined, return ? PutValue(lhs, v).
            if has_duplicates {
                put_value(agent, &lhs, v)?;
            } else {
                // 7. Return ? InitializeReferencedBinding(lhs, v).
                initialize_referenced_binding(agent, lhs, v)?;
            }
        }
    } else {
        todo!()
    }

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
                instantiated_var_names.push(*n);
                // 2. Perform ! env.CreateMutableBinding(n, false).
                env.create_mutable_binding(agent, *n, false).unwrap();
                // 3. Perform ! env.InitializeBinding(n, undefined).
                env.initialize_binding(agent, *n, Value::Undefined).unwrap();
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
                var_env.create_mutable_binding(agent, *n, false).unwrap();
                // 3. If parameterBindings does not contain n, or if functionNames contains n, then
                let initial_value = if !parameter_bindings.contains(n) || function_names.contains(n)
                {
                    // a. Let initialValue be undefined.
                    Value::Undefined
                } else {
                    // 4. Else,
                    // a. Let initialValue be ! env.GetBindingValue(n, false).
                    env.get_binding_value(agent, *n, false).unwrap()
                };
                // 5. Perform ! varEnv.InitializeBinding(n, initialValue).
                var_env
                    .initialize_binding(agent, *n, initial_value)
                    .unwrap();
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
                        let identifier = String::from_str(agent, identifier.name.as_str());
                        lex_env
                            .create_immutable_binding(agent, identifier, true)
                            .unwrap();
                    });
                } else {
                    decl.id.bound_names(&mut |identifier| {
                        // ii. Else,
                        // 1. Perform ! lexEnv.CreateMutableBinding(dn, false).
                        let identifier = String::from_str(agent, identifier.name.as_str());
                        lex_env
                            .create_mutable_binding(agent, identifier, false)
                            .unwrap();
                    });
                }
            }
            LexicallyScopedDeclaration::Function(decl) => {
                let identifier = String::from_str(agent, decl.id.as_ref().unwrap().name.as_str());
                lex_env
                    .create_mutable_binding(agent, identifier, false)
                    .unwrap();
            }
            LexicallyScopedDeclaration::Class(decl) => {
                let identifier = String::from_str(agent, decl.id.as_ref().unwrap().name.as_str());
                lex_env
                    .create_mutable_binding(agent, identifier, false)
                    .unwrap();
            }
            LexicallyScopedDeclaration::DefaultExport => {
                let identifier = BUILTIN_STRING_MEMORY._default_;
                lex_env
                    .create_mutable_binding(agent, identifier, false)
                    .unwrap();
            }
        }
    }
    // 35. Let privateEnv be the PrivateEnvironment of calleeContext.
    let private_env = callee_private_env;
    // 36. For each Parse Node f of functionsToInitialize, do
    for (f_name, f) in functions_to_initialize {
        // a. Let fn be the sole element of the BoundNames of f.
        // We calculated this above already.
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

impl<'gen> HeapMarkAndSweep<'gen> for ECMAScriptFunction<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.ecmascript_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.ecmascript_functions.shift_index(&mut self.0);
    }
}

impl<'gen> CreateHeapData<ECMAScriptFunctionHeapData<'gen>, ECMAScriptFunction<'gen>> for Heap<'gen> {
    fn create(&mut self, data: ECMAScriptFunctionHeapData<'gen>) -> ECMAScriptFunction<'gen> {
        self.ecmascript_functions.push(Some(data));
        ECMAScriptFunction(ECMAScriptFunctionIndex::last(&self.ecmascript_functions))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for ECMAScriptFunctionHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.name.mark_values(queues);
        self.object_index.mark_values(queues);

        self.ecmascript_function.environment.mark_values(queues);
        self.ecmascript_function
            .private_environment
            .mark_values(queues);
        self.ecmascript_function.realm.mark_values(queues);
        self.ecmascript_function
            .script_or_module
            .mark_values(queues);
        self.ecmascript_function.home_object.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.name.sweep_values(compactions);
        self.object_index.sweep_values(compactions);
        self.ecmascript_function
            .environment
            .sweep_values(compactions);
        self.ecmascript_function
            .private_environment
            .sweep_values(compactions);
        self.ecmascript_function.realm.sweep_values(compactions);
        self.ecmascript_function
            .script_or_module
            .sweep_values(compactions);
        self.ecmascript_function
            .home_object
            .sweep_values(compactions);
    }
}
