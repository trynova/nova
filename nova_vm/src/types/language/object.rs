mod data;
mod internal_methods;
mod property_key;
mod property_storage;
use super::{
    value::{ARRAY_DISCRIMINANT, FUNCTION_DISCRIMINANT, OBJECT_DISCRIMINANT},
    Value,
};
use crate::{
    builtins::ordinary,
    execution::{agent::ExceptionType, Agent, JsResult},
    heap::{
        indexes::{ArrayIndex, FunctionIndex, ObjectIndex},
        GetHeapData,
    },
    types::PropertyDescriptor,
    Heap,
};

pub use data::ObjectHeapData;
pub use internal_methods::InternalMethods;
pub use property_key::PropertyKey;
pub use property_storage::PropertyStorage;

/// 6.1.7 The Object Type
/// https://tc39.es/ecma262/#sec-object-type
///
/// In Nova
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Object {
    Object(ObjectIndex) = OBJECT_DISCRIMINANT,
    // Date(DateIndex) = DATE_DISCRIMINANT,
    // Error(ErrorIndex) = ERROR_DISCRIMINANT,
    Array(ArrayIndex) = ARRAY_DISCRIMINANT,
    Function(FunctionIndex) = FUNCTION_DISCRIMINANT,
    //RegExp(RegExpIndex) = REGEXP_DISCRIMINANT,
}

impl From<ObjectIndex> for Object {
    fn from(value: ObjectIndex) -> Self {
        Object::Object(value)
    }
}

impl From<ArrayIndex> for Object {
    fn from(value: ArrayIndex) -> Self {
        Object::Array(value)
    }
}

impl From<FunctionIndex> for Object {
    fn from(value: FunctionIndex) -> Self {
        Object::Function(value)
    }
}

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        // SAFETY: Sub-enum.
        unsafe { std::mem::transmute::<Object, Value>(value) }
    }
}

impl TryFrom<Value> for Object {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(x) => Ok(Object::Object(x)),
            Value::Array(x) => Ok(Object::Array(x)),
            Value::Function(x) => Ok(Object::Function(x)),
            _ => Err(()),
        }
    }
}

impl Object {
    pub fn into_value(self) -> Value {
        self.into()
    }

    fn get_object_index(self, heap: &Heap) -> ObjectIndex {
        match self {
            Object::Object(index) => index,
            Object::Array(array_index) => heap
                .arrays
                .get(array_index.into_index())
                .unwrap()
                .unwrap()
                .object_index
                .unwrap(),
            Object::Function(function_index) => heap
                .functions
                .get(function_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .object_index
                .unwrap(),
        }
    }

    /// [[Extensible]]
    pub fn extensible(self, agent: &mut Agent) -> bool {
        let heap = &agent.heap;
        let object_index = self.get_object_index(heap);
        heap.get(object_index).extensible
    }

    /// [[Extensible]]
    pub fn set_extensible(self, agent: &mut Agent, value: bool) {
        let heap = &mut agent.heap;
        let object_index = self.get_object_index(heap);
        let object = heap.get_mut(object_index);
        object.extensible = value;
    }

    /// [[Prototype]]
    pub fn prototype(self, agent: &mut Agent) -> Option<Value> {
        let heap = &agent.heap;
        let realm = agent.current_realm();

        match self {
            Object::Object(object) => {
                let object = heap.get(object);
                object.prototype.map(|v| v.into_value())
            }
            Object::Array(array) => {
                let array = heap.get(array);

                if let Some(object_index) = array.object_index {
                    let prototype = heap.get(object_index).prototype;
                    prototype.map(|v| v.into())
                } else {
                    Some(realm.intrinsics().array_prototype().into_value())
                }
            }
            Object::Function(_) => Some(realm.intrinsics().function_prototype().into_value()),
            _ => unreachable!(),
        }
    }

    /// [[Prototype]]
    pub fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        let heap = &mut agent.heap;
        let object_index = self.get_object_index(&heap);
        let object = heap.get_mut(object_index);
        object.prototype = prototype;
    }

    pub fn internal_methods<'a>(self, agent: &mut Agent) -> &'a InternalMethods {
        // TODO: Logic for fetching methods for objects/anything else.
        &ordinary::METHODS
    }

    pub fn property_storage(self) -> PropertyStorage {
        PropertyStorage::new(self)
    }

    /// /// 7.3.9 DefinePropertyOrThrow ( O, P, desc )
    /// https://tc39.es/ecma262/#sec-definepropertyorthrow
    pub fn define_property_or_throw(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<()> {
        // 1. Let success be ? O.[[DefineOwnProperty]](P, desc).
        let success = (self.internal_methods(agent).define_own_property)(
            agent,
            self,
            property_key,
            property_descriptor,
        )?;

        // 2. If success is false, throw a TypeError exception.
        if !success {
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                "Cannot assign to property on object.",
            ));
        }

        // 3. Return unused.
        Ok(())
    }

    /// 7.3.5 CreateDataProperty ( O, P, V )
    /// https://tc39.es/ecma262/#sec-createdataproperty
    pub fn create_data_property(
        self: Self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
    ) -> JsResult<bool> {
        // 1. Let newDesc be the PropertyDescriptor {
        //      [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true
        //    }.
        let new_descriptor = PropertyDescriptor {
            value: Some(value),
            writable: Some(true),
            enumerable: Some(true),
            configurable: Some(true),
            ..Default::default()
        };

        // 2. Return ? O.[[DefineOwnProperty]](P, newDesc).
        let define_own_property = self.internal_methods(agent).define_own_property;
        define_own_property(agent, self, property_key, new_descriptor)
    }
}
