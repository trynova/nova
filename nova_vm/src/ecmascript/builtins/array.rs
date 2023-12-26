//! ### 10.4.2 Array Exotic Objects
//!
//! https://tc39.es/ecma262/#sec-array-exotic-objects

mod data;

use std::ops::Deref;

use super::{create_builtin_function, ArgumentsList, Behaviour, Builtin, BuiltinFunctionArgs};
use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::same_value_non_number,
        execution::{Agent, JsResult},
        types::{InternalMethods, Object, OrdinaryObject, OrdinaryObjectInternalSlots, Value},
    },
    heap::{indexes::ArrayIndex, GetHeapData},
};

pub use data::ArrayHeapData;

#[derive(Debug, Clone, Copy)]
pub struct Array(ArrayIndex);

impl From<ArrayIndex> for Array {
    fn from(value: ArrayIndex) -> Self {
        Array(value)
    }
}

impl From<Array> for Object {
    fn from(value: Array) -> Self {
        Self::Array(value.0)
    }
}

impl Deref for Array {
    type Target = ArrayIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ArrayConstructor;

impl Builtin for ArrayConstructor {
    fn create(agent: &mut Agent) -> JsResult<Object> {
        let realm = agent.current_realm_id();
        let object = create_builtin_function(
            agent,
            Behaviour::Regular(Self::behaviour),
            BuiltinFunctionArgs::new(1, "Array", realm),
        );

        Ok(object.into_object())
    }
}

impl ArrayConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }
}

impl OrdinaryObjectInternalSlots for Array {
    fn extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).extensible(agent)
        } else {
            true
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).set_extensible(agent, value)
        } else if !value {
            // Create array base object and set inextensible
            todo!()
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).prototype(agent)
        } else {
            Some(agent.current_realm().intrinsics().array_prototype())
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).set_prototype(agent, prototype)
        } else {
            // Create array base object with custom prototype
            todo!()
        }
    }
}

impl InternalMethods for Array {
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).get_prototype_of(agent)
        } else {
            Ok(Some(agent.current_realm().intrinsics().array_prototype()))
        }
    }

    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).set_prototype_of(agent, prototype)
        } else {
            // 1. Let current be O.[[Prototype]].
            let current = agent.current_realm().intrinsics().array_prototype();
            let object_index = if let Some(v) = prototype {
                if same_value_non_number(agent, v, current) {
                    return Ok(true);
                } else {
                    // TODO: Proper handling
                    Some(agent.heap.create_object_with_prototype(v))
                }
            } else {
                Some(agent.heap.create_null_object(Default::default()))
            };
            agent.heap.get_mut(*self).object_index = object_index;
            Ok(true)
        }
    }

    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).is_extensible(agent)
        } else {
            Ok(true)
        }
    }

    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).prevent_extensions(agent)
        } else {
            // TODO: Create base array object and call prevent extensions on it.
            Ok(true)
        }
    }

    fn get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> JsResult<Option<crate::ecmascript::types::PropertyDescriptor>> {
        todo!()
    }

    fn define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!()
    }

    fn has_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> JsResult<bool> {
        todo!()
    }

    fn get(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _receiver: Value,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!()
    }

    fn delete(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> JsResult<bool> {
        todo!()
    }

    fn own_property_keys(
        self,
        _agent: &mut Agent,
    ) -> JsResult<Vec<crate::ecmascript::types::PropertyKey>> {
        todo!()
    }
}
