#![allow(dead_code)]

pub mod ecmascript;
pub mod engine;
pub mod heap;
pub use engine::small_integer::SmallInteger;
pub use engine::small_string::SmallString;
pub use heap::Heap;
