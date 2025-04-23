// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{marker::PhantomData, ptr::NonNull};

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
        context::{Bindable, GcScope, NoGcScope, ScopeToken},
        rootable::Scopable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

/// Marker struct for working with the active iterator of the currently active,
/// scoped VM.
pub struct ActiveIterator<'a> {
    scope: PhantomData<&'a ScopeToken>,
}

impl<'a> ActiveIterator<'a> {
    pub(super) fn new(agent: &Agent, _: NoGcScope<'_, 'a>) -> Self {
        let iterator = Self { scope: PhantomData };
        // Check that the iterator is found: this panics if the VM stack is
        // empty or the active VM's iterator stack is empty.
        let _ = iterator.get(agent);
        iterator
    }

    fn reborrow(&'a self) -> ActiveIterator<'a> {
        Self { scope: PhantomData }
    }

    pub(super) fn step_value<'gc>(
        &mut self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        match self.get(agent) {
            VmIteratorRecord::InvalidIterator => {
                Err(throw_not_callable(agent, gc.into_nogc()).unbind())
            }
            VmIteratorRecord::ObjectProperties(_) => {
                ObjectPropertiesIterator::new(self).step_value(agent, gc)
            }
            VmIteratorRecord::ArrayValues(_) => ArrayValuesIterator::new(self).next(agent, gc),
            VmIteratorRecord::GenericIterator(_) => GenericIterator::new(self).next(agent, gc),
            VmIteratorRecord::SliceIterator(slice_ref) => {
                Ok(slice_ref.unshift(agent, gc.into_nogc()))
            }
            VmIteratorRecord::EmptySliceIterator => Ok(None),
        }
    }

    pub(super) fn remaining_length_estimate(&mut self, agent: &mut Agent) -> Option<usize> {
        self.get(agent).remaining_length_estimate(agent)
    }

    pub(super) fn get<'agent>(&self, agent: &'agent Agent) -> &'agent VmIteratorRecord {
        // SAFETY: VM is held exclusively in an above call stack.
        unsafe { agent.vm_stack.last().expect("No VM found").as_ref() }.get_active_iterator()
    }

    fn get_mut<'agent>(
        &mut self,
        agent: &'agent mut Agent,
    ) -> &'agent mut VmIteratorRecord<'static> {
        // SAFETY: VM is held exclusively in an above call stack.
        unsafe { agent.vm_stack.last_mut().expect("No VM found").as_mut() }
            .get_active_iterator_mut()
    }
}

