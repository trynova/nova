// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{hash::Hash, num::NonZeroU32};
#[cfg(feature = "weak-refs")]
use std::ops::Range;
use std::{
    cell::UnsafeCell,
    hint::assert_unchecked,
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

use ahash::AHashMap;
#[cfg(feature = "weak-refs")]
use ahash::AHashSet;
use hashbrown::HashTable;
use soavec::{SoAVec, SoAble};
use soavec_derive::SoAble;

#[cfg(feature = "date")]
use crate::ecmascript::Date;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::{ArrayBuffer, DataView, VoidArray};
#[cfg(feature = "regexp")]
use crate::ecmascript::{RegExp, RegExpStringIterator};
#[cfg(feature = "set")]
use crate::ecmascript::{Set, SetIterator};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::{SharedArrayBuffer, SharedDataView, SharedVoidArray};
#[cfg(feature = "temporal")]
use crate::ecmascript::{TemporalDuration, TemporalInstant, TemporalPlainTime};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WeakMap, WeakRef, WeakSet};
use crate::{
    ecmascript::{
        Array, ArrayIterator, AsyncGenerator, AwaitReaction, BUILTIN_STRINGS_LIST, BoundFunction,
        BuiltinConstructorFunction, BuiltinFunction, BuiltinPromiseFinallyFunction,
        BuiltinPromiseResolvingFunction, DeclarativeEnvironment, ECMAScriptFunction,
        EmbedderObject, Error, FinalizationRegistry, FunctionEnvironment, Generator,
        GlobalEnvironment, HeapBigInt, HeapNumber, HeapString, Map, MapIterator, Module,
        ModuleEnvironment, ModuleRequest, ObjectEnvironment, ObjectShape, OrdinaryObject,
        PrimitiveObject, PrivateEnvironment, Promise, PromiseGroup, PromiseReaction,
        PropertyLookupCache, Proxy, Realm, Script, SourceCode, SourceTextModule, StringIterator,
        Symbol, Value, WeakKey,
    },
    engine::Executable,
    heap::{
        BaseIndex, ElementIndex, Heap, HeapIndexHandle, PropertyKeyIndex,
        element_array::ElementDescriptor,
    },
};

#[derive(Debug, Clone, Default)]
pub(crate) struct BitRange(Range<usize>);

impl BitRange {
    const fn from_bit_count_and_len(bit_count: &mut usize, len: usize) -> Self {
        if len == 0 {
            Self(Range { start: 0, end: 0 })
        } else {
            let start = *bit_count;
            *bit_count += len;
            Self(Range {
                start,
                end: *bit_count,
            })
        }
    }

    #[inline]
    pub(crate) const fn from_range(range: Range<usize>) -> Self {
        Self(range)
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Set a bit and return true if it was not already set.
    pub(crate) fn get_bit(&self, index: usize, bits: &[AtomicBits]) -> bool {
        let index = self.0.start + index;
        let byte_index = index / 8;
        let bit_index = (index % 8) as u8;
        let bits = &bits[byte_index];
        bits.get_bit(BitOffset::new(bit_index))
    }

    /// Set a bit and return true if it was not already set.
    pub(crate) fn set_bit(&self, index: usize, bits: &[AtomicBits]) -> bool {
        let index = self.0.start + index;
        let byte_index = index / 8;
        let bit_index = (index % 8) as u8;
        let bits = &bits[byte_index];
        bits.set_bit(BitOffset::new(bit_index))
    }

    pub(crate) fn mark_range(&self, range_to_mark: Range<u32>, bits: &mut [AtomicBits]) {
        let start = self.0.start + range_to_mark.start as usize;
        let end = self.0.start + range_to_mark.end as usize;

        let range = BitRange::from_range(start..end);

        range.for_each_byte_mut(bits, |mark_byte, bit_iterator| {
            if let Some(bit_iterator) = bit_iterator {
                *mark_byte =
                    bit_iterator.fold(*mark_byte, |acc, bitmask| acc | bitmask.as_bitmask());
            } else {
                *mark_byte = 0xFF;
            }
        });
    }

    pub(crate) fn iter<'a>(&self, bits: &'a [AtomicBits]) -> BitRangeIterator<'a> {
        let (byte_range, bit_range) = BitOffset::from_range(&self.0);
        let bits = &bits[byte_range.start..byte_range.end];
        BitRangeIterator { bits, bit_range }
    }

    #[inline]
    pub(crate) fn for_each_byte(
        &self,
        marks: &[AtomicBits],
        mut cb: impl FnMut(&AtomicBits, Option<BitIterator>),
    ) {
        let (byte_range, bit_range) = BitOffset::from_range(&self.0);
        let mut marks = &marks[byte_range.start..byte_range.end];
        debug_assert!(!marks.is_empty());
        if marks.len() == 1 && !bit_range.start.is_zero() && !bit_range.end.is_zero() {
            cb(&marks[0], Some(BitIterator::from_range(bit_range)));
            return;
        }
        if !bit_range.start.is_zero() {
            let (first, m) = marks.split_first().unwrap();
            marks = m;
            cb(first, Some(BitIterator::from_offset(bit_range.start)));
        }
        let mut end = None;
        if !bit_range.end.is_zero()
            && let Some((last, m)) = marks.split_last()
        {
            marks = m;
            end = Some(last);
        }
        for mark_byte in marks {
            cb(mark_byte, None);
        }
        if let Some(end) = end {
            cb(end, Some(BitIterator::until_offset(bit_range.end)));
        }
    }

    #[inline]
    pub(crate) fn for_each_byte_mut(
        &self,
        marks: &mut [AtomicBits],
        mut cb: impl FnMut(&mut u8, Option<BitIterator>),
    ) {
        let (byte_range, bit_range) = BitOffset::from_range(&self.0);
        let mut marks = &mut marks[byte_range.start..byte_range.end];
        debug_assert!(!marks.is_empty());
        if marks.len() == 1 && !bit_range.start.is_zero() && !bit_range.end.is_zero() {
            cb(marks[0].get_mut(), Some(BitIterator::from_range(bit_range)));
            return;
        }
        if !bit_range.start.is_zero() {
            let (first, m) = marks.split_first_mut().unwrap();
            marks = m;
            cb(
                first.get_mut(),
                Some(BitIterator::from_offset(bit_range.start)),
            );
        }
        let mut end = None;
        if !bit_range.end.is_zero() {
            let (last, m) = marks.split_last_mut().unwrap();
            marks = m;
            end = Some(last.get_mut());
        }
        for mark_byte in marks.iter_mut() {
            cb(mark_byte.get_mut(), None);
        }
        if let Some(end) = end {
            cb(end, Some(BitIterator::until_offset(bit_range.end)));
        }
    }
}

pub(crate) struct BitRangeIterator<'a> {
    bits: &'a [AtomicBits],
    bit_range: Range<BitOffset>,
}

impl<'a> Iterator for BitRangeIterator<'a> {
    type Item = bool;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut hint = self.bits.len() * 8;
        if !self.bit_range.start.is_zero() {
            // Reduce before-start bits.
            hint -= self.bit_range.start.0 as usize;
        }
        if !self.bit_range.end.is_zero() {
            // Reduce after-end bits.
            hint -= (8 - self.bit_range.end.0) as usize;
        }
        (hint, Some(hint))
    }

    fn next(&mut self) -> Option<Self::Item> {
        let byte = self.bits.first()?;
        let value = byte.get_bit(self.bit_range.start.advance());
        if self.bits.len() == 1 && self.bit_range.start == self.bit_range.end {
            // We've reached the end of our bit range.
            self.bits = &[];
        } else if self.bit_range.start.is_zero() {
            // We've passed the last bit of the current byte and need to
            // advance further.
            self.bits = self.bits.split_first().unwrap().1;
        }
        Some(value)
    }
}

impl ExactSizeIterator for BitRangeIterator<'_> {}

impl core::fmt::Debug for BitRangeIterator<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let clone = Self {
            bits: self.bits,
            bit_range: self.bit_range.clone(),
        };
        let result = clone.fold(String::with_capacity(self.bits.len()), |a, b| {
            a + if b { "1" } else { "0" }
        });
        f.debug_struct("BitRangeIterator")
            .field("bits", &result)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct BitOffset(u8);

impl BitOffset {
    const fn new(value: u8) -> Self {
        unsafe { assert_unchecked(value < 8) };
        Self(value)
    }

    const fn advance(&mut self) -> BitOffset {
        let offset = *self;
        self.0 = (self.0 + 1) % 8;
        offset
    }

    #[inline]
    pub(crate) const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Creates a byte with only this offset bit set.
    pub(crate) const fn as_bitmask(&self) -> u8 {
        1u8 << self.0
    }

    pub(crate) const fn from_range(range: &Range<usize>) -> (Range<usize>, Range<BitOffset>) {
        let start_byte_offset = range.start / 8;
        let start_bit_offset = range.start % 8;
        // SAFETY: rem should always return < 8.
        unsafe { assert_unchecked(start_bit_offset < 8) };
        let start_bit_offset = start_bit_offset as u8;
        let end_byte_offset = range.end.div_ceil(8);
        let end_bit_offset = range.end % 8;
        // SAFETY: rem should always return < 8.
        unsafe { assert_unchecked(end_bit_offset < 8) };
        let end_bit_offset = end_bit_offset as u8;
        (
            Range {
                start: start_byte_offset,
                end: end_byte_offset,
            },
            Range {
                start: BitOffset(start_bit_offset),
                end: BitOffset(end_bit_offset),
            },
        )
    }
}

#[derive(Debug)]
pub(crate) struct BitIterator {
    start: BitOffset,
    end: BitOffset,
}

impl BitIterator {
    fn from_range(range: Range<BitOffset>) -> Self {
        debug_assert!(
            !range.start.is_zero() || !range.end.is_zero(),
            "Full byte should not need a bitmask iterator"
        );
        BitIterator {
            start: range.start,
            end: range.end,
        }
    }

    fn from_offset(start: BitOffset) -> Self {
        debug_assert!(
            !start.is_zero(),
            "Full byte should not need a bitmask iterator"
        );
        BitIterator {
            start,
            end: BitOffset(0),
        }
    }

    fn until_offset(end: BitOffset) -> Self {
        debug_assert!(
            !end.is_zero(),
            "Full byte should not need a bitmask iterator"
        );
        BitIterator {
            start: BitOffset(0),
            end,
        }
    }
}

impl Iterator for BitIterator {
    type Item = BitOffset;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            // We've reached the end of our bit range.
            None
        } else {
            let bitmask = self.start.advance();
            Some(bitmask)
        }
    }
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub(crate) struct AtomicBits(AtomicU8);

impl AtomicBits {
    #[inline]
    fn get_mut(&mut self) -> &mut u8 {
        self.0.get_mut()
    }

    /// Get the nth bit atomically. Returns true if the bit is set.
    #[inline]
    fn get_bit(&self, offset: BitOffset) -> bool {
        (self.0.load(Ordering::Relaxed) & offset.as_bitmask()) > 0
    }

