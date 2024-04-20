use crate::ecmascript::{
    execution::{Agent, JsResult},
    scripts_and_modules::module::ModuleIdentifier,
    types::{
        InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
        PropertyDescriptor, PropertyKey, Value,
    },
};

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module(pub(crate) ModuleIdentifier);

impl From<Module> for ModuleIdentifier {
    fn from(val: Module) -> Self {
        val.0
    }
}

impl From<ModuleIdentifier> for Module {
    fn from(value: ModuleIdentifier) -> Self {
        Self(value)
    }
}

impl IntoValue for Module {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Module {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Module> for Value {
    fn from(val: Module) -> Self {
        Value::Module(val.0)
    }
}

impl From<Module> for Object {
    fn from(val: Module) -> Self {
        Object::Module(val.0)
    }
}

impl OrdinaryObjectInternalSlots for Module {
    fn internal_extensible(self, _agent: &Agent) -> bool {
        todo!();
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!();
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        todo!();
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!();
    }
}

impl InternalMethods for Module {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.internal_prototype(agent))
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<Object>,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        Ok(self.internal_extensible(agent))
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        self.internal_set_extensible(agent, false);
        Ok(true)
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        todo!();
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_get(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _receiver: Value,
    ) -> JsResult<Value> {
        todo!();
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_delete(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!();
    }

    fn internal_own_property_keys(self, _agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        todo!();
    }
}
