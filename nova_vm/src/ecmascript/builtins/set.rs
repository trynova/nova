// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, SetIndex},
        CompactionLists, CreateHeapData, HeapMarkAndSweep, ObjectEntry, WorkQueues,
    },
    Heap,
};

use self::data::SetHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Set<'gen>(pub(crate) SetIndex<'gen>);

impl Set<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<Set<'gen>> for SetIndex<'gen> {
    fn from(val: Set<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<SetIndex<'gen>> for Set<'gen> {
    fn from(value: SetIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for Set<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for Set<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<Set<'gen>> for Value<'gen> {
    fn from(val: Set<'gen>) -> Self {
        Value::Set(val)
    }
}

impl<'gen> From<Set<'gen>> for Object<'gen> {
    fn from(val: Set<'gen>) -> Self {
        Object::Set(val)
    }
}

impl<'gen> TryFrom<Value<'gen>> for Set<'gen> {
    type Error = ();

    fn try_from(value: Value<'gen>) -> Result<Self, Self::Error> {
        if let Value::Set(set) = value {
            Ok(set)
        } else {
            Err(())
        }
    }
}

impl<'gen> TryFrom<Object<'gen>> for Set<'gen> {
    type Error = ();

    fn try_from(value: Object<'gen>) -> Result<Self, Self::Error> {
        if let Object::Set(set) = value {
            Ok(set)
        } else {
            Err(())
        }
    }
}

fn create_set_base_object<'gen>(agent: &mut Agent<'gen>, set: Set<'gen>, entries: &[ObjectEntry<'gen>]) -> OrdinaryObject<'gen> {
    // TODO: An issue crops up if multiple realms are in play:
    // The prototype should not be dependent on the realm we're operating in
    // but should instead be bound to the realm the object was created in.
    // We'll have to cross this bridge at a later point, likely be designating
    // a "default realm" and making non-default realms always initialize ObjectHeapData.
    let prototype = agent.current_realm().intrinsics().set_prototype();
    let object_index = agent
        .heap
        .create_object_with_prototype(prototype.into(), entries);
    agent[set].object_index = Some(object_index);
    object_index
}

impl<'gen> InternalSlots<'gen> for Set<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Set;

    #[inline(always)]
    fn get_backing_object<'b>(self, agent: &'b Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> where 'gen: 'b {
        agent[self].object_index
    }

    fn create_backing_object<'b>(self, agent: &'b mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> where 'gen: 'b {
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

impl<'gen> InternalMethods<'gen> for Set<'gen> {}

impl<'gen> HeapMarkAndSweep<'gen> for Set<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.sets.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.sets.shift_index(&mut self.0);
    }
}

impl<'gen> Index<Set<'gen>> for Agent<'gen> {
    type Output = SetHeapData<'gen>;

    fn index(&self, index: Set<'gen>) -> &Self::Output {
        &self.heap.sets[index]
    }
}

impl<'gen> IndexMut<Set<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: Set<'gen>) -> &mut Self::Output {
        &mut self.heap.sets[index]
    }
}

impl<'gen> Index<Set<'gen>> for Vec<Option<SetHeapData<'gen>>> {
    type Output = SetHeapData<'gen>;

    fn index(&self, index: Set<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Set out of bounds")
            .as_ref()
            .expect("Set slot empty")
    }
}

impl<'gen> IndexMut<Set<'gen>> for Vec<Option<SetHeapData<'gen>>> {
    fn index_mut(&mut self, index: Set<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Set out of bounds")
            .as_mut()
            .expect("Set slot empty")
    }
}

impl<'gen> CreateHeapData<SetHeapData<'gen>, Set<'gen>> for Heap<'gen> {
    fn create(&mut self, data: SetHeapData<'gen>) -> Set<'gen> {
        self.sets.push(Some(data));
        Set(SetIndex::last(&self.sets))
    }
}
