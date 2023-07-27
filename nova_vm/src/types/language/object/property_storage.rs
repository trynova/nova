use crate::{
    execution::Agent,
    heap::GetHeapData,
    types::{PropertyDescriptor, Value},
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
                let realm = agent.current_realm();
                let realm = realm.borrow();
                let array = agent.current_realm().borrow().heap.get(array);
                true
            }
            Value::Function(_) => todo!(),
            _ => unreachable!(),
        }
    }

    pub fn get(self, agent: &mut Agent, key: PropertyKey) -> Option<PropertyDescriptor> {
        todo!();
    }
}
