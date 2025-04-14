// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            ArrayBuffer,
            array_buffer::{ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
        },
        types::OrdinaryObject,
    },
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TypedArrayArrayLength(pub u32);

impl TypedArrayArrayLength {
    pub const fn value(value: u32) -> Self {
        Self(value)
    }

    /// A sentinel value of `u32::MAX - 1` means that the byte length is stored in an
    /// associated map in the heap. This will most likely be a very rare case,
    /// only applicable for 4GB+ buffers.
    pub const fn heap() -> Self {
        Self(u32::MAX - 1)
    }

    /// A sentinel value of `u32::MAX` means that the byte length is the
    /// `AUTO` value used in the spec.
    pub const fn auto() -> Self {
        Self(u32::MAX)
    }
}

impl Default for TypedArrayArrayLength {
    fn default() -> Self {
        Self::auto()
    }
}

impl From<Option<usize>> for TypedArrayArrayLength {
    fn from(value: Option<usize>) -> Self {
        match value {
            Some(value) => {
                if value >= Self::heap().0 as usize {
                    Self::heap()
                } else {
                    Self::value(value as u32)
                }
            }
            None => Self::auto(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypedArrayHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) viewed_array_buffer: ArrayBuffer<'a>,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_length: ViewedArrayBufferByteLength,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_offset: ViewedArrayBufferByteOffset,
    /// ### [\[\[ArrayLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) array_length: TypedArrayArrayLength,
}

impl<'a> TypedArrayHeapData<'a> {
    pub fn new(object_index: Option<OrdinaryObject<'a>>) -> Self {
        Self {
            object_index,
            viewed_array_buffer: ArrayBuffer::_def(),
            byte_length: Default::default(),
            byte_offset: Default::default(),
            array_length: Default::default(),
        }
    }
}

impl Default for TypedArrayHeapData<'_> {
    fn default() -> Self {
        Self {
            object_index: Default::default(),
            viewed_array_buffer: ArrayBuffer::_def(),
            byte_length: Default::default(),
            byte_offset: Default::default(),
            array_length: Default::default(),
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for TypedArrayHeapData<'_> {
    type Of<'a> = TypedArrayHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for TypedArrayHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            viewed_array_buffer,
            byte_length: _,
            byte_offset: _,
            array_length: _,
        } = self;
        object_index.mark_values(queues);
        viewed_array_buffer.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            viewed_array_buffer,
            byte_length: _,
            byte_offset: _,
            array_length: _,
        } = self;
        object_index.sweep_values(compactions);
        viewed_array_buffer.sweep_values(compactions);
    }
}
