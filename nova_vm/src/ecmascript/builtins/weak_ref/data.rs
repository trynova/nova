// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{OrdinaryObject, Value},
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone)]
pub struct WeakRefHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) value: Value<'a>,
    pub(crate) is_strong: bool,
}

impl Default for WeakRefHeapData<'_> {
    fn default() -> Self {
        Self {
            object_index: None,
            value: Value::Undefined,
            is_strong: false,
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for WeakRefHeapData<'_> {
    type Of<'a> = WeakRefHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for WeakRefHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            value,
            is_strong,
        } = self;
        object_index.mark_values(queues);
        if *is_strong {
            value.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            value,
            is_strong,
        } = self;
        object_index.sweep_values(compactions);
        value.sweep_values(compactions);
        *is_strong = false;
    }
}
