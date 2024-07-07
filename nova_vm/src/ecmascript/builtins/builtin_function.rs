use std::ops::{Deref, Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{
            agent::ExceptionType, Agent, ExecutionContext, JsResult, ProtoIntrinsics,
            RealmIdentifier,
        },
        types::{
            BuiltinFunctionHeapData, Function, InternalMethods, InternalSlots, IntoFunction,
            IntoObject, IntoValue, Object, ObjectHeapData, PropertyDescriptor, PropertyKey, String,
            Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::BuiltinFunctionIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, ObjectEntry,
        ObjectEntryPropertyDescriptor, WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ArgumentsList<'a>(pub(crate) &'a [Value]);

impl<'a> Deref for ArgumentsList<'a> {
    type Target = &'a [Value];

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

pub type RegularFn = fn(&mut Agent, Value, ArgumentsList<'_>) -> JsResult<Value>;
pub type ConstructorFn =
    fn(&mut Agent, Value, ArgumentsList<'_>, Option<Object>) -> JsResult<Value>;

#[derive(Debug, Clone, Copy)]
pub enum Behaviour {
    Regular(RegularFn),
    Constructor(ConstructorFn),
}

pub trait Builtin {
    const NAME: String;
    const LENGTH: u8;
    const BEHAVIOUR: Behaviour;

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
pub trait BuiltinGetter: Builtin {
    const KEY: PropertyKey;
}

#[derive(Debug, Default)]
pub struct BuiltinFunctionArgs {
    pub length: u32,
    pub name: &'static str,
    pub realm: Option<RealmIdentifier>,
    pub prototype: Option<Object>,
    pub prefix: Option<&'static str>,
}

impl BuiltinFunctionArgs {
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
pub struct BuiltinFunction(pub(crate) BuiltinFunctionIndex);

impl BuiltinFunction {
    pub(crate) const fn _def() -> Self {
        Self(BuiltinFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<BuiltinFunctionIndex> for BuiltinFunction {
    fn from(value: BuiltinFunctionIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for BuiltinFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for BuiltinFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoFunction for BuiltinFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<BuiltinFunction> for Value {
    fn from(value: BuiltinFunction) -> Self {
        Value::BuiltinFunction(value)
    }
}

impl From<BuiltinFunction> for Object {
    fn from(value: BuiltinFunction) -> Self {
        Object::BuiltinFunction(value)
    }
}

impl From<BuiltinFunction> for Function {
    fn from(value: BuiltinFunction) -> Self {
        Function::BuiltinFunction(value)
    }
}

impl Index<BuiltinFunction> for Agent {
    type Output = BuiltinFunctionHeapData;

    fn index(&self, index: BuiltinFunction) -> &Self::Output {
        &self.heap.builtin_functions[index]
    }
}

impl IndexMut<BuiltinFunction> for Agent {
    fn index_mut(&mut self, index: BuiltinFunction) -> &mut Self::Output {
        &mut self.heap.builtin_functions[index]
    }
}

impl Index<BuiltinFunction> for Vec<Option<BuiltinFunctionHeapData>> {
    type Output = BuiltinFunctionHeapData;

    fn index(&self, index: BuiltinFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinFunction out of bounds")
            .as_ref()
            .expect("BuiltinFunction slot empty")
    }
}

impl IndexMut<BuiltinFunction> for Vec<Option<BuiltinFunctionHeapData>> {
    fn index_mut(&mut self, index: BuiltinFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinFunction out of bounds")
            .as_mut()
            .expect("BuiltinFunction slot empty")
    }
}

impl InternalSlots for BuiltinFunction {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
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
                    .initial_name
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
            prototype: Some(prototype),
            keys,
            values,
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl InternalMethods for BuiltinFunction {
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let Some(object_index) = self.get_backing_object(agent) {
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
                value: Some(agent[self].initial_name.into()),
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
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_define_own_property(agent, property_key, property_descriptor)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            let prototype = agent
                .current_realm()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
            let length_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let name_entry = ObjectEntry {
                key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: agent[self].initial_name.unwrap().into_value(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype, &[length_entry, name_entry]);
            agent[self].object_index = Some(object_index);
            Ok(true)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
            let prototype = agent
                .current_realm()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
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
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype, &[length_entry, name_entry]);
            agent[self].object_index = Some(object_index);
            Ok(true)
        } else {
            let prototype = agent
                .current_realm()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
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
                    value: agent[self].initial_name.unwrap().into_value(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let other_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype, &[length_entry, name_entry, other_entry]);
            agent[self].object_index = Some(object_index);
            Ok(true)
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = self.get_backing_object(agent) {
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
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_get(agent, property_key, receiver)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(agent[self].length.into())
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
            Ok(agent[self].initial_name.into())
        } else {
            let parent = self.internal_get_prototype_of(agent)?;
            parent.map_or(Ok(Value::Undefined), |parent| {
                parent.internal_get(agent, property_key, receiver)
            })
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
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

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = self.get_backing_object(agent) {
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

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_own_property_keys(agent)
        } else {
            Ok(vec![
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            ])
        }
    }

    /// ### [10.3.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-built-in-function-objects-call-thisargument-argumentslist)
    ///
    /// The [[Call]] internal method of a built-in function object F takes
    /// arguments thisArgument (an ECMAScript language value) and argumentsList
    /// (a List of ECMAScript language values) and returns either a normal
    /// completion containing an ECMAScript language value or a throw
    /// completion.
    fn internal_call(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Return ? BuiltinCallOrConstruct(F, thisArgument, argumentsList, undefined).
        builtin_call_or_construct(agent, self, Some(this_argument), arguments_list, None)
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
        builtin_call_or_construct(agent, self, None, arguments_list, Some(new_target))
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
pub(crate) fn builtin_call_or_construct(
    agent: &mut Agent,
    f: BuiltinFunction,
    this_argument: Option<Value>,
    arguments_list: ArgumentsList,
    new_target: Option<Function>,
) -> JsResult<Value> {
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
        function: Some(f.into()),
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
                Err(agent.throw_exception(ExceptionType::TypeError, "Not a constructor"))
            } else {
                func(
                    agent,
                    this_argument.unwrap_or(Value::Undefined),
                    arguments_list,
                )
            }
        }
        Behaviour::Constructor(func) => func(
            agent,
            this_argument.unwrap_or(Value::Undefined),
            arguments_list,
            new_target.map(|target| target.into_object()),
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
pub fn create_builtin_function(
    agent: &mut Agent,
    behaviour: Behaviour,
    args: BuiltinFunctionArgs,
) -> BuiltinFunction {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm = args.realm.unwrap_or(agent.current_realm_id());

    // 9. Set func.[[InitialName]] to null.
    // Note: SetFunctionName inlined here: We know name is a string
    let initial_name = if let Some(prefix) = args.prefix {
        // 12. Else,
        // a. Perform SetFunctionName(func, name, prefix).
        String::from_string(agent, format!("{} {}", args.name, prefix))
    } else {
        // 11. If prefix is not present, then
        // a. Perform SetFunctionName(func, name).
        String::from_str(agent, args.name)
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
        initial_name: Some(initial_name),
        // 10. Perform SetFunctionLength(func, length).
        length: args.length as u8,
        // 8. Set func.[[Realm]] to realm.
        realm,
        object_index,
    })
}

pub fn define_builtin_function(
    agent: &mut Agent,
    _object: Object,
    name: &'static str,
    behaviour: RegularFn,
    length: u32,
    realm: RealmIdentifier,
) -> JsResult<()> {
    let _function = create_builtin_function(
        agent,
        Behaviour::Regular(behaviour),
        BuiltinFunctionArgs::new(length, name, realm),
    );

    Ok(())
}

pub fn define_builtin_property(
    _object: Object,
    _name: &'static str,
    _descriptor: PropertyDescriptor,
) -> JsResult<()> {
    Ok(())
}

pub fn todo_builtin(agent: &mut Agent, _: Value, _: ArgumentsList) -> JsResult<Value> {
    agent.throw_exception(
        crate::ecmascript::execution::agent::ExceptionType::SyntaxError,
        "TODO: Builtin not implemented.",
    );
    Err(Default::default())
}

impl CreateHeapData<BuiltinFunctionHeapData, BuiltinFunction> for Heap {
    fn create(&mut self, data: BuiltinFunctionHeapData) -> BuiltinFunction {
        self.builtin_functions.push(Some(data));
        BuiltinFunctionIndex::last(&self.builtin_functions).into()
    }
}

impl HeapMarkAndSweep for BuiltinFunction {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.builtin_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = BuiltinFunctionIndex::from_u32(
            self_index
                - compactions
                    .builtin_functions
                    .get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep for BuiltinFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.initial_name.mark_values(queues);
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.initial_name.sweep_values(compactions);
        self.object_index.sweep_values(compactions);
    }
}
