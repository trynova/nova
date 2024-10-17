// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{
        bigint::HeapBigInt, HeapNumber, HeapPrimitive, HeapString, OrdinaryObject, Value,
        BIGINT_DISCRIMINANT, NUMBER_DISCRIMINANT, STRING_DISCRIMINANT,
    },
    heap::{CompactionLists, HeapMarkAndSweep, PrimitiveHeapIndexable, WorkQueues},
};
use ahash::AHasher;
use hashbrown::{hash_table::Entry, HashTable};
use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Default)]
pub struct MapHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    map_data: MapData,
    // TODO: When an non-terminal (start or end) iterator exists for the Map,
    // the items in the map cannot be compacted.
    // pub(crate) observed: bool;
}

#[derive(Debug, Default)]
pub(crate) struct MapData {
    // TODO: Use a ParallelVec to remove one unnecessary allocation.
    // pub(crate) key_values: ParallelVec<Option<Value>, Option<Value>>
    pub(crate) keys: Vec<Option<Value>>,
    pub(crate) values: Vec<Option<Value>>,
    /// Low-level hash table pointing to keys-values indexes.
    pub(crate) map_data: RefCell<HashTable<u32>>,
    /// Flag that lets the Map know if it needs to rehash its primitive keys.
    ///
    /// This happens when an object key needs to be moved in the map_data
    /// during garbage collection, and the move results in a primitive key
    /// moving as well. The primitive key's hash cannot be calculated during
    /// garbage collection due to the heap data being concurrently sweeped on
    /// another thread.
    pub(crate) needs_primitive_rehashing: AtomicBool,
}

impl MapHeapData {
    /// ### [24.2.1.5 MapDataSize ( setData )](https://tc39.es/ecma262/#sec-setdatasize)
    ///
    /// The abstract operation MapDataSize takes argument setData (a List of either
    /// ECMAScript language values or EMPTY) and returns a non-negative integer.
    #[inline(always)]
    pub fn size(&self) -> u32 {
        // 1. Let count be 0.
        // 2. For each element e of setData, do
        // a. If e is not EMPTY, set count to count + 1.
        // 3. Return count.
        self.map_data.map_data.borrow().len() as u32
    }

    pub fn keys(&self) -> &[Option<Value>] {
        &self.map_data.keys
    }

    pub fn values(&self) -> &[Option<Value>] {
        &self.map_data.values
    }

    pub fn clear(&mut self) {
        // 3. For each Record { [[Key]], [[Value]] } p of M.[[MapData]], do
        // a. Set p.[[Key]] to EMPTY.
        // b. Set p.[[Value]] to EMPTY.
        self.map_data.map_data.get_mut().clear();
        self.map_data.values.fill(None);
        self.map_data.keys.fill(None);
    }

    pub(crate) fn borrow(&self, arena: &impl PrimitiveHeapIndexable) -> &MapData {
        self.map_data.rehash_if_needed(arena);
        &self.map_data
    }

    pub(crate) fn borrow_mut(&mut self, arena: &impl PrimitiveHeapIndexable) -> &mut MapData {
        self.map_data.rehash_if_needed_mut(arena);
        &mut self.map_data
    }
}

impl MapData {
    fn rehash_if_needed_mut(&mut self, arena: &impl PrimitiveHeapIndexable) {
        if !*self.needs_primitive_rehashing.get_mut() {
            return;
        }

        rehash_map_data(&self.keys, self.map_data.get_mut(), arena);
        self.needs_primitive_rehashing
            .store(false, Ordering::Relaxed);
    }

    fn rehash_if_needed(&self, arena: &impl PrimitiveHeapIndexable) {
        if !self.needs_primitive_rehashing.load(Ordering::Relaxed) {
            return;
        }

        rehash_map_data(&self.keys, &mut self.map_data.borrow_mut(), arena);
        self.needs_primitive_rehashing
            .store(false, Ordering::Relaxed);
    }
}

