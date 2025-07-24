// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Object;
use crate::{
    ecmascript::builtins::ordinary::shape::{ObjectShape, ObjectShapeRecord},
    engine::context::{Bindable, NoGcScope},
    heap::{
        CompactionLists, HeapMarkAndSweep, WorkQueues,
        element_array::{ElementArrayKey, ElementArrays, ElementStorageRef, ElementStorageUninit},
        indexes::ElementIndex,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct ObjectHeapData<'a> {
    shape: ObjectShape<'a>,
    values: ElementIndex<'a>,
    cap: ElementArrayKey,
    len: u32,
    // TODO: move this bit to ObjectShape value.
    extensible: bool,
}

impl<'a> ObjectHeapData<'a> {
    pub(crate) fn new(
        shape: ObjectShape<'a>,
        values: ElementIndex<'a>,
        cap: ElementArrayKey,
        len: u32,
        extensible: bool,
    ) -> Self {
        Self {
            shape,
            values,
            cap,
            len,
            extensible,
        }
    }

    /// Reserve memory for given size property storage.
    pub(super) fn reserve(&mut self, elements: &mut impl AsMut<ElementArrays>, new_len: u32) {
        if self.cap >= ElementArrayKey::from(new_len) {
            // Enough room to hold the new data; nothing to do.
            return;
        }
        elements
            .as_mut()
            .reserve_elements_raw(&mut self.values, &mut self.cap, self.len, new_len);
    }

    pub(super) fn get_storage<'e>(&self, elements: &'e ElementArrays) -> ElementStorageRef<'e, 'a> {
        elements.get_element_storage_raw(self.values, self.cap, self.len)
    }

    pub(super) fn get_storage_uninit<'e>(
        &self,
        elements: &'e mut ElementArrays,
    ) -> ElementStorageUninit<'e> {
        elements.get_element_storage_uninit_raw(self.values, self.cap)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.cap == ElementArrayKey::Empty
    }

    pub(super) fn get_extensible(&self) -> bool {
        self.extensible
    }

    pub(super) fn set_extensible(&mut self, extensible: bool) {
        self.extensible = extensible;
    }

    pub(super) fn get_prototype(
        &self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> Option<Object<'a>> {
        self.shape.get_prototype(agent)
    }

    pub(super) fn get_shape(&self) -> ObjectShape<'a> {
        self.shape
    }

    pub(super) fn set_shape(&mut self, shape: ObjectShape) {
        self.shape = shape.unbind();
    }

    pub(super) fn get_values(&self) -> ElementIndex<'a> {
        self.values
    }

    pub(super) fn get_cap(&self) -> ElementArrayKey {
        self.cap
    }

    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn set_len(&mut self, len: u32) {
        self.len = len
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ObjectHeapData<'_> {
    type Of<'a> = ObjectHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for ObjectHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            shape,
            values,
            cap,
            len,
            extensible: _,
        } = self;
        shape.mark_values(queues);
        match cap {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => {
                queues.e_2_4.push((*values, *len));
            }
            ElementArrayKey::E6 => {
                queues.e_2_6.push((*values, *len));
            }
            ElementArrayKey::E8 => {
                queues.e_2_8.push((*values, *len));
            }
            ElementArrayKey::E10 => {
                queues.e_2_10.push((*values, *len));
            }
            ElementArrayKey::E12 => {
                queues.e_2_12.push((*values, *len));
            }
            ElementArrayKey::E16 => {
                queues.e_2_16.push((*values, *len));
            }
            ElementArrayKey::E24 => {
                queues.e_2_24.push((*values, *len));
            }
            ElementArrayKey::E32 => {
                queues.e_2_32.push((*values, *len));
            }
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            shape,
            values,
            cap,
            len: _,
            extensible: _,
        } = self;
        shape.sweep_values(compactions);
        match cap {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => {
                compactions.e_2_4.shift_index(values);
            }
            ElementArrayKey::E6 => {
                compactions.e_2_6.shift_index(values);
            }
            ElementArrayKey::E8 => {
                compactions.e_2_8.shift_index(values);
            }
            ElementArrayKey::E10 => {
                compactions.e_2_10.shift_index(values);
            }
            ElementArrayKey::E12 => {
                compactions.e_2_12.shift_index(values);
            }
            ElementArrayKey::E16 => {
                compactions.e_2_16.shift_index(values);
            }
            ElementArrayKey::E24 => {
                compactions.e_2_24.shift_index(values);
            }
            ElementArrayKey::E32 => {
                compactions.e_2_32.shift_index(values);
            }
        }
    }
}
