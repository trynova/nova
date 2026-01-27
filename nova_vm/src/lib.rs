// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "proposal-float16array", feature(f16))]

pub mod ecmascript;
pub mod engine;
pub mod heap;

/// DTrace / SystemTap USDT probes in Nova VM.
#[usdt::provider(provider = "nova_vm")]
mod ndt {
    // Start probes in alphabetical order.
    fn builtin_call_start(name: &str, id: u64) {}
    fn builtin_constructor_start(name: &str, id: u64) {}
    fn eval_evaluation_start(id: u64) {}
    fn gc_start() {}
    fn javascript_call_start(name: &str, id: u64) {}
    fn javascript_constructor_start(name: &str, id: u64) {}
    fn job_evaluation_start(id: u64) {}
    fn module_evaluation_start(id: u64) {}
    fn script_evaluation_start(id: u64) {}

    // Done probes in alphabetical order.
    fn builtin_call_done(id: u64) {}
    fn builtin_constructor_done(id: u64) {}
    fn eval_evaluation_done(id: u64) {}
    fn gc_done() {}
    fn javascript_constructor_done(id: u64) {}
    fn javascript_call_done(id: u64) {}
    fn job_evaluation_done(id: u64) {}
    fn module_evaluation_done(id: u64) {}
    fn script_evaluation_done(id: u64) {}
}

/// Function that should be called as the very first thing in `fn main()` of any
/// application using Nova JavaScript engine. This function registers USDT
/// probes with the DTrace kernel module on OS's that have one and is required
/// for them to work. On other OS's this is a no-op.
///
/// ```rust
/// fn main() {
///   nova_vm::register_probes();
/// }
/// ```
///
/// # usdt documentation
pub use usdt::register_probes;