    /// Set the nth bit atomically. Returns true if the bit was previously
    /// unset.
    #[inline]
    fn set_bit(&self, offset: BitOffset) -> bool {
        let bitmask = offset.as_bitmask();
        let old_value = self.0.fetch_or(bitmask, Ordering::Relaxed);
        (old_value & bitmask) == 0
    }
}

#[derive(Debug)]
pub(crate) struct HeapBits {
    pub(super) bits: Box<[AtomicBits]>,
    pub(super) e_2_1: BitRange,
    pub(super) e_2_2: BitRange,
    pub(super) e_2_3: BitRange,
    pub(super) e_2_4: BitRange,
    pub(super) e_2_6: BitRange,
    pub(super) e_2_8: BitRange,
    pub(super) e_2_10: BitRange,
    pub(super) e_2_12: BitRange,
    pub(super) e_2_16: BitRange,
    pub(super) e_2_24: BitRange,
    pub(super) e_2_32: BitRange,
    pub(super) k_2_1: BitRange,
    pub(super) k_2_2: BitRange,
    pub(super) k_2_3: BitRange,
    pub(super) k_2_4: BitRange,
    pub(super) k_2_6: BitRange,
    pub(super) k_2_8: BitRange,
    pub(super) k_2_10: BitRange,
    pub(super) k_2_12: BitRange,
    pub(super) k_2_16: BitRange,
    pub(super) k_2_24: BitRange,
    pub(super) k_2_32: BitRange,
    #[cfg(feature = "array-buffer")]
    pub(super) array_buffers: BitRange,
    pub(super) arrays: BitRange,
    pub(super) array_iterators: BitRange,
    pub(super) async_generators: BitRange,
    pub(super) await_reactions: BitRange,
    pub(super) bigints: BitRange,
    pub(super) bound_functions: BitRange,
    pub(super) builtin_constructors: BitRange,
    pub(super) builtin_functions: BitRange,
    pub(super) caches: BitRange,
    #[cfg(feature = "array-buffer")]
    pub(super) data_views: BitRange,
    #[cfg(feature = "date")]
    pub(super) dates: BitRange,
    #[cfg(feature = "temporal")]
    pub(super) instants: BitRange,
    #[cfg(feature = "temporal")]
    pub(super) durations: BitRange,
    #[cfg(feature = "temporal")]
    pub(super) plain_times: BitRange,
    pub(super) declarative_environments: BitRange,
    pub(super) ecmascript_functions: BitRange,
    pub(super) embedder_objects: BitRange,
    pub(super) errors: BitRange,
    pub(super) executables: BitRange,
    pub(super) source_codes: BitRange,
    pub(super) finalization_registrys: BitRange,
    pub(super) function_environments: BitRange,
    pub(super) generators: BitRange,
    pub(super) global_environments: BitRange,
    pub(super) maps: BitRange,
    pub(super) map_iterators: BitRange,
    pub(super) module_environments: BitRange,
    pub(super) modules: BitRange,
    pub(super) module_request_records: BitRange,
    pub(super) numbers: BitRange,
    pub(super) object_environments: BitRange,
    pub(super) object_shapes: BitRange,
    pub(super) objects: BitRange,
    pub(super) primitive_objects: BitRange,
    pub(super) private_environments: BitRange,
    pub(super) promise_reaction_records: BitRange,
    pub(super) promise_resolving_functions: BitRange,
    pub(super) promise_finally_functions: BitRange,
    pub(super) promises: BitRange,
    pub(super) promise_group_records: BitRange,
    pub(super) proxies: BitRange,
    pub(super) realms: BitRange,
    #[cfg(feature = "regexp")]
    pub(super) regexps: BitRange,
    #[cfg(feature = "regexp")]
    pub(super) regexp_string_iterators: BitRange,
    pub(super) scripts: BitRange,
    #[cfg(feature = "set")]
    pub(super) sets: BitRange,
    #[cfg(feature = "set")]
    pub(super) set_iterators: BitRange,
    #[cfg(feature = "shared-array-buffer")]
    pub(super) shared_array_buffers: BitRange,
    #[cfg(feature = "shared-array-buffer")]
    pub(super) shared_data_views: BitRange,
    #[cfg(feature = "shared-array-buffer")]
    pub(super) shared_typed_arrays: BitRange,
    pub(super) source_text_module_records: BitRange,
    pub(super) string_iterators: BitRange,
    pub(super) strings: BitRange,
    pub(super) symbols: BitRange,
    #[cfg(feature = "array-buffer")]
    pub(super) typed_arrays: BitRange,
    #[cfg(feature = "weak-refs")]
    pub(super) weak_maps: BitRange,
    #[cfg(feature = "weak-refs")]
    pub(super) weak_refs: BitRange,
    #[cfg(feature = "weak-refs")]
    pub(super) weak_sets: BitRange,
}

#[derive(Debug)]
pub(crate) struct WorkQueues<'a> {
    pub(crate) bits: &'a HeapBits,
    pub(crate) pending_ephemerons: Vec<(WeakKey<'static>, Value<'static>)>,
    #[cfg(feature = "array-buffer")]
    pub(crate) array_buffers: Vec<ArrayBuffer<'static>>,
    pub(crate) arrays: Vec<Array<'static>>,
    pub(crate) array_iterators: Vec<ArrayIterator<'static>>,
    pub(crate) async_generators: Vec<AsyncGenerator<'static>>,
    pub(crate) await_reactions: Vec<AwaitReaction<'static>>,
    pub(crate) bigints: Vec<HeapBigInt<'static>>,
    pub(crate) bound_functions: Vec<BoundFunction<'static>>,
    pub(crate) builtin_constructors: Vec<BuiltinConstructorFunction<'static>>,
    pub(crate) builtin_functions: Vec<BuiltinFunction<'static>>,
    pub(crate) caches: Vec<PropertyLookupCache<'static>>,
    #[cfg(feature = "array-buffer")]
    pub(crate) data_views: Vec<DataView<'static>>,
    #[cfg(feature = "date")]
    pub(crate) dates: Vec<Date<'static>>,
    #[cfg(feature = "temporal")]
    pub(crate) instants: Vec<TemporalInstant<'static>>,
    #[cfg(feature = "temporal")]
    pub(crate) durations: Vec<TemporalDuration<'static>>,
    #[cfg(feature = "temporal")]
    pub(crate) plain_times: Vec<TemporalPlainTime<'static>>,
    pub(crate) declarative_environments: Vec<DeclarativeEnvironment<'static>>,
    pub(crate) e_2_1: Vec<ElementIndex<'static>>,
    pub(crate) e_2_2: Vec<ElementIndex<'static>>,
    pub(crate) e_2_3: Vec<ElementIndex<'static>>,
    pub(crate) e_2_4: Vec<ElementIndex<'static>>,
    pub(crate) e_2_6: Vec<ElementIndex<'static>>,
    pub(crate) e_2_8: Vec<ElementIndex<'static>>,
    pub(crate) e_2_10: Vec<ElementIndex<'static>>,
    pub(crate) e_2_12: Vec<ElementIndex<'static>>,
    pub(crate) e_2_16: Vec<ElementIndex<'static>>,
    pub(crate) e_2_24: Vec<ElementIndex<'static>>,
    pub(crate) e_2_32: Vec<ElementIndex<'static>>,
    pub(crate) k_2_1: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_2: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_3: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_4: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_6: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_8: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_10: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_12: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_16: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_24: Vec<PropertyKeyIndex<'static>>,
    pub(crate) k_2_32: Vec<PropertyKeyIndex<'static>>,
    pub(crate) ecmascript_functions: Vec<ECMAScriptFunction<'static>>,
    pub(crate) embedder_objects: Vec<EmbedderObject<'static>>,
    pub(crate) source_codes: Vec<SourceCode<'static>>,
    pub(crate) errors: Vec<Error<'static>>,
    pub(crate) executables: Vec<Executable<'static>>,
    pub(crate) finalization_registrys: Vec<FinalizationRegistry<'static>>,
    pub(crate) function_environments: Vec<FunctionEnvironment<'static>>,
    pub(crate) generators: Vec<Generator<'static>>,
    pub(crate) global_environments: Vec<GlobalEnvironment<'static>>,
    pub(crate) maps: Vec<Map<'static>>,
    pub(crate) map_iterators: Vec<MapIterator<'static>>,
    pub(crate) module_environments: Vec<ModuleEnvironment<'static>>,
    pub(crate) modules: Vec<Module<'static>>,
    pub(crate) module_request_records: Vec<ModuleRequest<'static>>,
    pub(crate) numbers: Vec<HeapNumber<'static>>,
    pub(crate) object_environments: Vec<ObjectEnvironment<'static>>,
    pub(crate) objects: Vec<OrdinaryObject<'static>>,
    pub(crate) object_shapes: Vec<ObjectShape<'static>>,
    pub(crate) primitive_objects: Vec<PrimitiveObject<'static>>,
    pub(crate) private_environments: Vec<PrivateEnvironment<'static>>,
    pub(crate) promises: Vec<Promise<'static>>,
    pub(crate) promise_reaction_records: Vec<PromiseReaction<'static>>,
    pub(crate) promise_resolving_functions: Vec<BuiltinPromiseResolvingFunction<'static>>,
    pub(crate) promise_finally_functions: Vec<BuiltinPromiseFinallyFunction<'static>>,
    pub(crate) promise_group_records: Vec<PromiseGroup<'static>>,
    pub(crate) proxies: Vec<Proxy<'static>>,
    pub(crate) realms: Vec<Realm<'static>>,
    #[cfg(feature = "regexp")]
    pub(crate) regexps: Vec<RegExp<'static>>,
    #[cfg(feature = "regexp")]
    pub(crate) regexp_string_iterators: Vec<RegExpStringIterator<'static>>,
    pub(crate) scripts: Vec<Script<'static>>,
    #[cfg(feature = "set")]
    pub(crate) sets: Vec<Set<'static>>,
    #[cfg(feature = "set")]
    pub(crate) set_iterators: Vec<SetIterator<'static>>,
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) shared_array_buffers: Vec<SharedArrayBuffer<'static>>,
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) shared_data_views: Vec<SharedDataView<'static>>,
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) shared_typed_arrays: Vec<SharedVoidArray<'static>>,
    pub(crate) source_text_module_records: Vec<SourceTextModule<'static>>,
    pub(crate) string_iterators: Vec<StringIterator<'static>>,
    pub(crate) strings: Vec<HeapString<'static>>,
    pub(crate) symbols: Vec<Symbol<'static>>,
    #[cfg(feature = "array-buffer")]
    pub(crate) typed_arrays: Vec<VoidArray<'static>>,
    #[cfg(feature = "weak-refs")]
    pub(crate) weak_maps: Vec<WeakMap<'static>>,
    #[cfg(feature = "weak-refs")]
    pub(crate) weak_refs: Vec<WeakRef<'static>>,
    #[cfg(feature = "weak-refs")]
    pub(crate) weak_sets: Vec<WeakSet<'static>>,
}

