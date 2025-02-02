// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::VecDeque;

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::get_iterator_from_method;
use crate::ecmascript::abstract_operations::operations_on_objects::{call, get, get_method};
use crate::ecmascript::abstract_operations::type_conversion::to_boolean;
use crate::ecmascript::types::IntoValue;
use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::IteratorRecord,
        builtins::Array,
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{InternalMethods, Object, PropertyKey, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

use super::executable::SendableRef;

#[derive(Debug)]
pub(super) enum VmIterator {
    ObjectProperties(ObjectPropertiesIterator),
    ArrayValues(ArrayValuesIterator),
    GenericIterator(IteratorRecord),
    SliceIterator(SendableRef<[Value<'static>]>),
}

impl VmIterator {
    /// Unbind this VmIterator from its current lifetime. This is necessary to use
    /// the VmIterator as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> VmIterator {
        self
    }

    // Bind this VmIterator to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your VmIterators cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let number = number.bind(&gc);
    // ```
    // to make sure that the unbound VmIterator cannot be used after binding.
    pub const fn bind(self, _: NoGcScope) -> VmIterator {
        self
    }

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
    ) -> JsResult<Option<Value<'gc>>> {
        match self {
            VmIterator::ObjectProperties(iter) => {
                let result = iter.next(agent, gc.reborrow())?;
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
                let result = call(
                    agent,
                    iter.next_method,
                    iter.iterator.into_value(),
                    None,
                    gc.reborrow(),
                )?;
                let Ok(result) = Object::try_from(result) else {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Iterator returned a non-object result",
                        gc.nogc(),
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
                )?;
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
            VmIterator::SliceIterator(slice_ref) => {
                let slice = slice_ref.get();
                if slice.is_empty() {
                    Ok(None)
                } else {
                    let ret = slice[0];
                    *slice_ref = SendableRef::new(&slice[1..]);
                    Ok(Some(ret))
                }
            }
        }
    }

    pub(super) fn remaining_length_estimate(&self, agent: &mut Agent) -> Option<usize> {
        match self {
            VmIterator::ObjectProperties(iter) => Some(iter.remaining_keys.len()),
            VmIterator::ArrayValues(iter) => {
                Some(iter.array.len(agent).saturating_sub(iter.index) as usize)
            }
            VmIterator::GenericIterator(_) => None,
            VmIterator::SliceIterator(slice) => Some(slice.get().len()),
        }
    }

    /// ### [7.4.4 GetIterator ( obj, kind )](https://tc39.es/ecma262/#sec-getiterator)
    ///
    /// The abstract operation GetIterator takes arguments obj (an ECMAScript
    /// language value) and returns either a normal completion containing an
    /// Iterator Record or a throw completion.
    ///
    /// This method version performs the SYNC version of the method.
    pub(super) fn from_value(agent: &mut Agent, value: Value, mut gc: GcScope) -> JsResult<Self> {
        // a. Let method be ? GetMethod(obj, %Symbol.iterator%).
        let method = get_method(
            agent,
            value,
            PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
            gc.reborrow(),
        )?;
        // 3. If method is undefined, throw a TypeError exception.
        let Some(method) = method else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Iterator method cannot be undefined",
                gc.nogc(),
            ));
        };

        // 4. Return ? GetIteratorFromMethod(obj, method).
        match value {
            // Optimisation: Check if we're using the Array values iterator on
            // an Array.
            Value::Array(array)
                if method
                    == agent
                        .current_realm()
                        .intrinsics()
                        .array_prototype_values()
                        .into() =>
            {
                Ok(VmIterator::ArrayValues(ArrayValuesIterator::new(array)))
            }
            _ => {
                let js_iterator = get_iterator_from_method(agent, value, method.unbind(), gc)?;
                Ok(VmIterator::GenericIterator(js_iterator))
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ObjectPropertiesIterator {
    object: Object<'static>,
    object_was_visited: bool,
    visited_keys: Vec<PropertyKey<'static>>,
    remaining_keys: VecDeque<PropertyKey<'static>>,
}

impl ObjectPropertiesIterator {
    pub(super) fn new(object: Object) -> Self {
        Self {
            object: object.unbind(),
            object_was_visited: false,
            visited_keys: Default::default(),
            remaining_keys: Default::default(),
        }
    }

    pub(super) fn next<'a>(
        &mut self,
        agent: &mut Agent,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<Option<PropertyKey<'a>>> {
        loop {
            let object = self.object.scope(agent, gc.nogc());
            if !self.object_was_visited {
                let keys = object
                    .get(agent)
                    .internal_own_property_keys(agent, gc.reborrow())?;
                for key in keys {
                    if let PropertyKey::Symbol(_) = key {
                        continue;
                    } else {
                        // TODO: Properly handle potential GC.
                        self.remaining_keys.push_back(key.unbind());
                    }
                }
                self.object_was_visited = true;
            }
            while let Some(r) = self.remaining_keys.pop_front() {
                if self.visited_keys.contains(&r) {
                    continue;
                }
                // TODO: Properly handle potential GC.
                let desc = object
                    .get(agent)
                    .internal_get_own_property(agent, r, gc.reborrow())?;
                if let Some(desc) = desc {
                    self.visited_keys.push(r);
                    if desc.enumerable == Some(true) {
                        return Ok(Some(r));
                    }
                }
            }
            // TODO: Properly handle potential GC.
            let prototype = object
                .get(agent)
                .internal_get_prototype_of(agent, gc.reborrow())?;
            if let Some(prototype) = prototype {
                self.object_was_visited = false;
                self.object = prototype.unbind();
            } else {
                return Ok(None);
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ArrayValuesIterator {
    array: Array<'static>,
    index: u32,
}

impl ArrayValuesIterator {
    pub(super) fn new(array: Array) -> Self {
        Self {
            array: array.unbind(),
            // a. Let index be 0.
            index: 0,
        }
    }

    pub(super) fn next<'gc>(
        &mut self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Option<Value<'gc>>> {
        // b. Repeat,
        let array = self.array;
        // iv. Let indexNumber be 𝔽(index).
        let index = self.index;
        // 1. Let len be ? LengthOfArrayLike(array).
        let len = self.array.len(agent);
        // iii. If index ≥ len, return NormalCompletion(undefined).
        if index >= len {
            return Ok(None);
        }
        // viii. Set index to index + 1.
        self.index += 1;
        if let Some(element_value) = array.as_slice(agent)[index as usize] {
            // Fast path: If the element at this index has a Value, then it is
            // not an accessor nor a hole. Yield the result as-is.
            return Ok(Some(element_value.bind(gc.into_nogc())));
        }
        // 1. Let elementKey be ! ToString(indexNumber).
        // 2. Let elementValue be ? Get(array, elementKey).
        // TODO: Properly handle potential GC.
        let element_value = get(agent, self.array, index.into(), gc)?;
        // a. Let result be elementValue.
        // vii. Perform ? GeneratorYield(CreateIterResultObject(result, false)).
        Ok(Some(element_value))
    }
}

impl HeapMarkAndSweep for ObjectPropertiesIterator {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object,
            object_was_visited: _,
            visited_keys,
            remaining_keys,
        } = self;
        object.mark_values(queues);
        visited_keys.as_slice().mark_values(queues);
        for key in remaining_keys.iter() {
            key.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object,
            object_was_visited: _,
            visited_keys,
            remaining_keys,
        } = self;
        object.sweep_values(compactions);
        visited_keys.as_mut_slice().sweep_values(compactions);
        for key in remaining_keys.iter_mut() {
            key.sweep_values(compactions);
        }
    }
}

impl HeapMarkAndSweep for ArrayValuesIterator {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.array.mark_values(queues)
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.array.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for VmIterator {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            VmIterator::ObjectProperties(iter) => iter.mark_values(queues),
            VmIterator::ArrayValues(iter) => iter.mark_values(queues),
            VmIterator::GenericIterator(iter) => iter.mark_values(queues),
            VmIterator::SliceIterator(slice) => slice.get().mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            VmIterator::ObjectProperties(iter) => iter.sweep_values(compactions),
            VmIterator::ArrayValues(iter) => iter.sweep_values(compactions),
            VmIterator::GenericIterator(iter) => iter.sweep_values(compactions),
            VmIterator::SliceIterator(slice) => slice.get().sweep_values(compactions),
        }
    }
}
