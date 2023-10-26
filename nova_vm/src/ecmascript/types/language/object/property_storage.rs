use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::{
    ecmascript::{
        execution::{Agent, Realm},
        types::{PropertyDescriptor, String, Value},
    },
    heap::GetHeapData,
};

use super::{Object, PropertyKey};

#[derive(Clone, Copy)]
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
            Value::Object(object) => {
                let _keys = &agent.heap.get(object).keys;
                // realm.heap.elements.get(keys).iter().any(|k| {
                //     if let Some(value) = k {
                //         value.equals(agent, key)
                //     }
                //     false
                // });
                true
            }
            Value::Array(array) => {
                if key.equals(
                    agent,
                    PropertyKey::from(String::try_from("length").unwrap()),
                ) {
                    return true;
                }

                let array = agent.heap.get(array);

                if key.is_array_index() {
                    return agent.heap.elements.has(array.elements, key.into_value());
                }

                if let Some(object) = array.object_index {
                    agent
                        .heap
                        .elements
                        .has(object.get(&agent.heap).keys, key.into_value())
                } else {
                    false
                }
            }
            Value::Function(_) => todo!(),
            _ => unreachable!(),
        }
    }

    pub fn get(self, _agent: &mut Agent, _key: PropertyKey) -> Option<PropertyDescriptor> {
        todo!();
    }

    pub fn set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _descriptor: PropertyDescriptor,
    ) {
    }

    pub fn remove(self, _agent: &mut Agent, _property_key: PropertyKey) {}

    pub fn entries<'a, 'b>(self, _agent: &'a Agent<'b, 'b>) -> Entries<'a, 'b> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Entries<'a, 'b> {
    pub realm: Ref<'a, Realm<'b, 'b>>,
    _rc: std::marker::PhantomData<&'a Rc<RefCell<Realm<'b, 'b>>>>,
}

impl<'a, 'b> Entries<'a, 'b> {
    fn new(realm: Ref<'a, Realm<'b, 'b>>) -> Self {
        Self {
            realm,
            _rc: Default::default(),
        }
    }
}