impl HeapBits {
    pub(crate) fn new(heap: &Heap) -> Self {
        let mut bit_count = 0;

        let e_2_1 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow1.values.len());
        let e_2_2 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow2.values.len());
        let e_2_3 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow3.values.len());
        let e_2_4 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow4.values.len());
        let e_2_6 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow6.values.len());
        let e_2_8 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow8.values.len());
        let k_2_1 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow1.keys.len());
        let k_2_2 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow2.keys.len());
        let k_2_3 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow3.keys.len());
        let k_2_4 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow4.keys.len());
        let k_2_6 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow6.keys.len());
        let k_2_8 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow8.keys.len());
        let e_2_10 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow10.values.len());
        let e_2_12 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow12.values.len());
        let e_2_16 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow16.values.len());
        let k_2_10 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow10.keys.len());
        let k_2_12 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow12.keys.len());
        let k_2_16 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow16.keys.len());
        let e_2_24 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow24.values.len());
        let e_2_32 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.e2pow32.values.len());
        let k_2_24 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow24.keys.len());
        let k_2_32 =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.elements.k2pow32.keys.len());
        #[cfg(feature = "array-buffer")]
        let array_buffers =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.array_buffers.len());
        let arrays = BitRange::from_bit_count_and_len(&mut bit_count, heap.arrays.len() as usize);
        let array_iterators =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.array_iterators.len());
        let async_generators =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.async_generators.len());
        let await_reactions =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.await_reactions.len());
        let bigints = BitRange::from_bit_count_and_len(&mut bit_count, heap.bigints.len());
        let bound_functions =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.bound_functions.len());
        let builtin_constructors =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.builtin_constructors.len());
        let builtin_functions =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.builtin_functions.len());
        let caches = BitRange::from_bit_count_and_len(&mut bit_count, heap.caches.len());
        #[cfg(feature = "array-buffer")]
        let data_views = BitRange::from_bit_count_and_len(&mut bit_count, heap.data_views.len());
        #[cfg(feature = "date")]
        let dates = BitRange::from_bit_count_and_len(&mut bit_count, heap.dates.len());
        #[cfg(feature = "temporal")]
        let instants = BitRange::from_bit_count_and_len(&mut bit_count, heap.instants.len());
        #[cfg(feature = "temporal")]
        let durations = BitRange::from_bit_count_and_len(&mut bit_count, heap.durations.len());
        #[cfg(feature = "temporal")]
        let plain_times = BitRange::from_bit_count_and_len(&mut bit_count, heap.plain_times.len());
        let declarative_environments =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.environments.declarative.len());
        let ecmascript_functions =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.ecmascript_functions.len());
        let embedder_objects =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.embedder_objects.len());
        let errors = BitRange::from_bit_count_and_len(&mut bit_count, heap.errors.len());
        let executables = BitRange::from_bit_count_and_len(&mut bit_count, heap.executables.len());
        let source_codes =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.source_codes.len());
        let finalization_registrys = BitRange::from_bit_count_and_len(
            &mut bit_count,
            heap.finalization_registrys.len() as usize,
        );
        let function_environments =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.environments.function.len());
        let generators = BitRange::from_bit_count_and_len(&mut bit_count, heap.generators.len());
        let global_environments =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.environments.global.len());
        let maps = BitRange::from_bit_count_and_len(&mut bit_count, heap.maps.len() as usize);
        let map_iterators =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.map_iterators.len());
        let module_environments =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.environments.module.len());
        let modules = BitRange::from_bit_count_and_len(&mut bit_count, heap.modules.len());
        let module_request_records =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.module_request_records.len());
        let numbers = BitRange::from_bit_count_and_len(&mut bit_count, heap.numbers.len());
        let object_environments =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.environments.object.len());
        let object_shapes =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.object_shapes.len());
        let objects = BitRange::from_bit_count_and_len(&mut bit_count, heap.objects.len());
        let primitive_objects =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.primitive_objects.len());
        let promise_reaction_records =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.promise_reaction_records.len());
        let promise_resolving_functions = BitRange::from_bit_count_and_len(
            &mut bit_count,
            heap.promise_resolving_functions.len(),
        );
        let promise_finally_functions =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.promise_finally_functions.len());
        let private_environments =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.environments.private.len());
        let promises = BitRange::from_bit_count_and_len(&mut bit_count, heap.promises.len());
        let promise_group_records =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.promise_group_records.len());
        let proxies = BitRange::from_bit_count_and_len(&mut bit_count, heap.proxies.len());
        let realms = BitRange::from_bit_count_and_len(&mut bit_count, heap.realms.len());
        #[cfg(feature = "regexp")]
        let regexps = BitRange::from_bit_count_and_len(&mut bit_count, heap.regexps.len());
        #[cfg(feature = "regexp")]
        let regexp_string_iterators =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.regexp_string_iterators.len());
        let scripts = BitRange::from_bit_count_and_len(&mut bit_count, heap.scripts.len());
        #[cfg(feature = "set")]
        let sets = BitRange::from_bit_count_and_len(&mut bit_count, heap.sets.len() as usize);
        #[cfg(feature = "set")]
        let set_iterators =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.set_iterators.len());
        #[cfg(feature = "shared-array-buffer")]
        let shared_array_buffers =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.shared_array_buffers.len());
        #[cfg(feature = "shared-array-buffer")]
        let shared_data_views =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.shared_data_views.len());
        #[cfg(feature = "shared-array-buffer")]
        let shared_typed_arrays =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.shared_typed_arrays.len());
        let source_text_module_records =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.source_text_module_records.len());
        let string_iterators =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.string_iterators.len());
        let strings = BitRange::from_bit_count_and_len(&mut bit_count, heap.strings.len());
        let symbols = BitRange::from_bit_count_and_len(&mut bit_count, heap.symbols.len());
        #[cfg(feature = "array-buffer")]
        let typed_arrays =
            BitRange::from_bit_count_and_len(&mut bit_count, heap.typed_arrays.len());
        #[cfg(feature = "weak-refs")]
        let weak_maps = BitRange::from_bit_count_and_len(&mut bit_count, heap.weak_maps.len());
        #[cfg(feature = "weak-refs")]
        let weak_refs = BitRange::from_bit_count_and_len(&mut bit_count, heap.weak_refs.len());
        #[cfg(feature = "weak-refs")]
        let weak_sets = BitRange::from_bit_count_and_len(&mut bit_count, heap.weak_sets.len());
        let byte_count = bit_count.div_ceil(8);
        let mut bits = Box::<[AtomicBits]>::new_uninit_slice(byte_count);
        bits.fill_with(|| MaybeUninit::new(Default::default()));
        // SAFETY: filled in.
        let bits = unsafe { bits.assume_init() };
        Self {
            bits,
            #[cfg(feature = "array-buffer")]
            array_buffers,
            arrays,
            array_iterators,
            async_generators,
            await_reactions,
            bigints,
            bound_functions,
            builtin_constructors,
            builtin_functions,
            caches,
            #[cfg(feature = "array-buffer")]
            data_views,
            #[cfg(feature = "date")]
            dates,
            #[cfg(feature = "temporal")]
            instants,
            #[cfg(feature = "temporal")]
            durations,
            #[cfg(feature = "temporal")]
            plain_times,
            declarative_environments,
            e_2_1,
            e_2_2,
            e_2_3,
            e_2_4,
            e_2_6,
            e_2_8,
            e_2_10,
            e_2_12,
            e_2_16,
            e_2_24,
            e_2_32,
            k_2_1,
            k_2_2,
            k_2_3,
            k_2_4,
            k_2_6,
            k_2_8,
            k_2_10,
            k_2_12,
            k_2_16,
            k_2_24,
            k_2_32,
            ecmascript_functions,
            embedder_objects,
            errors,
            executables,
            source_codes,
            finalization_registrys,
            function_environments,
            generators,
            global_environments,
            maps,
            map_iterators,
            module_environments,
            modules,
            module_request_records,
            numbers,
            object_environments,
            object_shapes,
            objects,
            primitive_objects,
            promise_reaction_records,
            promise_resolving_functions,
            promise_finally_functions,
            private_environments,
            promises,
            promise_group_records,
            proxies,
            realms,
            #[cfg(feature = "regexp")]
            regexps,
            #[cfg(feature = "regexp")]
            regexp_string_iterators,
            scripts,
            #[cfg(feature = "set")]
            sets,
            #[cfg(feature = "set")]
            set_iterators,
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers,
            #[cfg(feature = "shared-array-buffer")]
            shared_data_views,
            #[cfg(feature = "shared-array-buffer")]
            shared_typed_arrays,
            source_text_module_records,
            string_iterators,
            strings,
            symbols,
            #[cfg(feature = "array-buffer")]
            typed_arrays,
            #[cfg(feature = "weak-refs")]
            weak_maps,
            #[cfg(feature = "weak-refs")]
            weak_refs,
            #[cfg(feature = "weak-refs")]
            weak_sets,
        }
    }

    pub(crate) fn is_marked(&self, key: &WeakKey) -> bool {
        match key {
            WeakKey::Symbol(d) => self.symbols.get_bit(d.get_index(), &self.bits),
            WeakKey::Object(d) => self.objects.get_bit(d.get_index(), &self.bits),
            WeakKey::BoundFunction(d) => self.bound_functions.get_bit(d.get_index(), &self.bits),
            WeakKey::BuiltinFunction(d) => {
                self.builtin_functions.get_bit(d.get_index(), &self.bits)
            }
            WeakKey::ECMAScriptFunction(d) => {
                self.ecmascript_functions.get_bit(d.get_index(), &self.bits)
            }
            WeakKey::BuiltinConstructorFunction(d) => {
                self.builtin_constructors.get_bit(d.get_index(), &self.bits)
            }
            WeakKey::BuiltinPromiseResolvingFunction(d) => self
                .promise_resolving_functions
                .get_bit(d.get_index(), &self.bits),
            WeakKey::BuiltinPromiseFinallyFunction(d) => self
                .promise_finally_functions
                .get_bit(d.get_index(), &self.bits),
            WeakKey::BuiltinPromiseCollectorFunction | WeakKey::BuiltinProxyRevokerFunction => {
                unreachable!()
            }
            WeakKey::PrimitiveObject(d) => {
                self.primitive_objects.get_bit(d.get_index(), &self.bits)
            }
            WeakKey::Arguments(d) => self.objects.get_bit(d.get_index(), &self.bits),
            WeakKey::Array(d) => self.arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "date")]
            WeakKey::Date(d) => self.dates.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "temporal")]
            WeakKey::Instant(d) => self.instants.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "temporal")]
            WeakKey::Duration(d) => self.durations.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "temporal")]
            WeakKey::PlainTime(d) => self.plain_times.get_bit(d.get_index(), &self.bits),
            WeakKey::Error(d) => self.errors.get_bit(d.get_index(), &self.bits),
            WeakKey::FinalizationRegistry(d) => self
                .finalization_registrys
                .get_bit(d.get_index(), &self.bits),
            WeakKey::Map(d) => self.maps.get_bit(d.get_index(), &self.bits),
            WeakKey::Promise(d) => self.promises.get_bit(d.get_index(), &self.bits),
            WeakKey::Proxy(d) => self.proxies.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "regexp")]
            WeakKey::RegExp(d) => self.regexps.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "set")]
            WeakKey::Set(d) => self.sets.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakMap(d) => self.weak_maps.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakRef(d) => self.weak_refs.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "weak-refs")]
            WeakKey::WeakSet(d) => self.weak_sets.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::ArrayBuffer(d) => self.array_buffers.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::DataView(d) => self.data_views.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int8Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint8ClampedArray(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int16Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint16Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Int32Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Uint32Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigInt64Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::BigUint64Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "proposal-float16array")]
            WeakKey::Float16Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float32Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "array-buffer")]
            WeakKey::Float64Array(d) => self.typed_arrays.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedArrayBuffer(d) => {
                self.shared_array_buffers.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedDataView(d) => self.shared_data_views.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt8Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint8Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint8ClampedArray(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt16Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint16Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedInt32Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedUint32Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedBigInt64Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedBigUint64Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            WeakKey::SharedFloat16Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedFloat32Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            #[cfg(feature = "shared-array-buffer")]
            WeakKey::SharedFloat64Array(d) => {
                self.shared_typed_arrays.get_bit(d.get_index(), &self.bits)
            }
            WeakKey::AsyncGenerator(d) => self.async_generators.get_bit(d.get_index(), &self.bits),
            WeakKey::ArrayIterator(d) => self.array_iterators.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "set")]
            WeakKey::SetIterator(d) => self.set_iterators.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "set")]
            WeakKey::MapIterator(d) => self.map_iterators.get_bit(d.get_index(), &self.bits),
            WeakKey::StringIterator(d) => self.string_iterators.get_bit(d.get_index(), &self.bits),
            #[cfg(feature = "regexp")]
            WeakKey::RegExpStringIterator(d) => self
                .regexp_string_iterators
                .get_bit(d.get_index(), &self.bits),
            WeakKey::Generator(d) => self.generators.get_bit(d.get_index(), &self.bits),
            WeakKey::Module(d) => self.modules.get_bit(d.get_index(), &self.bits),
            WeakKey::EmbedderObject(d) => self.embedder_objects.get_bit(d.get_index(), &self.bits),
        }
    }
}

