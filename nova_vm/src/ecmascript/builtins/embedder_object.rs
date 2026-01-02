// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::Agent,
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

use self::data::EmbedderObjectHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EmbedderObject<'a>(BaseIndex<'a, EmbedderObjectHeapData>);

impl EmbedderObject<'_> {}

bindable_handle!(EmbedderObject);

impl<'a> From<EmbedderObject<'a>> for Object<'a> {
    fn from(value: EmbedderObject<'a>) -> Self {
        Object::EmbedderObject(value)
    }
}

impl<'a> TryFrom<Value<'a>> for EmbedderObject<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::EmbedderObject(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl HeapIndexHandle for EmbedderObject<'_> {
    fn from_index_u32(index: u32) -> Self {
        Self(BaseIndex::from_index_u32(index))
    }

    fn get_index_u32(&self) -> u32 {
        self.0.get_index_u32()
    }
}

impl<'a> InternalSlots<'a> for EmbedderObject<'a> {
    #[inline(always)]
    fn get_backing_object(self, _agent: &Agent) -> Option<OrdinaryObject<'static>> {
        todo!();
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        todo!();
    }

    fn create_backing_object(self, _agent: &mut Agent) -> OrdinaryObject<'static> {
        todo!();
    }
    fn internal_extensible(self, _agent: &Agent) -> bool {
        todo!();
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!();
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object<'static>> {
        todo!();
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!();
    }
}

impl<'a> InternalMethods<'a> for EmbedderObject<'a> {}

impl TryFrom<HeapRootData> for EmbedderObject<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::EmbedderObject(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl HeapMarkAndSweep for EmbedderObject<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.embedder_objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.embedder_objects.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for EmbedderObject<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .embedder_objects
            .shift_weak_index(self.0)
            .map(Self)
    }
}
