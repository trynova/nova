// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Deref, Index, IndexMut};

use crate::ecmascript::types::{function_try_get, function_try_has_property, function_try_set};
use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable};
use crate::engine::{Scoped, TryResult};
use crate::{
    ecmascript::{
        execution::{
            Agent, ExecutionContext, JsResult, ProtoIntrinsics, RealmIdentifier,
            agent::ExceptionType,
        },
        types::{
            BUILTIN_STRING_MEMORY, BuiltinFunctionHeapData, Function, FunctionInternalProperties,
            InternalMethods, InternalSlots, IntoFunction, IntoObject, IntoValue, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value,
            function_create_backing_object, function_internal_define_own_property,
            function_internal_delete, function_internal_get, function_internal_get_own_property,
            function_internal_has_property, function_internal_own_property_keys,
            function_internal_set,
        },
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, IntrinsicConstructorIndexes,
        IntrinsicFunctionIndexes, ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues,
        indexes::BuiltinFunctionIndex,
    },
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ArgumentsList<'a>(pub(crate) &'a [Value<'static>]);

impl<'a> Deref for ArgumentsList<'a> {
    type Target = &'a [Value<'static>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ArgumentsList<'_> {
    #[inline]
    pub fn get(&self, index: usize) -> Value {
        *self.0.get(index).unwrap_or(&Value::Undefined)
    }
}

pub type RegularFn =
    for<'gc> fn(&mut Agent, Value, ArgumentsList, GcScope<'gc, '_>) -> JsResult<Value<'gc>>;
pub type ConstructorFn = for<'gc> fn(
    &mut Agent,
    Value,
    ArgumentsList,
    Option<Object>,
    GcScope<'gc, '_>,
) -> JsResult<Value<'gc>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    Regular(RegularFn),
    Constructor(ConstructorFn),
}

impl Behaviour {
    pub(crate) fn is_constructor(&self) -> bool {
        matches!(self, Behaviour::Constructor(_))
    }
}

pub trait Builtin {
    const NAME: String<'static>;
    const LENGTH: u8;
    const BEHAVIOUR: Behaviour;

    /// Set to Some if this builtin's property key is different from `NAME`.
    const KEY: Option<PropertyKey<'static>> = None;

    /// If the builtin function is created as a property then this controls the
    /// property's `[[Writable]]` value.
    const WRITABLE: bool = true;
    /// If the builtin function is created as a property then this controls the
    /// property's `[[Enumerable]]` value.
    const ENUMERABLE: bool = false;
    /// If the builtin function is created as a property then this controls the
    /// property's `[[Configurable]]` value.
    const CONFIGURABLE: bool = true;
}

pub(crate) trait BuiltinIntrinsicConstructor: Builtin {
    const INDEX: IntrinsicConstructorIndexes;
}

pub(crate) trait BuiltinIntrinsic: Builtin {
    const INDEX: IntrinsicFunctionIndexes;
}
pub trait BuiltinGetter: Builtin {}

#[derive(Debug, Default)]
pub struct BuiltinFunctionArgs<'a> {
    pub length: u32,
    pub name: &'static str,
    pub realm: Option<RealmIdentifier>,
    pub prototype: Option<Object<'a>>,
    pub prefix: Option<&'static str>,
}

impl BuiltinFunctionArgs<'static> {
    pub fn new(length: u32, name: &'static str, realm: RealmIdentifier) -> Self {
        Self {
            length,
            name,
            realm: Some(realm),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinFunction<'a>(pub(crate) BuiltinFunctionIndex<'a>);

impl<'a> BuiltinFunction<'a> {
    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, BuiltinFunction<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BuiltinFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn is_constructor(self, agent: &Agent) -> bool {
        // A builtin function has the [[Construct]] method if its behaviour is
        // a constructor behaviour.
        agent[self].behaviour.is_constructor()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for BuiltinFunction<'_> {
    type Of<'a> = BuiltinFunction<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<BuiltinFunctionIndex<'a>> for BuiltinFunction<'a> {
    fn from(value: BuiltinFunctionIndex<'a>) -> Self {
        Self(value)
    }
}

impl<'a> IntoValue<'a> for BuiltinFunction<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for BuiltinFunction<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> IntoFunction<'a> for BuiltinFunction<'a> {
    fn into_function(self) -> Function<'a> {
        self.into()
    }
}

impl<'a> From<BuiltinFunction<'a>> for Value<'a> {
    fn from(value: BuiltinFunction<'a>) -> Self {
        Value::BuiltinFunction(value)
    }
}