impl<'a> WorkQueues<'a> {
    pub(crate) fn new(heap: &Heap, bits: &'a HeapBits) -> Self {
        Self {
            bits,
            pending_ephemerons: vec![],
            #[cfg(feature = "array-buffer")]
            array_buffers: Vec::with_capacity(heap.array_buffers.len() / 4),
            arrays: Vec::with_capacity(heap.arrays.len() as usize / 4),
            array_iterators: Vec::with_capacity(heap.array_iterators.len() / 4),
            async_generators: Vec::with_capacity(heap.async_generators.len() / 4),
            await_reactions: Vec::with_capacity(heap.await_reactions.len() / 4),
            bigints: Vec::with_capacity(heap.bigints.len() / 4),
            bound_functions: Vec::with_capacity(heap.bound_functions.len() / 4),
            builtin_constructors: Vec::with_capacity(heap.builtin_constructors.len() / 4),
            builtin_functions: Vec::with_capacity(heap.builtin_functions.len() / 4),
            caches: Vec::with_capacity(heap.caches.len() / 4),
            #[cfg(feature = "array-buffer")]
            data_views: Vec::with_capacity(heap.data_views.len() / 4),
            #[cfg(feature = "date")]
            dates: Vec::with_capacity(heap.dates.len() / 4),
            #[cfg(feature = "temporal")]
            instants: Vec::with_capacity(heap.instants.len() / 4),
            #[cfg(feature = "temporal")]
            durations: Vec::with_capacity(heap.durations.len() / 4),
            #[cfg(feature = "temporal")]
            plain_times: Vec::with_capacity(heap.plain_times.len() / 4),
            declarative_environments: Vec::with_capacity(heap.environments.declarative.len() / 4),
            e_2_1: Vec::with_capacity(heap.elements.e2pow1.values.len() / 4),
            e_2_2: Vec::with_capacity(heap.elements.e2pow2.values.len() / 4),
            e_2_3: Vec::with_capacity(heap.elements.e2pow3.values.len() / 4),
            e_2_4: Vec::with_capacity(heap.elements.e2pow4.values.len() / 4),
            e_2_6: Vec::with_capacity(heap.elements.e2pow6.values.len() / 4),
            e_2_8: Vec::with_capacity(heap.elements.e2pow8.values.len() / 4),
            e_2_10: Vec::with_capacity(heap.elements.e2pow10.values.len() / 4),
            e_2_12: Vec::with_capacity(heap.elements.e2pow12.values.len() / 4),
            e_2_16: Vec::with_capacity(heap.elements.e2pow16.values.len() / 4),
            e_2_24: Vec::with_capacity(heap.elements.e2pow24.values.len() / 4),
            e_2_32: Vec::with_capacity(heap.elements.e2pow32.values.len() / 4),
            k_2_1: Vec::with_capacity(heap.elements.k2pow1.keys.len() / 4),
            k_2_2: Vec::with_capacity(heap.elements.k2pow2.keys.len() / 4),
            k_2_3: Vec::with_capacity(heap.elements.k2pow3.keys.len() / 4),
            k_2_4: Vec::with_capacity(heap.elements.k2pow4.keys.len() / 4),
            k_2_6: Vec::with_capacity(heap.elements.k2pow6.keys.len() / 4),
            k_2_8: Vec::with_capacity(heap.elements.k2pow8.keys.len() / 4),
            k_2_10: Vec::with_capacity(heap.elements.k2pow10.keys.len() / 4),
            k_2_12: Vec::with_capacity(heap.elements.k2pow12.keys.len() / 4),
            k_2_16: Vec::with_capacity(heap.elements.k2pow16.keys.len() / 4),
            k_2_24: Vec::with_capacity(heap.elements.k2pow24.keys.len() / 4),
            k_2_32: Vec::with_capacity(heap.elements.k2pow32.keys.len() / 4),
            ecmascript_functions: Vec::with_capacity(heap.ecmascript_functions.len() / 4),
            embedder_objects: Vec::with_capacity(heap.embedder_objects.len() / 4),
            errors: Vec::with_capacity(heap.errors.len() / 4),
            executables: Vec::with_capacity(heap.executables.len() / 4),
            source_codes: Vec::with_capacity(heap.source_codes.len() / 4),
            finalization_registrys: Vec::with_capacity(
                heap.finalization_registrys.len() as usize / 4,
            ),
            function_environments: Vec::with_capacity(heap.environments.function.len() / 4),
            generators: Vec::with_capacity(heap.generators.len() / 4),
            global_environments: Vec::with_capacity(heap.environments.global.len() / 4),
            maps: Vec::with_capacity(heap.maps.len() as usize / 4),
            map_iterators: Vec::with_capacity(heap.map_iterators.len() / 4),
            module_environments: Vec::with_capacity(heap.environments.module.len() / 4),
            modules: Vec::with_capacity(heap.modules.len() / 4),
            module_request_records: Vec::with_capacity(heap.module_request_records.len() / 4),
            numbers: Vec::with_capacity(heap.numbers.len() / 4),
            object_environments: Vec::with_capacity(heap.environments.object.len() / 4),
            object_shapes: Vec::with_capacity(heap.object_shapes.len() / 4),
            objects: Vec::with_capacity(heap.objects.len() / 4),
            primitive_objects: Vec::with_capacity(heap.primitive_objects.len() / 4),
            private_environments: Vec::with_capacity(heap.environments.private.len() / 4),
            promise_reaction_records: Vec::with_capacity(heap.promise_reaction_records.len() / 4),
            promise_resolving_functions: Vec::with_capacity(
                heap.promise_resolving_functions.len() / 4,
            ),
            promise_finally_functions: Vec::with_capacity(heap.promise_finally_functions.len() / 4),
            promises: Vec::with_capacity(heap.promises.len() / 4),
            promise_group_records: Vec::with_capacity(heap.promise_group_records.len() / 4),
            proxies: Vec::with_capacity(heap.proxies.len() / 4),
            realms: Vec::with_capacity(heap.realms.len() / 4),
            #[cfg(feature = "regexp")]
            regexps: Vec::with_capacity(heap.regexps.len() / 4),
            #[cfg(feature = "regexp")]
            regexp_string_iterators: Vec::with_capacity(heap.regexp_string_iterators.len() / 4),
            scripts: Vec::with_capacity(heap.scripts.len() / 4),
            #[cfg(feature = "set")]
            sets: Vec::with_capacity(heap.sets.len() as usize / 4),
            #[cfg(feature = "set")]
            set_iterators: Vec::with_capacity(heap.set_iterators.len() / 4),
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers: Vec::with_capacity(heap.shared_array_buffers.len() / 4),
            #[cfg(feature = "shared-array-buffer")]
            shared_data_views: Vec::with_capacity(heap.shared_data_views.len() / 4),
            #[cfg(feature = "shared-array-buffer")]
            shared_typed_arrays: Vec::with_capacity(heap.shared_typed_arrays.len() / 4),
            source_text_module_records: Vec::with_capacity(
                heap.source_text_module_records.len() / 4,
            ),
            string_iterators: Vec::with_capacity(heap.string_iterators.len() / 4),
            strings: Vec::with_capacity((heap.strings.len() / 4).max(BUILTIN_STRINGS_LIST.len())),
            symbols: Vec::with_capacity((heap.symbols.len() / 4).max(13)),
            #[cfg(feature = "array-buffer")]
            typed_arrays: Vec::with_capacity(heap.typed_arrays.len() / 4),
            #[cfg(feature = "weak-refs")]
            weak_maps: Vec::with_capacity(heap.weak_maps.len() / 4),
            #[cfg(feature = "weak-refs")]
            weak_refs: Vec::with_capacity(heap.weak_refs.len() / 4),
            #[cfg(feature = "weak-refs")]
            weak_sets: Vec::with_capacity(heap.weak_sets.len() / 4),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        let Self {
            bits: _,
            // Note: we can have pending ephemerons that will never resolve.
            pending_ephemerons: _,
            #[cfg(feature = "array-buffer")]
            array_buffers,
            arrays,
            array_iterators,
            async_generators,
            await_reactions,
            bigints,
            bound_functions,
            builtin_constructors,
            builtin_functions,
            caches,
            #[cfg(feature = "array-buffer")]
            data_views,
            #[cfg(feature = "date")]
            dates,
            #[cfg(feature = "temporal")]
            instants,
            #[cfg(feature = "temporal")]
            durations,
            #[cfg(feature = "temporal")]
            plain_times,
            declarative_environments,
            e_2_1,
            e_2_2,
            e_2_3,
            e_2_4,
            e_2_6,
            e_2_8,
            e_2_10,
            e_2_12,
            e_2_16,
            e_2_24,
            e_2_32,
            k_2_1,
            k_2_2,
            k_2_3,
            k_2_4,
            k_2_6,
            k_2_8,
            k_2_10,
            k_2_12,
            k_2_16,
            k_2_24,
            k_2_32,
            ecmascript_functions,
            embedder_objects,
            source_codes,
            errors,
            executables,
            finalization_registrys,
            function_environments,
            generators,
            global_environments,
            maps,
            map_iterators,
            module_environments,
            modules,
            module_request_records,
            numbers,
            object_environments,
            object_shapes,
            objects,
            primitive_objects,
            private_environments,
            promises,
            promise_reaction_records,
            promise_resolving_functions,
            promise_finally_functions,
            promise_group_records,
            proxies,
            realms,
            #[cfg(feature = "regexp")]
            regexps,
            #[cfg(feature = "regexp")]
            regexp_string_iterators,
            scripts,
            #[cfg(feature = "set")]
            sets,
            #[cfg(feature = "set")]
            set_iterators,
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers,
            #[cfg(feature = "shared-array-buffer")]
            shared_data_views,
            #[cfg(feature = "shared-array-buffer")]
            shared_typed_arrays,
            source_text_module_records,
            string_iterators,
            strings,
            symbols,
            #[cfg(feature = "array-buffer")]
            typed_arrays,
            #[cfg(feature = "weak-refs")]
            weak_maps,
            #[cfg(feature = "weak-refs")]
            weak_refs,
            #[cfg(feature = "weak-refs")]
            weak_sets,
        } = self;

        #[cfg(not(feature = "temporal"))]
        let instants: &[bool; 0] = &[];
        #[cfg(not(feature = "temporal"))]
        let durations: &[bool; 0] = &[];
        #[cfg(not(feature = "temporal"))]
        let plain_times: &[bool; 0] = &[];
        #[cfg(not(feature = "date"))]
        let dates: &[bool; 0] = &[];
        #[cfg(not(feature = "array-buffer"))]
        let data_views: &[bool; 0] = &[];
        #[cfg(not(feature = "array-buffer"))]
        let array_buffers: &[bool; 0] = &[];
        #[cfg(not(feature = "array-buffer"))]
        let typed_arrays: &[bool; 0] = &[];
        #[cfg(not(feature = "shared-array-buffer"))]
        let shared_array_buffers: &[bool; 0] = &[];
        #[cfg(not(feature = "shared-array-buffer"))]
        let shared_data_views: &[bool; 0] = &[];
        #[cfg(not(feature = "shared-array-buffer"))]
        let shared_typed_arrays: &[bool; 0] = &[];
        #[cfg(not(feature = "weak-refs"))]
        let weak_maps: &[bool; 0] = &[];
        #[cfg(not(feature = "weak-refs"))]
        let weak_refs: &[bool; 0] = &[];
        #[cfg(not(feature = "weak-refs"))]
        let weak_sets: &[bool; 0] = &[];
        #[cfg(not(feature = "regexp"))]
        let regexps: &[bool; 0] = &[];
        #[cfg(not(feature = "regexp"))]
        let regexp_string_iterators: &[bool; 0] = &[];
        #[cfg(not(feature = "set"))]
        let sets: &[bool; 0] = &[];
        #[cfg(not(feature = "set"))]
        let set_iterators: &[bool; 0] = &[];
        array_buffers.is_empty()
            && arrays.is_empty()
            && array_iterators.is_empty()
            && async_generators.is_empty()
            && await_reactions.is_empty()
            && bigints.is_empty()
            && bound_functions.is_empty()
            && builtin_constructors.is_empty()
            && builtin_functions.is_empty()
            && caches.is_empty()
            && data_views.is_empty()
            && dates.is_empty()
            && instants.is_empty()
            && durations.is_empty()
            && plain_times.is_empty()
            && declarative_environments.is_empty()
            && e_2_1.is_empty()
            && e_2_2.is_empty()
            && e_2_3.is_empty()
            && e_2_4.is_empty()
            && e_2_6.is_empty()
            && e_2_8.is_empty()
            && e_2_10.is_empty()
            && e_2_12.is_empty()
            && e_2_16.is_empty()
            && e_2_24.is_empty()
            && e_2_32.is_empty()
            && k_2_1.is_empty()
            && k_2_2.is_empty()
            && k_2_3.is_empty()
            && k_2_4.is_empty()
            && k_2_6.is_empty()
            && k_2_8.is_empty()
            && k_2_10.is_empty()
            && k_2_12.is_empty()
            && k_2_16.is_empty()
            && k_2_24.is_empty()
            && k_2_32.is_empty()
            && ecmascript_functions.is_empty()
            && embedder_objects.is_empty()
            && errors.is_empty()
            && executables.is_empty()
            && source_codes.is_empty()
            && finalization_registrys.is_empty()
            && function_environments.is_empty()
            && generators.is_empty()
            && global_environments.is_empty()
            && maps.is_empty()
            && map_iterators.is_empty()
            && module_environments.is_empty()
            && modules.is_empty()
            && module_request_records.is_empty()
            && numbers.is_empty()
            && object_environments.is_empty()
            && object_shapes.is_empty()
            && objects.is_empty()
            && primitive_objects.is_empty()
            && private_environments.is_empty()
            && promise_reaction_records.is_empty()
            && promise_resolving_functions.is_empty()
            && promise_finally_functions.is_empty()
            && promise_group_records.is_empty()
            && promises.is_empty()
            && proxies.is_empty()
            && realms.is_empty()
            && regexps.is_empty()
            && regexp_string_iterators.is_empty()
            && scripts.is_empty()
            && sets.is_empty()
            && set_iterators.is_empty()
            && shared_array_buffers.is_empty()
            && shared_data_views.is_empty()
            && shared_typed_arrays.is_empty()
            && source_text_module_records.is_empty()
            && string_iterators.is_empty()
            && strings.is_empty()
            && symbols.is_empty()
            && typed_arrays.is_empty()
            && weak_maps.is_empty()
            && weak_refs.is_empty()
            && weak_sets.is_empty()
    }
}

#[repr(transparent)]
pub(crate) struct CompactionList {
    shifts: SoAVec<ShiftData>,
}

impl core::fmt::Debug for CompactionList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slices = self.shifts.as_slice();

