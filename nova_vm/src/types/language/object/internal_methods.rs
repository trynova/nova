use super::{Object, PropertyKey};
use crate::{
    builtins::ArgumentsList,
    execution::{Agent, JsResult},
    types::{PropertyDescriptor, Value},
};

pub type GetPrototypeOf = fn(agent: &mut Agent, object: Object) -> Option<Object>;
pub type SetPrototypeOf =
    fn(agent: &mut Agent, object: Object, prototype: Option<Object>) -> JsResult<bool>;
pub type IsExtensible = fn(agent: &mut Agent, object: Object) -> JsResult<bool>;
pub type PreventExtensions = fn(agent: &mut Agent, object: Object) -> JsResult<bool>;
pub type GetOwnProperty = fn(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
) -> JsResult<Option<PropertyDescriptor>>;
pub type DefineOwnProperty = fn(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    property_descriptor: PropertyDescriptor,
) -> JsResult<bool>;
pub type HasProperty =
    fn(agent: &mut Agent, object: Object, property_key: PropertyKey) -> JsResult<bool>;
pub type Get = fn(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    receiver: Value,
) -> JsResult<Value>;
pub type Set = fn(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
) -> JsResult<bool>;
pub type Delete =
    fn(agent: &mut Agent, object: Object, property_key: PropertyKey) -> JsResult<bool>;
pub type OwnPropertyKeys = fn(agent: &mut Agent, object: Object) -> JsResult<Vec<PropertyKey>>;
pub type Call =
    fn(object: Object, this_value: Value, arguments_list: ArgumentsList) -> JsResult<Value>;
pub type Construct =
    fn(agent: &mut Agent, object: Object, arguments_list: ArgumentsList) -> JsResult<Object>;

/// 6.1.7.2 Object Internal Methods and Internal Slots
/// https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots
#[derive(Debug, Clone)]
pub struct InternalMethods {
    /// [[GetPrototypeOf]]
    pub get_prototype_of: GetPrototypeOf,

    /// [[SetPrototypeOf]]
    pub set_prototype_of: SetPrototypeOf,

    /// [[IsExtensible]]
    pub is_extensible: IsExtensible,

    /// [[PreventExtensions]]
    pub prevent_extensions: PreventExtensions,

    /// [[GetOwnProperty]]
    pub get_own_property: GetOwnProperty,

    /// [[DefineOwnProperty]]
    pub define_own_property: DefineOwnProperty,

    /// [[HasProperty]]
    pub has_property: HasProperty,

    /// [[Get]]
    pub get: Get,

    /// [[Set]]
    pub set: Set,

    /// [[Delete]]
    pub delete: Delete,

    /// [[OwnPropertyKeys]]
    pub own_property_keys: OwnPropertyKeys,

    /// [[Call]]
    pub call: Option<Call>,

    /// [[Construct]]
    pub construct: Option<Construct>,
}
