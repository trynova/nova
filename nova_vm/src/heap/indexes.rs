use crate::ecmascript::{
    builtins::{
        data_view::data::DataViewHeapData, date::data::DateHeapData,
        embedder_object::data::EmbedderObjectHeapData, error::ErrorHeapData,
        finalization_registry::data::FinalizationRegistryHeapData, map::data::MapHeapData,
        primitive_objects::PrimitiveObjectHeapData, promise::data::PromiseHeapData,
        proxy::data::ProxyHeapData, regexp::RegExpHeapData, set::data::SetHeapData,
        shared_array_buffer::data::SharedArrayBufferHeapData,
        typed_array::data::TypedArrayHeapData, weak_map::data::WeakMapHeapData,
        weak_ref::data::WeakRefHeapData, weak_set::data::WeakSetHeapData, ArrayBufferHeapData,
        ArrayHeapData,
    },
    execution::Agent,
    types::{
        BigIntHeapData, BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
        NumberHeapData, ObjectHeapData, StringHeapData, SymbolHeapData, Value,
    },
};

use core::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::{marker::PhantomData, mem::size_of, num::NonZeroU32};

/// A struct containing a non-zero index into an array or
/// vector of `T`s. Due to the non-zero value, the offset
/// in the vector is offset by one.
///
/// This index implies a tracing reference count from this
/// struct to T at the given index.
pub struct BaseIndex<T: ?Sized>(NonZeroU32, PhantomData<T>);

const _INDEX_SIZE_IS_U32: () = assert!(size_of::<BaseIndex<()>>() == size_of::<u32>());
const _OPTION_INDEX_SIZE_IS_U32: () =
    assert!(size_of::<Option<BaseIndex<()>>>() == size_of::<u32>());

impl<T: ?Sized> Debug for BaseIndex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.0.get() != 0);
        (&self.0.get() - 1).fmt(f)
    }
}

impl<T: ?Sized> Clone for BaseIndex<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for BaseIndex<T> {}

impl<T: ?Sized> PartialEq for BaseIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Eq for BaseIndex<T> {}

impl<T: ?Sized> PartialOrd for BaseIndex<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for BaseIndex<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Hash for BaseIndex<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> BaseIndex<T> {
    pub const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }

    pub const fn into_u32_index(self) -> u32 {
        self.0.get() - 1
    }

    pub const fn into_usize(self) -> usize {
        self.0.get() as usize
    }

    pub const fn into_u32(self) -> u32 {
        self.0.get()
    }

    pub const fn from_index(value: usize) -> Self {
        let value = value as u32;
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub const fn from_u32_index(value: u32) -> Self {
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub const fn from_usize(value: usize) -> Self {
        let value = value as u32;
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value) }, PhantomData)
    }

    pub const fn from_u32(value: u32) -> Self {
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value) }, PhantomData)
    }

    pub fn last<U: Sized>(vec: &[Option<U>]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

impl<T> Default for BaseIndex<T> {
    fn default() -> Self {
        Self::from_u32_index(0)
    }
}

pub type ArrayBufferIndex = BaseIndex<ArrayBufferHeapData>;
pub type ArrayIndex = BaseIndex<ArrayHeapData>;
pub type BigIntIndex = BaseIndex<BigIntHeapData>;
pub type BoundFunctionIndex = BaseIndex<BoundFunctionHeapData>;
pub type BuiltinFunctionIndex = BaseIndex<BuiltinFunctionHeapData>;
pub type DataViewIndex = BaseIndex<DataViewHeapData>;
pub type DateIndex = BaseIndex<DateHeapData>;
pub type ECMAScriptFunctionIndex = BaseIndex<ECMAScriptFunctionHeapData>;
pub type ElementIndex = BaseIndex<[Option<Value>]>;
pub type EmbedderObjectIndex = BaseIndex<EmbedderObjectHeapData>;
pub type ErrorIndex = BaseIndex<ErrorHeapData>;
pub type FinalizationRegistryIndex = BaseIndex<FinalizationRegistryHeapData>;
pub type MapIndex = BaseIndex<MapHeapData>;
pub type NumberIndex = BaseIndex<NumberHeapData>;
pub type ObjectIndex = BaseIndex<ObjectHeapData>;
pub type PrimitiveObjectIndex = BaseIndex<PrimitiveObjectHeapData>;
pub type PromiseIndex = BaseIndex<PromiseHeapData>;
pub type ProxyIndex = BaseIndex<ProxyHeapData>;
pub type RegExpIndex = BaseIndex<RegExpHeapData>;
pub type SetIndex = BaseIndex<SetHeapData>;
pub type SharedArrayBufferIndex = BaseIndex<SharedArrayBufferHeapData>;
pub type StringIndex = BaseIndex<StringHeapData>;
pub type SymbolIndex = BaseIndex<SymbolHeapData>;
pub type TypedArrayIndex = BaseIndex<TypedArrayHeapData>;
pub type WeakMapIndex = BaseIndex<WeakMapHeapData>;
pub type WeakRefIndex = BaseIndex<WeakRefHeapData>;
pub type WeakSetIndex = BaseIndex<WeakSetHeapData>;

impl ObjectIndex {
    pub fn get(self, agent: &Agent) -> &ObjectHeapData {
        agent
            .heap
            .objects
            .get(self.into_index())
            .unwrap()
            .as_ref()
            .unwrap()
    }
}
