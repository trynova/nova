// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    cmp::Ordering, collections::TryReserveError, marker::PhantomData, num::NonZeroU32, ptr::NonNull,
};

use ahash::AHashMap;
use hashbrown::{HashTable, hash_table::Entry};

use crate::{
    ecmascript::{
        execution::{Agent, PrivateField, Realm},
        types::{
            InternalMethods, IntoObject, Object, Primitive, PropertyKey, Symbol, TryGetResult,
            Value,
        },
    },
    engine::context::{Bindable, GcToken, NoGcScope, bindable_handle},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        IntrinsicObjectShapes, PropertyKeyHeap, WeakReference, WorkQueues,
        element_array::{ElementArrayKey, ElementArrays},
        indexes::PropertyKeyIndex,
    },
};

use super::caches::PropertyLookupCache;

/// Data structure describing the shape of an object.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectShape<'a>(
    // Non-zero u31; zero is reserved for None.
    // The top bit is reserved for OrdinaryObject extensible bit.
    NonZeroU32,
    PhantomData<&'a GcToken>,
);

impl<'a> ObjectShape<'a> {
    /// Object Shape for `{ __proto__: null }`.
    ///
    /// This is the root Object Shape for all null-prototype objects, hence why
    /// it can be accessed statically.
    pub(crate) const NULL: Self = Self::from_non_zero(NonZeroU32::new(1).unwrap());

    /// Returns true if the Object Shape belongs to an intrinsic object.
    ///
    /// Intrinsic Object Shapes are uniquely owned by their intrinsic object
    /// and will thus be mutated upon intrinsic object mutation. It is thus not
    /// safe to inherit an intrinsic Object Shape.
    pub(crate) fn is_intrinsic(self, agent: &Agent) -> bool {
        self.get(agent).keys_cap != ElementArrayKey::Empty
            && self.get_transitions(agent).parent.is_none()
    }

