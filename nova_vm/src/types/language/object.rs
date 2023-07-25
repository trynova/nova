mod data;
mod internal_methods;
mod property_key;

use crate::{execution::Agent, heap::GetHeapData};

use super::Value;
pub use data::ObjectData;
pub use internal_methods::InternalMethods;
pub use property_key::PropertyKey;

/// 6.1.7 The Object Type
/// https://tc39.es/ecma262/#sec-object-type
#[derive(Debug, Clone, Copy)]
pub struct Object(Value);

impl Object {
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn into_value(self) -> Value {
        self.0
    }

    pub fn prototype(self, agent: &mut Agent) -> Option<Object> {
        let realm = agent.current_realm();
        let object = self.into_value();

        match object {
            // Value::Object(object) => {
            //     let object = realm.heap.get(object);
            // }
            // Value::ArrayObject(array) => {
            //     let array = realm.heap.get(array);
            // }
            Value::Function(function) => {
                // let function = realm.heap.get(function);
                // function.binding;
                todo!()
            }
            _ => unreachable!(),
        }
    }
}
