use super::{Object, PropertyKey};
use crate::ecmascript::{
    builtins::ArgumentsList,
    execution::{Agent, JsResult},
    types::{Function, PropertyDescriptor, Value},
};

/// ### [6.1.7.2 Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots)
pub trait InternalMethods<T = Object>
where
    Self: Sized + Clone + Copy + Into<Object>,
{
    /// \[\[GetPrototypeOf\]\]
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>>;

    /// \[\[SetPrototypeOf\]\]
    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool>;

    /// \[\[IsExtensible\]\]
    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool>;

    /// \[\[PreventExtensions\]\]
    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool>;

    /// \[\[GetOwnProperty\]\]
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>>;

    /// \[\[DefineOwnProperty\]\]
    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool>;

    /// \[\[HasProperty\]\]
    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool>;

    /// \[\[Get\]\]
    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value>;

    /// \[\[Set\]\]
    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool>;

    /// \[\[Delete\]\]
    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool>;

    /// \[\[OwnPropertyKeys\]\]
    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>>;

    /// \[\[Call\]\]
    fn internal_call(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        unreachable!()
    }

    /// \[\[Construct\]\]
    fn internal_construct(
        self,
        _agent: &mut Agent,
        _arguments_list: ArgumentsList,
        _new_target: Function,
    ) -> JsResult<T> {
        unreachable!()
    }
}
