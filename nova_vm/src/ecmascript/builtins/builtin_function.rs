use std::ops::Deref;

use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::same_value_non_number,
        execution::{agent::ExceptionType, Agent, ExecutionContext, JsResult, RealmIdentifier},
        types::{
            property_builder, BuiltinFunctionHeapData, Function, InternalMethods, IntoFunction,
            IntoObject, IntoValue, Object, ObjectHeapData, OrdinaryObject,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, String, Value,
        },
    },
    heap::{
        element_array::ElementDescriptor,
        indexes::{BuiltinFunctionIndex, ObjectIndex},
        CreateHeapData, GetHeapData, ObjectEntry, ObjectEntryPropertyDescriptor,
    },
};

use property_builder::PropertyBuilder;

use super::ordinary::ordinary_set_prototype_of_check_loop;

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

impl IntoValue for BuiltinFunctionIndex {
    fn into_value(self) -> Value {
        Value::BuiltinFunction(self)
    }
}

impl IntoObject for BuiltinFunctionIndex {
    fn into_object(self) -> Object {
        Object::BuiltinFunction(self)
    }
}

impl IntoFunction for BuiltinFunctionIndex {
    fn into_function(self) -> Function {
        Function::BuiltinFunction(self)
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
    const NAME: &'static str;
    const LENGTH: u8;
    const BEHAVIOUR: Behaviour;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinFunction(BuiltinFunctionIndex);

impl BuiltinFunction {
    pub(crate) const fn from_index(index: BuiltinFunctionIndex) -> Self {
        Self(index)
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
        Value::BuiltinFunction(value.0)
    }
}

impl From<BuiltinFunction> for Object {
    fn from(value: BuiltinFunction) -> Self {
        Object::BuiltinFunction(value.0)
    }
}

impl From<BuiltinFunction> for Function {
    fn from(value: BuiltinFunction) -> Self {
        Function::BuiltinFunction(value.0)
    }
}

impl OrdinaryObjectInternalSlots for BuiltinFunction {
    fn extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).extensible(agent)
        } else {
            true
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set_extensible(agent, value)
        } else {
            // Create function base object and set inextensible
            todo!()
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).prototype(agent)
        } else {
            Some(agent.current_realm().intrinsics().function_prototype())
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set_prototype(agent, prototype)
        } else {
            // Create function base object and set inextensible
            todo!()
        }
    }
}

