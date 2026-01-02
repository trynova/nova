// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::types::SHARED_DATA_VIEW_DISCRIMINANT;
use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            DATA_VIEW_DISCRIMINANT, InternalMethods, InternalSlots, Object, OrdinaryObject, Value,
            Viewable,
        },
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

use self::data::DataViewRecord;
#[cfg(feature = "shared-array-buffer")]
use self::data::SharedDataViewRecord;

#[cfg(feature = "shared-array-buffer")]
use super::SharedArrayBuffer;
use super::{
    ArrayBuffer,
    array_buffer::{AnyArrayBuffer, ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
};

pub(crate) mod abstract_operations;
pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DataView<'a>(BaseIndex<'a, DataViewRecord<'static>>);
bindable_handle!(DataView);

impl<'gc> DataView<'gc> {
    /// \[\[ByteLength]]
    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = self.get(agent).byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(*agent.heap.data_view_byte_lengths.get(&self).unwrap())
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    /// \[\[ByteOffset]]
    #[inline]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = self.get(agent).byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent.heap.data_view_byte_offsets.get(&self).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    /// \[\[ViewedArrayBuffer]]
    #[inline]
    pub fn viewed_array_buffer(self, agent: &Agent) -> ArrayBuffer<'gc> {
        self.get(agent).viewed_array_buffer
    }

    /// ### [25.1.3.15 GetValueFromBuffer ( arrayBuffer, byteIndex, type, isTypedArray, order \[ , isLittleEndian \] )](https://tc39.es/ecma262/#sec-getvaluefrombuffer)
    ///
    /// # Safety
    ///
    /// The backing buffer must have enough room to read a T at `byte_index`
    /// and must not be detached.
    #[inline(always)]
    pub(crate) unsafe fn get_value_from_buffer<T: Viewable>(
        self,
        agent: &mut Agent,
        byte_index: usize,
    ) -> T {
        let array_buffer = self.viewed_array_buffer(agent);
        // 1. Assert: IsDetachedBuffer(arrayBuffer) is false.
        debug_assert!(!array_buffer.is_detached(agent));
        // 2. Assert: There are sufficient bytes in arrayBuffer starting at byteIndex to represent a value of type.
        // 4. Let elementSize be the Element Size value specified in Table 71 for Element Type type.
        // 3. Let block be arrayBuffer.[[ArrayBufferData]].
        let block = agent[array_buffer].get_data_block();
        // 5. If IsSharedArrayBuffer(arrayBuffer) is true, then
        // a. Assert: block is a Shared Data Block.
        // b. Let rawValue be GetRawBytesFromSharedBlock(block, byteIndex, type,
        //    isTypedArray, order).
        // 6. Else,
        // a. Let rawValue be a List whose elements are bytes from block at indices
        //    in the interval from byteIndex (inclusive) to byteIndex + elementSize
        //    (exclusive).
        // SAFETY: Caller guarantees the buffer has been checked for size.
        unsafe { block.read_unaligned::<T>(byte_index) }
    }

    /// Initialise the DataView's internal slots.
    ///
    /// # Safety
    ///
    /// The DataView must be uninitialised. Reinitialisation is not allowed.
    pub(crate) unsafe fn initialise_data(
        self,
        agent: &mut Agent,
        ab: ArrayBuffer<'gc>,
        byte_length: Option<usize>,
        byte_offset: usize,
    ) {
        let heap_byte_length = byte_length.into();
        let heap_byte_offset = byte_offset.into();

        let d = self.get_mut(agent);
        d.viewed_array_buffer = ab;
        d.byte_length = heap_byte_length;
        d.byte_offset = heap_byte_offset;

        if heap_byte_length == ViewedArrayBufferByteLength::heap() {
            agent
                .heap
                .data_view_byte_lengths
                .insert(self.unbind(), byte_length.unwrap());
        }

        if heap_byte_offset == ViewedArrayBufferByteOffset::heap() {
            agent
                .heap
                .data_view_byte_offsets
                .insert(self.unbind(), byte_offset);
        }
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> &'a DataViewRecord<'gc> {
        self.get_direct(&agent.heap.data_views)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut DataViewRecord<'gc> {
        self.get_direct_mut(&mut agent.heap.data_views)
    }

    #[inline(always)]
    fn get_direct<'a>(self, data_views: &'a [DataViewRecord<'static>]) -> &'a DataViewRecord<'gc> {
        data_views
            .get(self.get_index())
            .expect("Invalid DataView reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        data_views: &'a mut [DataViewRecord<'static>],
    ) -> &'a mut DataViewRecord<'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<&'a mut DataViewRecord<'static>, &'a mut DataViewRecord<'gc>>(
                data_views
                    .get_mut(self.get_index())
                    .expect("Invalid DataView reference"),
            )
        }
    }
}

