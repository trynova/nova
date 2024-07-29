// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
pub struct BoundFunction<'gen>(BoundFunctionIndex<'gen>);

impl<'gen> BoundFunction<'gen> {
    pub(crate) const fn _def() -> Self {
        BoundFunction(BoundFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn is_constructor(self, agent: &Agent<'gen>) -> bool {
        // A bound function has the [[Construct]] method if the target function
        // does.
        agent[self].bound_target_function.is_constructor(agent)
    }
}

impl<'gen> IntoValue<'gen> for BoundFunction<'gen> {
    fn into_value(self) -> Value<'gen> {
        Value::BoundFunction(self)
    }
}

impl<'gen> IntoObject<'gen> for BoundFunction<'gen> {
    fn into_object(self) -> Object<'gen> {
        Object::BoundFunction(self)
    }
}

impl<'gen> IntoFunction<'gen> for BoundFunction<'gen> {
    fn into_function(self) -> Function<'gen> {
        Function::BoundFunction(self)
    }
}

impl<'gen> InternalSlots<'gen> for BoundFunction<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
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

impl<'gen> InternalMethods<'gen> for BoundFunction<'gen> {
    fn internal_get_own_property(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
    ) -> JsResult<'gen, Option<PropertyDescriptor<'gen>>> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_get_own_property(agent, property_key)
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
        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        backing_object.internal_define_own_property(agent, property_key, property_descriptor)
    }

    fn internal_has_property(self, agent: &mut Agent<'gen>, property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_has_property(agent, property_key)
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
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_get(agent, property_key, receiver)
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
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_delete(agent, property_key)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            let backing_object = self.create_backing_object(agent);
            backing_object.internal_delete(agent, property_key)
        } else {
            // Non-existing property
            Ok(true)
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent<'gen>) -> JsResult<'gen, Vec<PropertyKey<'gen>>> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_own_property_keys(agent)
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
        agent: &mut Agent<'gen>,
        _: Value<'gen>,
        arguments_list: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
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
        agent: &mut Agent<'gen>,
        arguments_list: ArgumentsList<'_, 'gen>,
        new_target: Function<'gen>,
    ) -> JsResult<'gen, Object<'gen>> {
        // 1. Let target be F.[[BoundTargetFunction]].
        let target = agent[self].bound_target_function;
        // 2. Assert: IsConstructor(target) is true.
        assert!(is_constructor(agent, target).is_some());
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

impl<'gen> Index<BoundFunction<'gen>> for Agent<'gen> {
    type Output = BoundFunctionHeapData<'gen>;

    fn index(&self, index: BoundFunction<'gen>) -> &Self::Output {
        &self.heap.bound_functions[index]
    }
}

impl<'gen> IndexMut<BoundFunction<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: BoundFunction<'gen>) -> &mut Self::Output {
        &mut self.heap.bound_functions[index]
    }
}

impl<'gen> Index<BoundFunction<'gen>> for Vec<Option<BoundFunctionHeapData<'gen>>> {
    type Output = BoundFunctionHeapData<'gen>;

    fn index(&self, index: BoundFunction<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("BoundFunction out of bounds")
            .as_ref()
            .expect("BoundFunction slot empty")
    }
}

impl<'gen> IndexMut<BoundFunction<'gen>> for Vec<Option<BoundFunctionHeapData<'gen>>> {
    fn index_mut(&mut self, index: BoundFunction<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BoundFunction out of bounds")
            .as_mut()
            .expect("BoundFunction slot empty")
    }
}

impl<'gen> CreateHeapData<BoundFunctionHeapData<'gen>, BoundFunction<'gen>> for Heap<'gen> {
    fn create(&mut self, data: BoundFunctionHeapData<'gen>) -> BoundFunction<'gen> {
        self.bound_functions.push(Some(data));
        BoundFunction(BoundFunctionIndex::last(&self.bound_functions))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for BoundFunction<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.bound_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.bound_functions.shift_index(&mut self.0);
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for BoundFunctionHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.name.mark_values(queues);
        self.bound_target_function.mark_values(queues);
        self.object_index.mark_values(queues);
        self.bound_this.mark_values(queues);
        self.bound_arguments.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.name.sweep_values(compactions);
        self.bound_target_function.sweep_values(compactions);
        self.object_index.sweep_values(compactions);
        self.bound_this.sweep_values(compactions);
        self.bound_arguments.sweep_values(compactions);
    }
}