impl<'a> From<BuiltinFunction<'a>> for Object<'a> {
    fn from(value: BuiltinFunction) -> Self {
        Object::BuiltinFunction(value.unbind())
    }
}

impl<'a> From<BuiltinFunction<'a>> for Function<'a> {
    fn from(value: BuiltinFunction<'a>) -> Self {
        Function::BuiltinFunction(value)
    }
}

impl Index<BuiltinFunction<'_>> for Agent {
    type Output = BuiltinFunctionHeapData;

    fn index(&self, index: BuiltinFunction) -> &Self::Output {
        &self.heap.builtin_functions[index]
    }
}

impl IndexMut<BuiltinFunction<'_>> for Agent {
    fn index_mut(&mut self, index: BuiltinFunction) -> &mut Self::Output {
        &mut self.heap.builtin_functions[index]
    }
}

impl Index<BuiltinFunction<'_>> for Vec<Option<BuiltinFunctionHeapData>> {
    type Output = BuiltinFunctionHeapData;

    fn index(&self, index: BuiltinFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinFunction out of bounds")
            .as_ref()
            .expect("BuiltinFunction slot empty")
    }
}

impl IndexMut<BuiltinFunction<'_>> for Vec<Option<BuiltinFunctionHeapData>> {
    fn index_mut(&mut self, index: BuiltinFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinFunction out of bounds")
            .as_mut()
            .expect("BuiltinFunction slot empty")
    }
}

impl<'a> FunctionInternalProperties<'a> for BuiltinFunction<'a> {
    fn get_name(self, agent: &Agent) -> String<'static> {
        agent[self].initial_name.unwrap_or(String::EMPTY_STRING)
    }

    fn get_length(self, agent: &Agent) -> u8 {
        agent[self].length
    }
}

impl<'a> InternalSlots<'a> for BuiltinFunction<'a> {
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
}

