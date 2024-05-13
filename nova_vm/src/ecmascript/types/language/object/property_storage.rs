use std::cell::Ref;

use crate::{
    ecmascript::{
        execution::{Agent, Realm},
        types::{PropertyDescriptor, Value, BUILTIN_STRING_MEMORY},
    },
    Heap,
};

use super::{Object, ObjectHeapData, PropertyKey};

#[derive(Debug, Clone, Copy)]
pub struct PropertyStorage(Object);

impl PropertyStorage {
    pub fn new(object: Object) -> Self {
        Self(object)
    }

    fn into_object(self) -> Object {
        self.0
    }

    fn into_value(self) -> Value {
        self.into_object().into_value()
    }

    pub fn has(self, agent: &mut Agent, key: PropertyKey) -> bool {
        let object = self.into_value();

        match object {
            Value::Object(object) => agent
                .heap
                .elements
                .has(agent[object].keys, key.into_value()),
            Value::Array(array) => {
                if key.equals(agent, PropertyKey::from(BUILTIN_STRING_MEMORY.length)) {
                    return true;
                }

                let array = &agent[array];

                if key.is_array_index() {
                    return agent
                        .heap
                        .elements
                        .has(array.elements.into(), key.into_value());
                }

                if let Some(object) = array.object_index {
                    agent
                        .heap
                        .elements
                        .has(agent[object].keys, key.into_value())
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

    pub fn get(self, agent: &mut Agent, key: PropertyKey) -> Option<PropertyDescriptor> {
        match self.0 {
            Object::Object(object) => {
                let ObjectHeapData { keys, values, .. } = agent[object];
                let key = key.into_value();
                let result = agent
                    .heap
                    .elements
                    .get(keys)
                    .iter()
                    .enumerate()
                    .find(|(_, element_key)| element_key.unwrap() == key)
                    .map(|res| res.0);
                let values = agent.heap.elements.get(values);
                result.map(|index| PropertyDescriptor {
                    value: *values.get(index).unwrap(),
                    configurable: Some(true),
                    enumerable: Some(true),
                    get: None,
                    set: None,
                    writable: Some(true),
                })
            }
            _ => todo!(),
        }
    }

    pub fn set(self, agent: &mut Agent, property_key: PropertyKey, descriptor: PropertyDescriptor) {
        if descriptor.value.is_none() {
            todo!("Setters / getters");
        }
        match self.0 {
            Object::Object(object) => {
                let ObjectHeapData { keys, values, .. } = agent[object];
                let property_key = property_key.into_value();
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
                    *value_entry = descriptor.value;
                } else {
                    let Heap {
                        elements, objects, ..
                    } = &mut agent.heap;
                    let object_heap_data = objects
                        .get_mut(object.into_index())
                        .expect("Invalid ObjectIndex")
                        .as_mut()
                        .expect("Invalid ObjectIndex");
                    object_heap_data
                        .keys
                        .push(elements, Some(property_key), None);
                    object_heap_data
                        .values
                        .push(elements, descriptor.value, None);
                };
            }
            _ => todo!(),
        }
    }

    pub fn remove(self, _agent: &mut Agent, _property_key: PropertyKey) {}

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
