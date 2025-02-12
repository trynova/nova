// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cell::Ref;

use crate::{
    ecmascript::{
        execution::{Agent, Realm},
        types::{PropertyDescriptor, Value, BUILTIN_STRING_MEMORY},
    },
    heap::element_array::ElementDescriptor,
    Heap,
};

use super::{Object, ObjectHeapData, PropertyKey};

#[derive(Debug, Clone, Copy)]
pub struct PropertyStorage<'a>(Object<'a>);

impl<'a> PropertyStorage<'a> {
    pub fn new(object: Object<'a>) -> Self {
        Self(object)
    }

    fn into_object(self) -> Object<'a> {
        self.0
    }

    fn into_value(self) -> Value {
        self.into_object().into_value()
    }

    pub fn has(self, agent: &Agent, key: PropertyKey) -> bool {
        let object = self.into_value();

        // SAFETY: Key is only used to compare with other keys.
        let key = unsafe { key.into_value_unchecked() };
        match object {
            Value::Object(object) => agent.heap.elements.has(agent[object].keys, key),
            Value::Array(array) => {
                if key == BUILTIN_STRING_MEMORY.length.into_value() {
                    return true;
                }

                let array = &agent[array];

                if key.is_integer() {
                    return agent.heap.elements.has(array.elements.into(), key);
                }

                if let Some(object) = array.object_index {
                    agent.heap.elements.has(agent[object].keys, key)
                } else {
                    false
                }
            }
            Value::BoundFunction(_) => todo!(),
            Value::BuiltinFunction(_) => todo!(),
            Value::ECMAScriptFunction(_) => todo!(),
            _ => unreachable!(),
        }
    }

    pub fn get(self, agent: &Agent, key: PropertyKey) -> Option<PropertyDescriptor> {
        match self.0 {
            Object::Object(object) => {
                let ObjectHeapData { keys, values, .. } = agent[object];
                // SAFETY: Key is only used to compare with other keys.
                let key = unsafe { key.into_value_unchecked() };
                let result = agent
                    .heap
                    .elements
                    .get(keys)
                    .iter()
                    .enumerate()
                    .find(|(_, element_key)| element_key.unwrap() == key)
                    .map(|res| res.0);
                result.map(|index| {
                    let value = *agent.heap.elements.get(values).get(index).unwrap();
                    let descriptor = agent.heap.elements.get_descriptor(values, index);
                    ElementDescriptor::to_property_descriptor(descriptor, value)
                })
            }
            _ => todo!(),
        }
    }

    pub fn set(self, agent: &mut Agent, property_key: PropertyKey, descriptor: PropertyDescriptor) {
        match self.0 {
            Object::Object(object) => {
                let ObjectHeapData { keys, values, .. } = agent[object];
                // SAFETY: Key is only used to compare with other keys.
                let property_key = unsafe { property_key.into_value_unchecked() };

                let value = descriptor.value;
                let element_descriptor = ElementDescriptor::from_property_descriptor(descriptor);

                let result = agent
                    .heap
                    .elements
                    .get(keys)
                    .iter()
                    .enumerate()
                    .find(|(_, element_key)| element_key.unwrap() == property_key)
                    .map(|res| res.0);
                if let Some(index) = result {
                    let key_entry = agent.heap.elements.get_mut(keys).get_mut(index).unwrap();
                    *key_entry = Some(property_key);
                    let value_entry = agent.heap.elements.get_mut(values).get_mut(index).unwrap();
                    *value_entry = value;
                    agent
                        .heap
                        .elements
                        .set_descriptor(values, index, element_descriptor);
                } else {
                    let Heap {
                        elements, objects, ..
                    } = &mut agent.heap;
                    let object_heap_data = objects
                        .get_mut(object.get_index())
                        .expect("Invalid ObjectIndex")
                        .as_mut()
                        .expect("Invalid ObjectIndex");
                    object_heap_data
                        .keys
                        .push(elements, Some(property_key), None);
                    object_heap_data
                        .values
                        .push(elements, value, element_descriptor);
                };
            }
            _ => todo!(),
        }
    }

    pub fn remove(self, agent: &mut Agent, property_key: PropertyKey) {
        match self.0 {
            Object::Object(object) => {
                // SAFETY: Key is only used to compare with other keys.
                let property_key = unsafe { property_key.into_value_unchecked() };

                let result = agent
                    .heap
                    .elements
                    .get(agent[object].keys)
                    .iter()
                    .enumerate()
                    .find(|(_, element_key)| element_key.unwrap() == property_key)
                    .map(|res| res.0);
                if let Some(index) = result {
                    let Heap {
                        elements, objects, ..
                    } = &mut agent.heap;
                    let object_heap_data = objects
                        .get_mut(object.get_index())
                        .expect("Invalid ObjectIndex")
                        .as_mut()
                        .expect("Invalid ObjectIndex");
                    object_heap_data.keys.remove(elements, index);
                    object_heap_data.values.remove(elements, index);
                }
            }
            _ => todo!(),
        }
    }

    pub fn entries(self, _agent: &Agent) -> Entries {
        todo!()
    }
}

#[derive(Debug)]
pub struct Entries<'a> {
    pub realm: Ref<'a, Realm>,
}

impl<'a> Entries<'a> {
    fn new(realm: Ref<'a, Realm>) -> Self {
        Self { realm }
    }
}
