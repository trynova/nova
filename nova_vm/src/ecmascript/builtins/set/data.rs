// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::hash::Hasher;

use ahash::AHasher;
use hashbrown::HashTable;

use crate::{
    ecmascript::types::{OrdinaryObject, Value},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct SetHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) values: Vec<Option<Value>>,
    /// Low-level hash table pointing to value indexes.
    pub(crate) set_data: HashTable<u32>,
    // TODO: When an non-terminal (start or end) iterator exists for the Set,
    // the items in the map cannot be compacted.
    // pub(crate) observed: bool;
}

impl HeapMarkAndSweep for SetHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        self.values
            .iter()
            .for_each(|value| value.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            values,
            set_data,
        } = self;
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
                // handled eventually. Possible solutions are:
                // 1. Store hash in SetHeapData for heap primitive values. This
                //    requires eg. an Option<HashMap<u32, u64>>.
                // 2. Deduplicate heap Number and BigInts as well, and hash
                //    them based on their identity. This means they need to
                //    relocate in the HashTable as well which may be worse.
                // 2. Sweep primitives first or last and provide a
                //    reference to their data in compactions. The problem is:
                //    How do you know value has already been sweeped?
                panic!("Relocating an Object key in Set caused a rehashing of a primitive heap value; their data cannot be accessed during garbage collection. Sorry.");
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
                    // (necesssarily) different slot that contains the same
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
