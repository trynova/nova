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
        context::NoGcScope,
        rootable::{HeapRootData, HeapRootRef, Rootable},
        Scoped,
    },
    heap::{
        indexes::{BaseIndex, MapIndex},
        CompactionLists, CreateHeapData, HeapMarkAndSweep, WorkQueues,
    },
    Heap,
};

use self::data::MapHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Map<'a>(pub(crate) MapIndex<'a>);

impl<'a> Map<'a> {
    /// Unbind this Map from its current lifetime. This is necessary to use
    /// the Map as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> Map<'static> {
        unsafe { core::mem::transmute::<Self, Map<'static>>(self) }
    }

    // Bind this Map to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your Maps cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let map = map.bind(&gc);
    // ```
    // to make sure that the unbound Map cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> Map<'gc> {
        unsafe { core::mem::transmute::<Self, Map<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Map<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for Map<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for Map<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl From<Map<'_>> for Value {
    fn from(val: Map) -> Self {
        Value::Map(val.unbind())
    }
}

impl<'a> From<Map<'a>> for Object<'a> {
    fn from(val: Map) -> Self {
        Object::Map(val.unbind())
    }
}

impl<'a> TryFrom<Object<'a>> for Map<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::Map(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for Map<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Map;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }
}

impl<'a> InternalMethods<'a> for Map<'a> {}

impl HeapMarkAndSweep for Map<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.maps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.maps.shift_index(&mut self.0);
    }
}

impl Index<Map<'_>> for Agent {
    type Output = MapHeapData;

    fn index(&self, index: Map) -> &Self::Output {
        &self.heap.maps[index]
    }
}

impl IndexMut<Map<'_>> for Agent {
    fn index_mut(&mut self, index: Map) -> &mut Self::Output {
        &mut self.heap.maps[index]
    }
}

impl Index<Map<'_>> for Vec<Option<MapHeapData>> {
    type Output = MapHeapData;

    fn index(&self, index: Map) -> &Self::Output {
        self.get(index.get_index())
            .expect("Map out of bounds")
            .as_ref()
            .expect("Map slot empty")
    }
}

impl IndexMut<Map<'_>> for Vec<Option<MapHeapData>> {
    fn index_mut(&mut self, index: Map) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Map out of bounds")
            .as_mut()
            .expect("Map slot empty")
    }
}

impl Rootable for Map<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Map(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Map(object) => Some(object),
            _ => None,
        }
    }
}

impl CreateHeapData<MapHeapData, Map<'static>> for Heap {
    fn create(&mut self, data: MapHeapData) -> Map<'static> {
        self.maps.push(Some(data));
        Map(MapIndex::last(&self.maps))
    }
}
