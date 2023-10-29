use super::{Object, PropertyKey};
use crate::ecmascript::{
    builtins::ArgumentsList,
    execution::{Agent, JsResult},
    types::{PropertyDescriptor, Value},
};

pub type Call<T> = fn(
    agent: &mut Agent,
    object: T,
    this_value: Value,
    arguments_list: ArgumentsList,
) -> JsResult<Value>;
pub type Construct<T> =
    fn(agent: &mut Agent, object: T, arguments_list: ArgumentsList) -> JsResult<T>;

/// 6.1.7.2 Object Internal Methods and Internal Slots
/// https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots
pub trait InternalMethods<T = Object>
where
    Self: Sized,
{
    /// \[\[GetPrototypeOf\]\]
    fn get_prototype_of(agent: &mut Agent, object: Self) -> JsResult<Option<Object>>;

    /// \[\[SetPrototypeOf\]\]
    fn set_prototype_of(
        agent: &mut Agent,
        object: Self,
        prototype: Option<Object>,
    ) -> JsResult<bool>;

    /// \[\[IsExtensible\]\]
    fn is_extensible(agent: &mut Agent, object: Self) -> JsResult<bool>;

    /// \[\[PreventExtensions\]\]
    fn prevent_extensions(agent: &mut Agent, object: Self) -> JsResult<bool>;

    /// \[\[GetOwnProperty\]\]
    fn get_own_property(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>>;

    /// \[\[DefineOwnProperty\]\]
    fn define_own_property(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool>;

    /// \[\[HasProperty\]\]
    fn has_property(agent: &mut Agent, object: Self, property_key: PropertyKey) -> JsResult<bool>;

    /// \[\[Get\]\]
    fn get(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value>;

    /// \[\[Set\]\]
    fn set(
        agent: &mut Agent,
        object: Self,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool>;

    /// \[\[Delete\]\]
    fn delete(agent: &mut Agent, object: Self, property_key: PropertyKey) -> JsResult<bool>;

    /// \[\[OwnPropertyKeys\]\]
    fn own_property_keys(agent: &mut Agent, object: Self) -> JsResult<Vec<PropertyKey>>;

    /// \[\[Call\]\]
    fn call(
        agent: &mut Agent,
        object: Self,
        this_value: Value,
        arguments_list: &[Value],
    ) -> JsResult<Value> {
        unreachable!()
    }

    /// \[\[Construct\]\]
    fn construct(agent: &mut Agent, object: Self, arguments_list: &[Value]) -> JsResult<T> {
        unreachable!()
    }
}
