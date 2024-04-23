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
use std::{
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};
use std::{marker::PhantomData, mem::size_of, num::NonZeroU32};

use super::Heap;

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

    pub fn last(vec: &[Option<T>]) -> Self
    where
        T: Sized,
    {
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

impl Index<ArrayBufferIndex> for Agent {
    type Output = ArrayBufferHeapData;

    fn index(&self, index: ArrayBufferIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<ArrayBufferIndex> for Agent {
    fn index_mut(&mut self, index: ArrayBufferIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<ArrayBufferIndex> for Heap {
    type Output = ArrayBufferHeapData;

    fn index(&self, index: ArrayBufferIndex) -> &Self::Output {
        self.array_buffers
            .get(index.into_index())
            .expect("ArrayBufferIndex out of bounds")
            .as_ref()
            .expect("ArrayBufferIndex slot empty")
    }
}

impl IndexMut<ArrayBufferIndex> for Heap {
    fn index_mut(&mut self, index: ArrayBufferIndex) -> &mut Self::Output {
        self.array_buffers
            .get_mut(index.into_index())
            .expect("ArrayBufferIndex out of bounds")
            .as_mut()
            .expect("ArrayBufferIndex slot empty")
    }
}

impl Index<ArrayIndex> for Agent {
    type Output = ArrayHeapData;

    fn index(&self, index: ArrayIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<ArrayIndex> for Agent {
    fn index_mut(&mut self, index: ArrayIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<ArrayIndex> for Heap {
    type Output = ArrayHeapData;

    fn index(&self, index: ArrayIndex) -> &Self::Output {
        self.arrays
            .get(index.into_index())
            .expect("ArrayIndex out of bounds")
            .as_ref()
            .expect("ArrayIndex slot empty")
    }
}

impl IndexMut<ArrayIndex> for Heap {
    fn index_mut(&mut self, index: ArrayIndex) -> &mut Self::Output {
        self.arrays
            .get_mut(index.into_index())
            .expect("ArrayIndex out of bounds")
            .as_mut()
            .expect("ArrayIndex slot empty")
    }
}

impl Index<BigIntIndex> for Agent {
    type Output = BigIntHeapData;

    fn index(&self, index: BigIntIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<BigIntIndex> for Agent {
    fn index_mut(&mut self, index: BigIntIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<BigIntIndex> for Heap {
    type Output = BigIntHeapData;

    fn index(&self, index: BigIntIndex) -> &Self::Output {
        self.bigints
            .get(index.into_index())
            .expect("BigIntIndex out of bounds")
            .as_ref()
            .expect("BigIntIndex slot empty")
    }
}

impl IndexMut<BigIntIndex> for Heap {
    fn index_mut(&mut self, index: BigIntIndex) -> &mut Self::Output {
        self.bigints
            .get_mut(index.into_index())
            .expect("BigIntIndex out of bounds")
            .as_mut()
            .expect("BigIntIndex slot empty")
    }
}

impl Index<BoundFunctionIndex> for Agent {
    type Output = BoundFunctionHeapData;

    fn index(&self, index: BoundFunctionIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<BoundFunctionIndex> for Agent {
    fn index_mut(&mut self, index: BoundFunctionIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<BoundFunctionIndex> for Heap {
    type Output = BoundFunctionHeapData;

    fn index(&self, index: BoundFunctionIndex) -> &Self::Output {
        self.bound_functions
            .get(index.into_index())
            .expect("BoundFunctionIndex out of bounds")
            .as_ref()
            .expect("BoundFunctionIndex slot empty")
    }
}

impl IndexMut<BoundFunctionIndex> for Heap {
    fn index_mut(&mut self, index: BoundFunctionIndex) -> &mut Self::Output {
        self.bound_functions
            .get_mut(index.into_index())
            .expect("BoundFunctionIndex out of bounds")
            .as_mut()
            .expect("BoundFunctionIndex slot empty")
    }
}

impl Index<BuiltinFunctionIndex> for Agent {
    type Output = BuiltinFunctionHeapData;

    fn index(&self, index: BuiltinFunctionIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<BuiltinFunctionIndex> for Agent {
    fn index_mut(&mut self, index: BuiltinFunctionIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<BuiltinFunctionIndex> for Heap {
    type Output = BuiltinFunctionHeapData;

    fn index(&self, index: BuiltinFunctionIndex) -> &Self::Output {
        self.builtin_functions
            .get(index.into_index())
            .expect("BuiltinFunctionIndex out of bounds")
            .as_ref()
            .expect("BuiltinFunctionIndex slot empty")
    }
}

impl IndexMut<BuiltinFunctionIndex> for Heap {
    fn index_mut(&mut self, index: BuiltinFunctionIndex) -> &mut Self::Output {
        self.builtin_functions
            .get_mut(index.into_index())
            .expect("BuiltinFunctionIndex out of bounds")
            .as_mut()
            .expect("BuiltinFunctionIndex slot empty")
    }
}

impl Index<DataViewIndex> for Agent {
    type Output = DataViewHeapData;

    fn index(&self, index: DataViewIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<DataViewIndex> for Agent {
    fn index_mut(&mut self, index: DataViewIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<DataViewIndex> for Heap {
    type Output = DataViewHeapData;

    fn index(&self, index: DataViewIndex) -> &Self::Output {
        self.data_views
            .get(index.into_index())
            .expect("DataViewIndex out of bounds")
            .as_ref()
            .expect("DataViewIndex slot empty")
    }
}

impl IndexMut<DataViewIndex> for Heap {
    fn index_mut(&mut self, index: DataViewIndex) -> &mut Self::Output {
        self.data_views
            .get_mut(index.into_index())
            .expect("DataViewIndex out of bounds")
            .as_mut()
            .expect("DataViewIndex slot empty")
    }
}

impl Index<DateIndex> for Agent {
    type Output = DateHeapData;

    fn index(&self, index: DateIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<DateIndex> for Agent {
    fn index_mut(&mut self, index: DateIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<DateIndex> for Heap {
    type Output = DateHeapData;

    fn index(&self, index: DateIndex) -> &Self::Output {
        self.dates
            .get(index.into_index())
            .expect("DateIndex out of bounds")
            .as_ref()
            .expect("DateIndex slot empty")
    }
}

impl IndexMut<DateIndex> for Heap {
    fn index_mut(&mut self, index: DateIndex) -> &mut Self::Output {
        self.dates
            .get_mut(index.into_index())
            .expect("DateIndex out of bounds")
            .as_mut()
            .expect("DateIndex slot empty")
    }
}

impl Index<ECMAScriptFunctionIndex> for Agent {
    type Output = ECMAScriptFunctionHeapData;

    fn index(&self, index: ECMAScriptFunctionIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<ECMAScriptFunctionIndex> for Agent {
    fn index_mut(&mut self, index: ECMAScriptFunctionIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<ECMAScriptFunctionIndex> for Heap {
    type Output = ECMAScriptFunctionHeapData;

    fn index(&self, index: ECMAScriptFunctionIndex) -> &Self::Output {
        self.ecmascript_functions
            .get(index.into_index())
            .expect("ECMAScriptFunctionIndex out of bounds")
            .as_ref()
            .expect("ECMAScriptFunctionIndex slot empty")
    }
}

impl IndexMut<ECMAScriptFunctionIndex> for Heap {
    fn index_mut(&mut self, index: ECMAScriptFunctionIndex) -> &mut Self::Output {
        self.ecmascript_functions
            .get_mut(index.into_index())
            .expect("ECMAScriptFunctionIndex out of bounds")
            .as_mut()
            .expect("ECMAScriptFunctionIndex slot empty")
    }
}

impl Index<EmbedderObjectIndex> for Agent {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObjectIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<EmbedderObjectIndex> for Agent {
    fn index_mut(&mut self, index: EmbedderObjectIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<EmbedderObjectIndex> for Heap {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObjectIndex) -> &Self::Output {
        self.embedder_objects
            .get(index.into_index())
            .expect("EmbedderObjectIndex out of bounds")
            .as_ref()
            .expect("EmbedderObjectIndex slot empty")
    }
}

impl IndexMut<EmbedderObjectIndex> for Heap {
    fn index_mut(&mut self, index: EmbedderObjectIndex) -> &mut Self::Output {
        self.embedder_objects
            .get_mut(index.into_index())
            .expect("EmbedderObjectIndex out of bounds")
            .as_mut()
            .expect("EmbedderObjectIndex slot empty")
    }
}

impl Index<ErrorIndex> for Agent {
    type Output = ErrorHeapData;

    fn index(&self, index: ErrorIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<ErrorIndex> for Agent {
    fn index_mut(&mut self, index: ErrorIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<ErrorIndex> for Heap {
    type Output = ErrorHeapData;

    fn index(&self, index: ErrorIndex) -> &Self::Output {
        self.errors
            .get(index.into_index())
            .expect("ErrorIndex out of bounds")
            .as_ref()
            .expect("ErrorIndex slot empty")
    }
}

impl IndexMut<ErrorIndex> for Heap {
    fn index_mut(&mut self, index: ErrorIndex) -> &mut Self::Output {
        self.errors
            .get_mut(index.into_index())
            .expect("ErrorIndex out of bounds")
            .as_mut()
            .expect("ErrorIndex slot empty")
    }
}

impl Index<FinalizationRegistryIndex> for Agent {
    type Output = FinalizationRegistryHeapData;

    fn index(&self, index: FinalizationRegistryIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<FinalizationRegistryIndex> for Agent {
    fn index_mut(&mut self, index: FinalizationRegistryIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<FinalizationRegistryIndex> for Heap {
    type Output = FinalizationRegistryHeapData;

    fn index(&self, index: FinalizationRegistryIndex) -> &Self::Output {
        self.finalization_registrys
            .get(index.into_index())
            .expect("FinalizationRegistryIndex out of bounds")
            .as_ref()
            .expect("FinalizationRegistryIndex slot empty")
    }
}

impl IndexMut<FinalizationRegistryIndex> for Heap {
    fn index_mut(&mut self, index: FinalizationRegistryIndex) -> &mut Self::Output {
        self.finalization_registrys
            .get_mut(index.into_index())
            .expect("FinalizationRegistryIndex out of bounds")
            .as_mut()
            .expect("FinalizationRegistryIndex slot empty")
    }
}

impl Index<MapIndex> for Agent {
    type Output = MapHeapData;

    fn index(&self, index: MapIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<MapIndex> for Agent {
    fn index_mut(&mut self, index: MapIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<MapIndex> for Heap {
    type Output = MapHeapData;

    fn index(&self, index: MapIndex) -> &Self::Output {
        self.maps
            .get(index.into_index())
            .expect("MapIndex out of bounds")
            .as_ref()
            .expect("MapIndex slot empty")
    }
}

impl IndexMut<MapIndex> for Heap {
    fn index_mut(&mut self, index: MapIndex) -> &mut Self::Output {
        self.maps
            .get_mut(index.into_index())
            .expect("MapIndex out of bounds")
            .as_mut()
            .expect("MapIndex slot empty")
    }
}

impl Index<NumberIndex> for Agent {
    type Output = f64;

    fn index(&self, index: NumberIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<NumberIndex> for Agent {
    fn index_mut(&mut self, index: NumberIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<NumberIndex> for Heap {
    type Output = f64;

    fn index(&self, index: NumberIndex) -> &Self::Output {
        &self
            .numbers
            .get(index.into_index())
            .expect("NumberIndex out of bounds")
            .as_ref()
            .expect("NumberIndex slot empty")
            .data
    }
}

impl IndexMut<NumberIndex> for Heap {
    fn index_mut(&mut self, index: NumberIndex) -> &mut Self::Output {
        &mut self
            .numbers
            .get_mut(index.into_index())
            .expect("NumberIndex out of bounds")
            .as_mut()
            .expect("NumberIndex slot empty")
            .data
    }
}

impl Index<ObjectIndex> for Agent {
    type Output = ObjectHeapData;

    fn index(&self, index: ObjectIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<ObjectIndex> for Agent {
    fn index_mut(&mut self, index: ObjectIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<ObjectIndex> for Heap {
    type Output = ObjectHeapData;

    fn index(&self, index: ObjectIndex) -> &Self::Output {
        self.objects
            .get(index.into_index())
            .expect("ObjectIndex out of bounds")
            .as_ref()
            .expect("ObjectIndex slot empty")
    }
}

impl IndexMut<ObjectIndex> for Heap {
    fn index_mut(&mut self, index: ObjectIndex) -> &mut Self::Output {
        self.objects
            .get_mut(index.into_index())
            .expect("ObjectIndex out of bounds")
            .as_mut()
            .expect("ObjectIndex slot empty")
    }
}

impl Index<PrimitiveObjectIndex> for Agent {
    type Output = PrimitiveObjectHeapData;

    fn index(&self, index: PrimitiveObjectIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<PrimitiveObjectIndex> for Agent {
    fn index_mut(&mut self, index: PrimitiveObjectIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<PrimitiveObjectIndex> for Heap {
    type Output = PrimitiveObjectHeapData;

    fn index(&self, index: PrimitiveObjectIndex) -> &Self::Output {
        self.primitive_objects
            .get(index.into_index())
            .expect("PrimitiveObjectIndex out of bounds")
            .as_ref()
            .expect("PrimitiveObjectIndex slot empty")
    }
}

impl IndexMut<PrimitiveObjectIndex> for Heap {
    fn index_mut(&mut self, index: PrimitiveObjectIndex) -> &mut Self::Output {
        self.primitive_objects
            .get_mut(index.into_index())
            .expect("PrimitiveObjectIndex out of bounds")
            .as_mut()
            .expect("PrimitiveObjectIndex slot empty")
    }
}

impl Index<PromiseIndex> for Agent {
    type Output = PromiseHeapData;

    fn index(&self, index: PromiseIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<PromiseIndex> for Agent {
    fn index_mut(&mut self, index: PromiseIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<PromiseIndex> for Heap {
    type Output = PromiseHeapData;

    fn index(&self, index: PromiseIndex) -> &Self::Output {
        self.promises
            .get(index.into_index())
            .expect("PromiseIndex out of bounds")
            .as_ref()
            .expect("PromiseIndex slot empty")
    }
}

impl IndexMut<PromiseIndex> for Heap {
    fn index_mut(&mut self, index: PromiseIndex) -> &mut Self::Output {
        self.promises
            .get_mut(index.into_index())
            .expect("PromiseIndex out of bounds")
            .as_mut()
            .expect("PromiseIndex slot empty")
    }
}

impl Index<ProxyIndex> for Agent {
    type Output = ProxyHeapData;

    fn index(&self, index: ProxyIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<ProxyIndex> for Agent {
    fn index_mut(&mut self, index: ProxyIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<ProxyIndex> for Heap {
    type Output = ProxyHeapData;

    fn index(&self, index: ProxyIndex) -> &Self::Output {
        self.proxys
            .get(index.into_index())
            .expect("ProxyIndex out of bounds")
            .as_ref()
            .expect("ProxyIndex slot empty")
    }
}

impl IndexMut<ProxyIndex> for Heap {
    fn index_mut(&mut self, index: ProxyIndex) -> &mut Self::Output {
        self.proxys
            .get_mut(index.into_index())
            .expect("ProxyIndex out of bounds")
            .as_mut()
            .expect("ProxyIndex slot empty")
    }
}

impl Index<RegExpIndex> for Agent {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExpIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<RegExpIndex> for Agent {
    fn index_mut(&mut self, index: RegExpIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<RegExpIndex> for Heap {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExpIndex) -> &Self::Output {
        self.regexps
            .get(index.into_index())
            .expect("RegExpIndex out of bounds")
            .as_ref()
            .expect("RegExpIndex slot empty")
    }
}

impl IndexMut<RegExpIndex> for Heap {
    fn index_mut(&mut self, index: RegExpIndex) -> &mut Self::Output {
        self.regexps
            .get_mut(index.into_index())
            .expect("RegExpIndex out of bounds")
            .as_mut()
            .expect("RegExpIndex slot empty")
    }
}

impl Index<SetIndex> for Agent {
    type Output = SetHeapData;

    fn index(&self, index: SetIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<SetIndex> for Agent {
    fn index_mut(&mut self, index: SetIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<SetIndex> for Heap {
    type Output = SetHeapData;

    fn index(&self, index: SetIndex) -> &Self::Output {
        self.sets
            .get(index.into_index())
            .expect("SetIndex out of bounds")
            .as_ref()
            .expect("SetIndex slot empty")
    }
}

impl IndexMut<SetIndex> for Heap {
    fn index_mut(&mut self, index: SetIndex) -> &mut Self::Output {
        self.sets
            .get_mut(index.into_index())
            .expect("SetIndex out of bounds")
            .as_mut()
            .expect("SetIndex slot empty")
    }
}

impl Index<SharedArrayBufferIndex> for Agent {
    type Output = SharedArrayBufferHeapData;

    fn index(&self, index: SharedArrayBufferIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<SharedArrayBufferIndex> for Agent {
    fn index_mut(&mut self, index: SharedArrayBufferIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<SharedArrayBufferIndex> for Heap {
    type Output = SharedArrayBufferHeapData;

    fn index(&self, index: SharedArrayBufferIndex) -> &Self::Output {
        self.shared_array_buffers
            .get(index.into_index())
            .expect("SharedArrayBufferIndex out of bounds")
            .as_ref()
            .expect("SharedArrayBufferIndex slot empty")
    }
}

impl IndexMut<SharedArrayBufferIndex> for Heap {
    fn index_mut(&mut self, index: SharedArrayBufferIndex) -> &mut Self::Output {
        self.shared_array_buffers
            .get_mut(index.into_index())
            .expect("SharedArrayBufferIndex out of bounds")
            .as_mut()
            .expect("SharedArrayBufferIndex slot empty")
    }
}

impl Index<StringIndex> for Agent {
    type Output = StringHeapData;

    fn index(&self, index: StringIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<StringIndex> for Agent {
    fn index_mut(&mut self, index: StringIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<StringIndex> for Heap {
    type Output = StringHeapData;

    fn index(&self, index: StringIndex) -> &Self::Output {
        self.strings
            .get(index.into_index())
            .expect("StringIndex out of bounds")
            .as_ref()
            .expect("StringIndex slot empty")
    }
}

impl IndexMut<StringIndex> for Heap {
    fn index_mut(&mut self, index: StringIndex) -> &mut Self::Output {
        self.strings
            .get_mut(index.into_index())
            .expect("StringIndex out of bounds")
            .as_mut()
            .expect("StringIndex slot empty")
    }
}

impl Index<SymbolIndex> for Agent {
    type Output = SymbolHeapData;

    fn index(&self, index: SymbolIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<SymbolIndex> for Agent {
    fn index_mut(&mut self, index: SymbolIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<SymbolIndex> for Heap {
    type Output = SymbolHeapData;

    fn index(&self, index: SymbolIndex) -> &Self::Output {
        self.symbols
            .get(index.into_index())
            .expect("SymbolIndex out of bounds")
            .as_ref()
            .expect("SymbolIndex slot empty")
    }
}

impl IndexMut<SymbolIndex> for Heap {
    fn index_mut(&mut self, index: SymbolIndex) -> &mut Self::Output {
        self.symbols
            .get_mut(index.into_index())
            .expect("SymbolIndex out of bounds")
            .as_mut()
            .expect("SymbolIndex slot empty")
    }
}

impl Index<TypedArrayIndex> for Agent {
    type Output = TypedArrayHeapData;

    fn index(&self, index: TypedArrayIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<TypedArrayIndex> for Agent {
    fn index_mut(&mut self, index: TypedArrayIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<TypedArrayIndex> for Heap {
    type Output = TypedArrayHeapData;

    fn index(&self, index: TypedArrayIndex) -> &Self::Output {
        self.typed_arrays
            .get(index.into_index())
            .expect("TypedArrayIndex out of bounds")
            .as_ref()
            .expect("TypedArrayIndex slot empty")
    }
}

impl IndexMut<TypedArrayIndex> for Heap {
    fn index_mut(&mut self, index: TypedArrayIndex) -> &mut Self::Output {
        self.typed_arrays
            .get_mut(index.into_index())
            .expect("TypedArrayIndex out of bounds")
            .as_mut()
            .expect("TypedArrayIndex slot empty")
    }
}

impl Index<WeakMapIndex> for Agent {
    type Output = WeakMapHeapData;

    fn index(&self, index: WeakMapIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<WeakMapIndex> for Agent {
    fn index_mut(&mut self, index: WeakMapIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<WeakMapIndex> for Heap {
    type Output = WeakMapHeapData;

    fn index(&self, index: WeakMapIndex) -> &Self::Output {
        self.weak_maps
            .get(index.into_index())
            .expect("WeakMapIndex out of bounds")
            .as_ref()
            .expect("WeakMapIndex slot empty")
    }
}

impl IndexMut<WeakMapIndex> for Heap {
    fn index_mut(&mut self, index: WeakMapIndex) -> &mut Self::Output {
        self.weak_maps
            .get_mut(index.into_index())
            .expect("WeakMapIndex out of bounds")
            .as_mut()
            .expect("WeakMapIndex slot empty")
    }
}

impl Index<WeakRefIndex> for Agent {
    type Output = WeakRefHeapData;

    fn index(&self, index: WeakRefIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<WeakRefIndex> for Agent {
    fn index_mut(&mut self, index: WeakRefIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<WeakRefIndex> for Heap {
    type Output = WeakRefHeapData;

    fn index(&self, index: WeakRefIndex) -> &Self::Output {
        self.weak_refs
            .get(index.into_index())
            .expect("WeakRefIndex out of bounds")
            .as_ref()
            .expect("WeakRefIndex slot empty")
    }
}

impl IndexMut<WeakRefIndex> for Heap {
    fn index_mut(&mut self, index: WeakRefIndex) -> &mut Self::Output {
        self.weak_refs
            .get_mut(index.into_index())
            .expect("WeakRefIndex out of bounds")
            .as_mut()
            .expect("WeakRefIndex slot empty")
    }
}

impl Index<WeakSetIndex> for Agent {
    type Output = WeakSetHeapData;

    fn index(&self, index: WeakSetIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<WeakSetIndex> for Agent {
    fn index_mut(&mut self, index: WeakSetIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<WeakSetIndex> for Heap {
    type Output = WeakSetHeapData;

    fn index(&self, index: WeakSetIndex) -> &Self::Output {
        self.weak_sets
            .get(index.into_index())
            .expect("WeakSetIndex out of bounds")
            .as_ref()
            .expect("WeakSetIndex slot empty")
    }
}

impl IndexMut<WeakSetIndex> for Heap {
    fn index_mut(&mut self, index: WeakSetIndex) -> &mut Self::Output {
        self.weak_sets
            .get_mut(index.into_index())
            .expect("WeakSetIndex out of bounds")
            .as_mut()
            .expect("WeakSetIndex slot empty")
    }
}

impl ElementIndex {
    pub fn last_element_index<const N: usize>(vec: &[Option<[Option<Value>; N]>]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}
