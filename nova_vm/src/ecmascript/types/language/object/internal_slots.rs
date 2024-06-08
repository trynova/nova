use super::{Object, OrdinaryObject};
use crate::ecmascript::execution::{Agent, ProtoIntrinsics};

/// ### [10.1 Ordinary Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots)
pub trait OrdinaryObjectInternalSlots
where
    Self: Sized + Copy + Into<Object>,
{
    /// Default prototype of the object; this is used by
    /// [OrdinaryObjectInternalSlots::internal_prototype].
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Object;

    /// ### \[\[BackingObject\]\]
    ///
    /// This is a custom "internal slot" which defines how to find the generic
    /// object features of an item. For an ordinary object the object itself is
    /// the backing object. For exotic objects the backing object is generally
    /// found in the heap data as the `object_index` or `backing_object`.
    ///
    /// > NOTE: This should be marked `#[inline(always)]`
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject>;

    /// ### \[\[BackingObject\]\]
    ///
    /// Creates the custom \[\[BackingObject]] object data. This is called when
    /// the item's object features are required but the backing object is None.
    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject;

    /// #### \[\[Extensible\]\]
    ///
    /// Every ordinary object has a Boolean-valued \[\[Extensible\]\] internal
    /// slot which is used to fulfill the extensibility-related internal method
    /// invariants specified in [6.1.7.3](https://tc39.es/ecma262/#sec-invariants-of-the-essential-internal-methods).
    fn internal_extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_extensible(agent)
        } else {
            true
        }
    }

    /// #### \[\[Extensible\]\]
    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_extensible(agent, value)
        } else if !value {
            self.create_backing_object(agent)
                .internal_set_extensible(agent, value)
        }
    }

    /// #### \[\[Prototype\]\]
    ///
    /// All ordinary objects have an internal slot called \[\[Prototype\]\].
    /// The value of this internal slot is either null or an object and is used
    /// for implementing inheritance.
    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_prototype(agent)
        } else {
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE),
            )
        }
    }

    /// #### \[\[Prototype\]\]
    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_prototype(agent, prototype)
        } else if prototype != self.internal_prototype(agent) {
            self.create_backing_object(agent)
                .internal_set_prototype(agent, prototype)
        }
    }
}