impl<'a> InternalMethods<'a> for BuiltinFunction<'a> {
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _gc: NoGcScope,
    ) -> TryResult<Option<PropertyDescriptor>> {
        TryResult::Continue(function_internal_get_own_property(
            self,
            agent,
            property_key,
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

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<bool> {
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
    ) -> JsResult<Value<'gc>> {
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

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<bool> {
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
        this_argument: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Return ? BuiltinCallOrConstruct(F, thisArgument, argumentsList, undefined).
        builtin_call_or_construct(agent, self, Some(this_argument), arguments_list, None, gc)
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
    ) -> JsResult<Object<'gc>> {
        // 1. Return ? BuiltinCallOrConstruct(F, uninitialized, argumentsList, newTarget).
        builtin_call_or_construct(agent, self, None, arguments_list, Some(new_target), gc)
            .map(|result| result.try_into().unwrap())
    }
}

/// ### [10.3.3 BuiltinCallOrConstruct ( F, thisArgument, argumentsList, newTarget )](https://tc39.es/ecma262/#sec-builtincallorconstruct)
///
/// The abstract operation BuiltinCallOrConstruct takes arguments F (a built-in
/// function object), thisArgument (an ECMAScript language value or
/// uninitialized), argumentsList (a List of ECMAScript language values), and
/// newTarget (a constructor or undefined) and returns either a normal
/// completion containing an ECMAScript language value or a throw completion.
pub(crate) fn builtin_call_or_construct<'gc>(
    agent: &mut Agent,
    f: BuiltinFunction,
    this_argument: Option<Value>,
    arguments_list: ArgumentsList,
    new_target: Option<Function>,
    gc: GcScope<'gc, '_>,
) -> JsResult<Value<'gc>> {
    let f = f.bind(gc.nogc());
    let new_target = new_target.map(|f| f.bind(gc.nogc()));
    // 1. Let callerContext be the running execution context.
    let caller_context = agent.running_execution_context();
    // 2. If callerContext is not already suspended, suspend callerContext.
    caller_context.suspend();
    // 5. Let calleeRealm be F.[[Realm]].
    let Agent {
        heap: Heap {
            builtin_functions, ..
        },
        execution_context_stack,
        ..
    } = agent;
    let heap_data = &builtin_functions[f];
    let callee_realm = heap_data.realm;
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
    execution_context_stack.push(callee_context);
    // 10. Let result be the Completion Record that is the result of evaluating F in a manner that conforms to
    // the specification of F. If thisArgument is uninitialized, the this value is uninitialized; otherwise,
    // thisArgument provides the this value. argumentsList provides the named parameters. newTarget provides the NewTarget value.
    let func = heap_data.behaviour;
    let result = match func {
        Behaviour::Regular(func) => {
            if new_target.is_some() {
                Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Not a constructor",
                    gc.nogc(),
                ))
            } else {
                func(
                    agent,
                    this_argument.unwrap_or(Value::Undefined),
                    arguments_list,
                    gc,
                )
            }
        }
        Behaviour::Constructor(func) => func(
            agent,
            this_argument.unwrap_or(Value::Undefined),
            arguments_list,
            new_target.map(|target| target.into_object().unbind()),
            gc,
        ),
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
pub fn create_builtin_function<'a>(
    agent: &mut Agent,
    behaviour: Behaviour,
    args: BuiltinFunctionArgs,
    gc: NoGcScope<'a, '_>,
) -> BuiltinFunction<'a> {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm = args.realm.unwrap_or(agent.current_realm_id());

    // 9. Set func.[[InitialName]] to null.
    // Note: SetFunctionName inlined here: We know name is a string
    let initial_name = if let Some(prefix) = args.prefix {
        // 12. Else,
        // a. Perform SetFunctionName(func, name, prefix).
        String::from_string(agent, format!("{} {}", args.name, prefix), gc)
    } else {
        // 11. If prefix is not present, then
        // a. Perform SetFunctionName(func, name).
        String::from_str(agent, args.name, gc)
    };

    // 2. If prototype is not present, set prototype to realm.[[Intrinsics]].[[%Function.prototype%]].

    // 3. Let internalSlotsList be a List containing the names of all the internal slots that 10.3
    //    requires for the built-in function object that is about to be created.
    // 4. Append to internalSlotsList the elements of additionalInternalSlotsList.
    // Note: The BuiltinFunctionHeapData implements all internal slots that 10.3 requires.
    // The currently appearing spec-defined additional slots are:
    // * [[ConstructorKind]] and [[SourceText]] for class constructors.
    // * [[Promise]] and [[AlreadyResolved]] for Promise resolver functions
    // * [[AlreadyCalled]], [[Index]], [[Values]], [[Capability]], and [[RemainingElements]] for
    //   Promise.all's onFulfilled function.
    // We do not yet support these, and how these end up supported is not yet fully clear.

    // 5. Let func be a new built-in function object that, when called, performs the action
    //    described by behaviour using the provided arguments as the values of the corresponding
    //    parameters specified by behaviour. The new function object has internal slots whose names
    //    are the elements of internalSlotsList, and an [[InitialName]] internal slot.
    let object_index = if let Some(prototype) = args.prototype {
        // If a prototype is set, then check that it is not the %Function.prototype%
        let realm_function_prototype = agent
            .get_realm(realm)
            .intrinsics()
            .get_intrinsic_default_proto(BuiltinFunction::DEFAULT_PROTOTYPE);
        if prototype == realm_function_prototype {
            // If the prototype matched the realm function prototype, then ignore it
            // as the BuiltinFunctionHeapData indirectly implies this prototype.
            None
        } else {
            // If some other prototype is defined then we need to create a backing object.
            // 6. Set func.[[Prototype]] to prototype.
            // 7. Set func.[[Extensible]] to true.
            let length_entry = ObjectEntry {
                key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: args.length.into(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let name_entry = ObjectEntry {
                key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: initial_name.into_value(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            Some(
                agent
                    .heap
                    .create_object_with_prototype(prototype, &[length_entry, name_entry]),
            )
        }
    } else {
        None
    };

    // 13. Return func.
    agent.heap.create(BuiltinFunctionHeapData {
        behaviour,
        initial_name: Some(initial_name.unbind()),
        // 10. Perform SetFunctionLength(func, length).
        length: args.length as u8,
        // 8. Set func.[[Realm]] to realm.
        realm,
        object_index,
    })
}

impl CreateHeapData<BuiltinFunctionHeapData, BuiltinFunction<'static>> for Heap {
    fn create(&mut self, data: BuiltinFunctionHeapData) -> BuiltinFunction<'static> {
        self.builtin_functions.push(Some(data));
        BuiltinFunctionIndex::last(&self.builtin_functions).into()
    }
}

impl Rootable for BuiltinFunction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::BuiltinFunction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BuiltinFunction(d) => Some(d),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for BuiltinFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.builtin_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.builtin_functions.shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for BuiltinFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            length: _,
            realm,
            initial_name,
            behaviour: _,
        } = self;
        realm.mark_values(queues);
        initial_name.mark_values(queues);
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            length: _,
            realm,
            initial_name,
            behaviour: _,
        } = self;
        realm.sweep_values(compactions);
        initial_name.sweep_values(compactions);
        object_index.sweep_values(compactions);
    }
}
