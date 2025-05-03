// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use oxc_span::Span;

use crate::ecmascript::types::{function_try_get, function_try_has_property, function_try_set};
use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable};
use crate::engine::{Scoped, TryResult};
use crate::{
    ecmascript::{
        execution::{
            Agent, Environment, ExecutionContext, JsResult, PrivateEnvironment, ProtoIntrinsics,
            agent::ExceptionType,
        },
        scripts_and_modules::source_code::SourceCode,
        syntax_directed_operations::class_definitions::{
            base_class_default_constructor, derived_class_default_constructor,
        },
        types::{
            BUILTIN_STRING_MEMORY, BuiltinConstructorHeapData, Function,
            FunctionInternalProperties, InternalMethods, InternalSlots, IntoFunction, IntoObject,
            IntoValue, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value,
            function_create_backing_object, function_internal_define_own_property,
            function_internal_delete, function_internal_get, function_internal_get_own_property,
            function_internal_has_property, function_internal_own_property_keys,
            function_internal_set,
        },
    },
    engine::Executable,
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, ObjectEntry,
        ObjectEntryPropertyDescriptor, WorkQueues, indexes::BuiltinConstructorIndex,
    },
};

use super::ArgumentsList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinConstructorFunction<'a>(pub(crate) BuiltinConstructorIndex<'a>);

impl BuiltinConstructorFunction<'_> {
    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, BuiltinConstructorFunction<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for BuiltinConstructorFunction<'_> {
    type Of<'a> = BuiltinConstructorFunction<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<BuiltinConstructorIndex<'a>> for BuiltinConstructorFunction<'a> {
    fn from(value: BuiltinConstructorIndex<'a>) -> Self {
        Self(value)
    }
}

impl<'a> IntoFunction<'a> for BuiltinConstructorFunction<'a> {
    fn into_function(self) -> Function<'a> {
        self.into()
    }
}

impl<'a> From<BuiltinConstructorFunction<'a>> for Value<'a> {
    fn from(value: BuiltinConstructorFunction<'a>) -> Self {
        Value::BuiltinConstructorFunction(value)
    }
}

impl<'a> From<BuiltinConstructorFunction<'a>> for Object<'a> {
    fn from(value: BuiltinConstructorFunction) -> Self {
        Self::BuiltinConstructorFunction(value.unbind())
    }
}

impl<'a> From<BuiltinConstructorFunction<'a>> for Function<'a> {
    fn from(value: BuiltinConstructorFunction<'a>) -> Self {
        Self::BuiltinConstructorFunction(value)
    }
}

impl<'a> TryFrom<Value<'a>> for BuiltinConstructorFunction<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for BuiltinConstructorFunction<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Function<'a>> for BuiltinConstructorFunction<'a> {
    type Error = ();

    fn try_from(value: Function<'a>) -> Result<Self, Self::Error> {
        match value {
            Function::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl Index<BuiltinConstructorFunction<'_>> for Agent {
    type Output = BuiltinConstructorHeapData<'static>;

    fn index(&self, index: BuiltinConstructorFunction) -> &Self::Output {
        &self.heap.builtin_constructors[index]
    }
}

impl IndexMut<BuiltinConstructorFunction<'_>> for Agent {
    fn index_mut(&mut self, index: BuiltinConstructorFunction) -> &mut Self::Output {
        &mut self.heap.builtin_constructors[index]
    }
}

impl Index<BuiltinConstructorFunction<'_>> for Vec<Option<BuiltinConstructorHeapData<'static>>> {
    type Output = BuiltinConstructorHeapData<'static>;

    fn index(&self, index: BuiltinConstructorFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinConstructorFunction out of bounds")
            .as_ref()
            .expect("BuiltinConstructorFunction slot empty")
    }
}

impl IndexMut<BuiltinConstructorFunction<'_>> for Vec<Option<BuiltinConstructorHeapData<'static>>> {
    fn index_mut(&mut self, index: BuiltinConstructorFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinConstructorFunction out of bounds")
            .as_mut()
            .expect("BuiltinConstructorFunction slot empty")
    }
}

impl<'a> FunctionInternalProperties<'a> for BuiltinConstructorFunction<'a> {
    fn get_name(self, _: &Agent) -> String<'static> {
        unreachable!();
    }

    fn get_length(self, _: &Agent) -> u8 {
        unreachable!();
    }
}

impl<'a> InternalSlots<'a> for BuiltinConstructorFunction<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        function_create_backing_object(self, agent)
    }
}

impl<'a> InternalMethods<'a> for BuiltinConstructorFunction<'a> {
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

    /// ### [10.3.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-built-in-function-objects-call-thisargument-argumentslist)
    ///
    /// The [[Call]] internal method of a built-in function object F takes
    /// arguments thisArgument (an ECMAScript language value) and argumentsList
    /// (a List of ECMAScript language values) and returns either a normal
    /// completion containing an ECMAScript language value or a throw
    /// completion.
    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        _: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return ? BuiltinCallOrConstruct(F, thisArgument, argumentsList, undefined).
        // ii. If NewTarget is undefined, throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "class constructors must be invoked with 'new'",
            gc.into_nogc(),
        ))
    }

    /// ### [10.3.2 \[\[Construct\]\] ( argumentsList, newTarget )](https://tc39.es/ecma262/#sec-built-in-function-objects-construct-argumentslist-newtarget)
    ///
    /// The [[Construct]] internal method of a built-in function object F (when
    /// the method is present) takes arguments argumentsList (a List of
    /// ECMAScript language values) and newTarget (a constructor) and returns
    /// either a normal completion containing an Object or a throw completion.
    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        // 1. Return ? BuiltinCallOrConstruct(F, uninitialized, argumentsList, newTarget).
        builtin_call_or_construct(agent, self, arguments_list, new_target, gc)
    }
}

