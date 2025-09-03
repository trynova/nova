// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use soavec_derive::SoAble;

use crate::{
    ecmascript::types::OrdinaryObject,
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues, element_array::ElementsVector},
};

/// An Array is an exotic object that gives special treatment to array index
/// property keys (see 6.1.7). A property whose property name is an array index
/// is also called an element. Every Array has a non-configurable "**length**"
/// property whose value is always a non-negative integral Number whose
/// mathematical value is strictly less than 2**32.
#[derive(Debug, Clone, Copy, Default, SoAble)]
pub struct ArrayHeapData<'a> {
    pub elements: ElementsVector<'a>,
    pub object_index: Option<OrdinaryObject<'a>>,
}
bindable_handle!(ArrayHeapData);

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
