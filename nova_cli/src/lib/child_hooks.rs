// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The [`HostHooks`] implementation for macrotasks and promise jobs, i.e.
//! everything but the main thread.

use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{atomic::AtomicBool, mpsc},
    thread,
    time::Duration,
};

use nova_vm::ecmascript::{HostHooks, Job};

use crate::{ChildToHostMessage, HostToChildMessage};

pub struct CliChildHooks {
    promise_job_queue: RefCell<VecDeque<Job>>,
    macrotask_queue: RefCell<Vec<Job>>,
    pub(crate) receiver: mpsc::Receiver<HostToChildMessage>,
    pub(crate) host_sender: mpsc::SyncSender<ChildToHostMessage>,
    ready_to_leave: AtomicBool,
}

// RefCell doesn't implement Debug
impl std::fmt::Debug for CliChildHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliHostHooks")
            //.field("promise_job_queue", &*self.promise_job_queue.borrow())
            .finish()
    }
}

impl CliChildHooks {
    pub fn new(
        host_sender: mpsc::SyncSender<ChildToHostMessage>,
    ) -> (Self, mpsc::SyncSender<HostToChildMessage>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        (
            Self {
                promise_job_queue: Default::default(),
                macrotask_queue: Default::default(),
                receiver,
                host_sender,
                ready_to_leave: Default::default(),
            },
            sender,
        )
    }

    pub fn is_ready_to_leave(&self) -> bool {
        self.ready_to_leave
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn mark_ready_to_leave(&self) {
        self.ready_to_leave
            .store(true, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn has_promise_jobs(&self) -> bool {
        !self.promise_job_queue.borrow().is_empty()
    }

    pub fn pop_promise_job(&self) -> Option<Job> {
        self.promise_job_queue.borrow_mut().pop_front()
    }

    pub fn has_macrotasks(&self) -> bool {
        !self.macrotask_queue.borrow().is_empty()
    }

    pub fn pop_macrotask(&self) -> Option<Job> {
        let mut off_thread_job_queue = self.macrotask_queue.borrow_mut();
        let mut counter = 0u8;
        while !off_thread_job_queue.is_empty() {
            counter = counter.wrapping_add(1);
            for (i, job) in off_thread_job_queue.iter().enumerate() {
                if job.is_finished() {
                    let job = off_thread_job_queue.swap_remove(i);
                    return Some(job);
                }
            }
            if counter == 0 {
                thread::sleep(Duration::from_millis(5));
            } else {
                core::hint::spin_loop();
            }
        }
        None
    }
}

impl HostHooks for CliChildHooks {
    fn enqueue_generic_job(&self, job: Job) {
        self.macrotask_queue.borrow_mut().push(job);
    }

    fn enqueue_promise_job(&self, job: Job) {
        self.promise_job_queue.borrow_mut().push_back(job);
    }

    fn enqueue_timeout_job(&self, _timeout_job: Job, _milliseconds: u64) {}

    fn get_host_data(&self) -> &dyn std::any::Any {
        self
    }
}
