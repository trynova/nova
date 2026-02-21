// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The [`HostHooks`] implementation for the main thread.

use std::{
    cell::RefCell, collections::VecDeque, fmt::Debug, path::PathBuf, rc::Rc, sync::mpsc, thread,
    time::Duration,
};

use nova_vm::{
    ecmascript::{
        Agent, ExceptionType, GraphLoadingStateRecord, HostDefined, HostHooks, Job, ModuleRequest,
        Referrer, SharedDataBlock, String as JsString, finish_loading_imported_module,
        parse_module,
    },
    engine::{Bindable, Global, NoGcScope},
};

pub enum HostToChildMessage {
    Broadcast(SharedDataBlock),
}

pub enum ChildToHostMessage {
    Joined,
    Report(String),
}

pub struct CliHostHooks {
    promise_job_queue: RefCell<VecDeque<Job>>,
    macrotask_queue: RefCell<Vec<Job>>,
    pub(crate) receiver: mpsc::Receiver<ChildToHostMessage>,
    pub(crate) own_sender: mpsc::SyncSender<ChildToHostMessage>,
    pub(crate) child_senders: RefCell<Vec<mpsc::SyncSender<HostToChildMessage>>>,
}

// RefCell doesn't implement Debug
impl Debug for CliHostHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliHostHooks")
            //.field("promise_job_queue", &*self.promise_job_queue.borrow())
            .finish()
    }
}

impl Default for CliHostHooks {
    fn default() -> Self {
        Self::new()
    }
}

impl CliHostHooks {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(10);
        Self {
            promise_job_queue: Default::default(),
            macrotask_queue: Default::default(),
            receiver,
            own_sender: sender,
            child_senders: Default::default(),
        }
    }

    pub fn add_child(&self, child_sender: mpsc::SyncSender<HostToChildMessage>) {
        self.child_senders.borrow_mut().push(child_sender);
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

impl HostHooks for CliHostHooks {
    fn enqueue_generic_job(&self, job: Job) {
        self.macrotask_queue.borrow_mut().push(job);
    }

    fn enqueue_promise_job(&self, job: Job) {
        self.promise_job_queue.borrow_mut().push_back(job);
    }

    fn enqueue_timeout_job(&self, _timeout_job: Job, _milliseconds: u64) {}

    fn load_imported_module<'gc>(
        &self,
        agent: &mut Agent,
        referrer: Referrer<'gc>,
        module_request: ModuleRequest<'gc>,
        _host_defined: Option<HostDefined>,
        payload: &mut GraphLoadingStateRecord<'gc>,
        gc: NoGcScope<'gc, '_>,
    ) {
        let specifier = module_request.specifier(agent);
        let specifier = specifier.to_string_lossy(agent);
        let get_referrer_path = || {
            referrer
                .host_defined(agent)
                .unwrap()
                .downcast::<PathBuf>()
                .unwrap()
        };
        let specifier_target = crate::module_map::specifier_target(specifier, get_referrer_path);
        let realm = referrer.realm(agent, gc);
        let module_map = realm
            .host_defined(agent)
            .expect("No referrer realm [[HostDefined]] slot")
            .downcast::<super::ModuleMap>()
            .expect("No referrer realm ModuleMap");
        if let Some(module) = module_map.get(agent, &specifier_target, gc) {
            finish_loading_imported_module(
                agent,
                referrer,
                module_request,
                payload,
                Ok(module),
                gc,
            );
            return;
        }
        let file = match std::fs::read_to_string(&specifier_target) {
            Ok(file) => file,
            Err(err) => {
                let result = Err(agent.throw_exception(ExceptionType::Error, err.to_string(), gc));
                finish_loading_imported_module(
                    agent,
                    referrer,
                    module_request,
                    payload,
                    result,
                    gc,
                );
                return;
            }
        };
        let source_text = JsString::from_string(agent, file, gc);
        let result = parse_module(
            agent,
            source_text,
            referrer.realm(agent, gc),
            Some(Rc::new(specifier_target.clone())),
            gc,
        )
        .map(|m| {
            let global_m = Global::new(agent, m.unbind().into());
            module_map.add(specifier_target, global_m);
            m.into()
        })
        .map_err(|err| {
            agent.throw_exception(
                ExceptionType::SyntaxError,
                err.first().unwrap().to_string(),
                gc,
            )
        });
        finish_loading_imported_module(agent, referrer, module_request, payload, result, gc);
    }

    fn get_host_data(&self) -> &dyn std::any::Any {
        self
    }
}
