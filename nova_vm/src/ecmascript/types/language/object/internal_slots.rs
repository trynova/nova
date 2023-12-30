use super::Object;
use crate::ecmascript::execution::Agent;

/// ### [10.1 Ordinary Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots)
pub trait OrdinaryObjectInternalSlots
where
    Self: Sized + Into<Object>,
{
    /// #### \[\[Extensible\]\]
    ///
    /// Every ordinary object has a Boolean-valued \[\[Extensible\]\] internal slot
    /// which is used to fulfill the extensibility-related internal method
    /// invariants specified in [6.1.7.3](https://tc39.es/ecma262/#sec-invariants-of-the-essential-internal-methods).
    fn extensible(self, agent: &Agent) -> bool;

    /// #### \[\[Extensible\]\]
    fn set_extensible(self, agent: &mut Agent, value: bool);

    /// #### \[\[Prototype\]\]
    ///
    /// All ordinary objects have an internal slot called \[\[Prototype\]\]. The value
    /// of this internal slot is either null or an object and is used for
    /// implementing inheritance.
    fn prototype(self, agent: &Agent) -> Option<Object>;

    /// #### \[\[Prototype\]\]
    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>);
}