        f.debug_list()
            .entries(
                slices
                    .index
                    .iter()
                    .as_slice()
                    .iter()
                    .zip(slices.shift.iter()),
            )
            .finish()
    }
}

impl CompactionList {
    /// Perform a shift on a strongly held reference index. Returns a shifted
    /// index.
    fn shift_strong_u32_index(&self, index: u32) -> u32 {
        if self.shifts.is_empty() {
            // If there are no shifts, then all items stay where they are.
            return index;
        }
        let ShiftDataSlice {
            index: indexes,
            shift: shifts,
        } = self.shifts.as_slice();
        match indexes.binary_search(&index) {
            Ok(exact_index) => {
                // An exact match means we have the exact correct index to get
                // our shift from.
                index - shifts[exact_index]
            }
            Err(upper_bound_index) => {
                // Otherwise we find an upper-bound index; it can be zero.
                let own_location = upper_bound_index.checked_sub(1);
                // If the upper-bound index is zero, then our shift amount is
                // zero as well.
                index - own_location.map(|i| shifts[i]).unwrap_or(0)
            }
        }
    }

    /// Shift a weakly held bare u32 reference index. Returns a new index if
    /// the reference target is live, otherwise returns None.
    pub(crate) fn shift_weak_u32_index(&self, index: u32) -> Option<u32> {
        // If there are no shift indexes, then all values are live.
        if self.shifts.is_empty() {
            return Some(index);
        }
        let ShiftDataSlice {
            index: indexes,
            shift: shifts,
        } = self.shifts.as_slice();
        // Find the place in the indexes list where our index is or should be
        // placed to maintain order.
        let found_index = indexes.binary_search(&index);
        let insertion_index = match found_index {
            Ok(exact_index) => {
                // If we found an exact index then it means that our index is
                // necessarily live and we just need to shift it down by the
                // appropriate amount.
                let shift_amount = shifts[exact_index];
                return Some(index - shift_amount);
            }
            Err(i) => i,
        };
        // It's possible that our index is at the "top shift" position.
        // In that case our index is necessarily alive.
        if insertion_index == indexes.len() {
            let own_shift_amount = *shifts.last().unwrap();
            return Some(index - own_shift_amount);
        }
        // This is the lowest index that could overwrite our index...
        let upper_bound = indexes[insertion_index];
        // ... and this is how much it shifts down.
        let upper_bound_shift_amount = shifts[insertion_index];
        // After the shift, it ends up at this index.
        let post_shift_upper_bound = upper_bound - upper_bound_shift_amount;
        // Our own shift amount is found in the next slot below the insertion
        // index; the insertion index can be zero so we do a checked sub here.
        let own_location = insertion_index.checked_sub(1);
        // If insertion index is zero then our index does not shift but can
        // still be overwritten.
        let own_shift_amount = own_location.map(|i| shifts[i]).unwrap_or(0);
        let post_shift_index = index - own_shift_amount;

        // If the post-shift upper bound shifts to be less or equal than our
        // post-shift index, then it means that we're being overwritten and are
        // no longer live.
        if post_shift_upper_bound <= post_shift_index {
            None
        } else {
            // Otherwise, we're still live with the post-shift index value.
            Some(post_shift_index)
        }
    }

    /// Shift a strongly held bare u32 reference index.
    ///
    /// Where possible, prefer using the `shift_index` function.
    pub(crate) fn shift_u32_index(&self, base_index: &mut u32) {
        *base_index = self.shift_strong_u32_index(*base_index);
    }

    /// Shift a strongly held reference index.
    pub(crate) fn shift_index<T: ?Sized>(&self, index: &mut BaseIndex<T>) {
        *index = BaseIndex::from_index_u32(self.shift_strong_u32_index(index.get_index_u32()));
    }

    /// Shift a strongly held bare NonZeroU32 reference index.
    ///
    /// Where possible, prefer using `shift_index` function.
    pub(crate) fn shift_non_zero_u32_index(&self, index: &mut NonZeroU32) {
        // 1-indexed value
        let base_index: u32 = (*index).into();
        // 0-indexed value
        // SAFETY: NonZeroU32 as u32 cannot wrap when 1 is subtracted.
        let base_index = unsafe { base_index.unchecked_sub(1) };
        // SAFETY: Shifted base index can be 0, adding 1 makes it non-zero.
        *index = unsafe {
            NonZeroU32::new_unchecked(self.shift_strong_u32_index(base_index).unchecked_add(1))
        };
    }

