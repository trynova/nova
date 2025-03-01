// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{
        Scoped,
        context::NoGcScope,
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CreateHeapData, Heap, HeapMarkAndSweep,
        indexes::{BaseIndex, FinalizationRegistryIndex},
    },
};

use self::data::FinalizationRegistryHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FinalizationRegistry<'a>(pub(crate) FinalizationRegistryIndex<'a>);

impl FinalizationRegistry<'_> {
    /// Unbind this FinalizationRegistry from its current lifetime. This is necessary to use
    /// the FinalizationRegistry as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> FinalizationRegistry<'static> {
        unsafe { core::mem::transmute::<Self, FinalizationRegistry<'static>>(self) }
    }

    // Bind this FinalizationRegistry to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your FinalizationRegistrys cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let array_buffer = array_buffer.bind(&gc);
    // ```
    // to make sure that the unbound FinalizationRegistry cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> FinalizationRegistry<'gc> {
        unsafe { core::mem::transmute::<Self, FinalizationRegistry<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, FinalizationRegistry<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'a> IntoValue<'a> for FinalizationRegistry<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for FinalizationRegistry<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<FinalizationRegistry<'a>> for Value<'a> {
    fn from(value: FinalizationRegistry<'a>) -> Self {
        Value::FinalizationRegistry(value)
    }
}

impl<'a> From<FinalizationRegistry<'a>> for Object<'a> {
    fn from(value: FinalizationRegistry<'a>) -> Self {
        Object::FinalizationRegistry(value)
    }
}

impl<'a> InternalSlots<'a> for FinalizationRegistry<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::FinalizationRegistry;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for FinalizationRegistry<'a> {}

impl Index<FinalizationRegistry<'_>> for Agent {
    type Output = FinalizationRegistryHeapData;

    fn index(&self, index: FinalizationRegistry) -> &Self::Output {
        &self.heap.finalization_registrys[index]
    }
}

impl IndexMut<FinalizationRegistry<'_>> for Agent {
    fn index_mut(&mut self, index: FinalizationRegistry) -> &mut Self::Output {
        &mut self.heap.finalization_registrys[index]
    }
}

impl Index<FinalizationRegistry<'_>> for Vec<Option<FinalizationRegistryHeapData>> {
    type Output = FinalizationRegistryHeapData;

    fn index(&self, index: FinalizationRegistry) -> &Self::Output {
        self.get(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_ref()
            .expect("FinalizationRegistry slot empty")
    }
}

impl IndexMut<FinalizationRegistry<'_>> for Vec<Option<FinalizationRegistryHeapData>> {
    fn index_mut(&mut self, index: FinalizationRegistry) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_mut()
            .expect("FinalizationRegistry slot empty")
    }
}

impl Rootable for FinalizationRegistry<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::FinalizationRegistry(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::FinalizationRegistry(object) => Some(object),
            _ => None,
        }
    }
}

impl CreateHeapData<FinalizationRegistryHeapData, FinalizationRegistry<'static>> for Heap {
    fn create(&mut self, data: FinalizationRegistryHeapData) -> FinalizationRegistry<'static> {
        self.finalization_registrys.push(Some(data));
        FinalizationRegistry(FinalizationRegistryIndex::last(
            &self.finalization_registrys,
        ))
    }
}

impl HeapMarkAndSweep for FinalizationRegistry<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.finalization_registrys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.finalization_registrys.shift_index(&mut self.0);
    }
}
