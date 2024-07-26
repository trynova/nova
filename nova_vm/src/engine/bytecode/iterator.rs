// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::VecDeque;

use crate::ecmascript::{
    abstract_operations::{
        operations_on_iterator_objects::IteratorRecord,
        operations_on_objects::{call, get},
        type_conversion::to_boolean,
    },
    builtins::Array,
    execution::{agent::ExceptionType, Agent, JsResult},
    types::{InternalMethods, Object, PropertyKey, Value, BUILTIN_STRING_MEMORY},
};

#[derive(Debug)]
pub(super) enum VmIterator {
    ObjectProperties(ObjectPropertiesIterator),
    ArrayValues(ArrayValuesIterator),
    GenericIterator(IteratorRecord),
}

impl VmIterator {
    /// ### [7.4.8 IteratorStepValue ( iteratorRecord )](https://tc39.es/ecma262/#sec-iteratorstepvalue)
    ///
    /// While not exactly equal to the IteratorStepValue method in usage, this
    /// function implements much the same intent. It does the IteratorNext
    /// step, followed by a completion check, and finally extracts the value
    /// if the iterator did not complete yet.
    pub(super) fn step_value(&mut self, agent: &mut Agent) -> JsResult<Option<Value>> {
        match self {
            VmIterator::ObjectProperties(iter) => {
                let result = iter.next(agent)?;
                if let Some(result) = result {
                    Ok(Some(match result {
                        PropertyKey::Integer(int) => {
                            Value::from_string(agent, format!("{}", int.into_i64()))
                        }
                        PropertyKey::SmallString(data) => Value::SmallString(data),
                        PropertyKey::String(data) => Value::String(data),
                        _ => unreachable!(),
                    }))
                } else {
                    Ok(None)
                }
            }
            VmIterator::ArrayValues(iter) => iter.next(agent),
            VmIterator::GenericIterator(iter) => {
                let result = call(agent, iter.next_method, iter.iterator.into_value(), None)?;
                let Ok(result) = Object::try_from(result) else {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Iterator returned a non-object result",
                    ));
                };
                // 1. Return ToBoolean(? Get(iterResult, "done")).
                let done = get(agent, result, BUILTIN_STRING_MEMORY.done.into())?;
                let done = to_boolean(agent, done);
                if done {
                    Ok(None)
                } else {
                    // 1. Return ? Get(iterResult, "value").
                    let value = get(agent, result, BUILTIN_STRING_MEMORY.value.into())?;
                    Ok(Some(value))
                }
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ObjectPropertiesIterator {
    object: Object,
    object_was_visited: bool,
    visited_keys: Vec<PropertyKey>,
    remaining_keys: VecDeque<PropertyKey>,
}

impl ObjectPropertiesIterator {
    pub(super) fn new(object: Object) -> Self {
        Self {
            object,
            object_was_visited: false,
            visited_keys: Default::default(),
            remaining_keys: Default::default(),
        }
    }

    pub(super) fn next(&mut self, agent: &mut Agent) -> JsResult<Option<PropertyKey>> {
        loop {
            let object = self.object;
            if !self.object_was_visited {
                let keys = object.internal_own_property_keys(agent)?;
                for key in keys {
                    if let PropertyKey::Symbol(_) = key {
                        continue;
                    } else {
                        self.remaining_keys.push_back(key);
                    }
                }
                self.object_was_visited = true;
            }
            while let Some(r) = self.remaining_keys.pop_front() {
                if self.visited_keys.contains(&r) {
                    continue;
                }
                let desc = object.internal_get_own_property(agent, r)?;
                if let Some(desc) = desc {
                    self.visited_keys.push(r);
                    if desc.enumerable == Some(true) {
                        return Ok(Some(r));
                    }
                }
            }
            let prototype = object.internal_get_prototype_of(agent)?;
            if let Some(prototype) = prototype {
                self.object_was_visited = false;
                self.object = prototype;
            } else {
                return Ok(None);
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ArrayValuesIterator {
    array: Array,
    index: u32,
}

impl ArrayValuesIterator {
    pub(super) fn new(array: Array) -> Self {
        Self {
            array,
            // a. Let index be 0.
            index: 0,
        }
    }

    pub(super) fn next(&mut self, agent: &mut Agent) -> JsResult<Option<Value>> {
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
            return Ok(Some(element_value));
        }
        // 1. Let elementKey be ! ToString(indexNumber).
        // 2. Let elementValue be ? Get(array, elementKey).
        let element_value = get(agent, self.array, index.into())?;
        // a. Let result be elementValue.
        // vii. Perform ? GeneratorYield(CreateIterResultObject(result, false)).
        Ok(Some(element_value))
    }
}