#[derive(Debug)]
pub enum VmIteratorRecord<'a> {
    /// Special type for iterators that do not have a callable next method.
    InvalidIterator,
    ObjectProperties(Box<ObjectPropertiesIteratorRecord<'a>>),
    ArrayValues(ArrayValuesIteratorRecord<'a>),
    GenericIterator(IteratorRecord<'a>),
    SliceIterator(ScopedArgumentsList<'a>),
    EmptySliceIterator,
}

impl VmIteratorRecord<'_> {
    pub(super) fn remaining_length_estimate(&self, agent: &Agent) -> Option<usize> {
        match self {
            VmIteratorRecord::InvalidIterator => None,
            VmIteratorRecord::ObjectProperties(iter) => {
                Some(iter.remaining_keys.as_ref().map_or(0, |k| k.len()))
            }
            VmIteratorRecord::ArrayValues(iter) => {
                Some(iter.array.len(agent).saturating_sub(iter.index) as usize)
            }
            VmIteratorRecord::GenericIterator(_) => None,
            VmIteratorRecord::SliceIterator(slice) => Some(slice.len(agent)),
            VmIteratorRecord::EmptySliceIterator => Some(0),
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
                Ok(VmIteratorRecord::ArrayValues(
                    ArrayValuesIteratorRecord::new(array.unbind()),
                ))
            }
            _ => {
                if let Some(js_iterator) =
                    get_iterator_from_method(agent, value.unbind(), method.unbind(), gc)?
                {
                    Ok(VmIteratorRecord::GenericIterator(js_iterator.unbind()))
                } else {
                    Ok(VmIteratorRecord::InvalidIterator)
                }
            }
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for VmIteratorRecord<'_> {
    type Of<'a> = VmIteratorRecord<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute(self) }
    }
}

struct ObjectPropertiesIterator<'a> {
    iter: ActiveIterator<'a>,
}

impl<'a> ObjectPropertiesIterator<'a> {
    fn new(iter: &'a ActiveIterator) -> Self {
        Self {
            iter: iter.reborrow(),
        }
    }

    /// ### [7.4.8 IteratorStepValue ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstepvalue)
    ///
    /// While not exactly equal to the IteratorStepValue method in usage, this
    /// function implements much the same intent. It does the IteratorNext
    /// step, followed by a completion check, and finally extracts the value
    /// if the iterator did not complete yet.
    fn step_value<'gc>(
        &mut self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        loop {
            if !self.object_is_visited(agent) {
                let keys = self
                    .object(agent, gc.nogc())
                    .unbind()
                    .internal_own_property_keys(agent, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                let mut remaining_keys = Vec::with_capacity(keys.len());
                for key in keys {
                    if let PropertyKey::Symbol(_) = key {
                        continue;
                    } else {
                        remaining_keys.push(key);
                    }
                }
                self.set_remaining_keys(agent, remaining_keys);
            }
            while let Some((object, next_key)) = self.next_remaining_key(agent, gc.nogc()) {
                let scoped_next_key = next_key.scope(agent, gc.nogc());
                let desc = object
                    .unbind()
                    .internal_get_own_property(agent, next_key.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // SAFETY: scoped_next_key is not shared.
                let next_key = unsafe { scoped_next_key.take(agent) }.bind(gc.nogc());
                if let Some(desc) = desc {
                    self.mark_key_visited(agent, next_key);
                    if desc.enumerable == Some(true) {
                        return Ok(Some(Self::convert_result(
                            agent,
                            next_key.unbind(),
                            gc.into_nogc(),
                        )));
                    }
                }
            }
            let prototype = self
                .object(agent, gc.nogc())
                .unbind()
                .internal_get_prototype_of(agent, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            if let Some(prototype) = prototype {
                self.set_object(agent, prototype);
            } else {
                return Ok(None);
            }
        }
    }

    fn convert_result<'gc>(
        agent: &mut Agent,
        result: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> Value<'gc> {
        match result {
            PropertyKey::Integer(int) => Value::from_string(agent, int.into_i64().to_string(), gc),
            PropertyKey::SmallString(data) => Value::SmallString(data),
            PropertyKey::String(data) => Value::String(data.bind(gc)),
            _ => unreachable!(),
        }
    }

    fn object<'gc>(&self, agent: &Agent, gc: NoGcScope<'gc, '_>) -> Object<'gc> {
        self.get(agent).object.bind(gc)
    }

    fn object_is_visited(&self, agent: &Agent) -> bool {
        self.get(agent).remaining_keys.is_some()
    }

    fn set_remaining_keys(&mut self, agent: &mut Agent, mut remaining_keys: Vec<PropertyKey>) {
        remaining_keys.reverse();
        self.get_mut(agent).remaining_keys = Some(remaining_keys.unbind());
    }

    fn set_object(&mut self, agent: &mut Agent, object: Object) {
        let iter = self.get_mut(agent);
        iter.object = object.unbind();
        iter.remaining_keys = None;
    }

    fn next_remaining_key<'gc>(
        &mut self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<(Object<'gc>, PropertyKey<'gc>)> {
        let mut iter = NonNull::from(self.get_mut(agent));
        loop {
            // SAFETY: The iterator does not invalidate or move here.
            let iter_mut = unsafe { iter.as_mut() };
            let next_key = iter_mut.remaining_keys.as_mut().unwrap().pop()?;
            // SAFETY: The iterator does not invalidate or move here.
            let iter_ref = unsafe { iter.as_ref() };
            if iter_ref.visited_keys.contains(agent, next_key) {
                // Skip visited keys.
                continue;
            }
            return Some((iter_ref.object.bind(gc), next_key.bind(gc)));
        }
    }

    fn mark_key_visited(&mut self, agent: &mut Agent, key: PropertyKey) {
        let mut iter = NonNull::from(self.get_mut(agent));
        // SAFETY: The iterator does not invalidate or move here.
        let iter = unsafe { iter.as_mut() };
        iter.visited_keys.insert(agent, key);
    }

    fn get<'agent>(&self, agent: &'agent Agent) -> &'agent ObjectPropertiesIteratorRecord {
        let VmIteratorRecord::ObjectProperties(iter) = self.iter.get(agent) else {
            unreachable!()
        };
        iter
    }

    fn get_mut<'agent>(
        &mut self,
        agent: &'agent mut Agent,
    ) -> &'agent mut ObjectPropertiesIteratorRecord<'static> {
        let VmIteratorRecord::ObjectProperties(iter) = self.iter.get_mut(agent) else {
            unreachable!()
        };
        iter
    }
}