impl HeapIndexHandle for DataView<'_> {
    fn from_index_u32(index: u32) -> Self {
        Self(BaseIndex::from_index_u32(index))
    }

    fn get_index_u32(&self) -> u32 {
        self.0.get_index_u32()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg(feature = "shared-array-buffer")]
pub struct SharedDataView<'a>(BaseIndex<'a, SharedDataViewRecord<'static>>);
#[cfg(feature = "shared-array-buffer")]
bindable_handle!(SharedDataView);

#[cfg(feature = "shared-array-buffer")]
impl<'gc> SharedDataView<'gc> {
    /// \[\[ByteLength]]
    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = self.get(agent).byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(*agent.heap.shared_data_view_byte_lengths.get(&self).unwrap())
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    /// \[\[ByteOffset]]
    #[inline]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = self.get(agent).byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent.heap.shared_data_view_byte_offsets.get(&self).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    /// \[\[ViewedArrayBuffer]]
    #[inline]
    pub fn viewed_array_buffer(self, agent: &Agent) -> SharedArrayBuffer<'gc> {
        self.get(agent).viewed_array_buffer
    }

    /// ### [25.1.3.15 GetValueFromBuffer ( arrayBuffer, byteIndex, type, isTypedArray, order \[ , isLittleEndian \] )](https://tc39.es/ecma262/#sec-getvaluefrombuffer)
    ///
    /// # Safety
    ///
    /// The backing buffer must have enough room to read a T at `byte_index`
    /// and must not be detached.
    ///
    /// # Soundness
    ///
    /// This method is unsound, as it can cause data races depending on user
    /// code actions.
    #[inline(always)]
    pub(crate) unsafe fn get_value_from_buffer<T: Viewable>(
        self,
        agent: &mut Agent,
        byte_index: usize,
    ) -> T {
        let array_buffer = self.viewed_array_buffer(agent);
        // 1. Assert: IsDetachedBuffer(arrayBuffer) is false.
        debug_assert!(!array_buffer.is_detached(agent));
        // 2. Assert: There are sufficient bytes in arrayBuffer starting at byteIndex to represent a value of type.
        // 4. Let elementSize be the Element Size value specified in Table 71 for Element Type type.
        // 3. Let block be arrayBuffer.[[ArrayBufferData]].
        let block = array_buffer.get_data_block(agent);
        // 5. If IsSharedArrayBuffer(arrayBuffer) is true, then
        // a. Assert: block is a Shared Data Block.
        // b. Let rawValue be GetRawBytesFromSharedBlock(block, byteIndex, type,
        //    isTypedArray, order).
        // 6. Else,
        // a. Let rawValue be a List whose elements are bytes from block at indices
        //    in the interval from byteIndex (inclusive) to byteIndex + elementSize
        //    (exclusive).
        // SAFETY: We're entirely on unsound ground here; there's nothing I can
        // say to make this okay.
        block.load_unaligned::<T>(byte_index).unwrap()
    }

    /// Initialise the SharedDataView's internal slots.
    ///
    /// # Safety
    ///
    /// The SharedDataView must be uninitialised. Reinitialisation is not allowed.
    pub(crate) unsafe fn initialise_data(
        self,
        agent: &mut Agent,
        sab: SharedArrayBuffer<'gc>,
        byte_length: Option<usize>,
        byte_offset: usize,
    ) {
        let heap_byte_length = byte_length.into();
        let heap_byte_offset = byte_offset.into();

        let d = self.get_mut(agent);
        d.viewed_array_buffer = sab;
        d.byte_length = heap_byte_length;
        d.byte_offset = heap_byte_offset;

        if heap_byte_length == ViewedArrayBufferByteLength::heap() {
            agent
                .heap
                .shared_data_view_byte_lengths
                .insert(self.unbind(), byte_length.unwrap());
        }

        if heap_byte_offset == ViewedArrayBufferByteOffset::heap() {
            agent
                .heap
                .shared_data_view_byte_offsets
                .insert(self.unbind(), byte_offset);
        }
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> &'a SharedDataViewRecord<'gc> {
        self.get_direct(&agent.heap.shared_data_views)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut SharedDataViewRecord<'gc> {
        self.get_direct_mut(&mut agent.heap.shared_data_views)
    }

    #[inline(always)]
    fn get_direct<'a>(
        self,
        shared_data_views: &'a [SharedDataViewRecord<'static>],
    ) -> &'a SharedDataViewRecord<'gc> {
        shared_data_views
            .get(self.get_index())
            .expect("Invalid DataView reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        shared_data_views: &'a mut [SharedDataViewRecord<'static>],
    ) -> &'a mut SharedDataViewRecord<'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<
                &'a mut SharedDataViewRecord<'static>,
                &'a mut SharedDataViewRecord<'gc>,
            >(
                shared_data_views
                    .get_mut(self.get_index())
                    .expect("Invalid DataView reference"),
            )
        }
    }
}