    /// Get the Object Shape record.
    fn get(self, agent: &Agent) -> &ObjectShapeRecord<'a> {
        self.get_direct(&agent.heap.object_shapes)
    }

    /// Get the Object Shape record as mutable.
    #[inline(always)]
    fn get_mut(self, agent: &mut Agent) -> &mut ObjectShapeRecord<'static> {
        self.get_direct_mut(&mut agent.heap.object_shapes)
    }

    /// Get the Object Shape record as mutable.
    #[inline(always)]
    fn get_direct<'r>(
        self,
        object_shapes: &'r [ObjectShapeRecord<'a>],
    ) -> &'r ObjectShapeRecord<'a> {
        &object_shapes[self.get_index()]
    }

    /// Get the Object Shape record as mutable.
    #[inline(always)]
    fn get_direct_mut<'r>(
        self,
        object_shapes: &'r mut [ObjectShapeRecord<'static>],
    ) -> &'r mut ObjectShapeRecord<'static> {
        &mut object_shapes[self.get_index()]
    }

    /// Get the Object Shape transitions.
    fn get_transitions(self, agent: &Agent) -> &ObjectShapeTransitionMap<'a> {
        self.get_transitions_direct(&agent.heap.object_shape_transitions)
    }

    /// Get the Object Shape transitions.
    fn get_transitions_direct<'r>(
        self,
        transitions: &'r [ObjectShapeTransitionMap<'a>],
    ) -> &'r ObjectShapeTransitionMap<'a> {
        &transitions[self.get_index()]
    }

    /// Get the Object Shape transitions as mutable.
    pub(crate) fn get_transitions_direct_mut<'r>(
        self,
        transitions: &'r mut [ObjectShapeTransitionMap<'static>],
    ) -> &'r mut ObjectShapeTransitionMap<'a> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<
                &'r mut ObjectShapeTransitionMap<'static>,
                &'r mut ObjectShapeTransitionMap<'a>,
            >(&mut transitions[self.get_index()])
        }
    }

    /// Get the implied usize index of the ObjectShape reference.
    #[inline(always)]
    pub(crate) const fn get_index(self) -> usize {
        let raw_value = self.0.get();
        // Extract the raw value by masking out the extensible bit.
        (raw_value & 0x7FFF_FFFF) as usize - 1
    }

    /// Get the PropertyKeys of the Object Shape as a slice.
    pub(crate) fn keys<'e>(
        self,
        object_shapes: &[ObjectShapeRecord<'static>],
        elements: &'e ElementArrays,
    ) -> &'e [PropertyKey<'a>] {
        let data = &object_shapes[self.get_index()];
        debug_assert_eq!(data.values_cap, data.len.into());
        elements.get_keys_raw(data.keys_cap, data.keys, data.len)
    }

    /// Get the PropertyKeyIndex of the Object Shape.
    pub(crate) fn keys_index(
        self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> PropertyKeyIndex<'a> {
        self.get_direct(agent.as_ref()).keys
    }

    /// Get capacity of the keys storage referred to by this Object Shape.
    pub(crate) fn keys_capacity(
        self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> ElementArrayKey {
        self.get_direct(agent.as_ref()).keys_cap
    }

    /// Get the Object property values capacity implied by this Object Shape.
    pub(crate) fn values_capacity(
        self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> ElementArrayKey {
        self.get_direct(agent.as_ref()).values_cap
    }

    /// Get the length of the Object Shape keys.
    pub(crate) fn len(self, agent: &impl AsRef<[ObjectShapeRecord<'static>]>) -> u32 {
        self.get_direct(agent.as_ref()).len
    }

    /// Return true if the OrdinaryObject holder of this ObjectShape reference
    /// is extensible.
    ///
    /// Note that this is not an inherent property of the ObjectShape itself.
    pub(crate) fn extensible(self) -> bool {
        (self.0.get() & 0x8000_0000) == 0
    }

    pub(crate) fn set_extensible(&mut self, extensible: bool) {
        if !extensible {
            // SAFETY: Non-null u31 OR'd with a top bit is also non-null.
            self.0 = unsafe { NonZeroU32::new_unchecked(self.0.get() | 0x8000_0000) };
        } else {
            // SAFETY: Non-null u31 AND'd with all but top bit set is also non-null.
            self.0 = unsafe { NonZeroU32::new_unchecked(self.0.get() & 0x7FFF_FFFF) };
        }
    }

    /// Get the length of the Object Shape keys.
    pub(crate) fn is_empty(self, agent: &impl AsRef<[ObjectShapeRecord<'static>]>) -> bool {
        self.get_direct(agent.as_ref()).is_empty()
    }

    /// Get the prototype of the Object Shape.
    pub(crate) fn get_prototype(
        self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> Option<Object<'a>> {
        self.get_direct(agent.as_ref()).prototype
    }

    /// Get the parent Object Shape of this Object Shape.
    pub(crate) fn get_parent(
        self,
        agent: &impl AsRef<[ObjectShapeTransitionMap<'static>]>,
    ) -> Option<ObjectShape<'a>> {
        self.get_transitions_direct(agent.as_ref()).parent
    }

    /// Get the Object Shape that is reached by adding the given property to
    /// this Object Shape.
    ///
    /// Returns None if no transition exists.
    pub(crate) fn get_transition_to(
        self,
        agent: &Agent,
        key: PropertyKey<'a>,
    ) -> Option<ObjectShape<'a>> {
        let transitions = &agent.heap.object_shape_transitions[self.get_index()];
        let hash = key.heap_hash(agent);
        transitions
            .table
            .find(hash, |(k, _)| *k == key)
            .map(|(_, shape)| *shape)
    }

    /// Get an Object Shape pointing to the last Object Shape Record.
    pub(crate) fn last(shapes: &[ObjectShapeRecord<'static>]) -> Self {
        debug_assert!(!shapes.is_empty());

        ObjectShape::from_non_zero(
            // SAFETY: The shapes list is not empty.
            unsafe { NonZeroU32::new_unchecked(shapes.len() as u32) },
        )
    }

    /// Get an Object Shape containing the given NonZeroU32.
    #[inline(always)]
    pub(crate) const fn from_non_zero(idx: NonZeroU32) -> Self {
        if (idx.get() & 0x8000_0000) > 0 {
            handle_object_shape_count_overflow();
        }
        ObjectShape(idx, PhantomData)
    }

    /// Perform a cached lookup in the given cache for this Object Shape.
    ///
    /// If a match is found, it is returned. Otherwise, a request to fill this
    /// cache is prepared.
    pub(crate) fn get_cached<'gc>(
        self,
        agent: &mut Agent,
        p: PropertyKey,
        receiver: Value,
        cache: PropertyLookupCache,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<TryGetResult<'gc>> {
        let shape = self;
        if let Some((offset, prototype)) = cache.find_cached_property_offset(agent, shape) {
            // A cached lookup result was found.
            if offset.is_unset() {
                // The property is unset.
                TryGetResult::Unset.into()
            } else {
                let o = prototype.unwrap_or_else(|| Object::try_from(receiver).unwrap());
                Some(o.get_own_property_at_offset(agent, offset, gc))
            }
        } else {
            // No cache found.
            agent
                .heap
                .caches
                .set_current_cache(shape, p, receiver, cache);
            None
        }
    }

    pub(crate) fn get_shape_for_prototype<'gc>(
        agent: &mut Agent,
        prototype: Option<Object<'gc>>,
    ) -> ObjectShape<'gc> {
        if let Some(prototype) = prototype {
            if let Some(base_shape) = agent
                .heap
                .prototype_shapes
                .get_shape_for_prototype(prototype)
            {
                return base_shape;
            }
            agent.heap.create(ObjectShapeRecord::create_root(prototype))
        } else {
            ObjectShape::NULL
        }
    }

    /// Add a transition from self to child by key.
    fn add_transition(self, agent: &mut Agent, key: PropertyKey<'a>, child: Self) {
        let self_transitions =
            self.get_transitions_direct_mut(&mut agent.heap.object_shape_transitions);
        self_transitions.insert(
            key,
            child,
            &PropertyKeyHeap::new(&agent.heap.strings, &agent.heap.symbols),
        );
    }

    /// Mutate the Object Shape by pushing a key into the keys storage.
    ///
    /// ## Safety
    ///
    /// This is only safe to use on intrinsic Object Shapes.
    unsafe fn push_key(
        self,
        agent: &mut Agent,
        key: PropertyKey<'a>,
    ) -> Result<(), TryReserveError> {
        debug_assert_eq!(self.values_capacity(agent), self.len(agent).into());
        let ObjectShapeRecord {
            prototype: _,
            keys,
            keys_cap,
            values_cap,
            len,
        } = self.get_direct_mut(&mut agent.heap.object_shapes);
        unsafe { agent.heap.elements.push_key(keys_cap, keys, len, key) }?;
        *values_cap = (*len).into();
        Ok(())
    }

    /// Get or create an Object Shape with the given key added to this shape.
    ///
    /// This API preserves the `extensible` bit of the ObjectShape reference.
    /// It is necessary to preserve because private properties can be added
    /// onto frozen objects.
    ///
    /// > NOTE: This function will create a new Object Shape if an existing one
    /// > cannot be found.
    #[must_use]
    pub(crate) fn get_child_shape(
        self,
        agent: &mut Agent,
        key: PropertyKey<'a>,
    ) -> Result<Self, TryReserveError> {
        if self.is_intrinsic(agent) {
            // SAFETY: self is intrinsic.
            unsafe { self.push_key(agent, key)? };
            return Ok(self);
        }
        let frozen = !self.extensible();
        if let Some(mut next_shape) = self.get_transition_to(agent, key) {
            if frozen {
                next_shape.set_extensible(false);
            }
            return Ok(next_shape);
        }
        let prototype = self.get_prototype(agent);
        let len = self.len(agent) as usize;
        let cap = self.keys_capacity(agent);
        let keys_index = self.keys_index(agent);
        let keys_uninit = agent.heap.elements.get_keys_uninit_raw(cap, keys_index);
        let shape_record = if let Some(slot) = keys_uninit.get_mut(len)
            && slot.is_none()
        {
            // Our current shape keys is something like [a, b, None, None], and
            // we want to add c as the third key. In this case we can just add
            // it directly and create a new shape with the same keys.
            slot.replace(key.unbind());
            ObjectShapeRecord::create(prototype, keys_index, cap, len.checked_add(1).unwrap())
        } else {
            // Our current shape keys is something like [a, b, x] and we want
            // to add c as the third key. In this case we have to create a new
            // keys storage.
            let new_len = len.checked_add(1).unwrap();
            let (new_keys_cap, new_keys_index) = agent
                .heap
                .elements
                .copy_keys_with_addition(cap, keys_index, len as u32, key)?;
            ObjectShapeRecord::create(prototype, new_keys_index, new_keys_cap, new_len)
        };
        let mut child = agent
            .heap
            .create((shape_record, ObjectShapeTransitionMap::with_parent(self)));
        self.add_transition(agent, key, child);
        if frozen {
            child.set_extensible(false);
        }
        Ok(child)
    }

    /// Get an ancestor Object Shape with the given number of keys.
    fn get_ancestor_shape(self, agent: &mut Agent, new_len: u32) -> Option<Self> {
        let original_len = self.len(agent);
        debug_assert!(new_len < original_len);
        let prototype = self.get_prototype(agent);
        if new_len == 0 {
            // Asking for the prototype shape.
            return Some(Self::get_shape_for_prototype(agent, prototype));
        }
        let keys_cap = self.keys_capacity(agent);
        let keys_index = self.keys_index(agent);
        // Find the ancestor.
        let mut ancestor_len = original_len.wrapping_sub(1);
        let mut ancestor_shape = self.get_parent(agent);
        while let Some(parent) = ancestor_shape {
            debug_assert_eq!(parent.len(agent), ancestor_len);
            debug_assert_eq!(parent.get_prototype(agent), prototype);
            debug_assert_eq!(
                parent.keys(&agent.heap.object_shapes, &agent.heap.elements),
                agent
                    .heap
                    .elements
                    .get_keys_raw(keys_cap, keys_index, ancestor_len)
            );
            if parent.len(agent) == new_len {
                // Found the ancestor.
                return Some(parent);
            }
            ancestor_len = ancestor_len.wrapping_sub(1);
            ancestor_shape = parent.get_parent(agent);
        }
        None
    }

    /// Create a new Object Shape with the given prototype and property
    /// storage.
    ///
    /// The length of the Object Shape (number of keys) will be one more than
    /// this Object Shape has, and the added key is determined by the next key
    /// in the property storage.
    fn create_child_with_storage(
        self,
        agent: &mut Agent,
        prototype: Option<Object<'a>>,
        cap: ElementArrayKey,
        index: PropertyKeyIndex<'a>,
    ) -> Self {
        let new_len = self.len(agent).wrapping_add(1);
        let key = *agent
            .heap
            .elements
            .get_keys_raw(cap, index, new_len)
            .last()
            .unwrap();
        let shape_record = ObjectShapeRecord::create(prototype, index, cap, new_len as usize);
        let shape = agent
            .heap
            .create((shape_record, ObjectShapeTransitionMap::with_parent(self)));
        self.add_transition(agent, key, shape);
        shape
    }

    /// Create all needed Object Shapes to reach the end of the given property
    /// storage.
    ///
    /// Each created Object Shape reuses the same property storage with a
    /// different length.
    fn create_shapes_for_property_storage(
        self,
        agent: &mut Agent,
        prototype: Option<Object<'a>>,
        cap: ElementArrayKey,
        index: PropertyKeyIndex<'a>,
        len: u32,
    ) -> Self {
        let mut shape = self;
        for _ in self.len(agent)..len {
            shape = shape.create_child_with_storage(agent, prototype, cap, index);
        }
        shape
    }

    /// Get an Object Shape with the given key index removed.
    ///
    /// This method does not preserve the `extensible` bit of the ObjectShape
    /// reference, as it should be impossible to remove properties from frozen
    /// objects.
    ///
    /// > NOTE: This function will create a new Object Shape, or possibly
    /// > multiple ones, if an existing one cannot be found.
    pub(crate) fn get_shape_with_removal(
        self,
        agent: &mut Agent,
        index: u32,
    ) -> Result<Self, TryReserveError> {
        let len = self.len(agent);
        debug_assert!(index < len);
        let keys_cap = self.keys_capacity(agent);
        let keys_index = self.keys_index(agent);
        if self.is_intrinsic(agent) {
            debug_assert_eq!(self.values_capacity(agent), self.len(agent).into());
            let data = self.get_direct_mut(&mut agent.heap.object_shapes);
            // SAFETY: Mutating an intrinsic Object Shape.
            unsafe {
                agent
                    .heap
                    .elements
                    .remove_key(keys_cap, keys_index, &mut data.len, index)
            };
            // We have to update the values capacity in case we crossed a
            // border here.
            data.values_cap = data.len.into();
            debug_assert_eq!(self.values_capacity(agent), self.len(agent).into());
            return Ok(self);
        }
        let prototype = self.get_prototype(agent);
        if len == 1 {
            // Removing the last property; just get the prototype shape.
            return Ok(Self::get_shape_for_prototype(agent, prototype));
        }
        let ancestor_shape = self.get_ancestor_shape(agent, index);
        if let Some(mut parent_shape) = ancestor_shape {
            // We found an ancestor shape; now we just need to add in the
            // post-removal keys.
            for i in index.wrapping_add(1)..len {
                // Add old keys to parent shape.
                let key = agent.heap.elements.get_keys_raw(keys_cap, keys_index, len)[i as usize];
                if let Some(s) = parent_shape.get_transition_to(agent, key) {
                    parent_shape = s;
                    continue;
                }
                // Couldn't find a path to an existing Object Shape. Copy the
                // final keys into a new key storage.
                let (new_cap, new_keys_index) = agent.heap.elements.copy_keys_with_removal(
                    keys_cap,
                    keys_index,
                    len,
                    index as usize,
                )?;
                // Create remaining shapes using the final key storage.
                return Ok(parent_shape.create_shapes_for_property_storage(
                    agent,
                    prototype,
                    new_cap,
                    new_keys_index,
                    len.wrapping_sub(1),
                ));
            }
            Ok(parent_shape)
        } else {
            unreachable!()
        }
    }

    /// Insert private fields into an intrinsic Object Shape.
    ///
    /// ## Safety
    ///
    /// This Object Shape must be an intrinsic Object Shape.
    unsafe fn insert_intrinsic_private_fields(
        self,
        agent: &mut Agent,
        private_fields: &[PrivateField<'a>],
        insertion_index: usize,
    ) -> Result<(Self, usize), TryReserveError> {
        let ObjectShapeRecord {
            prototype: _,
            keys,
            keys_cap,
            len,
            values_cap,
        } = self.get_direct_mut(&mut agent.heap.object_shapes);
        let private_fields_count = u32::try_from(private_fields.len()).unwrap();
        agent
            .heap
            .elements
            .reserve_keys_raw(keys, keys_cap, *len, private_fields_count)?;
        let keys = agent.heap.elements.get_keys_uninit_raw(*keys_cap, *keys);
        keys.copy_within(
            insertion_index..*len as usize,
            insertion_index + private_fields.len(),
        );
        for (slot, key) in keys[insertion_index..]
            .iter_mut()
            .zip(private_fields.iter().map(|f| f.get_key()))
        {
            *slot = Some(key.into());
        }
        *len += private_fields_count;
        *values_cap = (*len).into();
        Ok((self, insertion_index))
    }

    /// Get an Object Shape with the given private field keys added. Returns
    /// the Object Shape and the index at which they were added in.
    ///
    /// > NOTE: This function will create a new Object Shape, or possibly
    /// > multiple ones, if an existing one cannot be found.
    ///
    /// ## Safety
    ///
    /// The private_fields parameter must not be backed by memory in the Agent
    /// heap's Elements or Object Shape related vectors.
    ///
    /// The method will read from the private_fields parameter but does not
    /// mutate them. The method also does not touch the Agent's environments at
    /// all. As a result, it is safe to pass in private fields backed by a
    /// PrivateEnvironment held in the Agent.
    pub(crate) unsafe fn add_private_fields(
        self,
        agent: &mut Agent,
        private_fields: NonNull<[PrivateField<'a>]>,
    ) -> Result<(Self, usize), TryReserveError> {
        // SAFETY: User guarantees that the fields are not backed by memory
        // that we're going to be mutating.
        let private_fields = unsafe { private_fields.as_ref() };
        let original_len = self.len(agent);
        let insertion_index = if original_len == 0 {
            // Property storage is currently empty: We don't need to do any
            // shifting of existing properties.
            0
        } else {
            let keys = self.keys(&agent.heap.object_shapes, &agent.heap.elements);
            if !keys[0].is_private_name() {
                // First key is not a PrivateName: we should insert before it.
                0
            } else if keys.last().unwrap().is_private_name() {
                // Inserting at the end.
                original_len as usize
            } else {
                // We're inserting somewhere in the middle.
                keys.binary_search_by(|k| {
                    if k.is_private_name() {
                        Ordering::Less
                    } else {
                        // Our PrivateName should be inserted before the first
                        // normal property.
                        Ordering::Greater
                    }
                })
                .unwrap_err()
            }
        };
        if self.is_intrinsic(agent) {
            if insertion_index == original_len as usize {
                // We're inserting the fields at the end.
                for field in private_fields {
                    // SAFETY: self is intrinsic.
                    unsafe { self.push_key(agent, field.get_key().into())? };
                }
                return Ok((self, insertion_index));
            }
            // SAFETY: self is intrinsic.
            return unsafe {
                self.insert_intrinsic_private_fields(agent, private_fields, insertion_index)
            };
        }
        if insertion_index == original_len as usize {
            // We're inserting the fields at the end; no need to do anything
            // fancy, just iterate through the fields and get a child shape for
            // each.
            let mut shape = self;
            for field in private_fields {
                shape = shape.get_child_shape(agent, field.get_key().into())?;
            }
            return Ok((shape, insertion_index));
        }
        // We're inserting fields into the start or middle of a shape. We need
        // to first find our common ancestor shape.
        let ancestor_shape = self.get_ancestor_shape(agent, insertion_index as u32);
        let prototype = self.get_prototype(agent);
        let cap = self.keys_capacity(agent);
        let keys_index = self.keys_index(agent);
        if let Some(mut parent_shape) = ancestor_shape {
            for field in private_fields {
                let key = field.get_key();
                parent_shape = parent_shape.get_child_shape(agent, key.into())?;
            }
            for i in insertion_index..original_len as usize {
                // Add old keys to parent shape.
                let key = agent
                    .heap
                    .elements
                    .get_keys_raw(cap, keys_index, original_len)[i];
                parent_shape = parent_shape.get_child_shape(agent, key)?;
            }
            Ok((parent_shape, insertion_index))
        } else {
            // Couldn't find a matching ancestor shape. This means that our
            // source shape comes from eg. an intrinsic which doesn't have a
            // full parent shape tree. This means we need to create the whole
            // shebang!
            debug_assert_eq!(
                insertion_index, 0,
                "Object Shape had Private Fields without having an ancestor with them"
            );
            let final_len = (original_len as usize)
                .checked_add(private_fields.len())
                .expect("Ridiculous number of fields");
            agent.heap.object_shapes.reserve(final_len);
            agent.heap.object_shape_transitions.reserve(final_len);
            let mut parent_shape = Self::get_shape_for_prototype(agent, prototype);
            for field in private_fields {
                let key = field.get_key();
                parent_shape = parent_shape.get_child_shape(agent, key.into())?;
            }
            for i in 0..original_len as usize {
                // Add old keys to parent shape.
                let key = agent
                    .heap
                    .elements
                    .get_keys_raw(cap, keys_index, original_len)[i];
                parent_shape = parent_shape.get_child_shape(agent, key)?;
            }
            Ok((parent_shape, 0))
        }
    }

    /// Get an Object Shape with the same keys as this one but with a different
    /// prototype.
    ///
    /// > NOTE: This function will create a new Object Shape, or possibly
    /// > multiple ones, if an existing one cannot be found.
    ///
    /// ## Cache invalidation
    ///
    /// Prototype lookup caches rely on the prototype chain being unchanged.
    /// When calling this method, the caller should check that they receive a
    /// new shape and if not (ie. if this shape is intrinsic), they should
    /// invalidate all property lookup caches related to this shape.
    pub(crate) fn get_shape_with_prototype(
        self,
        agent: &mut Agent,
        prototype: Option<Object<'a>>,
    ) -> Self {
        if self.is_intrinsic(agent) {
            // Intrinsic shape; set the prototype field directly.
            self.get_mut(agent).prototype = prototype.unbind();
            return self;
        }
        let original_len = self.len(agent);
        let original_cap = self.keys_capacity(agent);
        let original_keys_index = self.keys_index(agent);
        let mut shape = Self::get_shape_for_prototype(agent, prototype);
        let keys = self.keys(&agent.heap.object_shapes, &agent.heap.elements);
        for i in 0..original_len as usize {
            let key = keys[i];
            if let Some(next_shape) = shape.get_transition_to(agent, key) {
                shape = next_shape;
                continue;
            };
            // Couldn't find the next shape: we need to create all the rest.
            // We can reuse the original keys storage.
            let count = (original_len as usize).wrapping_sub(i);
            agent.heap.object_shapes.reserve(count);
            agent.heap.object_shape_transitions.reserve(count);
            let keys =
                agent
                    .heap
                    .elements
                    .get_keys_raw(original_cap, original_keys_index, original_len)
                    as *const [PropertyKey<'a>];
            // SAFETY: Creating shapes below cannot invalidate the keys pointer.
            let keys = unsafe { &*keys };
            for (i, key) in keys.iter().enumerate().take(original_len as usize).skip(i) {
                let next_shape = agent.heap.create((
                    ObjectShapeRecord::create(
                        prototype,
                        original_keys_index,
                        original_cap,
                        i.wrapping_add(1),
                    ),
                    ObjectShapeTransitionMap::with_parent(shape),
                ));
                shape.add_transition(agent, *key, next_shape);
                shape = next_shape.unbind();
            }
            break;
        }
        shape
    }

    /// Create an intrinsic copy of the given Object Shape.
    pub(crate) fn make_intrinsic(self, agent: &mut Agent) -> Result<Self, TryReserveError> {
        let properties_count = self.len(agent);
        let prototype = self.get_prototype(agent);
        // Note: intrinsics must always own their keys uniquely, so a copy must
        // be made here.
        let cap = self.keys_capacity(agent);
        let keys = self.keys_index(agent);
        let (cap, index) = agent.heap.elements.copy_keys_with_capacity(
            properties_count as usize,
            cap,
            keys,
            properties_count,
        )?;
        let cap = cap.make_intrinsic();
        Ok(agent.heap.create(ObjectShapeRecord::create(
            prototype,
            index,
            cap,
            properties_count as usize,
        )))
    }

    /// Create basic shapes for a new Realm's intrinsics.
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let base_shape = intrinsics.object_shape();
        fn create_intrinsic_shape(
            agent: &mut Agent,
            realm: Realm<'static>,
            base_shape: ObjectShape,
            shape_intrinsic: IntrinsicObjectShapes,
        ) {
            let shape = shape_intrinsic.get_object_shape_index(base_shape);
            let proto = shape_intrinsic.get_proto_intrinsic();
            let prototype = agent
                .get_realm_record_by_id(realm)
                .intrinsics()
                .get_intrinsic_default_proto(proto);
            let result = agent.heap.create(ObjectShapeRecord::create_root(prototype));
            debug_assert_eq!(shape, result);
        }

        create_intrinsic_shape(agent, realm, base_shape, IntrinsicObjectShapes::Object);
        create_intrinsic_shape(agent, realm, base_shape, IntrinsicObjectShapes::Array);
        create_intrinsic_shape(agent, realm, base_shape, IntrinsicObjectShapes::Number);
        create_intrinsic_shape(agent, realm, base_shape, IntrinsicObjectShapes::String);
    }
}

bindable_handle!(ObjectShape);

#[inline(never)]
#[cold]
const fn handle_object_shape_count_overflow() -> ! {
    panic!("ObjectShape count overflowed");
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
    /// (capacity) is determined by the `keys_cap` field.
    keys: PropertyKeyIndex<'a>,
    /// Keys storage capacity of the shape.
    ///
    /// The keys storage vector (capacity) is determined by this field, while
    /// the index in that vector is determined by the `keys` field.
    keys_cap: ElementArrayKey,
    /// Length of the keys/values storage of the shape.
    ///
    /// This is the number of properties that a object with this shape has.
    len: u32,
    /// Values storage capacity of objects with this shape.
    ///
    /// An ObjectRecord contains only an ElementIndex, ie. it only defines an
    /// index in a vector of value arrays. This capacity value defines the
    /// vector that the index points into.
    values_cap: ElementArrayKey,
}

impl<'a> ObjectShapeRecord<'a> {
    /// Null Object Shape Record.
    ///
    /// This record has a `null` prototype and no keys.
    pub(crate) const NULL: Self = Self {
        prototype: None,
        keys: PropertyKeyIndex::from_index(0),
        keys_cap: ElementArrayKey::Empty,
        len: 0,
        values_cap: ElementArrayKey::Empty,
    };

    /// Create an Object Shape for the given prototype.
    #[inline]
    pub(crate) fn create_root(prototype: Object<'a>) -> Self {
        Self {
            prototype: Some(prototype),
            keys: PropertyKeyIndex::from_index(0),
            keys_cap: ElementArrayKey::Empty,
            len: 0,
            values_cap: ElementArrayKey::Empty,
        }
    }

    #[inline]
    pub(crate) fn create(
        prototype: Option<Object<'a>>,
        keys: PropertyKeyIndex<'a>,
        keys_cap: ElementArrayKey,
        len: usize,
    ) -> Self {
        let len = u32::try_from(len).expect("Unreasonable object size");
        Self {
            prototype,
            keys,
            keys_cap,
            len,
            values_cap: len.into(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }
}

bindable_handle!(ObjectShapeRecord);

/// Data structure for finding a forward transition from an Object Shape to a
/// larger one when a property key is added.
#[derive(Debug)]
pub(crate) struct ObjectShapeTransitionMap<'a> {
    /// Parent Shape back-pointer.
    ///
    /// This is garbage collection wise the main way to access other shapes.
    parent: Option<ObjectShape<'a>>,
    /// Hash table keyed by PropertyKeys, pointing to an ObjectShape that is
    /// reached by adding said property key to the current Shape.
    ///
    /// > NOTE 1: The table is unordered.
    ///
    /// > NOTE 2: The Shapes in the table are weakly held.
    // TODO: Consider using a BTreeMap here instead. We can define a total
    // order over PropertyKeys as follows:
    // 1. PrivateName keys in order of definition/creation.
    // 2. Integer keys in order of value.
    // 3. Small string keys in lexicographic order.
    // 4. Heap string keys in order of index/creation.
    // 5. Symbol keys in order of index/creation.
    //
    // With this ordering (or any other) we can use binary search for our key
    // lookup, which may often be faster than a hash lookup. Importantly, the
    // order is defined entirely by the key value and does not depend on the
    // data of the key (the heap strings, specifically). This means that the
    // lookup does not need to access any other memory than the table itself.
    // The defined ordering is also stable as long as our GC implementation
    // index/creation order of heap strings and symbols.
    table: HashTable<(PropertyKey<'a>, ObjectShape<'a>)>,
}

impl<'a> ObjectShapeTransitionMap<'a> {
    /// Root Object Shape transition map.
    ///
    /// This transition map has no parent and (initially) contains no
    /// transitions.
    pub(crate) const ROOT: Self = Self {
        parent: None,
        table: HashTable::new(),
    };

    /// Create a new Object Shape transition map with the given parent.
    pub(crate) fn with_parent(parent: ObjectShape<'a>) -> Self {
        Self {
            parent: Some(parent),
            table: HashTable::new(),
        }
    }

    /// Insert a new transition into
    pub(crate) fn insert(&mut self, key: PropertyKey, shape: ObjectShape, heap: &PropertyKeyHeap) {
        let key = key.unbind();
        let shape = shape.unbind();
        let hash = key.heap_hash(heap);
        match self
            .table
            .entry(hash, |e| e.0 == key, |e| e.0.heap_hash(heap))
        {
            Entry::Occupied(_) => {
                unreachable!("Attempted to overwrite an existing Object Shape transition")
            }
            Entry::Vacant(e) => e.insert((key, shape)),
        };
    }
}

bindable_handle!(ObjectShapeTransitionMap);

/// Lookup-table to find a root Object Shape for a given prototype.
///
/// > NOTE: The values in the map are held weakly, while keys are held
/// > strongly. We do not mind keeping the prototype object in memory a single
/// > extra GC collection cycle. Entries are removed from the table if no child
/// > shape refers to them (transitively) anymore.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct PrototypeShapeTable {
    table: AHashMap<Object<'static>, WeakReference<ObjectShape<'static>>>,
}

impl PrototypeShapeTable {
    /// Create a new PrototypeShapeTable with the given capacity.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            table: AHashMap::with_capacity(capacity),
        }
    }

    /// Get the base Object Shape for the given prototype.
    ///
    /// Returns None if the no base Object Shape exists for the given prototype.
    pub(crate) fn get_shape_for_prototype<'a>(
        &self,
        prototype: Object<'a>,
    ) -> Option<ObjectShape<'a>> {
        let shape = self.table.get(&prototype)?;
        Some(shape.0)
    }

    pub(crate) fn set_shape_for_prototype<'a>(
        &mut self,
        prototype: Object<'a>,
        shape: ObjectShape<'a>,
    ) {
        let previous = self
            .table
            .insert(prototype.unbind(), WeakReference(shape.unbind()));
        assert!(previous.is_none(), "Re-set prototype root Object Shape");
    }
}

