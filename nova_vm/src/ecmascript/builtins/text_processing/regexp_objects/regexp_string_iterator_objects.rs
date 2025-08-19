// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [22.2.9 RegExp String Iterator Objects](https://tc39.es/ecma262/#sec-regexp-string-iterator-objects)
//!
//! A RegExp String Iterator is an object that represents a specific iteration
//! over some specific String instance object, matching against some specific
//! RegExp instance object. There is not a named constructor for RegExp String
//! Iterator objects. Instead, RegExp String Iterator objects are created by
//! calling certain methods of RegExp instance objects.

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, String, Value},
    },
    engine::{
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

/// ### [22.2.9.1 CreateRegExpStringIterator ( R, S, global, fullUnicode )](https://tc39.es/ecma262/#sec-createregexpstringiterator)
///
/// The abstract operation CreateRegExpStringIterator takes arguments R (an
/// Object), S (a String), global (a Boolean), and fullUnicode (a Boolean) and
/// returns an Object.
pub(crate) fn create_reg_exp_string_iterator<'gc>(
    agent: &mut Agent,
    r: Object,
    s: String,
    global: bool,
    full_unicode: bool,
    gc: NoGcScope<'gc, '_>,
) -> RegExpStringIterator<'gc> {
    // 1. Let iterator be OrdinaryObjectCreate(%RegExpStringIteratorPrototype%, « [[IteratingRegExp]], [[IteratedString]], [[Global]], [[Unicode]], [[Done]] »).
    // 7. Return iterator.
    agent.heap.create(RegExpStringIteratorRecord {
        backing_object: None,
        // 2. Set iterator.[[IteratingRegExp]] to R.
        iterating_regexp: r.bind(gc),
        // 3. Set iterator.[[IteratedString]] to S.
        iterated_string: s.bind(gc),
        // 4. Set iterator.[[Global]] to global.
        global,
        // 5. Set iterator.[[Unicode]] to fullUnicode.
        unicode: full_unicode,
        // 6. Set iterator.[[Done]] to false.
        done: false,
    })
}

/// [22.2.9 RegExp String Iterator Objects](https://tc39.es/ecma262/#sec-regexp-string-iterator-objects)
///
/// A RegExp String Iterator is an object that represents a specific iteration
/// over some specific String instance object, matching against some specific
/// RegExp instance object. There is not a named constructor for RegExp String
/// Iterator objects. Instead, RegExp String Iterator objects are created by
/// calling certain methods of RegExp instance objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegExpStringIterator<'a>(BaseIndex<'a, RegExpStringIteratorRecord<'static>>);
bindable_handle!(RegExpStringIterator);

impl<'r> RegExpStringIterator<'r> {
    /// \[\[IteratingRegExp]]
    pub(crate) fn iterating_regexp(self, agent: &Agent) -> Object<'r> {
        self.get(agent).iterating_regexp
    }

    /// \[\[S]]
    pub(crate) fn iterated_string(self, agent: &Agent) -> String<'r> {
        self.get(agent).iterated_string
    }

    /// \[\[Global]]
    pub(crate) fn global(self, agent: &Agent) -> bool {
        self.get(agent).global
    }

    /// \[\[Unicode]]
    pub(crate) fn unicode(self, agent: &Agent) -> bool {
        self.get(agent).unicode
    }

    /// \[\[Done]]
    pub(crate) fn done(self, agent: &Agent) -> bool {
        self.get(agent).done
    }

    /// Set \[\[Done]] to true.
    pub(crate) fn set_done(self, agent: &mut Agent) {
        self.get_mut(agent).done = true;
    }

    pub(crate) const fn _def() -> RegExpStringIterator<'static> {
        RegExpStringIterator(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    fn get(self, agent: &Agent) -> &RegExpStringIteratorRecord<'r> {
        agent
            .heap
            .regexp_string_iterators
            .get(self.get_index())
            .expect("Couldn't find RegExp String Iterator")
    }

    fn get_mut(self, agent: &mut Agent) -> &mut RegExpStringIteratorRecord<'static> {
        agent
            .heap
            .regexp_string_iterators
            .get_mut(self.get_index())
            .expect("Couldn't find RegExp String Iterator")
    }
}

