#![allow(dead_code)]

pub mod ecmascript;
pub mod engine;
pub mod heap;
pub use engine::small_integer::SmallInteger;
pub use heap::Heap;
pub use small_string::SmallString;