impl Symbol<'_> {
    pub(crate) fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        let prototype = agent.current_realm_record().intrinsics().symbol_prototype();
        ObjectShape::get_shape_for_prototype(agent, Some(prototype.into_object()))
    }
}

impl Primitive<'_> {
    pub(crate) fn object_shape(self, agent: &mut Agent) -> Option<ObjectShape<'static>> {
        let intrinsics = agent.current_realm_record().intrinsics();
        match self {
            // Not object-coercible.
            Self::Undefined | Self::Null => None,
            Self::Boolean(_) => {
                let prototype = intrinsics.boolean_prototype();
                Some(ObjectShape::get_shape_for_prototype(
                    agent,
                    Some(prototype.into_object()),
                ))
            }
            Self::String(_) | Self::SmallString(_) => Some(intrinsics.string_shape()),
            Self::Symbol(s) => Some(s.object_shape(agent)),
            Self::Number(_) | Self::Integer(_) | Self::SmallF64(_) => {
                Some(intrinsics.number_shape())
            }
            Self::BigInt(_) | Self::SmallBigInt(_) => {
                let prototype = intrinsics.big_int_prototype();
                Some(ObjectShape::get_shape_for_prototype(
                    agent,
                    Some(prototype.into_object()),
                ))
            }
        }
    }
}

