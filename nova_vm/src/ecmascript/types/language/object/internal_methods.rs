use super::{Object, PropertyKey};
use crate::ecmascript::{
    execution::{Agent, JsResult},
    types::{Function, PropertyDescriptor, Value},
};

/// 6.1.7.2 Object Internal Methods and Internal Slots
/// https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots
pub trait InternalMethods<T = Object>
where
    Self: Sized + Clone + Copy + Into<Object>,
{
    /// \[\[GetPrototypeOf\]\]
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>>;

    /// \[\[SetPrototypeOf\]\]
    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool>;

    /// \[\[IsExtensible\]\]
    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool>;

    /// \[\[PreventExtensions\]\]
    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool>;

    /// \[\[GetOwnProperty\]\]
    fn get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>>;

    /// \[\[DefineOwnProperty\]\]
    fn define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool>;

    /// \[\[HasProperty\]\]
    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool>;

    /// \[\[Get\]\]
    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value>;

    /// \[\[Set\]\]
    fn set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool>;

    /// \[\[Delete\]\]
    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool>;

    /// \[\[OwnPropertyKeys\]\]
    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>>;

    /// \[\[Call\]\]
    fn call(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: &[Value],
    ) -> JsResult<Value> {
        unreachable!()
    }

    /// \[\[Construct\]\]
    fn construct(
        self,
        _agent: &mut Agent,
        _arguments_list: &[Value],
        _new_target: Function,
    ) -> JsResult<T> {
        unreachable!()
    }
}
