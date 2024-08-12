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
        indexes::{BaseIndex, WeakMapIndex},
        CreateHeapData, HeapMarkAndSweep,
    },
    Heap,
};

use self::data::WeakMapHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakMap<'gen>(pub(crate) WeakMapIndex<'gen>);

impl WeakMap<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<WeakMap<'gen>> for WeakMapIndex<'gen> {
    fn from(val: WeakMap<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<WeakMapIndex<'gen>> for WeakMap<'gen> {
    fn from(value: WeakMapIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for WeakMap<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for WeakMap<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<WeakMap<'gen>> for Value<'gen> {
    fn from(val: WeakMap<'gen>) -> Self {
        Value::WeakMap(val)
    }
}

impl<'gen> From<WeakMap<'gen>> for Object<'gen> {
    fn from(val: WeakMap<'gen>) -> Self {
        Object::WeakMap(val)
    }
}

impl<'gen> InternalSlots<'gen> for WeakMap<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakMap;

    #[inline(always)]
    fn get_backing_object<'b>(self, agent: &'b Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> where 'gen: 'b {
        agent[self].object_index
    }

    fn create_backing_object<'b>(self, agent: &'b mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> where 'gen: 'b {
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

impl<'gen> InternalMethods<'gen> for WeakMap<'gen> {}

impl<'gen> Index<WeakMap<'gen>> for Agent<'gen> {
    type Output = WeakMapHeapData<'gen>;

    fn index(&self, index: WeakMap<'gen>) -> &Self::Output {
        &self.heap.weak_maps[index]
    }
}

impl<'gen> IndexMut<WeakMap<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: WeakMap<'gen>) -> &mut Self::Output {
        &mut self.heap.weak_maps[index]
    }
}

impl<'gen> Index<WeakMap<'gen>> for Vec<Option<WeakMapHeapData<'gen>>> {
    type Output = WeakMapHeapData<'gen>;

    fn index(&self, index: WeakMap<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakMap out of bounds")
            .as_ref()
            .expect("WeakMap slot empty")
    }
}

impl<'gen> IndexMut<WeakMap<'gen>> for Vec<Option<WeakMapHeapData<'gen>>> {
    fn index_mut(&mut self, index: WeakMap<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakMap out of bounds")
            .as_mut()
            .expect("WeakMap slot empty")
    }
}

impl<'gen> CreateHeapData<WeakMapHeapData<'gen>, WeakMap<'gen>> for Heap<'gen> {
    fn create(&mut self, data: WeakMapHeapData<'gen>) -> WeakMap<'gen> {
        self.weak_maps.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakMap(WeakMapIndex::last(&self.weak_maps))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for WeakMap<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.weak_maps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.weak_maps.shift_index(&mut self.0);
    }
}
