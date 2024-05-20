mod abstract_closure;
mod data_block;
mod property_descriptor;
mod reference;

pub(crate) use abstract_closure::AbstractClosureHeapData;
pub(crate) use data_block::DataBlock;
pub use property_descriptor::PropertyDescriptor;
pub use reference::ReferencedName;
pub(crate) use reference::*;
