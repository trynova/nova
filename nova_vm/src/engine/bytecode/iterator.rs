// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::VecDeque;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{IteratorRecord, get_iterator_from_method},
            operations_on_objects::{call_function, get, get_method, throw_not_callable},
            type_conversion::to_boolean,
        },
        builtins::{Array, ScopedArgumentsList},
        execution::{Agent, JsResult, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, IntoValue, Object, PropertyKey, PropertyKeySet,
            Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

#[derive(Debug)]
pub(super) enum VmIterator<'a> {
    /// Special type for iterators that do not have a callable next method.
    InvalidIterator,
    ObjectProperties(Box<ObjectPropertiesIterator<'a>>),
    ArrayValues(ArrayValuesIterator<'a>),
    GenericIterator(IteratorRecord<'a>),
    SliceIterator(ScopedArgumentsList<'a>),
    EmptySliceIterator,
}

impl VmIterator<'_> {
    /// ### [7.4.8 IteratorStepValue ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstepvalue)
    ///
    /// While not exactly equal to the IteratorStepValue method in usage, this
    /// function implements much the same intent. It does the IteratorNext
    /// step, followed by a completion check, and finally extracts the value
    /// if the iterator did not complete yet.
    pub(super) fn step_value<'gc>(
        &mut self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        match self {
            VmIterator::InvalidIterator => Err(throw_not_callable(agent, gc.into_nogc()).unbind()),
            VmIterator::ObjectProperties(iter) => {
                let result = iter.next(agent, gc.reborrow()).unbind()?.bind(gc.nogc());
                if let Some(result) = result {
                    let result = result.unbind();
                    let gc = gc.into_nogc();
                    let result = result.bind(gc);
                    Ok(Some(match result {
                        PropertyKey::Integer(int) => {
                            Value::from_string(agent, int.into_i64().to_string(), gc)
                        }
                        PropertyKey::SmallString(data) => Value::SmallString(data),
                        PropertyKey::String(data) => Value::String(data),
                        _ => unreachable!(),
                    }))
                } else {
                    Ok(None)
                }
            }
            VmIterator::ArrayValues(iter) => iter.next(agent, gc),
            VmIterator::GenericIterator(iter) => {
                let next_method = iter.next_method.bind(gc.nogc());
                let iterator = iter.iterator.bind(gc.nogc());
                let scoped_next_method = next_method.scope(agent, gc.nogc());
                let scoped_iterator = iterator.scope(agent, gc.nogc());

                let result = call_function(
                    agent,
                    next_method.unbind(),
                    iterator.into_value().unbind(),
                    None,
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let Ok(result) = Object::try_from(result) else {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Iterator returned a non-object result",
                        gc.into_nogc(),
                    ));
                };
                let result = result.unbind().bind(gc.nogc());
                let scoped_result = result.scope(agent, gc.nogc());
                // 1. Return ToBoolean(? Get(iterResult, "done")).
                let done = get(
                    agent,
                    result.unbind(),
                    BUILTIN_STRING_MEMORY.done.into(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let done = to_boolean(agent, done);
                // SAFETY: Neither is shared.
                unsafe {
                    iter.iterator = scoped_iterator.take(agent);
                    iter.next_method = scoped_next_method.take(agent);
                }
                if done {
                    Ok(None)
                } else {
                    // 1. Return ? Get(iterResult, "value").
                    let value = get(
                        agent,
                        scoped_result.get(agent),
                        BUILTIN_STRING_MEMORY.value.into(),
                        gc,
                    )?;
                    Ok(Some(value))
                }
            }
            VmIterator::SliceIterator(slice_ref) => Ok(slice_ref.unshift(agent, gc.into_nogc())),
            VmIterator::EmptySliceIterator => Ok(None),
        }
    }

    pub(super) fn remaining_length_estimate(&self, agent: &mut Agent) -> Option<usize> {
        match self {
            VmIterator::InvalidIterator => None,
            VmIterator::ObjectProperties(iter) => {
                Some(iter.remaining_keys.as_ref().map_or(0, |k| k.len()))
            }
            VmIterator::ArrayValues(iter) => {
                Some(iter.array.len(agent).saturating_sub(iter.index) as usize)
            }
            VmIterator::GenericIterator(_) => None,
            VmIterator::SliceIterator(slice) => Some(slice.len(agent)),
            VmIterator::EmptySliceIterator => Some(0),
        }
    }

    /// ### [7.4.4 GetIterator ( obj, kind )](https://tc39.es/ecma262/#sec-getiterator)
    ///
    /// The abstract operation GetIterator takes arguments obj (an ECMAScript
    /// language value) and returns either a normal completion containing an
    /// Iterator Record or a throw completion.
    ///
    /// This method version performs the SYNC version of the method.
    pub(super) fn from_value<'a>(
        agent: &mut Agent,
        value: Value,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        let value = value.bind(gc.nogc());
        let scoped_value = value.scope(agent, gc.nogc());
        // a. Let method be ? GetMethod(obj, %Symbol.iterator%).
        let method = get_method(
            agent,
            value.unbind(),
            PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 3. If method is undefined, throw a TypeError exception.
        let Some(method) = method else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator method cannot be undefined",
                gc.into_nogc(),
            ));
        };

        // SAFETY: scoped_value is not shared.
        let value = unsafe { scoped_value.take(agent).bind(gc.nogc()) };
        // 4. Return ? GetIteratorFromMethod(obj, method).
        match value {
            // Optimisation: Check if we're using the Array values iterator on
            // an Array.
            Value::Array(array)
                if method
                    == agent
                        .current_realm_record()
                        .intrinsics()
                        .array_prototype_values()
                        .into() =>
            {
                Ok(VmIterator::ArrayValues(ArrayValuesIterator::new(
                    array.unbind(),
                )))
            }
            _ => {
                if let Some(js_iterator) =
                    get_iterator_from_method(agent, value.unbind(), method.unbind(), gc)?
                {
                    Ok(VmIterator::GenericIterator(js_iterator.unbind()))
                } else {
                    Ok(VmIterator::InvalidIterator)
                }
            }
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for VmIterator<'_> {
    type Of<'a> = VmIterator<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute(self) }
    }
}

#[derive(Debug)]
pub(super) struct ObjectPropertiesIterator<'a> {
    object: Object<'a>,
    visited_keys: PropertyKeySet<'a>,
    remaining_keys: Option<VecDeque<PropertyKey<'a>>>,
}