/// ### [10.3.3 BuiltinCallOrConstruct ( F, thisArgument, argumentsList, newTarget )](https://tc39.es/ecma262/#sec-builtincallorconstruct)
///
/// The abstract operation BuiltinCallOrConstruct takes arguments F (a built-in
/// function object), thisArgument (an ECMAScript language value or
/// uninitialized), argumentsList (a List of ECMAScript language values), and
/// newTarget (a constructor or undefined) and returns either a normal
/// completion containing an ECMAScript language value or a throw completion.
fn builtin_call_or_construct<'a>(
    agent: &mut Agent,
    f: BuiltinConstructorFunction,
    arguments_list: ArgumentsList,
    new_target: Function,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    let f = f.bind(gc.nogc());
    let arguments_list = arguments_list.bind(gc.nogc());
    let new_target = new_target.bind(gc.nogc());
    // 1. Let callerContext be the running execution context.
    let caller_context = agent.running_execution_context();
    // 2. If callerContext is not already suspended, suspend callerContext.
    caller_context.suspend();
    // 5. Let calleeRealm be F.[[Realm]].
    let heap_data = &agent[f];
    let callee_realm = heap_data.realm;
    let is_derived = heap_data.is_derived;
    // 3. Let calleeContext be a new execution context.
    let callee_context = ExecutionContext {
        // 8. Perform any necessary implementation-defined initialization of calleeContext.
        ecmascript_code: None,
        // 4. Set the Function of calleeContext to F.
        function: Some(f.into_function().unbind()),
        // 6. Set the Realm of calleeContext to calleeRealm.
        realm: callee_realm,
        // 7. Set the ScriptOrModule of calleeContext to null.
        script_or_module: None,
    };
    // 9. Push calleeContext onto the execution context stack; calleeContext is now the running execution context.
    agent.push_execution_context(callee_context);
    // 10. Let result be the Completion Record that is the result of evaluating F in a manner that conforms to
    // the specification of F. If thisArgument is uninitialized, the this value is uninitialized; otherwise,
    // thisArgument provides the this value. argumentsList provides the named parameters. newTarget provides the NewTarget value.
    let result = if is_derived {
        derived_class_default_constructor(
            agent,
            arguments_list.unbind(),
            new_target.into_object().unbind(),
            gc,
        )
    } else {
        base_class_default_constructor(agent, new_target.into_object().unbind(), gc)
    };
    // 11. NOTE: If F is defined in this document, “the specification of F” is the behaviour specified for it via
    // algorithm steps or other means.
    // 12. Remove calleeContext from the execution context stack and restore callerContext as the running
    // execution context.
    // Note
    // When calleeContext is removed from the execution context stack it must not be destroyed if it has been
    // suspended and retained by an accessible Generator for later resumption.
    let _callee_context = agent.pop_execution_context();
    // 13. Return ? result.
    result
}

pub(crate) struct BuiltinConstructorArgs<'a> {
    pub(crate) is_derived: bool,
    pub(crate) class_name: String<'a>,
    pub(crate) prototype: Option<Object<'a>>,
    pub(crate) prototype_property: Object<'a>,
    pub(crate) compiled_initializer_bytecode: Option<Executable<'a>>,
    pub(crate) env: Environment<'a>,
    pub(crate) private_env: Option<PrivateEnvironment<'a>>,
    pub(crate) source_code: SourceCode<'a>,
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
pub(crate) fn create_builtin_constructor<'a>(
    agent: &mut Agent,
    args: BuiltinConstructorArgs,
    gc: NoGcScope<'a, '_>,
) -> BuiltinConstructorFunction<'a> {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm = agent.current_realm(gc);

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
    agent
        .heap
        .create(BuiltinConstructorHeapData {
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
        .bind(gc)
}

impl Rootable for BuiltinConstructorFunction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::BuiltinConstructorFunction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BuiltinConstructorFunction(d) => Some(d),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<BuiltinConstructorHeapData<'a>, BuiltinConstructorFunction<'a>> for Heap {
    fn create(&mut self, data: BuiltinConstructorHeapData) -> BuiltinConstructorFunction<'a> {
        self.builtin_constructors.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<BuiltinConstructorHeapData<'static>>>();

        BuiltinConstructorIndex::last(&self.builtin_constructors).into()
    }
}

impl HeapMarkAndSweep for BuiltinConstructorFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.builtin_constructors.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.builtin_constructors.shift_index(&mut self.0);
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for BuiltinConstructorHeapData<'_> {
    type Of<'a> = BuiltinConstructorHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for BuiltinConstructorHeapData<'static> {
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
