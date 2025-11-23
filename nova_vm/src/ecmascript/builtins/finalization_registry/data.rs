// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;
use soavec_derive::SoAble;

use crate::{
    ecmascript::{
        execution::{Realm, WeakKey},
        types::{Function, OrdinaryObject, Value},
    },
    engine::context::{Bindable, bindable_handle},
    heap::{CompactionLists, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues},
};

/// \[\[Cells]]
///
/// This maps a _cell_.\[\[WeakRefTarget]] to a _cell_.\[\[HeldValue]].
#[derive(Debug, Default)]
pub(crate) struct Cells<'a> {
    /// This maps a _cell_.\[\[WeakRefTarget]] to a _cell_.\[\[HeldValue]].
    cells_weak_ref_target_to_held_value: AHashMap<WeakKey<'a>, Value<'a>>,
    /// This maps a _cell_.\[\[UnregisterToken]] to a _cell_.\[\[WeakRefTarget]].
    cells_unregister_token_to_weak_ref_target: AHashMap<WeakKey<'a>, WeakKey<'a>>,
}

impl Cells<'_> {
    pub(super) fn register(
        &mut self,
        weak_ref_target: WeakKey,
        held_value: Value,
        unregister_token: Option<WeakKey>,
    ) {
        self.cells_weak_ref_target_to_held_value
            .insert(weak_ref_target.unbind(), held_value.unbind());
        if let Some(unregister_token) = unregister_token {
            self.cells_unregister_token_to_weak_ref_target
                .insert(unregister_token.unbind(), weak_ref_target.unbind());
        }
    }

    pub(super) fn unregister(&mut self, unregister_token: WeakKey) -> bool {
        // 4. Let removed be false.
        // 5. For each Record { [[WeakRefTarget]], [[HeldValue]], [[UnregisterToken]] }
        //    cell of finalizationRegistry.[[Cells]], do
        // a. If cell.[[UnregisterToken]] is not empty and
        //    SameValue(cell.[[UnregisterToken]], unregisterToken) is true,
        //    then
        if let Some(weak_ref_target) = self
            .cells_unregister_token_to_weak_ref_target
            .remove(&unregister_token.unbind())
        {
            // i. Remove cell from finalizationRegistry.[[Cells]].
            self.cells_weak_ref_target_to_held_value
                .remove(&weak_ref_target)
                .unwrap();
            // ii. Set removed to true.
            true
        } else {
            // 6. Return removed.
            false
        }
    }
}

#[derive(Debug)]
pub(crate) struct CleanupRecord<'a> {
    cleanup_queue: Vec<Value<'a>>,
    /// \[\[CleanupCallback]]
    callback: Function<'a>,
    /// \[\[Realm]]
    realm: Realm<'a>,
    cleanup_requested: bool,
}
bindable_handle!(CleanupRecord);

impl Default for CleanupRecord<'_> {
    fn default() -> Self {
        Self {
            cleanup_queue: Default::default(),
            // Note: impossible value currently.
            callback: Function::BuiltinPromiseCollectorFunction,
            realm: const { Realm::from_u32(u32::MAX - 1) },
            cleanup_requested: false,
        }
    }
}

impl<'fr> CleanupRecord<'fr> {
    pub(super) fn needs_cleanup(&mut self) -> bool {
        if !self.cleanup_queue.is_empty() && !self.cleanup_requested {
            // We request cleanup by returning true from this method.
            self.cleanup_requested = true;
            true
        } else {
            false
        }
    }

    /// # Safety
    ///
    /// FinalizationRegistry must be previously uninitialised.
    pub(super) unsafe fn initialise(&mut self, realm: Realm, cleanup_callback: Function) {
        debug_assert_eq!(self.realm, const { Realm::from_u32(u32::MAX - 1) });
        debug_assert_eq!(self.callback, Function::BuiltinPromiseCollectorFunction);
        self.realm = realm.unbind();
        self.callback = cleanup_callback.unbind();
    }

    pub(super) fn get_cleanup_queue(&mut self) -> (Function<'fr>, Vec<Value<'fr>>) {
        self.cleanup_requested = false;
        (
            self.callback,
            core::mem::replace(&mut self.cleanup_queue, vec![]),
        )
    }

