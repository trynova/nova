// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::agent::{HostHooks, Job};

/// A default implementation of host hooks, meant for applications that don't
/// need an event loop or microtasks.
///
/// Most applications are expected to define a custom implementation of the
/// [`HostHooks`] trait, and customize it further according to their needs.
/// `HostHooks` already provides the default implementations for the hooks that
/// are given by the spec, but hooks related to scheduling and module loading
/// don't have default implementations, since those are for the application to
/// handle job scheduling.
///
/// For those hooks, [`DefaultHostHooks`] provides an implementation that does
/// nothing, ignoring those jobs. This means that even if a promise is resolved
/// synchronously, its `.then` reactions will not run, since those are enqueued
/// as promise jobs. This is only meant for applications expecting to run a
/// simple synchronous script and get a result from it.
///
/// Other users of Nova should use a custom implementation of [`HostHooks`] that
/// overrides the scheduling hooks.
#[derive(Debug)]
pub struct DefaultHostHooks;

impl HostHooks for DefaultHostHooks {
    fn enqueue_generic_job(&self, _job: Job) {
        // No-op
    }

    fn enqueue_promise_job(&self, _job: Job) {
        // No-op
    }

    fn enqueue_timeout_job(&self, _timeout_job: Job, _milliseconds: u64) {
        // No-op
    }
}
