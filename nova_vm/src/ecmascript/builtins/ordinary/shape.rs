// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cmp::Ordering, marker::PhantomData, num::NonZeroU32, ptr::NonNull};

use ahash::AHashMap;
use hashbrown::{HashTable, hash_table::Entry};

use crate::{
    ecmascript::{
        execution::{Agent, PrivateField, Realm},
        types::{IntoObject, Object, OrdinaryObject, PropertyKey},
    },
    engine::context::{Bindable, GcToken, NoGcScope},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        IntrinsicObjectIndexes, PropertyKeyHeap, WeakReference, WorkQueues,
        element_array::{ElementArrayKey, ElementArrays},
        indexes::{ObjectIndex, PropertyKeyIndex},
    },
};

/// Data structure describing the shape of an object.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ObjectShape<'a>(NonZeroU32, PhantomData<&'a GcToken>);

impl<'a> ObjectShape<'a> {
    /// Object Shape for `{ __proto__: null }`.
    ///
    /// This is the root Object Shape for all null-prototype objects, hence why
    /// it can be accessed statically.
    // SAFETY: statically safe.
    pub(crate) const NULL: Self = Self(NonZeroU32::new(1).unwrap(), PhantomData);

    /// Get the implied usize index of the ObjectShape reference.
    #[inline(always)]
    pub(crate) fn get_index(self) -> usize {
        self.0.get().wrapping_sub(1) as usize
    }