    /// Shift a weakly held reference index. Returns a new index if the
    /// reference target is live, otherwise returns None.
    pub(crate) fn shift_weak_index<'a, T: ?Sized>(
        &self,
        index: BaseIndex<'a, T>,
    ) -> Option<BaseIndex<'a, T>> {
        let base_index = index.get_index_u32();
        let base_index = self.shift_weak_u32_index(base_index)?;
        Some(BaseIndex::from_index_u32(base_index))
    }

    /// Shift a weakly held non-zero reference index. Returns a new index if
    /// the reference target is live, otherwise returns None.
    pub(crate) fn shift_weak_non_zero_u32_index(&self, index: NonZeroU32) -> Option<NonZeroU32> {
        // 1-indexed value
        let base_index: u32 = index.into();
        // 0-indexed value
        let base_index = base_index.wrapping_sub(1);
        let base_index = self.shift_weak_u32_index(base_index)?.wrapping_add(1);
        // SAFETY: we added 1 to the u32, which itself comes from our original
        // index. It can be shifted down but will never wrap around, so adding
        // the 1 cannot wrap around to 0 either.
        Some(unsafe { NonZeroU32::new_unchecked(base_index) })
    }

    fn build(shifts: SoAVec<ShiftData>) -> Self {
        Self { shifts }
    }

    pub(crate) fn from_mark_bits(range: &BitRange, marks: &[AtomicBits]) -> Self {
        let builder = CompactionListBuilder::with_bits_length(range.len() as u32);
        if range.is_empty() {
            return builder.done();
        }
        let builder = UnsafeCell::new(builder);
        range.for_each_byte(marks, |mark_byte, bit_iterator| {
            // SAFETY: synchronous calls.
            let builder = unsafe { builder.get().as_mut().unwrap() };
            if let Some(bit_iterator) = bit_iterator {
                for bit_offset in bit_iterator {
                    if mark_byte.get_bit(bit_offset) {
                        builder.mark_used();
                    } else {
                        builder.mark_unused();
                    }
                }
            } else {
                for bit_offset in 0..8 {
                    if mark_byte.get_bit(BitOffset::new(bit_offset)) {
                        builder.mark_used();
                    } else {
                        builder.mark_unused();
                    }
                }
            }
        });
        builder.into_inner().done()
    }
}

#[derive(Debug, Clone, Copy, SoAble)]
pub(crate) struct ShiftData {
    /// Starting from this index...
    pub(crate) index: u32,
    /// ...shift reference values down by this much.
    pub(crate) shift: u32,
}

pub(crate) struct CompactionListBuilder {
    shifts: SoAVec<ShiftData>,
    current_index: u32,
    current_shift: u32,
    current_used: bool,
}

impl CompactionListBuilder {
    fn with_bits_length(bits_length: u32) -> Self {
        // Note: the maximum possible size of the indexes and shifts vectors is
        // half the bits length; this happens if every other bit is 1.
        // It's unlikely that we find this case, so we halve that for a fairly
        // conservative guess.
        let capacity = bits_length / 4;
        Self {
            shifts: SoAVec::with_capacity(capacity).unwrap(),
            current_index: 0,
            current_shift: 0,
            current_used: true,
        }
    }

    /// Add current index to indexes with the current shift.
    fn add_current_index(&mut self) {
        let index = self.current_index;
        let shift = self.current_shift;
        assert!(
            self.shifts.is_empty()
                || *self.shifts.get(self.shifts.len() - 1).unwrap().index < index
                    && *self.shifts.get(self.shifts.len() - 1).unwrap().shift < shift
        );
        self.shifts.push(ShiftData { index, shift }).unwrap();
    }

    fn mark_used(&mut self) {
        if !self.current_used {
            self.add_current_index();
            self.current_used = true;
        }
        self.current_index += 1;
    }

    fn mark_unused(&mut self) {
        if self.current_used {
            self.current_used = false;
        }
        self.current_shift += 1;
        self.current_index += 1;
    }

    fn done(mut self) -> CompactionList {
        // When building compactions is done, it's possible that the end of the
        // data contains dropped values; we must add an "end-cap" where the
        // start index is equal to the length of the data vector (and thus
        // unreachable; no reference can point to the end of the vector) and
        // where the shift value is such that it overwrites the dropped values.
        if !self.current_used {
            self.add_current_index();
        }
        CompactionList::build(self.shifts)
    }
}

impl Default for CompactionListBuilder {
    fn default() -> Self {
        Self {
            shifts: SoAVec::with_capacity(16).unwrap(),
            current_index: 0,
            current_shift: 0,
            current_used: true,
        }
    }
}

pub(crate) struct CompactionLists {
    #[cfg(feature = "array-buffer")]
    pub(crate) array_buffers: CompactionList,
    pub(crate) arrays: CompactionList,
    pub(crate) array_iterators: CompactionList,
    pub(crate) async_generators: CompactionList,
    pub(crate) await_reactions: CompactionList,
    pub(crate) bigints: CompactionList,
    pub(crate) bound_functions: CompactionList,
    pub(crate) builtin_constructors: CompactionList,
    pub(crate) builtin_functions: CompactionList,
    pub(crate) caches: CompactionList,
    #[cfg(feature = "array-buffer")]
    pub(crate) data_views: CompactionList,
    #[cfg(feature = "date")]
    pub(crate) dates: CompactionList,
    #[cfg(feature = "temporal")]
    pub(crate) instants: CompactionList,
    #[cfg(feature = "temporal")]
    pub(crate) durations: CompactionList,
    #[cfg(feature = "temporal")]
    pub(crate) plain_times: CompactionList,
    pub(crate) declarative_environments: CompactionList,
    pub(crate) e_2_1: CompactionList,
    pub(crate) e_2_2: CompactionList,
    pub(crate) e_2_3: CompactionList,
    pub(crate) e_2_4: CompactionList,
    pub(crate) e_2_6: CompactionList,
    pub(crate) e_2_8: CompactionList,
    pub(crate) e_2_10: CompactionList,
    pub(crate) e_2_12: CompactionList,
    pub(crate) e_2_16: CompactionList,
    pub(crate) e_2_24: CompactionList,
    pub(crate) e_2_32: CompactionList,
    pub(crate) k_2_1: CompactionList,
    pub(crate) k_2_2: CompactionList,
    pub(crate) k_2_3: CompactionList,
    pub(crate) k_2_4: CompactionList,
    pub(crate) k_2_6: CompactionList,
    pub(crate) k_2_8: CompactionList,
    pub(crate) k_2_10: CompactionList,
    pub(crate) k_2_12: CompactionList,
    pub(crate) k_2_16: CompactionList,
    pub(crate) k_2_24: CompactionList,
    pub(crate) k_2_32: CompactionList,
    pub(crate) ecmascript_functions: CompactionList,
    pub(crate) embedder_objects: CompactionList,
    pub(crate) source_codes: CompactionList,
    pub(crate) source_text_module_records: CompactionList,
    pub(crate) errors: CompactionList,
    pub(crate) executables: CompactionList,
    pub(crate) finalization_registrys: CompactionList,
    pub(crate) function_environments: CompactionList,
    pub(crate) generators: CompactionList,
    pub(crate) global_environments: CompactionList,
    pub(crate) maps: CompactionList,
    pub(crate) map_iterators: CompactionList,
    pub(crate) modules: CompactionList,
    pub(crate) module_environments: CompactionList,
    pub(crate) module_request_records: CompactionList,
    pub(crate) numbers: CompactionList,
    pub(crate) object_environments: CompactionList,
    pub(crate) object_shapes: CompactionList,
    pub(crate) objects: CompactionList,
    pub(crate) primitive_objects: CompactionList,
    pub(crate) private_environments: CompactionList,
    pub(crate) promise_reaction_records: CompactionList,
    pub(crate) promise_resolving_functions: CompactionList,
    pub(crate) promise_finally_functions: CompactionList,
    pub(crate) promises: CompactionList,
    pub(crate) promise_group_records: CompactionList,
    pub(crate) proxies: CompactionList,
    pub(crate) realms: CompactionList,
    #[cfg(feature = "regexp")]
    pub(crate) regexps: CompactionList,
    #[cfg(feature = "regexp")]
    pub(crate) regexp_string_iterators: CompactionList,
    pub(crate) scripts: CompactionList,
    #[cfg(feature = "set")]
    pub(crate) sets: CompactionList,
    #[cfg(feature = "set")]
    pub(crate) set_iterators: CompactionList,
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) shared_array_buffers: CompactionList,
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) shared_data_views: CompactionList,
    #[cfg(feature = "shared-array-buffer")]
    pub(crate) shared_typed_arrays: CompactionList,
    pub(crate) string_iterators: CompactionList,
    pub(crate) strings: CompactionList,
    pub(crate) symbols: CompactionList,
    #[cfg(feature = "array-buffer")]
    pub(crate) typed_arrays: CompactionList,
    #[cfg(feature = "weak-refs")]
    pub(crate) weak_maps: CompactionList,
    #[cfg(feature = "weak-refs")]
    pub(crate) weak_refs: CompactionList,
    #[cfg(feature = "weak-refs")]
    pub(crate) weak_sets: CompactionList,
    pub(crate) plain_times: (),
}

