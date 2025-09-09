// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Object;
use crate::{
    ecmascript::builtins::ordinary::shape::{ObjectShape, ObjectShapeRecord},
    engine::context::{Bindable, bindable_handle},
    heap::{
        CompactionLists, HeapMarkAndSweep, WorkQueues,
        element_array::{ElementArrayKey, ElementArrays, ElementStorageRef, ElementStorageUninit},
        indexes::ElementIndex,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ObjectRecord<'a> {
    shape: ObjectShape<'a>,
    values: ElementIndex<'a>,
}

impl<'a> ObjectRecord<'a> {
    pub(crate) const BLANK: Self = Self {
        shape: ObjectShape::NULL,
        values: ElementIndex::ZERO,
    };

    pub(crate) fn new(shape: ObjectShape<'a>, values: ElementIndex<'a>) -> Self {
        Self { shape, values }
    }

    pub(super) fn get_storage<'e>(
        &self,
        elements: &'e ElementArrays,
        shapes: &[ObjectShapeRecord<'static>],
    ) -> ElementStorageRef<'e, 'a> {
        elements.get_element_storage_raw(
            self.values,
            self.shape.capacity(&shapes),
            self.shape.len(&shapes),
        )
    }

    pub(super) fn get_storage_uninit<'e>(
        &self,
        elements: &'e mut ElementArrays,
        shapes: &[ObjectShapeRecord<'static>],
    ) -> ElementStorageUninit<'e> {
        elements.get_element_storage_uninit_raw(self.values, self.shape.capacity(&shapes))
    }

    pub(crate) fn is_empty(&self, agent: &impl AsRef<[ObjectShapeRecord<'static>]>) -> bool {
        self.shape == ObjectShape::NULL || self.shape.is_empty(agent)
    }

    pub(super) fn get_extensible(&self) -> bool {
        self.shape.extensible()
    }

    pub(super) fn set_extensible(&mut self, extensible: bool) {
        self.shape.set_extensible(extensible)
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

    pub(super) fn set_values(&mut self, values: ElementIndex<'a>) {
        self.values = values;
    }

    pub(super) fn capacity_key(
        &self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> ElementArrayKey {
        self.shape.capacity(agent)
    }

    pub(crate) fn len(&self, agent: &impl AsRef<[ObjectShapeRecord<'static>]>) -> u32 {
        self.shape.len(agent)
    }
}

bindable_handle!(ObjectRecord);

impl HeapMarkAndSweep for ObjectRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { shape, values: _ } = self;
        shape.mark_values(queues);
        // Note: we cannot mark the values here as we don't know the capacity
        // or length of it.
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { shape, values: _ } = self;
        shape.sweep_values(compactions);
        // Note: we cannot sweep the values here as we don't know the capacity
        // or length of it.
    }
}
