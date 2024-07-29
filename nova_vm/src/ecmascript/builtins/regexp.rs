// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) mod abstract_operations;
pub(crate) mod data;

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
        indexes::{BaseIndex, RegExpIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};
pub(crate) use abstract_operations::*;
pub(crate) use data::RegExpHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RegExp<'gen>(RegExpIndex<'gen>);

impl RegExp<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<RegExp<'gen>> for Value<'gen> {
    fn from(value: RegExp<'gen>) -> Self {
        Self::RegExp(value)
    }
}

impl<'gen> From<RegExp<'gen>> for Object<'gen> {
    fn from(value: RegExp<'gen>) -> Self {
        Self::RegExp(value)
    }
}

impl<'gen> TryFrom<Object<'gen>> for RegExp<'gen> {
    type Error = ();

    fn try_from(value: Object<'gen>) -> Result<Self, Self::Error> {
        match value {
            Object::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
    }
}

impl<'gen> TryFrom<Value<'gen>> for RegExp<'gen> {
    type Error = ();

    fn try_from(value: Value<'gen>) -> Result<Self, Self::Error> {
        match value {
            Value::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
    }
}

impl<'gen> IntoValue<'gen> for RegExp<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for RegExp<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> InternalSlots<'gen> for RegExp<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::RegExp;

    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> OrdinaryObject<'gen> {
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

impl<'gen> InternalMethods<'gen> for RegExp<'gen> {}

impl<'gen> HeapMarkAndSweep<'gen> for RegExp<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.regexps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.regexps.shift_index(&mut self.0);
    }
}

impl<'gen> Index<RegExp<'gen>> for Agent<'gen> {
    type Output = RegExpHeapData<'gen>;

    fn index(&self, index: RegExp<'gen>) -> &Self::Output {
        &self.heap.regexps[index]
    }
}

impl<'gen> IndexMut<RegExp<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: RegExp<'gen>) -> &mut Self::Output {
        &mut self.heap.regexps[index]
    }
}

impl<'gen> Index<RegExp<'gen>> for Vec<Option<RegExpHeapData<'gen>>> {
    type Output = RegExpHeapData<'gen>;

    fn index(&self, index: RegExp<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("RegExp out of bounds")
            .as_ref()
            .expect("RegExp slot empty")
    }
}

impl<'gen> IndexMut<RegExp<'gen>> for Vec<Option<RegExpHeapData<'gen>>> {
    fn index_mut(&mut self, index: RegExp<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("RegExp out of bounds")
            .as_mut()
            .expect("RegExp slot empty")
    }
}

impl<'gen> CreateHeapData<RegExpHeapData<'gen>, RegExp<'gen>> for Heap<'gen> {
    fn create(&mut self, data: RegExpHeapData<'gen>) -> RegExp<'gen> {
        self.regexps.push(Some(data));
        RegExp(RegExpIndex::last(&self.regexps))
    }
}