impl HeapIndexHandle for SharedDataView<'_> {
    fn from_index_u32(index: u32) -> Self {
        Self(BaseIndex::from_index_u32(index))
    }

    fn get_index_u32(&self) -> u32 {
        self.0.get_index_u32()
    }
}

impl<'a> From<DataView<'a>> for Object<'a> {
    fn from(value: DataView<'a>) -> Self {
        Object::DataView(value)
    }
}

impl<'a> TryFrom<Object<'a>> for DataView<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::DataView(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for DataView<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::DataView(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for DataView<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.unbind()
                .get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for DataView<'a> {}

impl Rootable for DataView<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::DataView(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::DataView(object) => Some(object),
            _ => None,
        }
    }
}

#[cfg(feature = "shared-array-buffer")]
impl<'a> From<SharedDataView<'a>> for Object<'a> {
    fn from(value: SharedDataView<'a>) -> Self {
        Object::SharedDataView(value)
    }
}

#[cfg(feature = "shared-array-buffer")]
impl<'a> TryFrom<Object<'a>> for SharedDataView<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::SharedDataView(data) => Ok(data),
            _ => Err(()),
        }
    }
}

#[cfg(feature = "shared-array-buffer")]
impl<'a> TryFrom<Value<'a>> for SharedDataView<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::SharedDataView(data) => Ok(data),
            _ => Err(()),
        }
    }
}

#[cfg(feature = "shared-array-buffer")]
impl<'a> InternalSlots<'a> for SharedDataView<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.unbind()
                .get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }
}

#[cfg(feature = "shared-array-buffer")]
impl<'a> InternalMethods<'a> for SharedDataView<'a> {}

#[cfg(feature = "shared-array-buffer")]
impl Rootable for SharedDataView<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::SharedDataView(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::SharedDataView(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<DataViewRecord<'a>, DataView<'a>> for Heap {
    fn create(&mut self, data: DataViewRecord<'a>) -> DataView<'a> {
        self.data_views.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<DataViewRecord<'static>>();
        DataView(BaseIndex::last(&self.data_views))
    }
}

impl HeapMarkAndSweep for DataView<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.data_views.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.data_views.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for DataView<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.data_views.shift_weak_index(self.0).map(Self)
    }
}

#[cfg(feature = "shared-array-buffer")]
impl<'a> CreateHeapData<SharedDataViewRecord<'a>, SharedDataView<'a>> for Heap {
    fn create(&mut self, data: SharedDataViewRecord<'a>) -> SharedDataView<'a> {
        self.shared_data_views.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<SharedDataViewRecord<'static>>();
        SharedDataView(BaseIndex::last(&self.shared_data_views))
    }
}

#[cfg(feature = "shared-array-buffer")]
impl HeapMarkAndSweep for SharedDataView<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.shared_data_views.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.shared_data_views.shift_index(&mut self.0);
    }
}

#[cfg(feature = "shared-array-buffer")]
impl HeapSweepWeakReference for SharedDataView<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .shared_data_views
            .shift_weak_index(self.0)
            .map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AnyDataView<'a> {
    DataView(DataView<'a>) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedDataView(SharedDataView<'a>) = SHARED_DATA_VIEW_DISCRIMINANT,
}
bindable_handle!(AnyDataView);

