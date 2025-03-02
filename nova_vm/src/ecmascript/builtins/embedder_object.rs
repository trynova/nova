// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::Agent,
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{
        context::{Bindable, NoGcScope},
        rootable::HeapRootData,
    },
    heap::{
        HeapMarkAndSweep,
        indexes::{BaseIndex, EmbedderObjectIndex},
    },
};

use self::data::EmbedderObjectHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EmbedderObject<'a>(pub(crate) EmbedderObjectIndex<'a>);

impl EmbedderObject<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for EmbedderObject<'_> {
    type Of<'a> = EmbedderObject<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> IntoValue<'a> for EmbedderObject<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for EmbedderObject<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<EmbedderObject<'a>> for Value<'a> {
    fn from(value: EmbedderObject<'a>) -> Self {
        Value::EmbedderObject(value.unbind())
    }
}

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

impl Index<EmbedderObject<'_>> for Agent {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObject) -> &Self::Output {
        &self.heap.embedder_objects[index]
    }
}

impl IndexMut<EmbedderObject<'_>> for Agent {
    fn index_mut(&mut self, index: EmbedderObject) -> &mut Self::Output {
        &mut self.heap.embedder_objects[index]
    }
}

impl Index<EmbedderObject<'_>> for Vec<Option<EmbedderObjectHeapData>> {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObject) -> &Self::Output {
        self.get(index.get_index())
            .expect("EmbedderObject out of bounds")
            .as_ref()
            .expect("EmbedderObject slot empty")
    }
}

impl IndexMut<EmbedderObject<'_>> for Vec<Option<EmbedderObjectHeapData>> {
    fn index_mut(&mut self, index: EmbedderObject) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("EmbedderObject out of bounds")
            .as_mut()
            .expect("EmbedderObject slot empty")
    }
}

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
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.embedder_objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.embedder_objects.shift_index(&mut self.0);
    }
}