fn rehash_map_data(
    keys: &[Option<Value>],
    map_data: &mut HashTable<u32>,
    arena: &impl PrimitiveHeapIndexable,
) {
    let hasher = |value: Value| {
        let mut hasher = AHasher::default();
        value.hash(arena, &mut hasher);
        hasher.finish()
    };
    let hashes = {
        let hasher = |discriminant: u8| {
            let mut hasher = AHasher::default();
            discriminant.hash(&mut hasher);
            hasher.finish()
        };
        [
            (0u8, hasher(STRING_DISCRIMINANT)),
            (1u8, hasher(NUMBER_DISCRIMINANT)),
            (2u8, hasher(BIGINT_DISCRIMINANT)),
        ]
    };
    for (id, hash) in hashes {
        let eq = |equal_hash_index: &u32| {
            let value = keys[*equal_hash_index as usize].unwrap();
            match id {
                0 => HeapString::try_from(value).is_ok(),
                1 => HeapNumber::try_from(value).is_ok(),
                2 => HeapBigInt::try_from(value).is_ok(),
                _ => unreachable!(),
            }
        };
        let mut entries = Vec::new();
        while let Ok(entry) = map_data.find_entry(hash, eq) {
            entries.push(*entry.get());
            entry.remove();
        }
        entries.iter().for_each(|entry| {
            let key = keys[*entry as usize].unwrap();
            let key_hash = hasher(key);
            let result = map_data.entry(
                key_hash,
                |equal_hash_index| {
                    // It should not be possible for there to be an equal item
                    // in the Map already.
                    debug_assert_ne!(keys[*equal_hash_index as usize].unwrap(), key);
                    false
                },
                |index_to_hash| hasher(keys[*index_to_hash as usize].unwrap()),
            );

            let Entry::Vacant(result) = result else {
                unreachable!();
            };
            result.insert(*entry);
        });
    }
}

impl HeapMarkAndSweep for MapHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            map_data,
        } = self;
        object_index.mark_values(queues);
        map_data
            .keys
            .iter()
            .for_each(|value| value.mark_values(queues));
        map_data
            .values
            .iter()
            .for_each(|value| value.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            map_data,
        } = self;
        let MapData {
            keys,
            values,
            needs_primitive_rehashing,
            map_data,
        } = map_data;
        let map_data = map_data.get_mut();
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
                // same Map, so it's not pressing right now. Still, it must be
                // handled eventually. We essentially mark the heap hash data
                // as "dirty". Any lookup shall then later check this boolean
                // and rehash primitive keys if necessary.
                needs_primitive_rehashing.store(true, Ordering::Relaxed);
                let value = HeapPrimitive::try_from(value).unwrap();
                // Return a hash based on the discriminant. This we are likely
                // to cause hash collisions but we avoid having to rehash all
                // keys; we can just rehash the primitive keys that match the
                // discriminant hash.
                core::mem::discriminant(&value).hash(&mut hasher);
            }
            hasher.finish()
        };
        assert_eq!(keys.len(), values.len());
        for index in 0..keys.len() as u32 {
            let key = &mut keys[index as usize];
            let Some(key) = key else {
                // Skip empty slots.
                continue;
            };
            // Sweep value without any concerns.
            values[index as usize].sweep_values(compactions);

            let old_key = *key;
            key.sweep_values(compactions);
            let new_key = *key;

            if old_key == new_key {
                // No identity change, no hash change.
                continue;
            }

            if new_key.is_object() {
                // Non-objects do not change their hash even if their identity
                // changes.
                continue;
            }
            // Object changed identity; it must be moved in the map_data.
            let old_hash = hasher(old_key);
            let new_hash = hasher(new_key);
            if let Ok(old_entry) =
                map_data.find_entry(old_hash, |equal_hash_index| *equal_hash_index == index)
            {
                // We do not always find an entry if a previous item has
                // shifted ontop of our old hash.
                old_entry.remove();
            }
            let new_entry = map_data.entry(
                new_hash,
                |equal_hash_index| {
                    // It's not possible for there to be another hash index that
                    // holds our item. But! It's possible that we're eg. shifting
                    // 32 to 30, and then 30 to 29. Thus 32 will happily override
                    // 30.
                    values[*equal_hash_index as usize].unwrap() == new_key
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
