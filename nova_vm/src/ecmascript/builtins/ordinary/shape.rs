// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::marker::PhantomData;

use hashbrown::HashTable;

use crate::{
    ecmascript::types::{Object, PropertyKey},
    engine::context::{Bindable, GcToken, NoGcScope},
    heap::{
        CompactionLists, HeapMarkAndSweep, WorkQueues, element_array::ElementArrayKey,
        indexes::PropertyKeyIndex,
    },
};

/// Data structure describing the shape of an object.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ObjectShape<'a>(u32, PhantomData<&'a GcToken>);

impl ObjectShape<'_> {
    pub(crate) fn get_index(self) -> usize {
        self.0 as usize
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ObjectShape<'_> {
    type Of<'a> = ObjectShape<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

/// Data structure describing the shape of an object.
///
/// ## What is a shape?
///
/// Object shapes describe the "shape", ie. the keys of an object and their
/// order. For shape-finding purposes, they also describe any descendants that
/// the shape may have, eg. the object shape `{ x, y }` is a descendant of the
/// shape `{ x }`, as it is created by adding `y` to the latter.
///
/// ### Why have shapes?
///
/// Shapes are a fundamental and important mechanism of JavaScript engines in
/// general. They are a requirement for a few critically important
/// optimisations without which a JavaScript engine is woefully inadequate as a
/// modern general-purpose programming tool.
///
/// The first optimisation they enable is deduplication of object keys; two
/// objects both containing `{ x, y }` do not need to store a list of keys
/// each, with both lists containing `x` and `y`. Instead, they both refer to
/// an object shape that contains the list of keys. This cuts object memory
/// usage roughly in half or more, as an object only needs to store its
/// property values without the keys.
///
/// The second optimisation they enable is inline caching of property lookups:
/// when JavaScript code performs a property lookup, eg. `obj.x`, it can store
/// the object shape and offset where it found the property in an "inline
/// cache" (the name stems from the cache data often being stored in the
/// bytecode or machine code data directly, "in line"). When the lookup gets
/// repeated, the code can check if the object shape matches and skip the
/// property search entirely if a match is found.
#[derive(Debug)]
pub struct ObjectShapeRecord<'a> {
    /// Prototype of the object shape.
    ///
    /// This takes the place of the \[\[Prototype]] internal slot for (most)
    /// objects. Two otherwise equivalent objects may take different code paths
    /// upon property lookup or assignment based on their prototypes, hence the
    /// prototype must be a part of the shape.
    prototype: Option<Object<'a>>,
    /// Keys storage of the shape.
    ///
    /// The keys storage index is given by this value, while the vector
    /// (capacity) is determined by the length value. The capacity is always
    /// the smallest possible capacity that can fit all keys.
    keys: PropertyKeyIndex<'a>,
    /// Length of the keys storage of the shape.
    ///
    /// This determines the keys storage vector (capacity) as well.
    len: u32,
}

/// Data structure for finding a forward transition from an Object Shape to a
/// larger one when a property key is added.
#[derive(Debug)]
pub(crate) struct ObjectShapeTransitionMap<'a> {
    /// Hash table keyed by PropertKeys, pointing to an ObjectShape that is
    /// reached by adding said property key to the current Shape.
    table: HashTable<u32>,
    keys: Vec<PropertyKey<'a>>,
    values: Vec<ObjectShape<'a>>,
}

impl HeapMarkAndSweep for ObjectShape<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.object_shapes.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.object_shapes.shift_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for ObjectShapeRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            prototype,
            keys,
            len,
        } = self;
        prototype.mark_values(queues);
        match ElementArrayKey::from(*len) {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => queues.k_2_4.push((*keys, *len)),
            ElementArrayKey::E6 => queues.k_2_6.push((*keys, *len)),
            ElementArrayKey::E8 => queues.k_2_8.push((*keys, *len)),
            ElementArrayKey::E10 => queues.k_2_10.push((*keys, *len)),
            ElementArrayKey::E12 => queues.k_2_12.push((*keys, *len)),
            ElementArrayKey::E16 => queues.k_2_16.push((*keys, *len)),
            ElementArrayKey::E24 => queues.k_2_24.push((*keys, *len)),
            ElementArrayKey::E32 => queues.k_2_32.push((*keys, *len)),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            prototype,
            keys,
            len,
        } = self;
        prototype.sweep_values(compactions);
        match ElementArrayKey::from(*len) {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => compactions.k_2_4.shift_index(keys),
            ElementArrayKey::E6 => compactions.k_2_6.shift_index(keys),
            ElementArrayKey::E8 => compactions.k_2_8.shift_index(keys),
            ElementArrayKey::E10 => compactions.k_2_10.shift_index(keys),
            ElementArrayKey::E12 => compactions.k_2_12.shift_index(keys),
            ElementArrayKey::E16 => compactions.k_2_16.shift_index(keys),
            ElementArrayKey::E24 => compactions.k_2_24.shift_index(keys),
            ElementArrayKey::E32 => compactions.k_2_32.shift_index(keys),
        }
    }
}

impl HeapMarkAndSweep for ObjectShapeTransitionMap<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            table: _,
            keys,
            values,
        } = self;
        keys.mark_values(queues);
        values.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            table: _,
            keys,
            values,
        } = self;
        keys.sweep_values(compactions);
        values.sweep_values(compactions);
    }
}
