// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod bytecode;
pub mod context;
pub mod register_value;
pub mod rootable;
pub mod small_f64;
pub mod small_integer;

use core::ops::ControlFlow;

pub(crate) use bytecode::*;
pub use rootable::{Global, Scoped};

/// Result of methods that are not allowed to call JavaScript or perform
/// garbage collection.
pub type TryResult<T> = ControlFlow<(), T>;

#[inline]
pub fn unwrap_try<T>(try_result: TryResult<T>) -> T {
    match try_result {
        TryResult::Continue(t) => t,
        TryResult::Break(_) => unreachable!(),
    }
}
