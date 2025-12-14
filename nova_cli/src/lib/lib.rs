// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Utilities for nova cli program.
//!
//! > [!IMPORTANT]
//! > This library is currently mainly aimed at internal use and might not
//! > adhere to semver versioning.

mod child_hooks;
mod fmt;
mod globals;
mod host_hooks;
mod module_map;

pub use child_hooks::CliChildHooks;
pub use fmt::{exit_with_parse_errors, print_result};
pub use host_hooks::{ChildToHostMessage, CliHostHooks, HostToChildMessage};
pub use module_map::ModuleMap;

use globals::{initialize_global_object, initialize_global_object_with_internals};
use nova_vm::{
    ecmascript::{Agent, AgentOptions, GcAgent, Job, JsResult, Object, RealmRoot, Value},
    engine::{Bindable, GcScope, NoGcScope},
};
use std::rc::Rc;

pub fn run_microtask_queue<'gc>(
    agent: &mut Agent,
    host_hooks: &CliHostHooks,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, ()> {
    while let Some(job) = host_hooks.pop_promise_job() {
        job.run(agent, gc.reborrow()).unbind()?.bind(gc.nogc());
    }
    Ok(())
}

pub struct InstanceConfig {
    /// Whether to enable garbage collection. Default `true`.
    pub enable_gc: bool,
    /// Whether to enable verbose logging. Default `false`.
    pub verbose: bool,
    /// Whether the main thread is allowed to block. Default `true`.
    pub block: bool,
    /// Whether to expose some internal functions like a function to run garbage collector. Default `false`.
    pub expose_internals: bool,
    /// Whether all scripts should be interpreted in strict mode. Default `false`.
    pub strict: bool,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            enable_gc: true,
            verbose: false,
            block: true,
            expose_internals: false,
            strict: false,
        }
    }
}

// SAFETY: Rust has a well-defined drop order; the fields drop in declaration order. `host_hooks` must be dropped after `realm`!
pub struct Instance {
    config: InstanceConfig,
    realm: InstanceRealm,
    // SAFETY: drop last
    host_hooks: Box<CliHostHooks>,
}

pub struct InstanceRealm {
    realm: RealmRoot,
    agent: GcAgent,
}

impl InstanceRealm {
    pub fn run_in<F, R>(&mut self, func: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(&'agent mut Agent, GcScope<'gc, 'scope>) -> R,
    {
        self.agent.run_in_realm(&self.realm, func)
    }

    pub fn initialize_module_map(&mut self, module_map: ModuleMap) {
        let host_defined = Rc::new(module_map);
        self.realm
            .initialize_host_defined(&mut self.agent, host_defined);
    }

    pub fn run_job<F, R>(&mut self, job: Job, then: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(
            &'agent mut Agent,
            JsResult<'_, ()>,
            GcScope<'gc, 'scope>,
        ) -> R,
    {
        self.agent.run_job(job, then)
    }

    pub fn run_gc(&mut self) {
        self.agent.gc();
    }
}

/// # Safety
/// The user is responsible to ensure that for the duration that the resulting reference exists, `reference` is valid for reads of type `T`.
#[allow(clippy::needless_lifetimes)]
unsafe fn extend_lifetime<'a, 'b, T>(reference: &'a T) -> &'b T {
    unsafe { &*std::ptr::from_ref(reference) }
}

impl Instance {
    pub fn new(config: InstanceConfig) -> Self {
        let host_hooks = Box::new(CliHostHooks::new());
        let mut agent = GcAgent::new(
            AgentOptions {
                disable_gc: !config.enable_gc,
                print_internals: config.verbose,
                no_block: !config.block,
            },
            // SAFETY: We keep the host hooks alive for at least as long as the agent
            unsafe { extend_lifetime(&*host_hooks) as &'static _ },
        );

        let create_global_object: Option<for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>> =
            None;
        let create_global_this_value: Option<
            for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        > = None;
        let initialize_global: Option<fn(&mut Agent, Object, GcScope)> = if config.expose_internals
        {
            Some(initialize_global_object_with_internals)
        } else {
            Some(initialize_global_object)
        };
        let realm = agent.create_realm(
            create_global_object,
            create_global_this_value,
            initialize_global,
        );

        Self {
            config,
            host_hooks,
            realm: InstanceRealm { realm, agent },
        }
    }

    pub fn split_mut(&mut self) -> (&InstanceConfig, &mut CliHostHooks, &mut InstanceRealm) {
        (&self.config, &mut self.host_hooks, &mut self.realm)
    }

    pub fn initialize_module_map(&mut self, module_map: ModuleMap) {
        self.realm.initialize_module_map(module_map)
    }

    pub fn run_tasks(&mut self) {
        let (_, host, realm) = self.split_mut();
        if host.has_macrotasks() {
            while let Some(job) = host.pop_macrotask() {
                realm.run_job(job, |agent, result, mut gc| {
                    let result = if result.is_ok() && { host.has_promise_jobs() } {
                        run_microtask_queue(agent, host, gc.reborrow())
                            .unbind()
                            .bind(gc.nogc())
                    } else {
                        result
                    };
                    print_result(agent, result.map(|_| Value::Undefined).unbind(), false, gc);
                });
            }
        }
    }
}

pub fn get_module_map(agent: &Agent, nogc: NoGcScope) -> Rc<ModuleMap> {
    agent
        .current_realm(nogc)
        .host_defined(agent)
        .unwrap()
        .downcast()
        .unwrap()
}
