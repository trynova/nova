// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{IntoObject, Object, ObjectHeapData, OrdinaryObject};
use crate::{
    ecmascript::execution::{Agent, ProtoIntrinsics},
    heap::CreateHeapData,
};

/// ### [10.1 Ordinary Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots)
pub trait InternalSlots
where
    Self: Sized + Copy + Into<Object> + IntoObject,
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
    /// This sets the custom "internal slot" \[\[BackingObject]].
    /// If the backing object is already set, this should panic.
    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject);

    /// ### \[\[BackingObject\]\]
    ///
    /// Creates the custom \[\[BackingObject]] object data. This is called when
    /// the item's object features are required but the backing object is None.
    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject {
        assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype,
            keys: Default::default(),
            values: Default::default(),
        });
        self.set_backing_object(agent, backing_object);
        backing_object
    }

    /// #### \[\[Extensible\]\]
    ///
    /// Every ordinary object has a Boolean-valued \[\[Extensible\]\] internal
    /// slot which is used to fulfill the extensibility-related internal method
    /// invariants specified in [6.1.7.3](https://tc39.es/ecma262/#sec-invariants-of-the-essential-internal-methods).
    fn internal_extensible(self, agent: &Agent) -> bool {
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_extensible(agent)
        } else {
            true
        }
    }

    /// #### \[\[Extensible\]\]
    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_set_extensible(agent, value)
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
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_prototype(agent)
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
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_set_prototype(agent, prototype)
        } else if prototype != self.internal_prototype(agent) {
            self.create_backing_object(agent)
                .internal_set_prototype(agent, prototype)
        }
    }
}