    pub(super) fn push_cleanup_queue(&mut self, queue: Vec<Value<'fr>>) -> bool {
        self.cleanup_queue.extend(queue);
        if !self.cleanup_requested {
            // We haven't requested cleanup yet, so we should do it now.
            self.cleanup_requested = true;
            true
        } else {
            // We're waiting for cleanup, no need to request it again.
            false
        }
    }
}

#[derive(Debug, Default, SoAble)]
pub(crate) struct FinalizationRegistryRecord<'a> {
    /// \[\[Cells]]
    pub(super) cells: Cells<'a>,
    pub(super) cleanup: CleanupRecord<'a>,
    pub(super) object_index: Option<OrdinaryObject<'a>>,
}
bindable_handle!(FinalizationRegistryRecord);

impl HeapMarkAndSweep for FinalizationRegistryRecordRef<'_, 'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            cells:
                Cells {
                    cells_weak_ref_target_to_held_value,
                    // Note: cells_unregister_token_to_weak_ref_target holds
                    // neither key nor value strongly and thus performs no
                    // marking.
                    cells_unregister_token_to_weak_ref_target: _,
                },
            cleanup,
            object_index,
        } = self;
        for value in cells_weak_ref_target_to_held_value.values().into_iter() {
            value.mark_values(queues);
        }
        cleanup.mark_values(queues);
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, _: &CompactionLists) {
        unreachable!()
    }
}

impl HeapMarkAndSweep for FinalizationRegistryRecordMut<'_, 'static> {
    fn mark_values(&self, _: &mut WorkQueues) {
        unreachable!()
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            cells:
                Cells {
                    cells_weak_ref_target_to_held_value,
                    cells_unregister_token_to_weak_ref_target,
                },
            cleanup,
            object_index,
        } = self;
        cleanup.sweep_values(compactions);
        object_index.sweep_values(compactions);
        if cells_weak_ref_target_to_held_value.is_empty() {
            cells_unregister_token_to_weak_ref_target.clear();
            return;
        }
        let old_cells = core::mem::replace(
            cells_weak_ref_target_to_held_value,
            AHashMap::with_capacity(cells_weak_ref_target_to_held_value.len()),
        );
        for (weak_ref_target, mut held_value) in old_cells {
            held_value.sweep_values(compactions);
            let new_weak_ref_target = weak_ref_target.sweep_weak_reference(compactions);
            if let Some(new_weak_ref_target) = new_weak_ref_target {
                // [[WeakRefTarget]] still lives, add it back to
                // cells_weak_ref_target_to_held_value.
                cells_weak_ref_target_to_held_value.insert(new_weak_ref_target, held_value);
            } else {
                cleanup.cleanup_queue.push(held_value);
            }
        }
        if cells_weak_ref_target_to_held_value.is_empty()
            || cells_unregister_token_to_weak_ref_target.is_empty()
        {
            cells_unregister_token_to_weak_ref_target.clear();
            return;
        }
        let old_token_map = core::mem::replace(
            cells_unregister_token_to_weak_ref_target,
            AHashMap::with_capacity(cells_unregister_token_to_weak_ref_target.len()),
        );
        for (unregister_token, weak_ref_target) in old_token_map {
            let unregister_token = unregister_token.sweep_weak_reference(compactions);
            let weak_ref_target = weak_ref_target.sweep_weak_reference(compactions);
            if let (Some(unregister_token), Some(weak_ref_target)) =
                (unregister_token, weak_ref_target)
            {
                // Both the unregister token and the weak_ref_target still
                // live, so we must continue tracking them.
                cells_unregister_token_to_weak_ref_target.insert(unregister_token, weak_ref_target);
            }
        }
    }
}

impl HeapMarkAndSweep for CleanupRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            callback,
            realm,
            cleanup_queue,
            cleanup_requested: _,
        } = self;
        callback.mark_values(queues);
        realm.mark_values(queues);
        cleanup_queue.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            callback,
            realm,
            cleanup_queue,
            cleanup_requested: _,
        } = self;
        callback.sweep_values(compactions);
        realm.sweep_values(compactions);
        cleanup_queue.sweep_values(compactions);
    }
}