impl CompactionLists {
    pub(crate) fn create_from_bits(bits: &HeapBits) -> Self {
        // TODO: Instead of each list creating its own Vecs, this
        // could instead be a singular Vec segmented into slices.
        // The total number of vector items needed for compactions can
        // be estimated from bits.len() / 2 - bits_marked. If only one bit
        // is marked then two compaction parts can exist. If only one bit
        // is unmarked then two compaction parts can exist. If exactly half
        // of bits are marked or unmarked then bits.len() / 2 number of compaction
        // areas can exist. We can use this mathematical bound to estimate a good
        // vector allocation.
        Self {
            modules: CompactionList::from_mark_bits(&bits.modules, &bits.bits),
            scripts: CompactionList::from_mark_bits(&bits.scripts, &bits.bits),
            realms: CompactionList::from_mark_bits(&bits.realms, &bits.bits),
            declarative_environments: CompactionList::from_mark_bits(
                &bits.declarative_environments,
                &bits.bits,
            ),
            function_environments: CompactionList::from_mark_bits(
                &bits.function_environments,
                &bits.bits,
            ),
            global_environments: CompactionList::from_mark_bits(
                &bits.global_environments,
                &bits.bits,
            ),
            object_environments: CompactionList::from_mark_bits(
                &bits.object_environments,
                &bits.bits,
            ),
            e_2_1: CompactionList::from_mark_bits(&bits.e_2_1, &bits.bits),
            e_2_2: CompactionList::from_mark_bits(&bits.e_2_2, &bits.bits),
            e_2_3: CompactionList::from_mark_bits(&bits.e_2_3, &bits.bits),
            e_2_4: CompactionList::from_mark_bits(&bits.e_2_4, &bits.bits),
            e_2_6: CompactionList::from_mark_bits(&bits.e_2_6, &bits.bits),
            e_2_8: CompactionList::from_mark_bits(&bits.e_2_8, &bits.bits),
            e_2_10: CompactionList::from_mark_bits(&bits.e_2_10, &bits.bits),
            e_2_12: CompactionList::from_mark_bits(&bits.e_2_12, &bits.bits),
            e_2_16: CompactionList::from_mark_bits(&bits.e_2_16, &bits.bits),
            e_2_24: CompactionList::from_mark_bits(&bits.e_2_24, &bits.bits),
            e_2_32: CompactionList::from_mark_bits(&bits.e_2_32, &bits.bits),
            k_2_1: CompactionList::from_mark_bits(&bits.k_2_1, &bits.bits),
            k_2_2: CompactionList::from_mark_bits(&bits.k_2_2, &bits.bits),
            k_2_3: CompactionList::from_mark_bits(&bits.k_2_3, &bits.bits),
            k_2_4: CompactionList::from_mark_bits(&bits.k_2_4, &bits.bits),
            k_2_6: CompactionList::from_mark_bits(&bits.k_2_6, &bits.bits),
            k_2_8: CompactionList::from_mark_bits(&bits.k_2_8, &bits.bits),
            k_2_10: CompactionList::from_mark_bits(&bits.k_2_10, &bits.bits),
            k_2_12: CompactionList::from_mark_bits(&bits.k_2_12, &bits.bits),
            k_2_16: CompactionList::from_mark_bits(&bits.k_2_16, &bits.bits),
            k_2_24: CompactionList::from_mark_bits(&bits.k_2_24, &bits.bits),
            k_2_32: CompactionList::from_mark_bits(&bits.k_2_32, &bits.bits),
            arrays: CompactionList::from_mark_bits(&bits.arrays, &bits.bits),
            #[cfg(feature = "array-buffer")]
            array_buffers: CompactionList::from_mark_bits(&bits.array_buffers, &bits.bits),
            array_iterators: CompactionList::from_mark_bits(&bits.array_iterators, &bits.bits),
            async_generators: CompactionList::from_mark_bits(&bits.async_generators, &bits.bits),
            await_reactions: CompactionList::from_mark_bits(&bits.await_reactions, &bits.bits),
            bigints: CompactionList::from_mark_bits(&bits.bigints, &bits.bits),
            bound_functions: CompactionList::from_mark_bits(&bits.bound_functions, &bits.bits),
            builtin_constructors: CompactionList::from_mark_bits(
                &bits.builtin_constructors,
                &bits.bits,
            ),
            builtin_functions: CompactionList::from_mark_bits(&bits.builtin_functions, &bits.bits),
            caches: CompactionList::from_mark_bits(&bits.caches, &bits.bits),
            ecmascript_functions: CompactionList::from_mark_bits(
                &bits.ecmascript_functions,
                &bits.bits,
            ),
            embedder_objects: CompactionList::from_mark_bits(&bits.embedder_objects, &bits.bits),
            generators: CompactionList::from_mark_bits(&bits.generators, &bits.bits),
            source_codes: CompactionList::from_mark_bits(&bits.source_codes, &bits.bits),
            #[cfg(feature = "date")]
            dates: CompactionList::from_mark_bits(&bits.dates, &bits.bits),
            #[cfg(feature = "temporal")]
            instants: CompactionList::from_mark_bits(&bits.instants, &bits.bits),
            #[cfg(feature = "temporal")]
            durations: CompactionList::from_mark_bits(&bits.durations, &bits.bits),
            #[cfg(feature = "temporal")]
            plain_times: CompactionList::from_mark_bits(&bits.plain_times, &bits.bits),
            errors: CompactionList::from_mark_bits(&bits.errors, &bits.bits),
            executables: CompactionList::from_mark_bits(&bits.executables, &bits.bits),
            maps: CompactionList::from_mark_bits(&bits.maps, &bits.bits),
            map_iterators: CompactionList::from_mark_bits(&bits.map_iterators, &bits.bits),
            module_environments: CompactionList::from_mark_bits(
                &bits.module_environments,
                &bits.bits,
            ),
            module_request_records: CompactionList::from_mark_bits(
                &bits.module_request_records,
                &bits.bits,
            ),
            numbers: CompactionList::from_mark_bits(&bits.numbers, &bits.bits),
            object_shapes: CompactionList::from_mark_bits(&bits.object_shapes, &bits.bits),
            objects: CompactionList::from_mark_bits(&bits.objects, &bits.bits),
            primitive_objects: CompactionList::from_mark_bits(&bits.primitive_objects, &bits.bits),
            private_environments: CompactionList::from_mark_bits(
                &bits.private_environments,
                &bits.bits,
            ),
            promise_reaction_records: CompactionList::from_mark_bits(
                &bits.promise_reaction_records,
                &bits.bits,
            ),
            promise_resolving_functions: CompactionList::from_mark_bits(
                &bits.promise_resolving_functions,
                &bits.bits,
            ),
            promise_finally_functions: CompactionList::from_mark_bits(
                &bits.promise_finally_functions,
                &bits.bits,
            ),
            promises: CompactionList::from_mark_bits(&bits.promises, &bits.bits),
            promise_group_records: CompactionList::from_mark_bits(
                &bits.promise_group_records,
                &bits.bits,
            ),
            #[cfg(feature = "regexp")]
            regexps: CompactionList::from_mark_bits(&bits.regexps, &bits.bits),
            #[cfg(feature = "regexp")]
            regexp_string_iterators: CompactionList::from_mark_bits(
                &bits.regexp_string_iterators,
                &bits.bits,
            ),
            #[cfg(feature = "set")]
            sets: CompactionList::from_mark_bits(&bits.sets, &bits.bits),
            #[cfg(feature = "set")]
            set_iterators: CompactionList::from_mark_bits(&bits.set_iterators, &bits.bits),
            string_iterators: CompactionList::from_mark_bits(&bits.string_iterators, &bits.bits),
            strings: CompactionList::from_mark_bits(&bits.strings, &bits.bits),
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers: CompactionList::from_mark_bits(
                &bits.shared_array_buffers,
                &bits.bits,
            ),
            #[cfg(feature = "shared-array-buffer")]
            shared_data_views: CompactionList::from_mark_bits(&bits.shared_data_views, &bits.bits),
            #[cfg(feature = "shared-array-buffer")]
            shared_typed_arrays: CompactionList::from_mark_bits(
                &bits.shared_typed_arrays,
                &bits.bits,
            ),
            source_text_module_records: CompactionList::from_mark_bits(
                &bits.source_text_module_records,
                &bits.bits,
            ),
            symbols: CompactionList::from_mark_bits(&bits.symbols, &bits.bits),
            #[cfg(feature = "array-buffer")]
            data_views: CompactionList::from_mark_bits(&bits.data_views, &bits.bits),
            finalization_registrys: CompactionList::from_mark_bits(
                &bits.finalization_registrys,
                &bits.bits,
            ),
            proxies: CompactionList::from_mark_bits(&bits.proxies, &bits.bits),
            #[cfg(feature = "weak-refs")]
            weak_maps: CompactionList::from_mark_bits(&bits.weak_maps, &bits.bits),
            #[cfg(feature = "weak-refs")]
            weak_refs: CompactionList::from_mark_bits(&bits.weak_refs, &bits.bits),
            #[cfg(feature = "weak-refs")]
            weak_sets: CompactionList::from_mark_bits(&bits.weak_sets, &bits.bits),
            #[cfg(feature = "array-buffer")]
            typed_arrays: CompactionList::from_mark_bits(&bits.typed_arrays, &bits.bits),
        }
    }
}

/// Trait for sweeping live heap data and references.
pub(crate) trait HeapMarkAndSweep {
    /// Mark all Heap references contained in self
    ///
    /// To mark a HeapIndex, push it into the relevant queue in
    /// WorkQueues.
    #[allow(unused_variables)]
    fn mark_values(&self, queues: &mut WorkQueues);

    /// Handle potential sweep of and update Heap references in self
    ///
    /// Sweeping of self is needed for Heap vectors: They must compact
    /// according to the `compactions` parameter. Additionally, any
    /// Heap references in self must be updated according to the
    /// compactions list.
    #[allow(unused_variables)]
    fn sweep_values(&mut self, compactions: &CompactionLists);
}

impl<T> HeapMarkAndSweep for &T
where
    T: HeapMarkAndSweep,
{
    #[inline(always)]
    fn mark_values(&self, queues: &mut WorkQueues) {
        T::mark_values(self, queues);
    }

    #[inline(always)]
    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        unreachable!();
    }
}

impl<T> HeapMarkAndSweep for &mut T
where
    T: HeapMarkAndSweep,
{
    #[inline(always)]
    fn mark_values(&self, queues: &mut WorkQueues) {
        T::mark_values(self, queues);
    }

    #[inline(always)]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        T::sweep_values(self, compactions);
    }
}

impl<T> HeapMarkAndSweep for Option<T>
where
    T: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        if let Some(content) = self {
            content.mark_values(queues);
        }
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        if let Some(content) = self {
            content.sweep_values(compactions);
        }
    }
}

impl<const N: usize, T> HeapMarkAndSweep for [T; N]
where
    T: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        for elem in self {
            elem.mark_values(queues);
        }
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        for elem in self {
            elem.sweep_values(compactions);
        }
    }
}

macro_rules! trivially_sweepable {
    ($self:ty) => {
        impl crate::heap::heap_bits::HeapMarkAndSweep for $self {
            #[inline]
            fn mark_values(&self, _: &mut crate::heap::heap_bits::WorkQueues) {}

            #[inline]
            fn sweep_values(&mut self, _: &crate::heap::heap_bits::CompactionLists) {}
        }
    };
}

trivially_sweepable!(());
trivially_sweepable!(bool);
trivially_sweepable!(i8);
trivially_sweepable!(u8);
trivially_sweepable!(i16);
trivially_sweepable!(u16);
trivially_sweepable!(i32);
trivially_sweepable!(u32);
trivially_sweepable!(i64);
trivially_sweepable!(u64);
trivially_sweepable!(isize);
trivially_sweepable!(usize);
#[cfg(feature = "proposal-float16array")]
trivially_sweepable!(f16);
trivially_sweepable!(f32);
trivially_sweepable!(f64);

impl<T> HeapMarkAndSweep for (T,)
where
    T: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.0.mark_values(queues);
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.0.sweep_values(compactions);
    }
}

impl<T, U> HeapMarkAndSweep for (T, U)
where
    T: HeapMarkAndSweep,
    U: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        let (t, u) = self;
        t.mark_values(queues);
        u.mark_values(queues);
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let (t, u) = self;
        t.sweep_values(compactions);
        u.sweep_values(compactions);
    }
}

impl<T, U, V> HeapMarkAndSweep for (T, U, V)
where
    T: HeapMarkAndSweep,
    U: HeapMarkAndSweep,
    V: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        let (t, u, v) = self;
        t.mark_values(queues);
        u.mark_values(queues);
        v.mark_values(queues);
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let (t, u, v) = self;
        t.sweep_values(compactions);
        u.sweep_values(compactions);
        v.sweep_values(compactions);
    }
}

impl<T, U, V, W> HeapMarkAndSweep for (T, U, V, W)
where
    T: HeapMarkAndSweep,
    U: HeapMarkAndSweep,
    V: HeapMarkAndSweep,
    W: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        let (t, u, v, w) = self;
        t.mark_values(queues);
        u.mark_values(queues);
        v.mark_values(queues);
        w.mark_values(queues);
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let (t, u, v, w) = self;
        t.sweep_values(compactions);
        u.sweep_values(compactions);
        v.sweep_values(compactions);
        w.sweep_values(compactions);
    }
}

