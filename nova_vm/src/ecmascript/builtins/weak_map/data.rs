use crate::{
    ecmascript::types::{Object, Symbol, Value},
    heap::{CompactionLists, HeapMarkAndSweep, PrimitiveHeapIndexable, WorkQueues},
};
use ahash::AHashMap;
use core::{
    hash::Hash,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) enum SymbolOrObject<'a> {
    Symbol(Symbol<'a>),
    Object(Object<'a>),
}

#[derive(Debug, Default)]
pub(crate) struct WeakMapData {
    pub(crate) weak_map_data: AHashMap<SymbolOrObject<'static>, Value<'static>>,
    pub(crate) needs_primitive_rehashing: AtomicBool,
}

#[derive(Debug, Default)]
pub struct WeakMapHeapData {
    pub(crate) weak_map_data: WeakMapData,
}

impl WeakMapHeapData {
    pub fn clear(&mut self) {
        self.weak_map_data.weak_map_data.clear();
    }

    pub(crate) fn borrow(&mut self, arena: &impl PrimitiveHeapIndexable) -> &WeakMapData {
        self.weak_map_data.rehash_if_needed(arena);
        &self.weak_map_data
    }

    pub(crate) fn borrow_mut(&mut self, arena: &impl PrimitiveHeapIndexable) -> &mut WeakMapData {
        self.weak_map_data.rehash_if_needed_mut(arena);
        &mut self.weak_map_data
    }
}

impl WeakMapData {
    fn rehash_if_needed_mut(&mut self, arena: &impl PrimitiveHeapIndexable) {
        if !self.needs_primitive_rehashing.load(Ordering::Relaxed) {
            return;
        }
        self.rehash_map_data();
        self.needs_primitive_rehashing
            .store(false, Ordering::Relaxed);
    }

    fn rehash_if_needed(&mut self, arena: &impl PrimitiveHeapIndexable) {
        if !self.needs_primitive_rehashing.load(Ordering::Relaxed) {
            return;
        }
        self.rehash_map_data();
        self.needs_primitive_rehashing
            .store(false, Ordering::Relaxed);
    }

    fn rehash_map_data(&mut self) {
        let mut new_map = AHashMap::new();
        for (key, value) in self.weak_map_data.drain() {
            new_map.insert(key, value);
        }
        self.weak_map_data = new_map;
    }
}

impl HeapMarkAndSweep for WeakMapHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.weak_map_data
            .weak_map_data
            .iter()
            .for_each(|(_, value)| {
                value.mark_values(queues);
            });
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let mut new_map = AHashMap::new();
        for (key, mut value) in self.weak_map_data.weak_map_data.drain() {
            value.sweep_values(compactions);
            new_map.insert(key, value);
        }
        self.weak_map_data.weak_map_data = new_map;
    }
}
