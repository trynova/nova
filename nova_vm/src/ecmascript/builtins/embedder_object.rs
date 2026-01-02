// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

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
        CompactionLists, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, indexes::BaseIndex,
    },
};

use self::data::EmbedderObjectHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EmbedderObject<'a>(BaseIndex<'a, EmbedderObjectHeapData>);

impl EmbedderObject<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

bindable_handle!(EmbedderObject);

impl<'a> From<EmbedderObject<'a>> for Object<'a> {
    fn from(value: EmbedderObject<'a>) -> Self {
        Object::EmbedderObject(value)
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
