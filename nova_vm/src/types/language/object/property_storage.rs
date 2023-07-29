use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::{
    execution::{Agent, Realm},
    heap::GetHeapData,
    types::{PropertyDescriptor, String, Value},
};

use super::{Object, PropertyKey};

#[derive(Clone, Copy)]
pub struct PropertyStorage(Object);

impl PropertyStorage {
    pub(crate) fn new(object: Object) -> Self {
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
                let realm = agent.current_realm();
                let realm = realm.borrow();
                let keys = &realm.heap.get(object).keys;
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

                let realm = agent.current_realm();
                let realm = realm.borrow();
                let array = realm.heap.get(array);

                if let Value::Integer(number) = key.into_value() {
                    if let Some(_) = TryInto::<usize>::try_into(number.into_i64())
                        .map(|idx| array.elements.get(idx))
                        .ok()
                    {
                        return true;
                    }
                }

                if let Some(object) = array.object {
                    return object.property_storage().has(agent, key);
                }

                false
            }
            Value::Function(_) => todo!(),
            _ => unreachable!(),
        }
    }

    pub fn get(self, agent: &mut Agent, key: PropertyKey) -> Option<PropertyDescriptor> {
        todo!();
    }

    pub fn set(self, agent: &mut Agent, property_key: PropertyKey, descriptor: PropertyDescriptor) {
    }

    pub fn remove(self, agent: &mut Agent, property_key: PropertyKey) {}

    pub fn entries<'a, 'b>(self, agent: &'a Agent<'b, 'b>) -> Entries<'a, 'b> {
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
