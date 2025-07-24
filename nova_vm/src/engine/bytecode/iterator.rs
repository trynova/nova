// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{marker::PhantomData, ptr::NonNull};

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                IteratorRecord, async_iterator_close_with_value, create_iter_result_object,
                get_iterator_from_method, iterator_close_with_value,
            },
            operations_on_objects::{
                call_function, get, get_method, get_object_method, throw_not_callable,
            },
            type_conversion::to_boolean,
        },
        builtins::{
            ArgumentsList, Array, ScopedArgumentsList,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator,
            iteration::async_from_sync_iterator_objects::{
                AsyncFromSyncIteratorPrototype, create_async_from_sync_iterator,
            },
            promise::Promise,
        },
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, JsError},
        },
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyKey, PropertyKeySet, Value,
        },
    },
    engine::{
        Scoped, TryResult,
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

fn convert_to_iter_result_object<'a>(
    agent: &mut Agent,
    result: Option<Value<'a>>,
) -> OrdinaryObject<'a> {
    if let Some(result) = result {
        create_iter_result_object(agent, result, false)
    } else {
        create_iter_result_object(agent, Value::Undefined, true)
    }
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

    pub(super) fn call_next<'gc>(
        &mut self,
        agent: &mut Agent,
        value: Option<Value>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        match self.get(agent) {
            VmIteratorRecord::InvalidIterator { .. } => {
                Err(throw_not_callable(agent, gc.into_nogc()).unbind())
            }
            VmIteratorRecord::ObjectProperties(_) => {
                // I believe it is impossible to do yield* on object properties.
                unreachable!()
            }
            VmIteratorRecord::ArrayValues(_) => ArrayValuesIterator::new(self)
                .next(agent, gc)
                .map(|r| convert_to_iter_result_object(agent, r).into_value()),
            VmIteratorRecord::AsyncFromSyncGenericIterator(_) => {
                Ok(AsyncFromSyncGenericIterator::new(self)
                    .call_next(agent, value, gc)
                    .into_value())
            }
            VmIteratorRecord::GenericIterator(_) => {
                GenericIterator::new(self).call_next(agent, value, gc)
            }
            VmIteratorRecord::SliceIterator(slice_ref) => Ok(convert_to_iter_result_object(
                agent,
                slice_ref.unshift(agent, gc.into_nogc()),
            )
            .into_value()),
            VmIteratorRecord::EmptySliceIterator => {
                Ok(create_iter_result_object(agent, Value::Undefined, true).into_value())
            }
        }
    }

    pub(super) fn throw<'gc>(
        &mut self,
        agent: &mut Agent,
        received_value: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        match self.get(agent) {
            VmIteratorRecord::ArrayValues(_) => {
                ArrayValuesIterator::new(self).throw(agent, received_value, gc)
            }
            VmIteratorRecord::AsyncFromSyncGenericIterator(_) => Ok(Some(
                AsyncFromSyncGenericIterator::new(self)
                    .throw(agent, received_value, gc)
                    .into_value(),
            )),
            VmIteratorRecord::GenericIterator(_) => {
                GenericIterator::new(self).throw(agent, received_value, gc)
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn r#return<'gc>(
        &mut self,
        agent: &mut Agent,
        received_value: Option<Value>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        match self.get(agent) {
            VmIteratorRecord::ArrayValues(_) => {
                ArrayValuesIterator::new(self).r#return(agent, received_value, gc)
            }
            VmIteratorRecord::AsyncFromSyncGenericIterator(_) => Ok(Some(
                AsyncFromSyncGenericIterator::new(self)
                    .r#return(agent, received_value, gc)
                    .into_value(),
            )),
            VmIteratorRecord::GenericIterator(IteratorRecord { iterator, .. })
            | VmIteratorRecord::InvalidIterator { iterator } => {
                GenericIterator::r#return(agent, *iterator, received_value, gc)
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn step_value<'gc>(
        &mut self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        match self.get(agent) {
            VmIteratorRecord::InvalidIterator { .. } => {
                Err(throw_not_callable(agent, gc.into_nogc()).unbind())
            }
            VmIteratorRecord::ObjectProperties(_) => {
                ObjectPropertiesIterator::new(self).step_value(agent, gc)
            }
            VmIteratorRecord::ArrayValues(_) => ArrayValuesIterator::new(self).next(agent, gc),
            VmIteratorRecord::AsyncFromSyncGenericIterator(_) => {
                // We should never call this for async iterators!
                unreachable!()
            }
            VmIteratorRecord::GenericIterator(_) => {
                GenericIterator::new(self).step_value(agent, gc)
            }
            VmIteratorRecord::SliceIterator(slice_ref) => {
                Ok(slice_ref.unshift(agent, gc.into_nogc()))
            }
            VmIteratorRecord::EmptySliceIterator => Ok(None),
        }
    }

    pub(super) fn remaining_length_estimate(&mut self, agent: &mut Agent) -> Option<usize> {
        self.get(agent).remaining_length_estimate(agent)
    }

    pub(super) fn get<'agent>(&self, agent: &'agent Agent) -> &'agent VmIteratorRecord<'a> {
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
pub(crate) enum VmIteratorRecord<'a> {
    /// Special type for iterators that do not have a callable next method.
    InvalidIterator {
        iterator: Object<'a>,
    },
    ObjectProperties(Box<ObjectPropertiesIteratorRecord<'a>>),
    ArrayValues(ArrayValuesIteratorRecord<'a>),
    AsyncFromSyncGenericIterator(IteratorRecord<'a>),
    GenericIterator(IteratorRecord<'a>),
    SliceIterator(ScopedArgumentsList<'a>),
    EmptySliceIterator,
}

impl<'a> VmIteratorRecord<'a> {
    pub(super) fn remaining_length_estimate(&self, agent: &Agent) -> Option<usize> {
        match self {
            VmIteratorRecord::InvalidIterator { .. } => Some(0),
            VmIteratorRecord::ObjectProperties(iter) => {
                Some(iter.remaining_keys.as_ref().map_or(0, |k| k.len()))
            }
            VmIteratorRecord::ArrayValues(iter) => {
                Some(iter.array.len(agent).saturating_sub(iter.index) as usize)
            }
            VmIteratorRecord::AsyncFromSyncGenericIterator(_) => todo!(),
            VmIteratorRecord::GenericIterator(_) => None,
            VmIteratorRecord::SliceIterator(slice) => Some(slice.len(agent)),
            VmIteratorRecord::EmptySliceIterator => Some(0),
        }
    }

    pub(super) fn requires_return_call(&self, agent: &mut Agent, gc: NoGcScope) -> bool {
        match self {
            VmIteratorRecord::ObjectProperties(_)
            | VmIteratorRecord::SliceIterator(_)
            | VmIteratorRecord::EmptySliceIterator => false,
            VmIteratorRecord::ArrayValues(_) => {
                array_iterator_record_requires_return_call(agent, gc)
            }
            VmIteratorRecord::InvalidIterator { iterator }
            | VmIteratorRecord::GenericIterator(IteratorRecord { iterator, .. })
            | VmIteratorRecord::AsyncFromSyncGenericIterator(IteratorRecord { iterator, .. }) => {
                generic_iterator_record_requires_return_call(agent, *iterator, gc)
            }
        }
    }

    pub(super) fn requires_throw_call(&self, agent: &mut Agent, gc: NoGcScope) -> bool {
        match self {
            VmIteratorRecord::ObjectProperties(_)
            | VmIteratorRecord::SliceIterator(_)
            | VmIteratorRecord::EmptySliceIterator => false,
            VmIteratorRecord::ArrayValues(_) => {
                // Note: no one can access the array values iterator while it is
                // iterating so we know its prototype is the intrinsic
                // ArrayIteratorPrototype. But that may have a throw method
                // set on it by a horrible-horrible person somewhere.
                // TODO: This should actually maybe be the proto of the Array's
                // realm?
                let array_iterator_prototype = agent
                    .current_realm_record()
                    .intrinsics()
                    .array_iterator_prototype()
                    .bind(gc);
                // IteratorClose calls GetMethod on the iterator: if a
                // non-nullable value is found this way then things happen.
                match array_iterator_prototype.try_get(
                    agent,
                    BUILTIN_STRING_MEMORY.throw.into(),
                    array_iterator_prototype.into_value(),
                    gc,
                ) {
                    TryResult::Continue(return_method) => {
                        !return_method.is_undefined() && !return_method.is_null()
                    }
                    // Note: here it's still possible that we won't actually
                    // call a return method but this break already means that
                    // the user can observe the ArrayIterator object.
                    TryResult::Break(_) => true,
                }
            }
            VmIteratorRecord::InvalidIterator { iterator }
            | VmIteratorRecord::GenericIterator(IteratorRecord { iterator, .. })
            | VmIteratorRecord::AsyncFromSyncGenericIterator(IteratorRecord { iterator, .. }) => {
                match iterator.try_get(
                    agent,
                    BUILTIN_STRING_MEMORY.throw.into(),
                    iterator.into_value(),
                    gc,
                ) {
                    TryResult::Continue(return_method) => {
                        !return_method.is_undefined() && !return_method.is_null()
                    }
                    // Note: here it's still possible that we won't actually
                    // call a return method but this break already means that
                    // we'll need garbage collection.
                    TryResult::Break(_) => true,
                }
            }
        }
    }

    pub(super) fn is_arguments_iterator(&self) -> bool {
        matches!(self, VmIteratorRecord::SliceIterator(_))
    }

    pub(super) fn close_with_value(
        self,
        agent: &mut Agent,
        completion: Value,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        match self {
            VmIteratorRecord::ObjectProperties(_)
            | VmIteratorRecord::SliceIterator(_)
            | VmIteratorRecord::EmptySliceIterator => {
                unreachable!()
            }
            VmIteratorRecord::ArrayValues(iter) => iter.close_with_value(agent, completion, gc),
            VmIteratorRecord::InvalidIterator { iterator }
            | VmIteratorRecord::GenericIterator(IteratorRecord { iterator, .. })
            | VmIteratorRecord::AsyncFromSyncGenericIterator(IteratorRecord { iterator, .. }) => {
                iterator_close_with_value(agent, iterator, completion, gc)
            }
        }
    }

    pub(super) fn async_close_with_value(
        self,
        agent: &mut Agent,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Option<Value<'a>>> {
        match self {
            VmIteratorRecord::ObjectProperties(_)
            | VmIteratorRecord::SliceIterator(_)
            | VmIteratorRecord::EmptySliceIterator => {
                unreachable!()
            }
            VmIteratorRecord::ArrayValues(iter) => iter.async_close_with_value(agent, gc),
            VmIteratorRecord::InvalidIterator { iterator }
            | VmIteratorRecord::GenericIterator(IteratorRecord { iterator, .. })
            | VmIteratorRecord::AsyncFromSyncGenericIterator(IteratorRecord { iterator, .. }) => {
                async_iterator_close_with_value(agent, iterator, gc)
            }
        }
    }

    /// ### [7.4.4 GetIterator ( obj, kind )](https://tc39.es/ecma262/#sec-getiterator)
    ///
    /// The abstract operation GetIterator takes arguments obj (an ECMAScript
    /// language value) and returns either a normal completion containing an
    /// Iterator Record or a throw completion.
    ///
    /// This method version performs the SYNC version of the method.
    pub(super) fn from_value(
        agent: &mut Agent,
        obj: Value,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        let value = obj.bind(gc.nogc());
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
            return Err(throw_iterator_method_undefined(agent, gc.into_nogc()));
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
            _ => Ok(
                get_iterator_from_method(agent, value.unbind(), method.unbind(), gc)?
                    .into_vm_iterator_record(),
            ),
        }
    }

    /// ### [7.4.4 GetIterator ( obj, kind )](https://tc39.es/ecma262/#sec-getiterator)
    ///
    /// The abstract operation GetIterator takes arguments obj (an ECMAScript
    /// language value) and returns either a normal completion containing an
    /// Iterator Record or a throw completion.
    ///
    /// This method version performs the ASYNC version of the method.
    pub(super) fn async_from_value(
        agent: &mut Agent,
        obj: Value,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        let obj = obj.bind(gc.nogc());
        let scoped_obj = obj.scope(agent, gc.nogc());
        // a. Let method be ? GetMethod(obj, %Symbol.asyncIterator%).
        let method = get_method(
            agent,
            obj.unbind(),
            PropertyKey::Symbol(WellKnownSymbolIndexes::AsyncIterator.into()),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // b. If method is undefined, then
        let Some(method) = method else {
            return Self::async_from_sync_value(agent, scoped_obj, gc);
        };

        // SAFETY: scoped_value is not shared.
        let obj = unsafe { scoped_obj.take(agent).bind(gc.nogc()) };

        // 4. Return ? GetIteratorFromMethod(obj, method).
        if let Some(array) = Array::is_iterable_array(agent, obj, method) {
            // Optimisation: if we're using the Array values iterator on
            // an Array then we can use a special iterator case.
            Ok(VmIteratorRecord::ArrayValues(
                ArrayValuesIteratorRecord::new(array.unbind()),
            ))
        } else {
            Ok(
                get_iterator_from_method(agent, obj.unbind(), method.unbind(), gc)?
                    .into_vm_iterator_record(),
            )
        }
    }

    fn async_from_sync_value(
        agent: &mut Agent,
        obj: Scoped<Value>,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        // i. Let syncMethod be ? GetMethod(obj, %Symbol.iterator%).
        let sync_method = get_method(
            agent,
            obj.get(agent),
            WellKnownSymbolIndexes::Iterator.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // ii. If syncMethod is undefined, throw a TypeError exception.
        let Some(sync_method) = sync_method else {
            return Err(throw_iterator_method_undefined(agent, gc.into_nogc()));
        };
        // SAFETY: obj is not shared.
        let obj = unsafe { obj.take(agent) }.bind(gc.nogc());
        // iii. Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
        let sync_iterator_record =
            get_iterator_from_method(agent, obj.unbind(), sync_method.unbind(), gc)?;
        // iv. Return CreateAsyncFromSyncIterator(syncIteratorRecord).
        Ok(create_async_from_sync_iterator(sync_iterator_record))
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

    fn get<'agent>(&self, agent: &'agent Agent) -> &'agent ObjectPropertiesIteratorRecord<'a> {
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

    fn throw<'gc>(
        &mut self,
        agent: &mut Agent,
        _received_value: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        // 1. Let throw be ? GetMethod(iterator, "throw").
        let throw = get_object_method(
            agent,
            agent
                .current_realm_record()
                .intrinsics()
                .array_prototype()
                .into_object(),
            BUILTIN_STRING_MEMORY.throw.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 2. If throw is not undefined, then
        if let Some(throw) = throw {
            // i. Let innerResult be
            //    ? Call(throw, iterator, « received.[[Value]] »).
            todo!("Handle ArrayValuesIterator with throw method {throw:?}");
        }
        Ok(None)
    }

    fn r#return<'gc>(
        &mut self,
        agent: &mut Agent,
        _received_value: Option<Value>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        // 1. Let return be ? GetMethod(iterator, "return").
        let r#return = get_object_method(
            agent,
            agent
                .current_realm_record()
                .intrinsics()
                .array_prototype()
                .into_object(),
            BUILTIN_STRING_MEMORY.r#return.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 2. If return is not undefined, then
        if let Some(r#return) = r#return {
            // i. Let innerResult be
            //    ? Call(return, iterator, « received.[[Value]] »).
            todo!("Handle ArrayValuesIterator with return method {return:?}");
        }
        Ok(None)
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

    fn close_with_value<'gc>(
        self,
        agent: &mut Agent,
        completion: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let ArrayValuesIteratorRecord { array, index } = self;
        let array = array.bind(gc.nogc());
        let completion = completion.bind(gc.nogc());
        // We need to create the ArrayIterator object for user to
        // interact with.
        let iter = ArrayIterator::from_vm_iterator(agent, array, index, gc.nogc());
        iterator_close_with_value(agent, iter.into_object().unbind(), completion.unbind(), gc)
    }

    fn async_close_with_value<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        let ArrayValuesIteratorRecord { array, index } = self;
        let array = array.bind(gc.nogc());
        // We need to create the ArrayIterator object for user to
        // interact with.
        let iter = ArrayIterator::from_vm_iterator(agent, array, index, gc.nogc());
        async_iterator_close_with_value(agent, iter.into_object().unbind(), gc)
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

    fn call_next<'gc>(
        &mut self,
        agent: &mut Agent,
        value: Option<Value>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let iter = self.get(agent);
        let next_method = iter.next_method.bind(gc.nogc());
        let iterator = iter.iterator.bind(gc.nogc());
        let value = value.bind(gc.nogc());

        // 1. If value is not present, then
        if let Some(value) = value {
            // 2. Else,
            // a. Let result be Completion(Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « value »)).
            call_function(
                agent,
                next_method.unbind(),
                iterator.into_value().unbind(),
                Some(ArgumentsList::from_mut_value(&mut value.unbind())),
                gc,
            )
        } else {
            // a. Let result be Completion(Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]])).
            call_function(
                agent,
                next_method.unbind(),
                iterator.into_value().unbind(),
                None,
                gc,
            )
        }
    }

    fn step_value<'gc>(
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
            return Err(throw_iterator_returned_non_object(agent, gc.into_nogc()));
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

    fn throw<'gc>(
        &mut self,
        agent: &mut Agent,
        received_value: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        let received_value = received_value.scope(agent, gc.nogc());
        let iterator = self.get(agent).iterator.bind(gc.nogc());
        // 1. Let throw be ? GetMethod(iterator, "throw").
        let throw = get_object_method(
            agent,
            iterator.unbind(),
            BUILTIN_STRING_MEMORY.throw.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 2. If throw is not undefined, then
        if let Some(throw) = throw {
            let iterator = self.get(agent).iterator.bind(gc.nogc());
            // i. Return Call(throw, iterator, « received.[[Value]] »).
            Ok(Some(call_function(
                agent,
                throw.unbind(),
                iterator.into_value().unbind(),
                // SAFETY: not shared.
                Some(ArgumentsList::from_mut_value(&mut unsafe {
                    received_value.take(agent)
                })),
                gc,
            )?))
        } else {
            Ok(None)
        }
    }

    fn r#return<'gc>(
        agent: &mut Agent,
        iterator: Object,
        value: Option<Value>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Value<'gc>>> {
        let value = value.map(|v| v.scope(agent, gc.nogc()));
        let iterator = iterator.bind(gc.nogc());
        let scoped_iterator = iterator.scope(agent, gc.nogc());
        // 1. Assert: iteratorRecord.[[Iterator]] is an Object.
        // 2. Let iterator be iteratorRecord.[[Iterator]].
        // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
        let r#return = get_object_method(
            agent,
            iterator.unbind(),
            BUILTIN_STRING_MEMORY.r#return.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 4. If completion is a throw completion, return ? completion.
        // 5. If innerResult is a throw completion, return ? innerResult.
        // 4. If innerResult is a normal completion, then
        // a. Let return be innerResult.[[Value]].
        // b. If return is undefined, ...
        if let Some(r#return) = r#return {
            let iterator = unsafe { scoped_iterator.take(agent) }.bind(gc.nogc());
            // c. Set innerResult to Completion(Call(return, iterator)).
            // d. If innerResult is a normal completion, set innerResult to
            //    Completion(innerResult.[[Value]]).
            // SAFETY: not shared.
            let mut value = value.map(|v| unsafe { v.take(agent) });
            Ok(Some(call_function(
                agent,
                r#return.unbind(),
                iterator.into_value().unbind(),
                value.as_mut().map(ArgumentsList::from_mut_value),
                gc,
            )?))
        } else {
            // ... return ? completion.
            Ok(None)
        }
    }

    fn get<'agent>(&self, agent: &'agent Agent) -> &'agent IteratorRecord<'a> {
        match self.iter.get(agent) {
            VmIteratorRecord::GenericIterator(iter) => iter,
            _ => unreachable!(),
        }
    }
}

struct AsyncFromSyncGenericIterator<'a> {
    iter: ActiveIterator<'a>,
}

impl<'a> AsyncFromSyncGenericIterator<'a> {
    fn new(iter: &'a ActiveIterator) -> Self {
        Self {
            iter: iter.reborrow(),
        }
    }

    /// ### [27.1.6.2.1 %AsyncFromSyncIteratorPrototype%.next ( \[ value \] )](https://tc39.es/ecma262/#sec-%asyncfromsynciteratorprototype%.next)
    fn call_next<'gc>(
        &mut self,
        agent: &mut Agent,
        value: Option<Value>,
        gc: GcScope<'gc, '_>,
    ) -> Promise<'gc> {
        // 1. Let O be the this value.
        // 2. Assert: O is an Object that has a [[SyncIteratorRecord]] internal slot.
        // 4. Let syncIteratorRecord be O.[[SyncIteratorRecord]].
        // 5. If value is present, then
        let iter = self.get(agent);
        let sync_iterator_record = iter.bind(gc.nogc());
        AsyncFromSyncIteratorPrototype::next(agent, sync_iterator_record.unbind(), value, gc)
    }

    /// ### [27.1.6.2.2 %AsyncFromSyncIteratorPrototype%.return ( \[ value \] )](https://tc39.es/ecma262/#sec-%asyncfromsynciteratorprototype%.return)
    fn r#return<'gc>(
        &mut self,
        agent: &mut Agent,
        value: Option<Value>,
        gc: GcScope<'gc, '_>,
    ) -> Promise<'gc> {
        // 1. Let O be the this value.
        // 2. Assert: O is an Object that has a [[SyncIteratorRecord]] internal slot.
        // 4. Let syncIteratorRecord be O.[[SyncIteratorRecord]].
        // 5. Let syncIterator be syncIteratorRecord.[[Iterator]].
        let iter = self.get(agent);
        let sync_iterator = iter.iterator.bind(gc.nogc());
        AsyncFromSyncIteratorPrototype::r#return(agent, sync_iterator.unbind(), value, gc)
    }

    /// ### [27.1.6.2.3 %AsyncFromSyncIteratorPrototype%.throw ( \[ value \] )](https://tc39.es/ecma262/#sec-%asyncfromsynciteratorprototype%.throw)
    ///
    /// > NOTE: In this specification, value is always provided, but is left
    /// > optional for consistency with
    /// > %AsyncFromSyncIteratorPrototype%.return ( [ value ] ).
    fn throw<'gc>(
        &mut self,
        agent: &mut Agent,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> Promise<'gc> {
        // 1. Let O be the this value.
        // 2. Assert: O is an Object that has a [[SyncIteratorRecord]] internal slot.
        // 4. Let syncIteratorRecord be O.[[SyncIteratorRecord]].
        // 5. Let syncIterator be syncIteratorRecord.[[Iterator]].
        let iter = self.get(agent);
        let sync_iterator = iter.iterator.bind(gc.nogc());
        AsyncFromSyncIteratorPrototype::throw(agent, sync_iterator.unbind(), value, gc)
    }

    fn get<'agent>(&self, agent: &'agent Agent) -> &'agent IteratorRecord<'a> {
        match self.iter.get(agent) {
            VmIteratorRecord::AsyncFromSyncGenericIterator(iter) => iter,
            _ => unreachable!(),
        }
    }
}

