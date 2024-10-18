// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use oxc_span::Span;

use crate::{
    ecmascript::{
        execution::{
            agent::ExceptionType, Agent, EnvironmentIndex, ExecutionContext, JsResult,
            PrivateEnvironmentIndex, ProtoIntrinsics,
        },
        scripts_and_modules::source_code::SourceCode,
        syntax_directed_operations::class_definitions::{
            base_class_default_constructor, derived_class_default_constructor,
        },
        types::{
            function_create_backing_object, function_internal_define_own_property,
            function_internal_delete, function_internal_get, function_internal_get_own_property,
            function_internal_has_property, function_internal_own_property_keys,
            function_internal_set, BuiltinConstructorHeapData, Function,
            FunctionInternalProperties, InternalMethods, InternalSlots, IntoFunction, IntoObject,
            IntoValue, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    engine::Executable,
    heap::{
        indexes::BuiltinConstructorIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues,
    },
};

use super::ArgumentsList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinConstructorFunction(pub(crate) BuiltinConstructorIndex);

impl BuiltinConstructorFunction {
    pub(crate) const fn _def() -> Self {
        Self(BuiltinConstructorIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub const fn is_constructor(self) -> bool {
        true
    }
}

impl From<BuiltinConstructorIndex> for BuiltinConstructorFunction {
    fn from(value: BuiltinConstructorIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for BuiltinConstructorFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for BuiltinConstructorFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoFunction for BuiltinConstructorFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<BuiltinConstructorFunction> for Value {
    fn from(value: BuiltinConstructorFunction) -> Self {
        Value::BuiltinConstructorFunction(value)
    }
}

impl From<BuiltinConstructorFunction> for Object {
    fn from(value: BuiltinConstructorFunction) -> Self {
        Self::BuiltinConstructorFunction(value)
    }
}

impl From<BuiltinConstructorFunction> for Function {
    fn from(value: BuiltinConstructorFunction) -> Self {
        Self::BuiltinConstructorFunction(value)
    }
}

impl TryFrom<Value> for BuiltinConstructorFunction {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for BuiltinConstructorFunction {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Function> for BuiltinConstructorFunction {
    type Error = ();

    fn try_from(value: Function) -> Result<Self, Self::Error> {
        match value {
            Function::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl Index<BuiltinConstructorFunction> for Agent {
    type Output = BuiltinConstructorHeapData;

    fn index(&self, index: BuiltinConstructorFunction) -> &Self::Output {
        &self.heap.builtin_constructors[index]
    }
}

impl IndexMut<BuiltinConstructorFunction> for Agent {
    fn index_mut(&mut self, index: BuiltinConstructorFunction) -> &mut Self::Output {
        &mut self.heap.builtin_constructors[index]
    }
}

impl Index<BuiltinConstructorFunction> for Vec<Option<BuiltinConstructorHeapData>> {
    type Output = BuiltinConstructorHeapData;

    fn index(&self, index: BuiltinConstructorFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinConstructorFunction out of bounds")
            .as_ref()
            .expect("BuiltinConstructorFunction slot empty")
    }
}

impl IndexMut<BuiltinConstructorFunction> for Vec<Option<BuiltinConstructorHeapData>> {
    fn index_mut(&mut self, index: BuiltinConstructorFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinConstructorFunction out of bounds")
            .as_mut()
            .expect("BuiltinConstructorFunction slot empty")
    }
}

impl FunctionInternalProperties for BuiltinConstructorFunction {
    fn get_name(self, _: &Agent) -> String {
        unreachable!();
    }

    fn get_length(self, _: &Agent) -> u8 {
        unreachable!();
    }
}

impl InternalSlots for BuiltinConstructorFunction {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject {
        function_create_backing_object(self, agent)
    }
}

impl InternalMethods for BuiltinConstructorFunction {
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

    /// ### [10.3.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-built-in-function-objects-call-thisargument-argumentslist)
    ///
    /// The [[Call]] internal method of a built-in function object F takes
    /// arguments thisArgument (an ECMAScript language value) and argumentsList
    /// (a List of ECMAScript language values) and returns either a normal
    /// completion containing an ECMAScript language value or a throw
    /// completion.
    fn internal_call(self, agent: &mut Agent, _: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Return ? BuiltinCallOrConstruct(F, thisArgument, argumentsList, undefined).
        // ii. If NewTarget is undefined, throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "class constructors must be invoked with 'new'",
        ))
    }

    /// ### [10.3.2 \[\[Construct\]\] ( argumentsList, newTarget )](https://tc39.es/ecma262/#sec-built-in-function-objects-construct-argumentslist-newtarget)
    ///
    /// The [[Construct]] internal method of a built-in function object F (when
    /// the method is present) takes arguments argumentsList (a List of
    /// ECMAScript language values) and newTarget (a constructor) and returns
    /// either a normal completion containing an Object or a throw completion.
    fn internal_construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        // 1. Return ? BuiltinCallOrConstruct(F, uninitialized, argumentsList, newTarget).
        builtin_call_or_construct(agent, self, arguments_list, new_target)
    }
}

/// ### [10.3.3 BuiltinCallOrConstruct ( F, thisArgument, argumentsList, newTarget )](https://tc39.es/ecma262/#sec-builtincallorconstruct)
///
/// The abstract operation BuiltinCallOrConstruct takes arguments F (a built-in
/// function object), thisArgument (an ECMAScript language value or
/// uninitialized), argumentsList (a List of ECMAScript language values), and
/// newTarget (a constructor or undefined) and returns either a normal
/// completion containing an ECMAScript language value or a throw completion.
fn builtin_call_or_construct(
    agent: &mut Agent,
    f: BuiltinConstructorFunction,
    arguments_list: ArgumentsList,
    new_target: Function,
) -> JsResult<Object> {
    // 1. Let callerContext be the running execution context.
    let caller_context = agent.running_execution_context();
    // 2. If callerContext is not already suspended, suspend callerContext.
    caller_context.suspend();
    // 5. Let calleeRealm be F.[[Realm]].
    let Agent {
        heap: Heap {
            builtin_constructors,
            ..
        },
        execution_context_stack,
        ..
    } = agent;
    let heap_data = &builtin_constructors[f];
    let callee_realm = heap_data.realm;
    // 3. Let calleeContext be a new execution context.
    let callee_context = ExecutionContext {
        // 8. Perform any necessary implementation-defined initialization of calleeContext.
        ecmascript_code: None,
        // 4. Set the Function of calleeContext to F.
        function: Some(f.into_function()),
        // 6. Set the Realm of calleeContext to calleeRealm.
        realm: callee_realm,
        // 7. Set the ScriptOrModule of calleeContext to null.
        script_or_module: None,
    };
    // 9. Push calleeContext onto the execution context stack; calleeContext is now the running execution context.
    execution_context_stack.push(callee_context);
    // 10. Let result be the Completion Record that is the result of evaluating F in a manner that conforms to
    // the specification of F. If thisArgument is uninitialized, the this value is uninitialized; otherwise,
    // thisArgument provides the this value. argumentsList provides the named parameters. newTarget provides the NewTarget value.
    let result = if heap_data.is_derived {
        derived_class_default_constructor(agent, arguments_list, new_target.into_object())
    } else {
        base_class_default_constructor(agent, new_target.into_object())
    };
    // 11. NOTE: If F is defined in this document, “the specification of F” is the behaviour specified for it via
    // algorithm steps or other means.
    // 12. Remove calleeContext from the execution context stack and restore callerContext as the running
    // execution context.
    // Note
    // When calleeContext is removed from the execution context stack it must not be destroyed if it has been
    // suspended and retained by an accessible Generator for later resumption.
    let _callee_context = agent.execution_context_stack.pop();
    // 13. Return ? result.
    result
}

pub(crate) struct BuiltinConstructorArgs {
    pub(crate) is_derived: bool,
    pub(crate) class_name: String,
    pub(crate) prototype: Option<Object>,
    pub(crate) prototype_property: Object,
    pub(crate) compiled_initializer_bytecode: Option<Executable>,
    pub(crate) env: EnvironmentIndex,
    pub(crate) private_env: Option<PrivateEnvironmentIndex>,
    pub(crate) source_code: SourceCode,
    pub(crate) source_text: Span,
}

/// ### [10.3.4 CreateBuiltinFunction ( behaviour, length, name, additionalInternalSlotsList \[ , realm \[ , prototype \[ , prefix \] \] \] )](https://tc39.es/ecma262/#sec-createbuiltinfunction)
///
/// The abstract operation CreateBuiltinFunction takes arguments behaviour (an
/// Abstract Closure, a set of algorithm steps, or some other definition of a
/// function's behaviour provided in this specification), length (a
/// non-negative integer or +∞), name (a property key or a Private Name), and
/// additionalInternalSlotsList (a List of names of internal slots) and
/// optional arguments realm (a Realm Record), prototype (an Object or null),
/// and prefix (a String) and returns a function object.
/// additionalInternalSlotsList contains the names of additional internal slots
/// that must be defined as part of the object. This operation creates a
/// built-in function object.
pub(crate) fn create_builtin_constructor(
    agent: &mut Agent,
    args: BuiltinConstructorArgs,
) -> BuiltinConstructorFunction {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm = agent.current_realm_id();

    // 9. Set func.[[InitialName]] to null.

    // 2. If prototype is not present, set prototype to realm.[[Intrinsics]].[[%Function.prototype%]].

    // 3. Let internalSlotsList be a List containing the names of all the internal slots that 10.3
    //    requires for the built-in function object that is about to be created.
    // 4. Append to internalSlotsList the elements of additionalInternalSlotsList.
    // * [[ConstructorKind]] and [[SourceText]] for class constructors.

    // 5. Let func be a new built-in function object that, when called, performs the action
    //    described by behaviour using the provided arguments as the values of the corresponding
    //    parameters specified by behaviour. The new function object has internal slots whose names
    //    are the elements of internalSlotsList, and an [[InitialName]] internal slot.

    // 7. Set func.[[Extensible]] to true.
    let length_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
        value: ObjectEntryPropertyDescriptor::Data {
            value: 0.into(),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    };
    let name_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
        value: ObjectEntryPropertyDescriptor::Data {
            value: args.class_name.into_value(),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    };
    let prototype_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.prototype),
        value: ObjectEntryPropertyDescriptor::Data {
            value: args.prototype_property.into_value(),
            writable: false,
            enumerable: false,
            configurable: false,
        },
    };
    let entries = [length_entry, name_entry, prototype_entry];
    let backing_object = if let Some(prototype) = args.prototype {
        agent.heap.create_object_with_prototype(prototype, &entries)
    } else {
        agent.heap.create_null_object(&entries)
    };

    // 13. Return func.
    agent.heap.create(BuiltinConstructorHeapData {
        // 10. Perform SetFunctionLength(func, length).
        // Skipped as length of builtin constructors is always 0.
        // 8. Set func.[[Realm]] to realm.
        realm,
        compiled_initializer_bytecode: args.compiled_initializer_bytecode,
        is_derived: args.is_derived,
        object_index: Some(backing_object),
        environment: args.env,
        private_environment: args.private_env,
        source_text: args.source_text,
        source_code: args.source_code,
    })
}

impl CreateHeapData<BuiltinConstructorHeapData, BuiltinConstructorFunction> for Heap {
    fn create(&mut self, data: BuiltinConstructorHeapData) -> BuiltinConstructorFunction {
        self.builtin_constructors.push(Some(data));
        BuiltinConstructorIndex::last(&self.builtin_constructors).into()
    }
}

impl HeapMarkAndSweep for BuiltinConstructorFunction {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.builtin_constructors.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.builtin_constructors.shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for BuiltinConstructorHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            realm,
            is_derived: _,
            compiled_initializer_bytecode,
            environment,
            private_environment,
            source_text: _,
            source_code,
        } = self;
        realm.mark_values(queues);
        object_index.mark_values(queues);
        environment.mark_values(queues);
        private_environment.mark_values(queues);
        source_code.mark_values(queues);
        compiled_initializer_bytecode.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            realm,
            is_derived: _,
            compiled_initializer_bytecode,
            environment,
            private_environment,
            source_text: _,
            source_code,
        } = self;
        realm.sweep_values(compactions);
        object_index.sweep_values(compactions);
        environment.sweep_values(compactions);
        private_environment.sweep_values(compactions);
        source_code.sweep_values(compactions);
        compiled_initializer_bytecode.sweep_values(compactions);
    }
}