impl<T, U, V, W, X> HeapMarkAndSweep for (T, U, V, W, X)
where
    T: HeapMarkAndSweep,
    U: HeapMarkAndSweep,
    V: HeapMarkAndSweep,
    W: HeapMarkAndSweep,
    X: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        let (t, u, v, w, x) = self;
        t.mark_values(queues);
        u.mark_values(queues);
        v.mark_values(queues);
        w.mark_values(queues);
        x.mark_values(queues);
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let (t, u, v, w, x) = self;
        t.sweep_values(compactions);
        u.sweep_values(compactions);
        v.sweep_values(compactions);
        w.sweep_values(compactions);
        x.sweep_values(compactions);
    }
}

impl<T, U, V, W, X, Y> HeapMarkAndSweep for (T, U, V, W, X, Y)
where
    T: HeapMarkAndSweep,
    U: HeapMarkAndSweep,
    V: HeapMarkAndSweep,
    W: HeapMarkAndSweep,
    X: HeapMarkAndSweep,
    Y: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        let (t, u, v, w, x, y) = self;
        t.mark_values(queues);
        u.mark_values(queues);
        v.mark_values(queues);
        w.mark_values(queues);
        x.mark_values(queues);
        y.mark_values(queues);
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let (t, u, v, w, x, y) = self;
        t.sweep_values(compactions);
        u.sweep_values(compactions);
        v.sweep_values(compactions);
        w.sweep_values(compactions);
        x.sweep_values(compactions);
        y.sweep_values(compactions);
    }
}

impl<T, U, V, W, X, Y, Z> HeapMarkAndSweep for (T, U, V, W, X, Y, Z)
where
    T: HeapMarkAndSweep,
    U: HeapMarkAndSweep,
    V: HeapMarkAndSweep,
    W: HeapMarkAndSweep,
    X: HeapMarkAndSweep,
    Y: HeapMarkAndSweep,
    Z: HeapMarkAndSweep,
{
    #[inline]
    fn mark_values(&self, queues: &mut WorkQueues) {
        let (t, u, v, w, x, y, z) = self;
        t.mark_values(queues);
        u.mark_values(queues);
        v.mark_values(queues);
        w.mark_values(queues);
        x.mark_values(queues);
        y.mark_values(queues);
        z.mark_values(queues);
    }

    #[inline]
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let (t, u, v, w, x, y, z) = self;
        t.sweep_values(compactions);
        u.sweep_values(compactions);
        v.sweep_values(compactions);
        w.sweep_values(compactions);
        x.sweep_values(compactions);
        y.sweep_values(compactions);
        z.sweep_values(compactions);
    }
}

impl<T> HeapMarkAndSweep for Box<T>
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.as_ref().mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.as_mut().sweep_values(compactions)
    }
}

impl<T> HeapMarkAndSweep for Box<[T]>
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.iter().for_each(|entry| entry.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.iter_mut()
            .for_each(|entry| entry.sweep_values(compactions))
    }
}

impl<T> HeapMarkAndSweep for &[T]
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.iter().for_each(|entry| entry.mark_values(queues));
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        const {
            panic!("Cannot sweep immutable slice");
        }
    }
}

impl<T> HeapMarkAndSweep for &mut [T]
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.iter().for_each(|entry| entry.mark_values(queues))
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.iter_mut()
            .for_each(|entry| entry.sweep_values(compactions))
    }
}

impl<T> HeapMarkAndSweep for Vec<T>
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.as_slice().mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.as_mut_slice().sweep_values(compactions);
    }
}

impl<K: HeapMarkAndSweep + core::fmt::Debug + Copy + Hash + Eq + Ord, V: HeapMarkAndSweep>
    HeapMarkAndSweep for AHashMap<K, V>
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        for (key, value) in self.iter() {
            key.mark_values(queues);
            value.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let mut replacements = Vec::new();
        // Sweep all values, while also sweeping keys and making note of all
        // changes in them: Those need to be updated in a separate loop.
        for (key, value) in self.iter_mut() {
            value.sweep_values(compactions);
            let old_key = *key;
            let mut new_key = *key;
            new_key.sweep_values(compactions);
            if old_key != new_key {
                replacements.push((old_key, new_key));
            }
        }
        // Note: Replacement keys are in indeterminate order, we need to sort
        // them so that "cascading" replacements are applied in the correct
        // order.
        replacements.sort();
        for (old_key, new_key) in replacements.into_iter() {
            let binding = self.remove(&old_key).unwrap();
            let did_insert = self.insert(new_key, binding).is_none();
            assert!(did_insert, "Failed to insert key {new_key:#?}");
        }
    }
}

// HeapMarkAndSweep implementation for hash maps from strong keys to weak
// values. If the weak value drops, the entire entry is dropped.
//
// Note that this is not an emphemeron map (weak key if strongly held holds the
// value strongly as well).
impl<K: HeapMarkAndSweep + core::fmt::Debug + Copy + Hash + Eq + Ord, V: HeapSweepWeakReference>
    HeapMarkAndSweep for AHashMap<K, WeakReference<V>>
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        // Note: we do not mark values as they are held weakly.
        for (key, _) in self.iter() {
            key.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let mut replacements = Vec::new();
        // Sweep all values, while also sweeping keys and making note of all
        // changes in them: Those need to be updated in a separate loop.
        for (key, value) in self.iter_mut() {
            let old_key = *key;
            let Some(new_value) = value.sweep_weak_reference(compactions) else {
                // Value was dropped: remove the old key.
                replacements.push((old_key, None));
                continue;
            };
            *value = new_value;
            let mut new_key = *key;
            new_key.sweep_values(compactions);
            if old_key != new_key {
                replacements.push((old_key, Some(new_key)));
            }
        }
        // Note: Replacement keys are in indeterminate order, we need to sort
        // them so that "cascading" replacements are applied in the correct
        // order.
        replacements.sort();
        for (old_key, new_key) in replacements.into_iter() {
            let value = self.remove(&old_key).unwrap();
            if let Some(new_key) = new_key {
                let did_insert = self.insert(new_key, value).is_none();
                assert!(did_insert, "Failed to insert key {new_key:#?}");
            }
        }
    }
}

pub(crate) fn mark_descriptors(
    descriptors: &AHashMap<u32, ElementDescriptor<'static>>,
    queues: &mut WorkQueues,
) {
    for descriptor in descriptors.values() {
        descriptor.mark_values(queues);
    }
}

pub(crate) fn sweep_heap_vector_values<T: HeapMarkAndSweep>(
    vec: &mut Vec<T>,
    compactions: &CompactionLists,
    range: &BitRange,
    bits: &[AtomicBits],
) {
    assert_eq!(vec.len(), range.len());
    let mut iter = range.iter(bits);
    vec.retain_mut(|item| {
        let do_retain = iter.next().unwrap();
        if do_retain {
            item.sweep_values(compactions);
            true
        } else {
            false
        }
    });
}

pub(crate) fn sweep_heap_soa_vector_values<T: SoAble>(
    vec: &mut SoAVec<T>,
    compactions: &CompactionLists,
    range: &BitRange,
    bits: &[AtomicBits],
) where
    for<'a> T::Mut<'a>: HeapMarkAndSweep,
{
    assert_eq!(vec.len() as usize, range.len());
    let mut iter = range.iter(bits);
    vec.retain_mut(|mut item| {
        let do_retain = iter.next().unwrap();
        if do_retain {
            item.sweep_values(compactions);
            true
        } else {
            false
        }
    });
}

pub(crate) fn sweep_heap_elements_vector_descriptors(
    descriptors: &mut AHashMap<ElementIndex<'static>, AHashMap<u32, ElementDescriptor<'static>>>,
    compactions: &CompactionLists,
    self_compactions: &CompactionList,
    range: &BitRange,
    bits: &[AtomicBits],
) {
    let mut keys_to_remove = Vec::with_capacity(range.len() / 4);
    let mut keys_to_reassign = Vec::with_capacity(range.len() / 4);
    for (key, descriptor) in descriptors.iter_mut() {
        let old_key = *key;
        if !range.get_bit(old_key.get_index(), bits) {
            keys_to_remove.push(old_key);
        } else {
            for descriptor in descriptor.values_mut() {
                descriptor.sweep_values(compactions);
            }
            let mut new_key = old_key;
            self_compactions.shift_index(&mut new_key);
            if new_key != old_key {
                keys_to_reassign.push((old_key, new_key));
            }
        }
    }
    keys_to_remove.sort();
    keys_to_reassign.sort();
    for old_key in keys_to_remove.iter() {
        descriptors.remove(old_key);
    }
    for (old_key, new_key) in keys_to_reassign {
        // SAFETY: The old key came from iterating descriptors, and the same
        // key cannot appear in both keys to remove and keys to reassign. Thus
        // the key must necessarily exist in the descriptors hash map.
        let descriptor = unsafe { descriptors.remove(&old_key).unwrap_unchecked() };
        descriptors.insert(new_key, descriptor);
    }
}

/// Weakly held garbage-collectable reference value.
///
/// This is a thin wrapper struct over the reference value, intended only for
/// enabling automatic weak reference sweeping.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct WeakReference<T: HeapSweepWeakReference>(pub(crate) T);

impl<T: Sized + Copy + HeapSweepWeakReference> HeapSweepWeakReference for WeakReference<T> {
    #[inline(always)]
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        self.0.sweep_weak_reference(compactions).map(Self)
    }
}

/// Trait for sweeping weak references.
pub(crate) trait HeapSweepWeakReference: Sized + Copy {
    /// Perform sweep on a weakly held reference; if the reference target is
    /// still alive then the value is mutated and true is returned, otherwise
    /// false is returned.
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self>;
}

#[cfg(feature = "array-buffer")]
pub(crate) fn sweep_side_table_values<K, V>(
    side_table: &mut AHashMap<K, V>,
    compactions: &CompactionLists,
) where
    K: HeapSweepWeakReference + Hash + Eq,
{
    *side_table = side_table
        .drain()
        .filter_map(|(k, v)| k.sweep_weak_reference(compactions).map(|k| (k, v)))
        .collect();
}

#[cfg(feature = "weak-refs")]
pub(crate) fn sweep_side_set<K>(side_table: &mut AHashSet<K>, compactions: &CompactionLists)
where
    K: HeapSweepWeakReference + Hash + Eq,
{
    *side_table = side_table
        .drain()
        .filter_map(|k| k.sweep_weak_reference(compactions))
        .collect();
}

pub(crate) fn sweep_lookup_table<T>(lookup_table: &mut HashTable<T>, compactions: &CompactionLists)
where
    T: HeapSweepWeakReference,
{
    lookup_table.retain(|entry| {
        if let Some(updated_value) = entry.sweep_weak_reference(compactions) {
            *entry = updated_value;
            true
        } else {
            false
        }
    });
}
