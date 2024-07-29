// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, WeakRefIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::WeakRefHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakRef<'gen>(pub(crate) WeakRefIndex<'gen>);

impl WeakRef<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<WeakRef<'gen>> for WeakRefIndex<'gen> {
    fn from(val: WeakRef<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<WeakRefIndex<'gen>> for WeakRef<'gen> {
    fn from(value: WeakRefIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for WeakRef<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for WeakRef<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<WeakRef<'gen>> for Value<'gen> {
    fn from(val: WeakRef<'gen>) -> Self {
        Value::WeakRef(val)
    }
}

impl<'gen> From<WeakRef<'gen>> for Object<'gen> {
    fn from(val: WeakRef<'gen>) -> Self {
        Object::WeakRef(val)
    }
}

impl<'gen> InternalSlots<'gen> for WeakRef<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakRef;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
        debug_assert!(self.get_backing_object(agent).is_none());
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl<'gen> InternalMethods<'gen> for WeakRef<'gen> {}

impl<'gen> Index<WeakRef<'gen>> for Agent<'gen> {
    type Output = WeakRefHeapData<'gen>;

    fn index(&self, index: WeakRef<'gen>) -> &Self::Output {
        &self.heap.weak_refs[index]
    }
}

impl<'gen> IndexMut<WeakRef<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: WeakRef<'gen>) -> &mut Self::Output {
        &mut self.heap.weak_refs[index]
    }
}

impl<'gen> Index<WeakRef<'gen>> for Vec<Option<WeakRefHeapData<'gen>>> {
    type Output = WeakRefHeapData<'gen>;

    fn index(&self, index: WeakRef<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakRef out of bounds")
            .as_ref()
            .expect("WeakRef slot empty")
    }
}

impl<'gen> IndexMut<WeakRef<'gen>> for Vec<Option<WeakRefHeapData<'gen>>> {
    fn index_mut(&mut self, index: WeakRef<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakRef out of bounds")
            .as_mut()
            .expect("WeakRef slot empty")
    }
}

impl<'gen> CreateHeapData<WeakRefHeapData<'gen>, WeakRef<'gen>> for Heap<'gen> {
    fn create(&mut self, data: WeakRefHeapData<'gen>) -> WeakRef<'gen> {
        self.weak_refs.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakRef(WeakRefIndex::last(&self.weak_refs))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for WeakRef<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.weak_refs.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.weak_refs.shift_index(&mut self.0);
    }
}