impl<'a> CreateHeapData<ObjectShapeRecord<'a>, ObjectShape<'a>> for Heap {
    fn create(&mut self, data: ObjectShapeRecord<'a>) -> ObjectShape<'a> {
        self.create((data, ObjectShapeTransitionMap::ROOT))
    }
}

impl<'a> CreateHeapData<(ObjectShapeRecord<'a>, ObjectShapeTransitionMap<'a>), ObjectShape<'a>>
    for Heap
{
    fn create(
        &mut self,
        data: (ObjectShapeRecord<'a>, ObjectShapeTransitionMap<'a>),
    ) -> ObjectShape<'a> {
        let (record, transitions) = data;
        let is_root = record.keys_cap == ElementArrayKey::Empty;
        let prototype = record.prototype;
        if is_root {
            debug_assert_eq!(
                transitions.parent, None,
                "Object Shape has zero properties but has a parent"
            );
            debug_assert_eq!(
                record.len, 0,
                "Object Shape has zero capacity but non-zero length"
            );
            debug_assert_eq!(
                record.keys.into_index(),
                0,
                "Object Shape has zero capacity but non-zero keys index"
            );
        }
        self.object_shapes.push(record.unbind());
        self.object_shape_transitions.push(transitions.unbind());
        self.alloc_counter += core::mem::size_of::<ObjectShapeRecord>()
            + core::mem::size_of::<ObjectShapeTransitionMap>();
        let shape = ObjectShape::last(&self.object_shapes);
        if let Some(prototype) = prototype
            && is_root
        {
            self.prototype_shapes
                .set_shape_for_prototype(prototype, shape);
        }
        shape
    }
}

