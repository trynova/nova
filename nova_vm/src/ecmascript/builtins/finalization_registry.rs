// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use soavec::SoAVec;

use crate::{
    ecmascript::{
        builtins::finalization_registry::data::{
            FinalizationRegistryRecordMut, FinalizationRegistryRecordRef,
        },
        execution::{
            Agent, FinalizationRegistryCleanupJob, ProtoIntrinsics, Realm, WeakKey,
            agent::{InnerJob, Job},
        },
        types::{
            Function, InternalMethods, InternalSlots, OrdinaryObject, Value, object_handle,
        },
    },
    engine::{
        context::{Bindable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

use self::data::FinalizationRegistryRecord;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FinalizationRegistry<'a>(BaseIndex<'a, FinalizationRegistryRecord<'static>>);
object_handle!(FinalizationRegistry);

impl<'fr> FinalizationRegistry<'fr> {
    pub(crate) fn get_cleanup_queue(self, agent: &mut Agent) -> (Function<'fr>, Vec<Value<'fr>>) {
        self.get_mut(agent).cleanup.get_cleanup_queue()
    }

    pub(crate) fn add_cleanups(self, agent: &mut Agent, queue: Vec<Value<'fr>>) {
        if queue.is_empty() {
            return;
        }
        let do_request_cleanup = self.get_mut(agent).cleanup.push_cleanup_queue(queue);
        if do_request_cleanup {
            agent
                .host_hooks
                .enqueue_finalization_registry_cleanup_job(Job {
                    realm: None,
                    inner: InnerJob::FinalizationRegistry(FinalizationRegistryCleanupJob::new(
                        agent, self,
                    )),
                });
        }
    }

    pub(crate) fn enqueue_cleanup_jobs(agent: &mut Agent) {
        let frs_to_enqueue = agent
            .heap
            .finalization_registrys
            .as_mut_slice()
            .cleanup
            .iter_mut()
            .enumerate()
            .filter_map(|(i, record)| {
                let i = i as u32;
                if record.needs_cleanup() {
                    Some(FinalizationRegistry(BaseIndex::from_index_u32(i)))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for fr in frs_to_enqueue {
            agent
                .host_hooks
                .enqueue_finalization_registry_cleanup_job(Job {
                    realm: None,
                    inner: InnerJob::FinalizationRegistry(FinalizationRegistryCleanupJob::new(
                        agent, fr,
                    )),
                });
        }
    }

    /// # Safety
    ///
    /// FinalizationRegistry must be previously uninitialised.
    pub(crate) unsafe fn initialise(
        self,
        agent: &mut Agent,
        realm: Realm,
        cleanup_callback: Function,
    ) {
        // SAFETY: precondition.
        unsafe {
            self.get_mut(agent)
                .cleanup
                .initialise(realm, cleanup_callback)
        };
    }

    pub(crate) fn register(
        self,
        agent: &mut Agent,
        target: WeakKey<'fr>,
        held_value: Value<'fr>,
        unregister_token: Option<WeakKey<'fr>>,
    ) {
        self.get_mut(agent)
            .cells
            .register(target, held_value, unregister_token);
    }

    pub(crate) fn unregister(self, agent: &mut Agent, unregister_token: WeakKey<'fr>) -> bool {
        self.get_mut(agent).cells.unregister(unregister_token)
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> FinalizationRegistryRecordRef<'a, 'fr> {
        self.get_direct(&agent.heap.finalization_registrys)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> FinalizationRegistryRecordMut<'a, 'fr> {
        self.get_direct_mut(&mut agent.heap.finalization_registrys)
    }

    #[inline(always)]
    fn get_direct<'a>(
        self,
        finalization_registrys: &'a SoAVec<FinalizationRegistryRecord<'static>>,
    ) -> FinalizationRegistryRecordRef<'a, 'fr> {
        finalization_registrys
            .get(self.0.into_u32_index())
            .expect("Invalid FinalizationRegistry reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        finalization_registrys: &'a mut SoAVec<FinalizationRegistryRecord<'static>>,
    ) -> FinalizationRegistryRecordMut<'a, 'fr> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<
                FinalizationRegistryRecordMut<'a, 'static>,
                FinalizationRegistryRecordMut<'a, 'fr>,
            >(
                finalization_registrys
                    .get_mut(self.0.into_u32_index())
                    .expect("Invalid FinalizationRegistry reference"),
            )
        }
    }
}

impl<'fr> InternalSlots<'fr> for FinalizationRegistry<'fr> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::FinalizationRegistry;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for FinalizationRegistry<'a> {}

impl<'a> CreateHeapData<FinalizationRegistryRecord<'a>, FinalizationRegistry<'a>> for Heap {
    fn create(&mut self, data: FinalizationRegistryRecord<'a>) -> FinalizationRegistry<'a> {
        let i = self.finalization_registrys.len();
        self.finalization_registrys
            .push(data.unbind())
            .expect("Failed to allocate FinalizationRegistry");
        self.alloc_counter += core::mem::size_of::<FinalizationRegistryRecord<'static>>();
        FinalizationRegistry(BaseIndex::from_index_u32(i))
    }
}

impl HeapMarkAndSweep for FinalizationRegistry<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.finalization_registrys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.finalization_registrys.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for FinalizationRegistry<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .finalization_registrys
            .shift_weak_index(self.0)
            .map(Self)
    }
}