impl<'a> ObjectPropertiesIterator<'a> {
    pub(super) fn new(object: Object<'a>) -> Self {
        Self {
            object,
            visited_keys: Default::default(),
            remaining_keys: Default::default(),
        }
    }

    pub(super) fn next<'gc>(
        &mut self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyKey<'gc>>> {
        let mut object = self.object.scope(agent, gc.nogc());
        loop {
            if self.remaining_keys.is_none() {
                let keys = object
                    .get(agent)
                    .internal_own_property_keys(agent, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                let mut remaining_keys = VecDeque::with_capacity(keys.len());
                for key in keys {
                    if let PropertyKey::Symbol(_) = key {
                        continue;
                    } else {
                        remaining_keys.push_back(key.unbind());
                    }
                }
                self.remaining_keys = Some(remaining_keys);
            }
            while let Some(r) = self.remaining_keys.as_mut().unwrap().pop_front() {
                if self.visited_keys.contains(agent, r) {
                    continue;
                }
                let desc = object
                    .get(agent)
                    .internal_get_own_property(agent, r, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                if let Some(desc) = desc {
                    self.visited_keys.insert(agent, r);
                    if desc.enumerable == Some(true) {
                        return Ok(Some(r.unbind()));
                    }
                }
            }
            let prototype = object
                .get(agent)
                .internal_get_prototype_of(agent, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            if let Some(prototype) = prototype {
                self.remaining_keys = None;
                self.object = prototype.unbind();
                // SAFETY: object is not shared.
                unsafe { object.replace(agent, prototype.unbind()) };
            } else {
                return Ok(None);
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ArrayValuesIterator<'a> {
    array: Array<'a>,
    index: u32,
}

impl<'a> ArrayValuesIterator<'a> {
    pub(super) fn new(array: Array<'a>) -> Self {
        Self {
            array,
            // a. Let index be 0.
            index: 0,
        }
    }

    pub(super) fn next<'gc>(
        &mut self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        // b. Repeat,
        let array = self.array.bind(gc.nogc());
        // iv. Let indexNumber be 𝔽(index).
        let index = self.index;
        // 1. Let len be ? LengthOfArrayLike(array).
        let len = array.len(agent);
        // iii. If index ≥ len, return NormalCompletion(undefined).
        if index >= len {
            return Ok(None);
        }
        // viii. Set index to index + 1.
        self.index += 1;
        if let Some(element_value) = array.as_slice(agent)[index as usize] {
            // Fast path: If the element at this index has a Value, then it is
            // not an accessor nor a hole. Yield the result as-is.
            return Ok(Some(element_value.unbind()));
        }
        // 1. Let elementKey be ! ToString(indexNumber).
        // 2. Let elementValue be ? Get(array, elementKey).
        let scoped_array = array.scope(agent, gc.nogc());
        let element_value = get(agent, array.unbind(), index.into(), gc)?;
        // SAFETY: scoped_array is not shared.
        self.array = unsafe { scoped_array.take(agent) };
        // a. Let result be elementValue.
        // vii. Perform ? GeneratorYield(CreateIterResultObject(result, false)).
        Ok(Some(element_value))
    }
}

impl HeapMarkAndSweep for ObjectPropertiesIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object,
            visited_keys,
            remaining_keys,
        } = self;
        object.mark_values(queues);
        visited_keys.mark_values(queues);
        if let Some(remaining_keys) = remaining_keys {
            for key in remaining_keys.iter() {
                key.mark_values(queues);
            }
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object,
            visited_keys,
            remaining_keys,
        } = self;
        object.sweep_values(compactions);
        visited_keys.sweep_values(compactions);
        if let Some(remaining_keys) = remaining_keys {
            for key in remaining_keys.iter_mut() {
                key.sweep_values(compactions);
            }
        }
    }
}

impl HeapMarkAndSweep for ArrayValuesIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.array.mark_values(queues)
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.array.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for VmIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            VmIterator::InvalidIterator => {}
            VmIterator::ObjectProperties(iter) => iter.mark_values(queues),
            VmIterator::ArrayValues(iter) => iter.mark_values(queues),
            VmIterator::GenericIterator(iter) => iter.mark_values(queues),
            VmIterator::SliceIterator(_) => {}
            VmIterator::EmptySliceIterator => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            VmIterator::InvalidIterator => {}
            VmIterator::ObjectProperties(iter) => iter.sweep_values(compactions),
            VmIterator::ArrayValues(iter) => iter.sweep_values(compactions),
            VmIterator::GenericIterator(iter) => iter.sweep_values(compactions),
            VmIterator::SliceIterator(_) => {}
            VmIterator::EmptySliceIterator => {}
        }
    }
}
