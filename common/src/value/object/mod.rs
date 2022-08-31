use crate::value::{JsString, JsSymbol};
use crate::{JsResult, Value};
use gc::{Finalize, Gc, Trace};

pub trait JsObject: Trace + Finalize {
    fn get_prototype_of(&self) -> JsResult<Option<Gc<dyn JsObject>>>;
    fn set_prototype_of(&self, prototype: Option<Gc<dyn JsObject>>) -> JsResult<bool>;
    fn is_extensible(&self) -> JsResult<bool>;
    fn prevent_extensions(&self) -> JsResult<bool>;
    fn get_own_property(&self, key: PropertyKeyRef) -> JsResult<Option<PropertyDescriptor>>;
    fn define_own_property(
        &self,
        key: PropertyKey,
        descriptor: PropertyDescriptor,
    ) -> JsResult<bool>;
    fn has_property(&self, key: PropertyKeyRef) -> JsResult<bool>;
    fn get(&self, key: PropertyKeyRef, receiver: &Value) -> JsResult<Value>;
    fn set(&self, key: PropertyKey, value: Value, receiver: &Value) -> JsResult<bool>;
    fn delete(&self, key: PropertyKeyRef) -> JsResult<bool>;
    fn own_property_keys(&self) -> JsResult<Vec<PropertyKeyRef>>;
}

pub trait JsFunction: JsObject {
    fn call(&self, this: &Value, args: &[Value]) -> JsResult<Value>;
}

pub trait JsConstructor: JsFunction {
    fn construct(&self, args: &[Value], target: &dyn JsObject) -> JsResult<Gc<dyn JsObject>>;
}

pub enum PropertyKey {
    String(Gc<JsString>),
    Symbol(Gc<JsSymbol>),
}
pub enum PropertyKeyRef<'a> {
    String(&'a JsString),
    Symbol(&'a JsSymbol),
}

pub struct PropertyDescriptor {
    value: PropertyDescriptorValue,
    enumerable: bool,
    configurable: bool,
}

// TODO(andreubotella): This name isn't great.
pub enum PropertyDescriptorValue {
    Data {
        value: Value,
        writable: bool,
    },
    Accessor {
        get: Option<Gc<dyn JsFunction>>,
        set: Option<Gc<dyn JsFunction>>,
    },
}
