use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, construct},
            testing_and_comparison::is_constructor,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            BoundFunctionHeapData, Function, InternalMethods, InternalSlots, IntoFunction,
            IntoObject, IntoValue, Object, ObjectHeapData, PropertyDescriptor, PropertyKey, String,
            Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::BoundFunctionIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues,
    },
};

use super::ArgumentsList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BoundFunction(BoundFunctionIndex);

impl BoundFunction {
    pub(crate) const fn _def() -> Self {
        BoundFunction(BoundFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for BoundFunction {
    fn into_value(self) -> Value {
        Value::BoundFunction(self)
    }
}

impl IntoObject for BoundFunction {
    fn into_object(self) -> Object {
        Object::BoundFunction(self)
    }
}

impl IntoFunction for BoundFunction {
    fn into_function(self) -> Function {
        Function::BoundFunction(self)
    }
}

impl InternalSlots for BoundFunction {
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
            prototype: Some(prototype),
            keys,
            values,
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl InternalMethods for BoundFunction {
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
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
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        let object_index = agent[self]
            .object_index
            .unwrap_or_else(|| self.create_backing_object(agent));
        object_index.internal_define_own_property(agent, property_key, property_descriptor)
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
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
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
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
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set(agent, property_key, value, receiver)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            // length and name are not writable
            Ok(false)
        } else {
            let object_index = self.create_backing_object(agent);
            object_index.internal_set(agent, property_key, value, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
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

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_own_property_keys(agent)
        } else {
            Ok(vec![
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            ])
        }
    }

    /// ### [10.4.1.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-bound-function-exotic-objects-call-thisargument-argumentslist)
    ///
    /// The \[\[Call]] internal method of a bound function exotic object F
    /// takes arguments thisArgument (an ECMAScript language value) and
    /// argumentsList (a List of ECMAScript language values) and returns either
    /// a normal completion containing an ECMAScript language value or a throw
    /// completion.
    fn internal_call(
        self,
        agent: &mut Agent,
        _: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let target be F.[[BoundTargetFunction]].
        let target = agent[self].bound_target_function;
        // 2. Let boundThis be F.[[BoundThis]].
        let bound_this = agent[self].bound_this;
        // 3. Let boundArgs be F.[[BoundArguments]].
        let bound_args = agent[self].bound_arguments;
        // 4. Let args be the list-concatenation of boundArgs and argumentsList.
        if bound_args.is_empty() {
            // Optimisation: If only `this` is bound, then we can pass the
            // arguments list without changes to the bound function.
            call_function(agent, target, bound_this, Some(arguments_list))
        } else {
            // Note: We currently cannot optimise against an empty arguments
            // list, as we must create a Vec from the bound_args ElementsVector
            // in any case to use it as arguments. A slice pointing to it would
            // be unsound as calling to JS may invalidate the slice pointer.
            let mut args = Vec::with_capacity(bound_args.len() as usize + arguments_list.len());
            agent[bound_args]
                .iter()
                .for_each(|item| args.push(item.unwrap()));
            args.extend_from_slice(&arguments_list);
            // 5. Return ? Call(target, boundThis, args).
            call_function(agent, target, bound_this, Some(ArgumentsList(&args)))
        }
    }

    /// ### [10.4.1.2 \[\[Construct\]\] ( argumentsList, newTarget )](https://tc39.es/ecma262/#sec-bound-function-exotic-objects-construct-argumentslist-newtarget)
    ///
    /// The \[\[Construct]] internal method of a bound function exotic object F
    /// takes arguments argumentsList (a List of ECMAScript language values)
    /// and newTarget (a constructor) and returns either a normal completion
    /// containing an Object or a throw completion.
    fn internal_construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        // 1. Let target be F.[[BoundTargetFunction]].
        let target = agent[self].bound_target_function;
        // 2. Assert: IsConstructor(target) is true.
        assert!(is_constructor(agent, target.into_value()));
        // 3. Let boundArgs be F.[[BoundArguments]].
        let bound_args = agent[self].bound_arguments;
        // 5. If SameValue(F, newTarget) is true, set newTarget to target.
        let new_target = if self.into_function() == new_target {
            target
        } else {
            new_target
        };
        // 4. Let args be the list-concatenation of boundArgs and argumentsList.
        // Note: We currently cannot optimise against an empty arguments
        // list, as we must create a Vec from the bound_args ElementsVector
        // in any case to use it as arguments. A slice pointing to it would
        // be unsound as calling to JS may invalidate the slice pointer.
        let mut args = Vec::with_capacity(bound_args.len() as usize + arguments_list.len());
        agent[bound_args]
            .iter()
            .for_each(|item| args.push(item.unwrap()));
        args.extend_from_slice(&arguments_list);
        // 6. Return ? Construct(target, args, newTarget).
        construct(agent, target, Some(ArgumentsList(&args)), Some(new_target))
    }
}

impl Index<BoundFunction> for Agent {
    type Output = BoundFunctionHeapData;

    fn index(&self, index: BoundFunction) -> &Self::Output {
        self.heap
            .bound_functions
            .get(index.0.into_index())
            .expect("BoundFunction out of bounds")
            .as_ref()
            .expect("BoundFunction slot empty")
    }
}

impl IndexMut<BoundFunction> for Agent {
    fn index_mut(&mut self, index: BoundFunction) -> &mut Self::Output {
        self.heap
            .bound_functions
            .get_mut(index.0.into_index())
            .expect("BoundFunction out of bounds")
            .as_mut()
            .expect("BoundFunction slot empty")
    }
}

impl CreateHeapData<BoundFunctionHeapData, BoundFunction> for Heap {
    fn create(&mut self, data: BoundFunctionHeapData) -> BoundFunction {
        self.bound_functions.push(Some(data));
        BoundFunction(BoundFunctionIndex::last(&self.bound_functions))
    }
}

impl HeapMarkAndSweep for BoundFunction {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.bound_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = BoundFunctionIndex::from_u32(
            self_index - compactions.bound_functions.get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep for BoundFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.name.mark_values(queues);
        self.bound_target_function.mark_values(queues);
        self.object_index.mark_values(queues);
        self.bound_arguments.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.name.sweep_values(compactions);
        self.bound_target_function.sweep_values(compactions);
        self.object_index.sweep_values(compactions);
        self.bound_arguments.sweep_values(compactions);
    }
}