    pub(crate) fn keys<'e>(
        self,
        object_shapes: &[ObjectShapeRecord<'static>],
        elements: &'e ElementArrays,
    ) -> &'e [PropertyKey<'a>] {
        let data = &object_shapes[self.get_index()];
        let cap = ElementArrayKey::from(data.len);
        elements.get_keys_raw(cap, data.keys, data.len)
    }

    /// Get the PropertyKeyIndex of the Object Shape.
    pub(crate) fn get_keys(
        self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> PropertyKeyIndex<'a> {
        agent.as_ref()[self.get_index()].keys
    }

    /// Get the capacity of the Object Shape.
    pub(crate) fn get_cap(
        self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> ElementArrayKey {
        agent.as_ref()[self.get_index()].cap
    }

    /// Get the length of the Object Shape keys.
    pub(crate) fn get_length(self, agent: &impl AsRef<[ObjectShapeRecord<'static>]>) -> u32 {
        agent.as_ref()[self.get_index()].len
    }

    /// Get the prototype of the Object Shape.
    pub(crate) fn get_prototype(
        self,
        agent: &impl AsRef<[ObjectShapeRecord<'static>]>,
    ) -> Option<Object<'a>> {
        agent.as_ref()[self.get_index()].prototype
    }

    /// Get the parent Object Shape of this Object Shape.
    pub(crate) fn get_parent(
        self,
        agent: &impl AsRef<[ObjectShapeTransitionMap<'static>]>,
    ) -> Option<ObjectShape<'a>> {
        agent.as_ref()[self.get_index()].parent
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

    /// Get the Object Shape transitions as mutable.
    pub(crate) fn get_transitions_mut(
        self,
        transitions: &mut impl AsMut<[ObjectShapeTransitionMap<'static>]>,
    ) -> &mut ObjectShapeTransitionMap<'static> {
        &mut transitions.as_mut()[self.get_index()]
    }

    /// Get an Object Shape pointing to the last Object Shape Record.
    pub(crate) fn last(shapes: &[ObjectShapeRecord<'static>]) -> Self {
        debug_assert!(!shapes.is_empty());
        ObjectShape(
            // SAFETY: The shapes list is not empty.
            unsafe { NonZeroU32::new_unchecked(shapes.len() as u32) },
            PhantomData,
        )
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        // Create basic shapes.
        let prototype = agent
            .get_realm_record_by_id(realm)
            .intrinsics()
            .object_prototype()
            .into_object();
        agent
            .heap
            .object_shapes
            .push(ObjectShapeRecord::create_root(prototype));
        agent
            .heap
            .object_shape_transitions
            .push(ObjectShapeTransitionMap::ROOT);
        let shape = ObjectShape::last(&agent.heap.object_shapes);
        agent
            .heap
            .prototype_shapes
            .set_shape_for_prototype(prototype, shape);
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
            agent.heap.create((
                ObjectShapeRecord::create_root(prototype),
                ObjectShapeTransitionMap::ROOT,
            ))
        } else {
            ObjectShape::NULL
        }
    }

    /// Add a transition from self to child by key.
    fn add_transition(self, agent: &mut Agent, key: PropertyKey<'a>, child: Self) {
        let self_transitions = self.get_transitions_mut(&mut agent.heap.object_shape_transitions);
        self_transitions.insert(
            key,
            child,
            &PropertyKeyHeap::new(&agent.heap.strings, &agent.heap.symbols),
        );
    }

    /// Get an Object Shape with the given key added to this shape.
    ///
    /// > NOTE: This function will create a new Object Shape if an existing one
    /// > cannot be found.
    pub(crate) fn get_child_shape(self, agent: &mut Agent, key: PropertyKey<'a>) -> Self {
        if let Some(next_shape) = self.get_transition_to(agent, key) {
            return next_shape;
        }
        let prototype = self.get_prototype(agent);
        let len = self.get_length(agent) as usize;
        let cap = self.get_cap(agent);
        let keys_index = self.get_keys(agent);
        let keys_uninit = agent.heap.elements.get_keys_uninit_raw(cap, keys_index);
        let shape_record = if let Some(slot) = keys_uninit.get_mut(len)
            && slot.is_none()
        {
            // Our current shape keys is something like [a, b, None, None], and
            // we want to add c as the third key. In this case we can just add
            // it directly and create a new shape with the same keys.
            slot.replace(key.unbind());
            ObjectShapeRecord::create(prototype, keys_index, cap, len.wrapping_add(1))
        } else {
            // Our current shape keys is something like [a, b, x] and we want
            // to add c as the third key. In this case we have to create a new
            // keys storage.
            let new_len = len.wrapping_add(1);
            let (new_keys_cap, new_keys_index) = agent
                .heap
                .elements
                .copy_keys_with_capacity(new_len, cap, keys_index, len as u32);
            let new_keys_memory = agent
                .heap
                .elements
                .get_keys_uninit_raw(new_keys_cap, new_keys_index);
            new_keys_memory[len] = Some(key.unbind());
            ObjectShapeRecord::create(prototype, new_keys_index, new_keys_cap, new_len)
        };
        let child = agent
            .heap
            .create((shape_record, ObjectShapeTransitionMap::with_parent(self)));
        self.add_transition(agent, key, child);
        child
    }

    /// Get an ancestor Object Shape with the given number of keys.
    fn get_ancestor_shape(self, agent: &mut Agent, new_len: u32) -> Option<Self> {
        let original_len = self.get_length(agent);
        debug_assert!(new_len < original_len);
        let prototype = self.get_prototype(agent);
        if new_len == 0 {
            // Asking for the prototype shape.
            return Some(Self::get_shape_for_prototype(agent, prototype));
        }
        let cap = self.get_cap(agent);
        let keys_index = self.get_keys(agent);
        // Find the ancestor.
        let mut ancestor_len = original_len.wrapping_sub(1);
        let mut ancestor_shape = self.get_parent(agent);
        while let Some(parent) = ancestor_shape {
            debug_assert_eq!(parent.get_length(agent), ancestor_len);
            debug_assert_eq!(parent.get_prototype(agent), prototype);
            debug_assert_eq!(
                parent.keys(&agent.heap.object_shapes, &agent.heap.elements),
                agent
                    .heap
                    .elements
                    .get_keys_raw(cap, keys_index, ancestor_len)
            );
            if parent.get_length(agent) == new_len {
                // Found the ancestor.
                return Some(parent);
            }
            ancestor_len = ancestor_len.wrapping_sub(1);
            ancestor_shape = parent.get_parent(agent);
        }
        None
    }

    /// Get an Object Shape with the given key index removed.
    ///
    /// > NOTE: This function will create a new Object Shape, or possibly
    /// > multiple ones, if an existing one cannot be found.
    pub(crate) fn get_shape_with_removal(self, agent: &mut Agent, index: u32) -> Self {
        let len = self.get_length(agent);
        debug_assert!(index < len);
        let prototype = self.get_prototype(agent);
        if len == 1 {
            // Removing the last property; just get the prototype shape.
            return Self::get_shape_for_prototype(agent, prototype);
        }
        let cap = self.get_cap(agent);
        let keys_index = self.get_keys(agent);
        let ancestor_shape = self.get_ancestor_shape(agent, index);
        if let Some(mut parent_shape) = ancestor_shape {
            // We found an ancestor shape; now we just need to add in the
            // post-removal keys.
            for i in index.wrapping_add(1)..len {
                let key = agent.heap.elements.get_keys_raw(cap, keys_index, len)[i as usize];
                parent_shape = parent_shape.get_child_shape(agent, key);
            }
            parent_shape
        } else {
            // Couldn't find a matching ancestor shape. This means that our
            // source shape comes from eg. an intrinsic which doesn't have a
            // full parent shape tree. This means we need to create the whole
            // shebang!
            agent.heap.object_shapes.reserve(len as usize);
            agent.heap.object_shape_transitions.reserve(len as usize);
            let mut parent_shape = Self::get_shape_for_prototype(agent, prototype);
            for i in 0..len {
                // Add old keys to parent shape.
                if i == index {
                    // Skip the removal key.
                    continue;
                }
                let key = agent.heap.elements.get_keys_raw(cap, keys_index, len)[i as usize];
                parent_shape = parent_shape.get_child_shape(agent, key);
            }
            parent_shape
        }
    }

    /// Get an Object Shape with the given private field keys added.
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
    ) -> (Self, usize) {
        // SAFETY: User guarantees that the fields are not backed by memory
        // that we're going to be mutating.
        let private_fields = unsafe { private_fields.as_ref() };
        let original_len = self.get_length(agent);
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
        if insertion_index == original_len as usize {
            // We're inserting the fields at the end; no need to do anything
            // fancy, just iterate through the fields and get a child shape for
            // each.
            let mut shape = self;
            for field in private_fields {
                shape = shape.get_child_shape(agent, field.get_key().into());
            }
            return (shape, insertion_index);
        }
        // We're inserting fields into the start or middle of a shape. We need
        // to first find our common ancestor shape.
        let ancestor_shape = self.get_ancestor_shape(agent, insertion_index as u32);
        let prototype = self.get_prototype(agent);
        let cap = self.get_cap(agent);
        let keys_index = self.get_keys(agent);
        if let Some(mut parent_shape) = ancestor_shape {
            for field in private_fields {
                let key = field.get_key();
                parent_shape = parent_shape.get_child_shape(agent, key.into());
            }
            for i in insertion_index..original_len as usize {
                // Add old keys to parent shape.
                let key = agent
                    .heap
                    .elements
                    .get_keys_raw(cap, keys_index, original_len)[i];
                parent_shape = parent_shape.get_child_shape(agent, key);
            }
            (parent_shape, insertion_index)
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
                parent_shape = parent_shape.get_child_shape(agent, key.into());
            }
            for i in 0..original_len as usize {
                // Add old keys to parent shape.
                let key = agent
                    .heap
                    .elements
                    .get_keys_raw(cap, keys_index, original_len)[i];
                parent_shape = parent_shape.get_child_shape(agent, key);
            }
            (parent_shape, 0)
        }
    }

    /// Get an Object Shape with the same keys as this one but with a different
    /// prototype.
    ///
    /// > NOTE: This function will create a new Object Shape, or possibly
    /// > multiple ones, if an existing one cannot be found.
    pub(crate) fn get_shape_with_prototype(
        self,
        agent: &mut Agent,
        prototype: Option<Object<'a>>,
    ) -> Self {
        let original_len = self.get_length(agent);
        let original_cap = self.get_cap(agent);
        let original_keys_index = self.get_keys(agent);
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
    /// (capacity) is determined by the cap field.
    keys: PropertyKeyIndex<'a>,
    cap: ElementArrayKey,
    /// Length of the keys storage of the shape.
    len: u32,
}

impl<'a> ObjectShapeRecord<'a> {
    /// Null Object Shape Record.
    ///
    /// This record has a `null` prototype and no keys.
    pub(crate) const NULL: Self = Self {
        prototype: None,
        keys: PropertyKeyIndex::from_index(0),
        cap: ElementArrayKey::Empty,
        len: 0,
    };

    /// Base Object Shape Record.
    ///
    /// This record has a `%Object.prototype%` prototype and no keys.
    ///
    /// > NOTE: The `%Object.prototype%` is created statically and does not
    /// > point to the current Realm's intrinsic but to the "0th" Realm's
    /// > intrinsic. This should only be used in static initialisation of the
    /// > heap.
    pub(crate) const BASE: Self = Self {
        prototype: Some(Object::Object(OrdinaryObject::new(
            IntrinsicObjectIndexes::ObjectPrototype.get_object_index(ObjectIndex::from_index(0)),
        ))),
        keys: PropertyKeyIndex::from_index(0),
        cap: ElementArrayKey::Empty,
        len: 0,
    };

    /// Create an Object Shape for the given prototype.
    #[inline]
    pub(crate) fn create_root(prototype: Object<'a>) -> Self {
        Self {
            prototype: Some(prototype),
            keys: PropertyKeyIndex::from_index(0),
            cap: ElementArrayKey::Empty,
            len: 0,
        }
    }

    #[inline]
    pub(crate) fn create(
        prototype: Option<Object<'a>>,
        keys: PropertyKeyIndex<'a>,
        cap: ElementArrayKey,
        len: usize,
    ) -> Self {
        Self {
            prototype,
            keys,
            cap,
            len: u32::try_from(len).expect("Unreasonable object size"),
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ObjectShapeRecord<'_> {
    type Of<'a> = ObjectShapeRecord<'static>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ObjectShapeTransitionMap<'_> {
    type Of<'a> = ObjectShapeTransitionMap<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}
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

impl<'a> CreateHeapData<(ObjectShapeRecord<'a>, ObjectShapeTransitionMap<'a>), ObjectShape<'a>>
    for Heap
{
    fn create(
        &mut self,
        data: (ObjectShapeRecord<'a>, ObjectShapeTransitionMap<'a>),
    ) -> ObjectShape<'a> {
        let (record, transitions) = data;
        let is_root = record.len == 0;
        let prototype = record.prototype;
        if is_root {
            debug_assert_eq!(
                transitions.parent, None,
                "Object Shape has zero properties but has a parent"
            );
            debug_assert_eq!(
                record.cap,
                ElementArrayKey::Empty,
                "Object Shape has zero properties but non-zero capacity"
            );
            debug_assert_eq!(
                record.keys.into_index(),
                0,
                "Object Shape has zero properties but non-zero keys index"
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
        compactions
            .object_shapes
            .shift_non_zero_u32_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for ObjectShape<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .object_shapes
            .shift_weak_non_zero_u32_index(self.0)
            .map(|i| Self(i, PhantomData))
    }
}

impl HeapMarkAndSweep for ObjectShapeRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            prototype,
            keys,
            cap,
            len,
        } = self;
        prototype.mark_values(queues);
        match cap {
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
            cap,
            len: _,
        } = self;
        prototype.sweep_values(compactions);
        match cap {
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