impl InternalMethods for BuiltinFunction {
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.prototype(agent))
    }

    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set_prototype_of(agent, prototype)
        } else {
            // If we're setting %ArrayBuffer.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().function_prototype();
            if same_value_non_number(agent, prototype, Some(current)) {
                return Ok(true);
            }
            if ordinary_set_prototype_of_check_loop(agent, current, prototype) {
                // OrdinarySetPrototypeOf 7.b.i: Setting prototype would cause a loop to occur.
                return Ok(false);
            }
            self.set_prototype(agent, prototype);
            Ok(true)
        }
    }

    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        Ok(self.extensible(agent))
    }

    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        self.set_extensible(agent, false);
        Ok(true)
    }

    fn get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).get_own_property(agent, property_key)
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "length") {
            Ok(Some(PropertyDescriptor {
                value: Some(agent.heap.get(self.0).length.into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "name") {
            Ok(Some(PropertyDescriptor {
                value: Some(agent.heap.get(self.0).initial_name.into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
        } else {
            Ok(None)
        }
    }

    fn define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).has_property(agent, property_key)
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "length") {
            let prototype = agent.current_realm().intrinsics().array_prototype();
            let length_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let name_entry = ObjectEntry {
                key: PropertyKey::from_str(&mut agent.heap, "name"),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: agent.heap.get(self.0).initial_name.unwrap().into_value(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype, vec![length_entry, name_entry]);
            agent.heap.get_mut(self.0).object_index = Some(object_index);
            Ok(true)
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "name") {
            let prototype = agent.current_realm().intrinsics().array_prototype();
            let length_entry = ObjectEntry {
                key: PropertyKey::from_str(&mut agent.heap, "length"),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: agent.heap.get(self.0).length.into(),
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
                .create_object_with_prototype(prototype, vec![length_entry, name_entry]);
            agent.heap.get_mut(self.0).object_index = Some(object_index);
            Ok(true)
        } else {
            let prototype = agent.current_realm().intrinsics().array_prototype();
            let length_entry = ObjectEntry {
                key: PropertyKey::from_str(&mut agent.heap, "length"),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: agent.heap.get(self.0).length.into(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let name_entry = ObjectEntry {
                key: PropertyKey::from_str(&mut agent.heap, "name"),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: agent.heap.get(self.0).initial_name.unwrap().into_value(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let other_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let object_index = agent.heap.create_object_with_prototype(
                prototype,
                vec![length_entry, name_entry, other_entry],
            );
            agent.heap.get_mut(self.0).object_index = Some(object_index);
            Ok(true)
        }
    }

    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).has_property(agent, property_key)
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "length")
            || property_key == PropertyKey::from_str(&mut agent.heap, "name")
        {
            Ok(true)
        } else {
            let parent = self.get_prototype_of(agent)?;
            parent.map_or(Ok(false), |parent| parent.has_property(agent, property_key))
        }
    }

    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).get(agent, property_key, receiver)
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "length") {
            Ok(agent.heap.get(self.0).length.into())
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "name") {
            Ok(agent.heap.get(self.0).initial_name.into())
        } else {
            let parent = self.get_prototype_of(agent)?;
            parent.map_or(Ok(Value::Undefined), |parent| {
                parent.get(agent, property_key, receiver)
            })
        }
    }

    fn set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set(agent, property_key, value, receiver)
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "length")
            || property_key == PropertyKey::from_str(&mut agent.heap, "name")
        {
            // length and name are not writable
            Ok(false)
        } else {
            let prototype = agent.current_realm().intrinsics().array_prototype();
            prototype.set(agent, property_key, value, receiver)
        }
    }

    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).delete(agent, property_key)
        } else if property_key == PropertyKey::from_str(&mut agent.heap, "length")
            || property_key == PropertyKey::from_str(&mut agent.heap, "name")
        {
            let prototype = agent.current_realm().intrinsics().array_prototype();
            let entry = if property_key == PropertyKey::from_str(&mut agent.heap, "length") {
                ObjectEntry {
                    key: PropertyKey::from_str(&mut agent.heap, "length"),
                    value: ObjectEntryPropertyDescriptor::Data {
                        value: agent.heap.get(self.0).length.into(),
                        writable: false,
                        enumerable: false,
                        configurable: true,
                    },
                }
            } else {
                ObjectEntry {
                    key: PropertyKey::from_str(&mut agent.heap, "name"),
                    value: ObjectEntryPropertyDescriptor::Data {
                        value: agent.heap.get(self.0).initial_name.unwrap().into_value(),
                        writable: false,
                        enumerable: false,
                        configurable: true,
                    },
                }
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype, vec![entry]);
            agent.heap.get_mut(self.0).object_index = Some(object_index);
            Ok(true)
        } else {
            // Non-existing property
            Ok(true)
        }
    }

    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).own_property_keys(agent)
        } else {
            Ok(vec![
                PropertyKey::from_str(&mut agent.heap, "length"),
                PropertyKey::from_str(&mut agent.heap, "name"),
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
    fn call(
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
    fn construct(
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
        heap,
        execution_context_stack,
        ..
    } = agent;
    let heap_data = heap.get(f.0);
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
        String::from_str(agent, &format!("{} {}", args.name, prefix))
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
        let realm_function_prototype = agent.get_realm(realm).intrinsics().function_prototype();
        if prototype == realm_function_prototype {
            // If the prototype matched the realm function prototype, then ignore it
            // as the BuiltinFunctionHeapData indirectly implies this prototype.
            None
        } else {
            // If some other prototype is defined then we need to create a backing object.
            // 6. Set func.[[Prototype]] to prototype.
            // 7. Set func.[[Extensible]] to true.
            let length_entry = ObjectEntry {
                key: PropertyKey::from_str(&mut agent.heap, "length"),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: args.length.into(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let name_entry = ObjectEntry {
                key: PropertyKey::from_str(&mut agent.heap, "name"),
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
                    .create_object_with_prototype(prototype, vec![length_entry, name_entry]),
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

#[derive(Default, Clone, Copy)]
pub struct NoPrototype;

#[derive(Clone, Copy)]
pub struct CreatorPrototype(Object);

#[derive(Default, Clone, Copy)]
pub struct NoLength;

#[derive(Clone, Copy)]
pub struct CreatorLength(u8);

#[derive(Default, Clone, Copy)]
pub struct NoName;

#[derive(Clone, Copy)]
pub struct CreatorName(String);

#[derive(Default, Clone, Copy)]
pub struct NoBehaviour;

#[derive(Clone, Copy)]
pub struct CreatorBehaviour(Behaviour);

#[derive(Default, Clone, Copy)]
pub struct NoProperties;

#[derive(Clone)]
pub struct CreatorProperties(Vec<(PropertyKey, Option<ElementDescriptor>, Option<Value>)>);

pub struct BuiltinFunctionBuilder<'agent, P, L, N, B, Pr> {
    pub(crate) agent: &'agent mut Agent,
    this: BuiltinFunction,
    object_index: Option<ObjectIndex>,
    prototype: P,
    length: L,
    name: N,
    behaviour: B,
    properties: Pr,
}

impl<'agent>
    BuiltinFunctionBuilder<'agent, NoPrototype, NoLength, NoName, NoBehaviour, NoProperties>
{
    pub fn new<T: Builtin>(
        agent: &'agent mut Agent,
    ) -> BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    > {
        agent.heap.builtin_functions.push(None);
        let this = BuiltinFunctionIndex::last(&agent.heap.builtin_functions).into();
        let name = String::from_str(agent, T::NAME);
        BuiltinFunctionBuilder {
            agent,
            this,
            object_index: None,
            prototype: Default::default(),
            length: CreatorLength(T::LENGTH),
            name: CreatorName(name),
            behaviour: CreatorBehaviour(T::BEHAVIOUR),
            properties: Default::default(),
        }
    }
}

impl<'agent, P, L, N, Pr> BuiltinFunctionBuilder<'agent, P, L, N, NoBehaviour, Pr> {
    pub fn with_behaviour(
        self,
        behaviour: Behaviour,
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, CreatorBehaviour, Pr> {
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: CreatorBehaviour(behaviour),
            properties: self.properties,
        }
    }
}

impl<'agent, L, N, B, Pr> BuiltinFunctionBuilder<'agent, NoPrototype, L, N, B, Pr> {
    pub fn with_prototype(
        self,
        prototype: Object,
    ) -> BuiltinFunctionBuilder<'agent, CreatorPrototype, L, N, B, Pr> {
        let object_index = if prototype
            != self.agent.current_realm().intrinsics().function_prototype()
            && self.object_index.is_none()
        {
            self.agent.heap.objects.push(None);
            Some(ObjectIndex::last(&self.agent.heap.objects))
        } else {
            self.object_index
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            prototype: CreatorPrototype(prototype),
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent, P, N, B, Pr> BuiltinFunctionBuilder<'agent, P, NoLength, N, B, Pr> {
    pub fn with_length(
        self,
        length: u8,
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, N, B, Pr> {
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            prototype: self.prototype,
            length: CreatorLength(length),
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent, P, L, B, Pr> BuiltinFunctionBuilder<'agent, P, L, NoName, B, Pr> {
    pub fn with_name_from_str(
        self,
        str: &str,
    ) -> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
        let name = String::from_str(self.agent, str);
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            prototype: self.prototype,
            length: self.length,
            name: CreatorName(name),
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    pub fn with_prefixed_name_from_str(
        self,
        prefix: &str,
        name: &str,
    ) -> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
        let name = String::from_str(self.agent, &format!("{} {}", name, prefix));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            prototype: self.prototype,
            length: self.length,
            name: CreatorName(name),
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    pub fn with_name_from_string(
        self,
        name: String,
    ) -> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            prototype: self.prototype,
            length: self.length,
            name: CreatorName(name),
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent, P, L, N, B> BuiltinFunctionBuilder<'agent, P, L, N, B, NoProperties> {
    pub fn with_data_property(
        self,
        key: PropertyKey,
        value: Value,
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let object_index = Some(self.object_index.unwrap_or_else(|| {
            self.agent.heap.objects.push(None);
            ObjectIndex::last(&self.agent.heap.objects)
        }));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: CreatorProperties(vec![(key, None, Some(value))]),
        }
    }

    pub fn with_property(
        self,
        creator: impl FnOnce(
            PropertyBuilder<'_, property_builder::NoKey, property_builder::NoDefinition>,
        ) -> (PropertyKey, Option<ElementDescriptor>, Option<Value>),
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let object_index = Some(self.object_index.unwrap_or_else(|| {
            self.agent.heap.objects.push(None);
            ObjectIndex::last(&self.agent.heap.objects)
        }));
        let property = {
            let builder = PropertyBuilder::new(self.agent, self.this.into_object());
            creator(builder)
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: CreatorProperties(vec![property]),
        }
    }
}

impl<'agent, P, L, N, B> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
    pub fn with_data_property(
        mut self,
        key: PropertyKey,
        value: Value,
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        self.properties.0.push((key, None, Some(value)));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    pub fn with_property(
        mut self,
        creator: impl FnOnce(
            PropertyBuilder<'_, property_builder::NoKey, property_builder::NoDefinition>,
        ) -> (PropertyKey, Option<ElementDescriptor>, Option<Value>),
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let builder = PropertyBuilder::new(self.agent, self.this.into_object());
        let property = creator(builder);
        self.properties.0.push(property);
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent>
    BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    >
{
    pub fn build(&mut self) -> BuiltinFunction {
        let data = BuiltinFunctionHeapData {
            object_index: None,
            length: self.length.0,
            realm: self.agent.current_realm_id(),
            initial_name: Some(self.name.0),
            behaviour: self.behaviour.0,
        };

        let slot = self
            .agent
            .heap
            .builtin_functions
            .get_mut(self.this.0.into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(data);
        self.this
    }
}

impl<'agent>
    BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        CreatorProperties,
    >
{
    pub fn build(self) -> BuiltinFunction {
        let Self {
            agent,
            length,
            name,
            behaviour,
            properties,
            object_index,
            ..
        } = self;
        let properties = properties.0;

        let (keys, values) = agent.heap.elements.create_with_stuff(properties);

        let prototype = agent
            .current_realm()
            .intrinsics()
            .function_prototype()
            .into();
        let slot = agent
            .heap
            .objects
            .get_mut(object_index.unwrap().into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: true,
            prototype,
            keys,
            values,
        });

        let data = BuiltinFunctionHeapData {
            object_index,
            length: length.0,
            realm: agent.current_realm_id(),
            initial_name: Some(name.0),
            behaviour: behaviour.0,
        };

        let slot = agent
            .heap
            .builtin_functions
            .get_mut(self.this.0.into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(data);
        self.this
    }
}

impl<'agent>
    BuiltinFunctionBuilder<
        'agent,
        CreatorPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        CreatorProperties,
    >
{
    pub fn build(self) -> BuiltinFunction {
        let Self {
            agent,
            length,
            name,
            behaviour,
            properties,
            object_index,
            prototype,
            ..
        } = self;
        let properties = properties.0;

        let (keys, values) = agent.heap.elements.create_with_stuff(properties);

        let slot = agent
            .heap
            .objects
            .get_mut(object_index.unwrap().into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype.0),
            keys,
            values,
        });

        let data = BuiltinFunctionHeapData {
            object_index,
            length: length.0,
            realm: agent.current_realm_id(),
            initial_name: Some(name.0),
            behaviour: behaviour.0,
        };

        let slot = agent
            .heap
            .builtin_functions
            .get_mut(self.this.0.into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(data);
        self.this
    }
}