impl HeapMarkAndSweep for ObjectShape<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.object_shapes.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let top_bit = self.0.get() & 0x8000_0000;
        // SAFETY: non-null 31-bit number is still non-null.
        self.0 = unsafe { NonZeroU32::new_unchecked(self.0.get() & 0x7FFF_FFFF) };
        compactions
            .object_shapes
            .shift_non_zero_u32_index(&mut self.0);
        self.0 = unsafe { NonZeroU32::new_unchecked(self.0.get() | top_bit) };
    }
}

impl HeapSweepWeakReference for ObjectShape<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        // Mask the top bit into a separate u32.
        let top_bit = self.0.get() & 0x8000_0000;
        // SAFETY: Non-zero u31; masking the top bit still leaves a non-zero
        // value.
        let raw_value = unsafe { NonZeroU32::new_unchecked(self.0.get() & 0x7FFF_FFFF) };
        compactions
            .object_shapes
            .shift_weak_non_zero_u32_index(raw_value)
            .map(|i| {
                // SAFETY: Non-zero u31 OR'd with a possibly set top bit is
                // still non-zero.
                let i = unsafe { NonZeroU32::new_unchecked(i.get() | top_bit) };
                Self(i, PhantomData)
            })
    }
}

impl HeapMarkAndSweep for ObjectShapeRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            prototype,
            keys,
            keys_cap,
            len,
            // Note: values capacity is only used for marking and sweeping
            // objects, not object shapes themselves.
            values_cap: _,
        } = self;
        prototype.mark_values(queues);
        match keys_cap {
            ElementArrayKey::Empty | ElementArrayKey::EmptyIntrinsic => {}
            ElementArrayKey::E1 => queues.k_2_1.push((*keys, *len)),
            ElementArrayKey::E2 => queues.k_2_2.push((*keys, *len)),
            ElementArrayKey::E3 => queues.k_2_3.push((*keys, *len)),
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
            keys_cap,
            len: _,
            // Note: values capacity is only used for marking and sweeping
            // objects, not object shapes themselves.
            values_cap: _,
        } = self;
        prototype.sweep_values(compactions);
        match keys_cap {
            ElementArrayKey::Empty | ElementArrayKey::EmptyIntrinsic => {}
            ElementArrayKey::E1 => compactions.k_2_1.shift_index(keys),
            ElementArrayKey::E2 => compactions.k_2_2.shift_index(keys),
            ElementArrayKey::E3 => compactions.k_2_3.shift_index(keys),
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
        let Self { parent, table } = self;
        parent.mark_values(queues);
        // NOTE: values are weakly held; we do not mark them.
        for (key, _) in table {
            key.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { parent, table } = self;
        parent.sweep_values(compactions);
        table.retain(|(key, value)| {
            // Note: if our value was held strongly by someone else, then we
            // keep it in our transition table and sweep it and its key.
            let Some(new_value) = value.sweep_weak_reference(compactions) else {
                // Otherwise, we drop it off the table, key and all.
                return false;
            };
            key.sweep_values(compactions);
            *value = new_value;
            true
        });
    }
}

impl HeapMarkAndSweep for PrototypeShapeTable {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { table } = self;
        table.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { table } = self;
        table.sweep_values(compactions);
    }
}

impl AsRef<[ObjectShapeRecord<'static>]> for Agent {
    fn as_ref(&self) -> &[ObjectShapeRecord<'static>] {
        &self.heap.object_shapes
    }
}

impl AsMut<[ObjectShapeRecord<'static>]> for Agent {
    fn as_mut(&mut self) -> &mut [ObjectShapeRecord<'static>] {
        &mut self.heap.object_shapes
    }
}

impl AsRef<[ObjectShapeTransitionMap<'static>]> for Agent {
    fn as_ref(&self) -> &[ObjectShapeTransitionMap<'static>] {
        &self.heap.object_shape_transitions
    }
}

impl AsMut<[ObjectShapeTransitionMap<'static>]> for Agent {
    fn as_mut(&mut self) -> &mut [ObjectShapeTransitionMap<'static>] {
        &mut self.heap.object_shape_transitions
    }
}
