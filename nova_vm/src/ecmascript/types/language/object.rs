mod data;
mod internal_methods;
mod property_key;
mod property_storage;
use super::{
    value::{ARRAY_DISCRIMINANT, FUNCTION_DISCRIMINANT, OBJECT_DISCRIMINANT},
    Function, Value,
};
use crate::{
    ecmascript::{
        builtins::ordinary,
        execution::{
            agent::{ExceptionType, JsError},
            Agent, JsResult,
        },
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{ArrayIndex, FunctionIndex, ObjectIndex},
        GetHeapData,
    },
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

pub struct OrdinaryObject(ObjectIndex);

impl From<OrdinaryObject> for Object {
    fn from(value: OrdinaryObject) -> Self {
        Self::Object(value.0)
    }
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
        let object_index = self.get_object_index(heap);
        let object = heap.get_mut(object_index);
        object.prototype = prototype;
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
        let success = Object::define_own_property(agent, self, property_key, property_descriptor)?;

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
        self,
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
        Object::define_own_property(agent, self, property_key, new_descriptor)
    }
}

impl InternalMethods for Object {
    fn get_prototype_of(agent: &mut Agent, object: Self) -> JsResult<Option<Object>> {
        match object {
            Object::Object(idx) => OrdinaryObject::get_prototype_of(agent, OrdinaryObject(idx)),
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::get_prototype_of(agent, Function(idx)),
        }
    }

    fn set_prototype_of(
        agent: &mut Agent,
        object: Self,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        match object {
            Object::Object(idx) => {
                OrdinaryObject::set_prototype_of(agent, OrdinaryObject(idx), prototype)
            }
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::set_prototype_of(agent, Function(idx), prototype),
        }
    }

    fn is_extensible(agent: &mut Agent, object: Self) -> JsResult<bool> {
        match object {
            Object::Object(idx) => OrdinaryObject::is_extensible(agent, OrdinaryObject(idx)),
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::is_extensible(agent, Function(idx)),
        }
    }

    fn prevent_extensions(agent: &mut Agent, object: Self) -> JsResult<bool> {
        match object {
            Object::Object(idx) => OrdinaryObject::prevent_extensions(agent, OrdinaryObject(idx)),
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::prevent_extensions(agent, Function(idx)),
        }
    }

    fn get_own_property(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match object {
            Object::Object(idx) => {
                OrdinaryObject::get_own_property(agent, OrdinaryObject(idx), property_key)
            }
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::get_own_property(agent, Function(idx), property_key),
        }
    }

    fn define_own_property(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        match object {
            Object::Object(idx) => OrdinaryObject::define_own_property(
                agent,
                OrdinaryObject(idx),
                property_key,
                property_descriptor,
            ),
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::define_own_property(
                agent,
                Function(idx),
                property_key,
                property_descriptor,
            ),
        }
    }

    fn has_property(agent: &mut Agent, object: Self, property_key: PropertyKey) -> JsResult<bool> {
        match object {
            Object::Object(idx) => {
                OrdinaryObject::has_property(agent, OrdinaryObject(idx), property_key)
            }
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::has_property(agent, Function(idx), property_key),
        }
    }

    fn get(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        match object {
            Object::Object(idx) => {
                OrdinaryObject::get(agent, OrdinaryObject(idx), property_key, receiver)
            }
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::get(agent, Function(idx), property_key, receiver),
        }
    }

    fn set(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        match object {
            Object::Object(idx) => {
                OrdinaryObject::set(agent, OrdinaryObject(idx), property_key, value, receiver)
            }
            Object::Array(idx) => todo!(),
            Object::Function(idx) => {
                Function::set(agent, Function(idx), property_key, value, receiver)
            }
        }
    }

    fn delete(agent: &mut Agent, object: Self, property_key: PropertyKey) -> JsResult<bool> {
        match object {
            Object::Object(idx) => OrdinaryObject::delete(agent, OrdinaryObject(idx), property_key),
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::delete(agent, Function(idx), property_key),
        }
    }

    fn own_property_keys(agent: &mut Agent, object: Self) -> JsResult<Vec<PropertyKey>> {
        match object {
            Object::Object(idx) => OrdinaryObject::own_property_keys(agent, OrdinaryObject(idx)),
            Object::Array(idx) => todo!(),
            Object::Function(idx) => Function::own_property_keys(agent, Function(idx)),
        }
    }

    fn call(
        agent: &mut Agent,
        object: Self,
        this_value: Value,
        arguments_list: &[Value],
    ) -> JsResult<Value> {
        match object {
            Object::Function(idx) => {
                Function::call(agent, Function(idx), this_value, arguments_list)
            }
            _ => unreachable!(),
        }
    }

    fn construct(agent: &mut Agent, object: Self, arguments_list: &[Value]) -> JsResult<Object> {
        match object {
            Object::Function(idx) => Function::construct(agent, Function(idx), arguments_list),
            _ => unreachable!(),
        }
    }
}
