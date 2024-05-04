use std::fmt::Debug;

use crate::{ecmascript::{builtins::ArgumentsList, execution::{Agent, JsResult, RealmIdentifier}, types::Value}, heap::indexes::ObjectIndex};

pub struct AbstractClosureHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: RealmIdentifier,
    /// #### \[\[InitialName]]
    /// A String that is the initial name of the function. It is used by
    /// 20.2.3.5 (`Function.prototype.toString()`).
    pub(crate) initial_name: Option<String>,
    pub(crate) behaviour: Box<dyn Fn(&mut Agent, Value, Option<ArgumentsList>) -> JsResult<Value>>,
}

impl Debug for AbstractClosureHeapData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbstractClosureHeapData").field("object_index", &self.object_index).field("length", &self.length).field("realm", &self.realm).field("initial_name", &self.initial_name).field("behaviour", &"some closure").finish()
    }
}