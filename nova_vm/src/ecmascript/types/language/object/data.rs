// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Object;
use crate::{
    ecmascript::{ObjectShape, ObjectShapeRecord},
    engine::context::{Bindable, bindable_handle},
    heap::{
        CompactionLists, ElementIndex, HeapMarkAndSweep, WorkQueues,
        {ElementArrayKey, ElementArrays, ElementStorageRef},
    },
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ObjectRecord<'a> {
    pub(crate) shape: ObjectShape<'a>,
    pub(crate) values: ElementIndex<'a>,
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
        shapes: &Vec<ObjectShapeRecord<'static>>,
    ) -> ElementStorageRef<'e, 'a> {
        elements.get_element_storage_raw(
            self.values,
            self.shape.values_capacity(&shapes),
            self.shape.len(&shapes),
        )
    }

    pub(crate) fn is_empty(&self, agent: &impl AsRef<Vec<ObjectShapeRecord<'static>>>) -> bool {
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
        agent: &impl AsRef<Vec<ObjectShapeRecord<'static>>>,
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

    pub(super) fn values_capacity(
        &self,
        agent: &impl AsRef<Vec<ObjectShapeRecord<'static>>>,
    ) -> ElementArrayKey {
        self.shape.values_capacity(agent)
    }

    pub(crate) fn len(&self, agent: &impl AsRef<Vec<ObjectShapeRecord<'static>>>) -> u32 {
        self.shape.len(agent)
    }
}

impl ObjectRecord<'static> {
    /// Manual implementation of marking for ObjectRecord. This needs access to
    /// the shapes vector as well.
    pub(crate) fn mark_values(
        &self,
        queues: &mut WorkQueues,
        shapes: &Vec<ObjectShapeRecord<'static>>,
    ) {
        let Self { shape, values } = self;
        shape.mark_values(queues);
        match shape.values_capacity(&shapes) {
            ElementArrayKey::Empty | ElementArrayKey::EmptyIntrinsic => {}
            ElementArrayKey::E1 => queues.e_2_1.push(*values),
            ElementArrayKey::E2 => queues.e_2_2.push(*values),
            ElementArrayKey::E3 => queues.e_2_3.push(*values),
            ElementArrayKey::E4 => queues.e_2_4.push(*values),
            ElementArrayKey::E6 => queues.e_2_6.push(*values),
            ElementArrayKey::E8 => queues.e_2_8.push(*values),
            ElementArrayKey::E10 => queues.e_2_10.push(*values),
            ElementArrayKey::E12 => queues.e_2_12.push(*values),
            ElementArrayKey::E16 => queues.e_2_16.push(*values),
            ElementArrayKey::E24 => queues.e_2_24.push(*values),
            ElementArrayKey::E32 => queues.e_2_32.push(*values),
        }
    }

    /// Manual implementation of marking for ObjectRecord. This needs access to
    /// the shapes vector as well. The shapes are assumed to have been sweeped
    /// already.
    pub(crate) fn sweep_values(
        &mut self,
        compactions: &CompactionLists,
        shapes: &Vec<ObjectShapeRecord<'static>>,
    ) {
        let Self { shape, values } = self;
        shape.sweep_values(compactions);
        match shape.values_capacity(&shapes) {
            ElementArrayKey::Empty | ElementArrayKey::EmptyIntrinsic => {}
            ElementArrayKey::E1 => compactions.e_2_1.shift_index(values),
            ElementArrayKey::E2 => compactions.e_2_2.shift_index(values),
            ElementArrayKey::E3 => compactions.e_2_3.shift_index(values),
            ElementArrayKey::E4 => compactions.e_2_4.shift_index(values),
            ElementArrayKey::E6 => compactions.e_2_6.shift_index(values),
            ElementArrayKey::E8 => compactions.e_2_8.shift_index(values),
            ElementArrayKey::E10 => compactions.e_2_10.shift_index(values),
            ElementArrayKey::E12 => compactions.e_2_12.shift_index(values),
            ElementArrayKey::E16 => compactions.e_2_16.shift_index(values),
            ElementArrayKey::E24 => compactions.e_2_24.shift_index(values),
            ElementArrayKey::E32 => compactions.e_2_32.shift_index(values),
        };
    }
}

bindable_handle!(ObjectRecord);
