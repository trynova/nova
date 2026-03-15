// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "proposal-float16array", feature(f16))]
#![warn(missing_docs)]

//! # Nova JavaScript engine
//!
//! Nova is a [JavaScript] engine focused on being lightweight, modular, and
//! easy to embed. The engine's architecture is built close to the ECMAScript
//! specification in structure with the implementation relying on idiomatic Rust
//! and data-oriented design over traditional JavaScript engine building
//! strategies. Interpreter performance is also a goal, but not yet a high
//! priority.
//!
//! The engine is exposed as a library with an API for implementation in Rust
//! projects which themselves must serve as a runtime for JavaScript code. The
//! execution model is greatly inspired by [Kiesel] and [LibJS].
//!
//! ## Basic usage
//!
//! The engine has very little bells or whistles and is very easy to set up for
//! one-off script runs or simple call-and-return instances. The engine uses the
//! [WTF-8] encoding internally for [`String`] storage, making interfacing with
//! JavaScript look and act similar to normal Rust code.
//!
//! ```rust
//! use nova_vm::{ecmascript::{DefaultHostHooks, GcAgent}, engine::GcScope};
//! let mut agent = GcAgent::new(Default::default(), &DefaultHostHooks);
//! let realm = agent.create_default_realm();
//! let _ = agent.run_in_realm(&realm, |_agent, _gc| {
//!   // do work here
//! });
//! ```
//!
//! ## Architecture
//!
//! The engine's public API relies on idiomatic Rust over traditional JavaScript
//! engine building wisdom. This is most apparent in the [`Value`] type and its
//! subvariants such as [`Object`]: instead of using NaN-boxing, NuN-boxing, or
//! other traditional and known efficient strategies for building a dynamically
//! typed language, Nova uses normal Rust enums carrying either on-stack data or
//! a 32-bit handle to heap-allocated data. The only pointer that gets
//! consistently passed through call stacks is the [`Agent`] reference, and
//! handles are merely ways to access heap-allocated JavaScript data held inside
//! the `Agent`.
//!
//! Internally, the architecture and structure of the engine follows the
//! ECMAScript specification but uses data-oriented design for the actual
//! implementation. Data on the heap is allocated in homogenous (containing data
//! of only one type) arenas with hot data split apart from cold data, and
//! optional data stored behind keyed indirections using the arena's associated
//! 32-bit handle as the key, thus using no memory to store the default null
//! case. The arenas are additionally compacted during garbage collection,
//! trading some extra collection time for better runtime cache locality for hot
//! data.
//!
//! ## Shortcomings and unexpected edge cases
//!
//! Nova JavaScript engine is not perfect and has many shortcomings.
//!
//! 1. The engine performance is acceptable, but it is not fast by any means.
//! 1. The [`Array`] implementation does not support sparse storage internally.
//!    Calling `new Array(10 ** 9)` will request an allocation for 1 billion
//!    JavaScript [`Value`]s.
//! 1. The [`RegExp`] implementation does not support lookaheads, lookbehinds,
//!    or backreferences. It is always in UTF-8 / Unicode sets mode, does not
//!    support RegExp patterns containing unpaired surrogates, and its groups
//!    are slightly different from what the ECMAScript specification defines. In
//!    short: it is not compliant.
//! 1. [`Promise`] subclassing is currently not supported.
//! 1. The engine does not support [WebAssembly] execution.
//!
//! [`Agent`]: crate::ecmascript::Agent
//! [`Array`]: crate::ecmascript::Array
//! [`RegExp`]: crate::ecmascript::RegExp
//! [`Promise`]: crate::ecmascript::Promise
//! [`Object`]: crate::ecmascript::Object
//! [`String`]: crate::ecmascript::String
//! [`Value`]: crate::ecmascript::Value
//! [WebAssembly]: https://webassembly.org
//! [WTF-8]: https://wtf-8.codeberg.page/
//! [JavaScript]: https://tc39.es/ecma262
//! [Kiesel]: https://codeberg.org/kiesel-js/kiesel
//! [LibJS]: https://github.com/LadybirdBrowser/ladybird/tree/master/Libraries/LibJS

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
