// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{OrdinaryObject, Value},
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
use ahash::AHasher;
use core::{
    cell::RefCell,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicBool, Ordering},
};
use hashbrown::HashTable;
use soavec_derive::SoAble;

#[derive(Debug, Default, SoAble)]
pub struct SetHeapData<'a> {
    pub(crate) set_data: RefCell<HashTable<u32>>,
    pub(crate) values: Vec<Option<Value<'a>>>,
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    /// Low-level hash table pointing to value indexes.
    /// Flag that lets the Set know if it needs to rehash its primitive keys.
    ///
    /// This happens when an object key needs to be moved in the set_data
    /// during garbage collection, and the move results in a primitive key
    /// moving as well. The primitive key's hash cannot be calculated during
    /// garbage collection due to the heap data being concurrently sweeped on
    /// another thread.
    pub(crate) needs_primitive_rehashing: AtomicBool,
    // TODO: When an non-terminal (start or end) iterator exists for the Set,
    // the items in the set cannot be compacted.
    // pub(crate) observed: bool;
}

bindable_handle!(SetHeapData);

impl HeapMarkAndSweep for SetHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            values,
            ..
        } = self;
        object_index.mark_values(queues);
        values.iter().for_each(|value| value.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            values,
            set_data,
            needs_primitive_rehashing,
            ..
        } = self;
        let set_data = set_data.get_mut();
        object_index.sweep_values(compactions);

        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            if value.try_hash(&mut hasher).is_err() {
                // A heap String, Number, or BigInt required rehashing as part
                // of moving an Object key inside the HashTable. The heap
                // vectors for those data points are currently being sweeped on
                // another thread, so we cannot access them right now even if
                // we wanted to. This situation should be fairly improbable as
                // it requires mixing eg. long string and object values in the
                // same Set, so it's not pressing right now. Still, it must be
                // handled eventually. We essentially mark the heap hash data
                // as "dirty". Any lookup shall then later check this boolean
                // and rehash primitive keys if necessary.
                needs_primitive_rehashing.store(true, Ordering::Relaxed);
                // Return a hash based on the discriminant. This we are likely
                // to cause hash collisions but we avoid having to rehash all
                // keys; we can just rehash the primitive keys that match the
                // discriminant hash.
                core::mem::discriminant(&value).hash(&mut hasher);
            }
            hasher.finish()
        };
        for index in 0..values.len() as u32 {
            let value = &mut values[index as usize];
            let Some(value) = value else {
                // Skip empty slots.
                continue;
            };
            let old_value = *value;
            value.sweep_values(compactions);
            let new_value = *value;
            if old_value == new_value {
                // No identity change, no hash change.
                continue;
            }

            if !new_value.is_object() {
                // Non-objects do not change their hash even if their identity
                // changes.
                continue;
            }
            // Object changed identity; it must be moved in the set_data.
            let old_hash = hasher(old_value);
            let new_hash = hasher(new_value);
            if let Ok(old_entry) =
                set_data.find_entry(old_hash, |equal_hash_index| *equal_hash_index == index)
            {
                // We do not always find an entry if a previous item has
                // shifted ontop of our old hash.
                old_entry.remove();
            }
            let new_entry = set_data.entry(
                new_hash,
                |equal_hash_index| {
                    // It's not possible for there to be another hash index that
                    // holds our item. But! It's possible that we're eg. shifting
                    // 32 to 30, and then 30 to 29. Thus 32 will happily override
                    // 30.
                    values[*equal_hash_index as usize].unwrap() == new_value
                },
                |index_to_hash| {
                    let value = values[*index_to_hash as usize].unwrap();
                    hasher(value)
                },
            );
            match new_entry {
                hashbrown::hash_table::Entry::Occupied(mut occupied) => {
                    // We found an existing entry that points to a
                    // (necessarily) different slot that contains the same
                    // value. This value will necessarily be removed later; we
                    // can just reuse this slot.
                    *occupied.get_mut() = index;
                }
                hashbrown::hash_table::Entry::Vacant(vacant) => {
                    // This is the expected case: We're not overwriting a slot.
                    vacant.insert(index);
                }
            }
        }
    }
}
