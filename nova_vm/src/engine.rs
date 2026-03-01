// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # Nova JavaScript engine
//!
//! This module contains Nova JavaScript engine's implementation specific types
//! that do not directly relate to the ECMAScript specification but are required
//! for its proper working.
//!
//! ## Handles and garbage collection
//!
//! As JavaScript is a garbage collected language, all JavaScript values are
//! garbage collected. Any [`Value`], [`Object`], or other JavaScript value held
//! in Rust code is always necessarily effectively a shared reference to the
//! value's heap allocated data (if it has any). Whenever Rust code holds a
//! JavaScript value and then calls into user JavaScript code with it as a
//! parameter, it must assume that the value is captured by the user code and
//! stored somewhere. Therefore, the Rust code cannot subsequently deallocate
//! the value's heap data as that would make the user-stored value dangling and
//! usage of it "use-after-free".
//!
//! Therefore, Nova does not offer direct references (or pointers) to the heap
//! data of JavaScript values from its API. Instead of JavaScript values being
//! pointers to their heap allocated data and being identified by that address,
//! they instead are "handles" which are plain integer values that don't have
//! any direct connection with the heap data address. But using handles instead
//! of pointers, combined with Nova not using stack scanning (also known as
//! conservative garbage collection), combined with not "rooting" handles
//! automatically means that when Rust code calls into JavaScript, it cannot be
//! sure that any handles it held before the call haven't been dropped or moved
//! by the garbage collector during the call. Conceptually this means that
//! JavaScript values in Nova are by default unsafe: there is no type-defined,
//! compiler-protected guarantee that handles do not become use-after-free.
//!
//! To deal with this unsafety, Nova offers a combination of three features:
//! 1. "Binding" of handles to the garbage collector lifetime to detect
//!    use-after-free.
//! 2. "Rooting" of handles onto the heap to keep them from being garbage
//!    collected.
//! 3. Runtime checks to ensure that use-after-free handles do not read
//!    uninitialised memory.
//!
//! A handle can be "bound" using the [`Bindable::bind`] trait method, which
//! makes use of the borrow checker to guarantee that a handle does not become
//! use-after-free: it is recommended to always use this when a handle is
//! received (be it as a function parameter or through calling a method). When a
//! handle needs to be kept alive past a function that may perform garbage
//! collection, the handle should be rooted using the [`Scopable::scope`] trait
//! method. If these steps are not taken and a handle does become
//! use-after-free, in JavaScript world this means that a value changes to
//! another value of the same type without any assignment, or the engine crashes
//! from an out-of-bounds memory access.

mod bytecode;
mod context;
mod rootable;

pub(crate) use bytecode::*;
pub use context::*;
pub use rootable::*;
