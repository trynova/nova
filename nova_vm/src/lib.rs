// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "proposal-float16array", feature(f16))]
#![warn(missing_docs)]

//! # Nova JavaScript engine
//!
//! Nova is a JavaScript engine aiming to be lightweight, easy to embed, and
//! close to the ECMAScript specification in form with the implementation
//! relying on idiomatic Rust rather than traditional JavaScript engine building
//! wisdom. Great performance is also an aspirational goal of the engine, but
//! not something that can be said to really be a reality today.
//!
//! ## API architecture
//!
//! The API of the engine relies on idiomatic Rust rather than traditional
//! JavaScript engine building wisdom. This is most apparent in the [`Value`]
//! type and its subtypes: instead of using NaN-boxing, NuN-boxing, or other
//! traditional and known efficient strategies for building a dynamically typed
//! language, Nova uses normal Rust enums carrying either on-stack data or a
//! handle to heap-allocated data. The only pointer that gets consistently
//! passed through call stacks is the [`Agent`] reference, and handles are
//! merely ways to access heap-allocated JavaScript data held inside the
//! `Agent`.
//!
//! ## Lightweight engine
//!
//! The engine's heap is set up to keep heap allocations small, trading speed
//! for a smaller memory footprint in the general case. This should make working
//! with large, regular datasets fairly low-impact on the memory usage of the
//! engine.
//!
//! ## Ease of embedding
//!
//! The engine has very little bells or whistles and is very easy to set up for
//! one-off script runs. The engine uses the [WTF-8] encoding internally for
//! [`String`] storage, making interfacing between the engine and normal Rust
//! code much nicer than one might expect.
//!
//! ## Shortcomings and unexpected edge cases
//!
//! Nova JavaScript engine has not been born perfect, and has many shortcomings.
//!
//! 1. The engine performance is acceptable, but it is not fast by any means.
//!
//! 1. The [`Array`] implementation does not support sparse storage internally.
//!    Calling `new Array(10 ** 9)` will request an allocation for 8 billion
//!    bytes.
//!
//! 1. The [`RegExp`] implementation does not support lookaheads, lookbehinds,
//!    or backreferences. It is always in UTF-8 / Unicode sets mode, does not
//!    support RegExp patterns containing unpaired surrogates, and its groups
//!    are slightly different from what the ECMAScript specification defines. In
//!    short: it is not compliant.
//!
//! 1. [`Promise`] subclassing is currently not supported.
//!
//! [`Agent`]: crate::ecmascript::Agent
//! [`Array`]: crate::ecmascript::Array
//! [`RegExp`]: crate::ecmascript::RegExp
//! [`Promise`]: crate::ecmascript::Promise
//! [`String`]: crate::ecmascript::String
//! [`Value`]: crate::ecmascript::Value
//! [WTF-8]: https://wtf-8.codeberg.page/

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
/// nova_vm::register_probes();
/// ```
///
/// # usdt documentation
pub use usdt::register_probes;
