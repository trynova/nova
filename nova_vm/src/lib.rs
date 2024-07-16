// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![allow(dead_code)]
// #![feature(round_ties_even)]

pub mod ecmascript;
pub mod engine;
pub mod heap;
pub use engine::small_integer::SmallInteger;
use heap::Heap;
pub use small_string::SmallString;