fn array_iterator_record_requires_return_call(agent: &mut Agent, gc: NoGcScope) -> bool {
    // Note: no one can access the array values iterator while it is
    // iterating so we know its prototype is the intrinsic
    // ArrayIteratorPrototype. But that may have a return method
    // set on it by a horrible-horrible person somewhere.
    // TODO: This should actually maybe be the proto of the Array's
    // realm?
    let array_iterator_prototype = agent
        .current_realm_record()
        .intrinsics()
        .array_iterator_prototype();
    // IteratorClose calls GetMethod on the iterator: if a
    // non-nullable value is found this way then things happen.
    match array_iterator_prototype.try_get(
        agent,
        BUILTIN_STRING_MEMORY.r#return.into(),
        array_iterator_prototype.into_value(),
        gc,
    ) {
        TryResult::Continue(return_method) => {
            !return_method.is_undefined() && !return_method.is_null()
        }
        // Note: here it's still possible that we won't actually
        // call a return method but this break already means that
        // the user can observe the ArrayIterator object.
        TryResult::Break(_) => true,
    }
}

fn generic_iterator_record_requires_return_call(
    agent: &mut Agent,
    iterator: Object,
    gc: NoGcScope,
) -> bool {
    match iterator.try_get(
        agent,
        BUILTIN_STRING_MEMORY.r#return.into(),
        iterator.into_value(),
        gc,
    ) {
        TryResult::Continue(return_method) => {
            !return_method.is_undefined() && !return_method.is_null()
        }
        // Note: here it's still possible that we won't actually
        // call a return method but this break already means that
        // we'll need garbage collection.
        TryResult::Break(_) => true,
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
            VmIteratorRecord::InvalidIterator { iterator } => iterator.mark_values(queues),
            VmIteratorRecord::ObjectProperties(iter) => iter.mark_values(queues),
            VmIteratorRecord::ArrayValues(iter) => iter.mark_values(queues),
            VmIteratorRecord::AsyncFromSyncGenericIterator(iter) => iter.mark_values(queues),
            VmIteratorRecord::GenericIterator(iter) => iter.mark_values(queues),
            VmIteratorRecord::SliceIterator(_) => {}
            VmIteratorRecord::EmptySliceIterator => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            VmIteratorRecord::InvalidIterator { iterator } => iterator.sweep_values(compactions),
            VmIteratorRecord::ObjectProperties(iter) => iter.sweep_values(compactions),
            VmIteratorRecord::ArrayValues(iter) => iter.sweep_values(compactions),
            VmIteratorRecord::AsyncFromSyncGenericIterator(iter) => iter.sweep_values(compactions),
            VmIteratorRecord::GenericIterator(iter) => iter.sweep_values(compactions),
            VmIteratorRecord::SliceIterator(_) => {}
            VmIteratorRecord::EmptySliceIterator => {}
        }
    }
}

pub(crate) fn throw_iterator_returned_non_object<'a>(
    agent: &mut Agent,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    agent.throw_exception_with_static_message(
        ExceptionType::TypeError,
        "Iterator returned a non-object result",
        gc,
    )
}

fn throw_iterator_method_undefined<'a>(agent: &mut Agent, gc: NoGcScope<'a, '_>) -> JsError<'a> {
    agent.throw_exception_with_static_message(
        ExceptionType::TypeError,
        "Iterator method cannot be undefined",
        gc,
    )
}
