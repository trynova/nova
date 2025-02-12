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
    engine::{context::NoGcScope, rootable::HeapRootData, Scoped},
    heap::{
        indexes::{BaseIndex, SetIndex},
        CompactionLists, CreateHeapData, HeapMarkAndSweep, WorkQueues,
    },
    Heap,
};

use self::data::SetHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Set<'a>(pub(crate) SetIndex<'a>);

impl Set<'_> {
    /// Unbind this Set from its current lifetime. This is necessary to use
    /// the Set as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> Set<'static> {
        unsafe { core::mem::transmute::<Self, Set<'static>>(self) }
    }

    // Bind this Set to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your Sets cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let set = set.bind(&gc);
    // ```
    // to make sure that the unbound Set cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> Set<'gc> {
        unsafe { core::mem::transmute::<Self, Set<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Set<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for Set<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for Set<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl From<Set<'_>> for Value {
    fn from(val: Set) -> Self {
        Value::Set(val.unbind())
    }
}

impl<'a> From<Set<'a>> for Object<'a> {
    fn from(val: Set) -> Self {
        Object::Set(val.unbind())
    }
}

impl TryFrom<Value> for Set<'_> {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Set(set) = value {
            Ok(set)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Object<'a>> for Set<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        if let Object::Set(set) = value {
            Ok(set)
        } else {
            Err(())
        }
    }
}

impl<'a> InternalSlots<'a> for Set<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Set;

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

impl<'a> InternalMethods<'a> for Set<'a> {}

impl HeapMarkAndSweep for Set<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.sets.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.sets.shift_index(&mut self.0);
    }
}

impl Index<Set<'_>> for Agent {
    type Output = SetHeapData;

    fn index(&self, index: Set) -> &Self::Output {
        &self.heap.sets[index]
    }
}

impl IndexMut<Set<'_>> for Agent {
    fn index_mut(&mut self, index: Set) -> &mut Self::Output {
        &mut self.heap.sets[index]
    }
}

impl Index<Set<'_>> for Vec<Option<SetHeapData>> {
    type Output = SetHeapData;

    fn index(&self, index: Set) -> &Self::Output {
        self.get(index.get_index())
            .expect("Set out of bounds")
            .as_ref()
            .expect("Set slot empty")
    }
}

impl IndexMut<Set<'_>> for Vec<Option<SetHeapData>> {
    fn index_mut(&mut self, index: Set) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Set out of bounds")
            .as_mut()
            .expect("Set slot empty")
    }
}

impl TryFrom<HeapRootData> for Set<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::Set(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl CreateHeapData<SetHeapData, Set<'static>> for Heap {
    fn create(&mut self, data: SetHeapData) -> Set<'static> {
        self.sets.push(Some(data));
        Set(SetIndex::last(&self.sets))
    }
}