#[derive(Debug)]
pub struct ObjectPropertiesIteratorRecord<'a> {
    object: Object<'a>,
    visited_keys: PropertyKeySet<'a>,
    remaining_keys: Option<Vec<PropertyKey<'a>>>,
}

impl<'a> ObjectPropertiesIteratorRecord<'a> {
    pub(super) fn new(object: Object<'a>) -> Self {
        Self {
            object,
            visited_keys: Default::default(),
            remaining_keys: Default::default(),
        }
    }
}

struct ArrayValuesIterator<'a> {
    iter: ActiveIterator<'a>,
}

impl<'a> ArrayValuesIterator<'a> {
    fn new(iter: &'a ActiveIterator) -> Self {
        Self {
            iter: iter.reborrow(),
        }
    }

    pub(super) fn next<'gc>(
        &mut self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        // b. Repeat,
        let Some((array, index)) = self.prep_step(agent, gc.nogc()) else {
            // The iterator is exhausted.
            return Ok(None);
        };
        if let Some(element_value) = array.as_slice(agent)[index as usize] {
            // Fast path: If the element at this index has a Value, then it is
            // not an accessor nor a hole. Yield the result as-is.
            return Ok(Some(element_value.unbind()));
        }
        // 1. Let elementKey be ! ToString(indexNumber).
        // 2. Let elementValue be ? Get(array, elementKey).
        let element_value = get(agent, array.unbind(), index.into(), gc)?;
        // a. Let result be elementValue.
        // vii. Perform ? GeneratorYield(CreateIterResultObject(result, false)).
        Ok(Some(element_value))
    }

    fn prep_step<'gc>(
        &mut self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<(Array<'gc>, u32)> {
        let VmIteratorRecord::ArrayValues(iter) = self.iter.get_mut(agent) else {
            unreachable!()
        };
        let array = iter.array.bind(gc);
        // iv. Let indexNumber be 𝔽(index).
        let index = iter.index;
        let mut iter = NonNull::from(iter);
        // 1. Let len be ? LengthOfArrayLike(array).
        let len = array.len(agent);
        // iii. If index ≥ len, return NormalCompletion(undefined).
        if index >= len {
            return None;
        }
        // viii. Set index to index + 1.
        // SAFETY: Getting the length of array does not mutate anything in
        // Agent, so we can safely re-reference the iterator.
        unsafe { iter.as_mut().index += 1 };
        Some((array, index))
    }
}

#[derive(Debug)]
pub struct ArrayValuesIteratorRecord<'a> {
    array: Array<'a>,
    index: u32,
}

impl<'a> ArrayValuesIteratorRecord<'a> {
    pub(super) fn new(array: Array<'a>) -> Self {
        Self {
            array,
            // a. Let index be 0.
            index: 0,
        }
    }
}

struct GenericIterator<'a> {
    iter: ActiveIterator<'a>,
}

impl<'a> GenericIterator<'a> {
    fn new(iter: &'a ActiveIterator) -> Self {
        Self {
            iter: iter.reborrow(),
        }
    }

    fn next<'gc>(
        &mut self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        let iter = self.get(agent);
        let next_method = iter.next_method.bind(gc.nogc());
        let iterator = iter.iterator.bind(gc.nogc());

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

    fn get<'agent>(&self, agent: &'agent Agent) -> &'agent IteratorRecord {
        let VmIteratorRecord::GenericIterator(iter) = self.iter.get(agent) else {
            unreachable!()
        };
        iter
    }
}

impl HeapMarkAndSweep for ObjectPropertiesIteratorRecord<'static> {
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

impl HeapMarkAndSweep for ArrayValuesIteratorRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.array.mark_values(queues)
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.array.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for VmIteratorRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            VmIteratorRecord::InvalidIterator => {}
            VmIteratorRecord::ObjectProperties(iter) => iter.mark_values(queues),
            VmIteratorRecord::ArrayValues(iter) => iter.mark_values(queues),
            VmIteratorRecord::GenericIterator(iter) => iter.mark_values(queues),
            VmIteratorRecord::SliceIterator(_) => {}
            VmIteratorRecord::EmptySliceIterator => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            VmIteratorRecord::InvalidIterator => {}
            VmIteratorRecord::ObjectProperties(iter) => iter.sweep_values(compactions),
            VmIteratorRecord::ArrayValues(iter) => iter.sweep_values(compactions),
            VmIteratorRecord::GenericIterator(iter) => iter.sweep_values(compactions),
            VmIteratorRecord::SliceIterator(_) => {}
            VmIteratorRecord::EmptySliceIterator => {}
        }
    }
}