impl<'a> From<RegExpStringIterator<'a>> for Object<'a> {
    fn from(value: RegExpStringIterator) -> Self {
        Self::RegExpStringIterator(value.unbind())
    }
}

impl<'a> From<RegExpStringIterator<'a>> for Value<'a> {
    fn from(value: RegExpStringIterator<'a>) -> Self {
        Self::RegExpStringIterator(value)
    }
}

impl Rootable for RegExpStringIterator<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::RegExpStringIterator(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::RegExpStringIterator(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> InternalSlots<'a> for RegExpStringIterator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::RegExpStringIterator;

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).backing_object.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        let prev = self.get_mut(agent).backing_object.replace(backing_object);
        debug_assert!(prev.is_none());
    }
}

impl<'a> InternalMethods<'a> for RegExpStringIterator<'a> {}

/// ### [22.2.9.3 Properties of RegExp String Iterator Instances](https://tc39.es/ecma262/#sec-properties-of-regexp-string-iterator-instances)
///
/// RegExp String Iterator instances are ordinary objects that inherit
/// properties from the %RegExpStringIteratorPrototype% intrinsic object.
/// RegExp String Iterator instances are initially created with the internal
/// slots listed in [Table 71](https://tc39.es/ecma262/#table-regexp-string-iterator-instance-slots).
#[derive(Debug)]
pub(crate) struct RegExpStringIteratorRecord<'a> {
    backing_object: Option<OrdinaryObject<'a>>,
    /// \[\[IteratingRegExp]]
    ///
    /// The regular expression used for iteration.
    /// `IsRegExp(\[\[IteratingRegExp]])` is initially true.
    iterating_regexp: Object<'a>,
    /// \[\[IteratedString]]
    ///
    /// The String value being iterated upon.
    iterated_string: String<'a>,
    /// \[\[Global]]
    ///
    /// Indicates whether the \[\[IteratingRegExp]] is global or not.
    global: bool,
    /// \[\[Unicode]]
    ///
    /// Indicates whether the \[\[IteratingRegExp]] is in Unicode mode or not.
    unicode: bool,
    /// \[\[Done]]
    ///
    /// Indicates whether the iteration is complete or not.
    done: bool,
}
bindable_handle!(RegExpStringIteratorRecord);

impl<'a> CreateHeapData<RegExpStringIteratorRecord<'a>, RegExpStringIterator<'a>> for Heap {
    fn create(&mut self, data: RegExpStringIteratorRecord<'a>) -> RegExpStringIterator<'a> {
        self.regexp_string_iterators.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<Option<RegExpStringIteratorRecord<'static>>>();
        RegExpStringIterator(BaseIndex::<RegExpStringIteratorRecord>::last_t(
            &self.regexp_string_iterators,
        ))
    }
}

impl HeapMarkAndSweep for RegExpStringIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.regexp_string_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.regexp_string_iterators.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for RegExpStringIterator<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .regexp_string_iterators
            .shift_weak_index(self.0)
            .map(Self)
    }
}

impl HeapMarkAndSweep for RegExpStringIteratorRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            backing_object,
            iterating_regexp,
            iterated_string,
            global: _,
            unicode: _,
            done: _,
        } = self;
        backing_object.mark_values(queues);
        iterating_regexp.mark_values(queues);
        iterated_string.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            backing_object,
            iterating_regexp,
            iterated_string,
            global: _,
            unicode: _,
            done: _,
        } = self;
        backing_object.sweep_values(compactions);
        iterating_regexp.sweep_values(compactions);
        iterated_string.sweep_values(compactions);
    }
}
