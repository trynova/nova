// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::OrdinaryObject,
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues, element_array::ElementsVector},
};

/// An Array is an exotic object that gives special treatment to array index
/// property keys (see 6.1.7). A property whose property name is an array index
/// is also called an element. Every Array has a non-configurable "**length**"
/// property whose value is always a non-negative integral Number whose
/// mathematical value is strictly less than 2**32.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArrayHeapData<'a> {
    pub object_index: Option<OrdinaryObject<'a>>,
    // TODO: Use enum { ElementsVector, SmallVec<[Value; 3]> }
    // to get some inline benefit together with a 32 byte size
    // for ArrayHeapData to fit two in one cache line.
    pub elements: ElementsVector<'a>,
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ArrayHeapData<'_> {
    type Of<'a> = ArrayHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for ArrayHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            elements,
        } = self;
        object_index.mark_values(queues);
        elements.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            elements,
        } = self;
        object_index.sweep_values(compactions);
        elements.sweep_values(compactions);
    }
}
