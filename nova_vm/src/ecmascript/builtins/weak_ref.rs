// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{context::NoGcScope, rootable::HeapRootData, Scoped},
    heap::{
        indexes::{BaseIndex, WeakRefIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::WeakRefHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakRef<'a>(pub(crate) WeakRefIndex<'a>);

impl WeakRef<'_> {
    /// Unbind this WeakRef from its current lifetime. This is necessary to use
    /// the WeakRef as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> WeakRef<'static> {
        unsafe { std::mem::transmute::<Self, WeakRef<'static>>(self) }
    }

    // Bind this WeakRef to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your WeakRefs cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let weak_ref = weak_ref.bind(&gc);
    // ```
    // to make sure that the unbound WeakRef cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> WeakRef<'gc> {
        unsafe { std::mem::transmute::<Self, WeakRef<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, WeakRef<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for WeakRef<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for WeakRef<'_> {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<WeakRef<'_>> for Value {
    fn from(val: WeakRef) -> Self {
        Value::WeakRef(val.unbind())
    }
}

impl From<WeakRef<'_>> for Object {
    fn from(val: WeakRef) -> Self {
        Object::WeakRef(val.unbind())
    }
}

impl InternalSlots for WeakRef<'_> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakRef;

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

impl InternalMethods for WeakRef<'_> {}

impl Index<WeakRef<'_>> for Agent {
    type Output = WeakRefHeapData;

    fn index(&self, index: WeakRef) -> &Self::Output {
        &self.heap.weak_refs[index]
    }
}

impl IndexMut<WeakRef<'_>> for Agent {
    fn index_mut(&mut self, index: WeakRef) -> &mut Self::Output {
        &mut self.heap.weak_refs[index]
    }
}

impl Index<WeakRef<'_>> for Vec<Option<WeakRefHeapData>> {
    type Output = WeakRefHeapData;

    fn index(&self, index: WeakRef) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakRef out of bounds")
            .as_ref()
            .expect("WeakRef slot empty")
    }
}

impl IndexMut<WeakRef<'_>> for Vec<Option<WeakRefHeapData>> {
    fn index_mut(&mut self, index: WeakRef) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakRef out of bounds")
            .as_mut()
            .expect("WeakRef slot empty")
    }
}

impl TryFrom<HeapRootData> for WeakRef<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::WeakRef(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl CreateHeapData<WeakRefHeapData, WeakRef<'static>> for Heap {
    fn create(&mut self, data: WeakRefHeapData) -> WeakRef<'static> {
        self.weak_refs.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakRef(WeakRefIndex::last(&self.weak_refs))
    }
}

impl HeapMarkAndSweep for WeakRef<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.weak_refs.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.weak_refs.shift_index(&mut self.0);
    }
}