macro_rules! data_view_delegate {
    ($value: ident, $method: ident, $($arg:expr),*) => {
        match $value {
            Self::DataView(ta) => ta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sta) => sta.$method($($arg),+),
        }
    };
}

impl<'gc> AnyDataView<'gc> {
    /// \[\[ByteLength]]
    #[inline(always)]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        data_view_delegate!(self, byte_length, agent)
    }

    /// \[\[ByteOffset]]
    #[inline(always)]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        data_view_delegate!(self, byte_offset, agent)
    }

    /// \[\[ViewedArrayBuffer]]
    #[inline(always)]
    pub fn viewed_array_buffer(self, agent: &Agent) -> AnyArrayBuffer<'gc> {
        match self {
            Self::DataView(ta) => ta.viewed_array_buffer(agent).into(),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sta) => sta.viewed_array_buffer(agent).into(),
        }
    }

    /// ### [25.1.3.15 GetValueFromBuffer ( arrayBuffer, byteIndex, type, isTypedArray, order \[ , isLittleEndian \] )](https://tc39.es/ecma262/#sec-getvaluefrombuffer)
    ///
    /// # Safety
    ///
    /// The backing buffer must have enough room to read a T at `byte_index`
    /// and must not be detached.
    ///
    /// # Soundness
    ///
    /// This method is unsound, as it can cause data races depending on user
    /// code actions.
    #[inline(always)]
    pub(crate) unsafe fn get_value_from_buffer<T: Viewable>(
        self,
        agent: &mut Agent,
        byte_index: usize,
    ) -> T {
        match self {
            Self::DataView(ta) => unsafe { ta.get_value_from_buffer(agent, byte_index) },
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sta) => unsafe { sta.get_value_from_buffer(agent, byte_index) },
        }
    }

    /// Initialise the DataView's internal slots.
    ///
    /// # Safety
    ///
    /// The DataView must be uninitialised. Reinitialisation is not allowed.
    pub(crate) unsafe fn initialise_data(
        self,
        agent: &mut Agent,
        buffer: AnyArrayBuffer<'gc>,
        byte_length: Option<usize>,
        byte_offset: usize,
    ) {
        match (self, buffer) {
            (AnyDataView::DataView(dv), AnyArrayBuffer::ArrayBuffer(ab)) => {
                // SAFETY: pass-through
                unsafe { dv.initialise_data(agent, ab, byte_length, byte_offset) }
            }
            #[cfg(feature = "shared-array-buffer")]
            (AnyDataView::SharedDataView(sdv), AnyArrayBuffer::SharedArrayBuffer(sab)) => {
                // SAFETY: pass-through
                unsafe { sdv.initialise_data(agent, sab, byte_length, byte_offset) }
            }
            #[cfg(feature = "shared-array-buffer")]
            _ => unreachable!(),
        }
    }
}

impl<'a> From<DataView<'a>> for AnyDataView<'a> {
    #[inline(always)]
    fn from(value: DataView<'a>) -> Self {
        Self::DataView(value)
    }
}

#[cfg(feature = "shared-array-buffer")]
impl<'a> From<SharedDataView<'a>> for AnyDataView<'a> {
    #[inline(always)]
    fn from(value: SharedDataView<'a>) -> Self {
        Self::SharedDataView(value)
    }
}

impl<'a> From<AnyDataView<'a>> for Object<'a> {
    #[inline(always)]
    fn from(value: AnyDataView<'a>) -> Self {
        match value {
            AnyDataView::DataView(dv) => Self::DataView(dv),
            #[cfg(feature = "shared-array-buffer")]
            AnyDataView::SharedDataView(sdv) => Self::SharedDataView(sdv),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for AnyDataView<'a> {
    type Error = ();

    #[inline]
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::DataView(dv) => Ok(Self::DataView(dv)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedDataView(sdv) => Ok(Self::SharedDataView(sdv)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for AnyDataView<'a> {
    type Error = ();

    #[inline]
    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::DataView(dv) => Ok(Self::DataView(dv)),
            #[cfg(feature = "shared-array-buffer")]
            Object::SharedDataView(sdv) => Ok(Self::SharedDataView(sdv)),
            _ => Err(()),
        }
    }
}

impl TryFrom<HeapRootData> for AnyDataView<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::DataView(dv) => Ok(AnyDataView::DataView(dv)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedDataView(sdv) => Ok(AnyDataView::SharedDataView(sdv)),
            _ => Err(()),
        }
    }
}
