// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cell::Ref;
use std::{collections::hash_map::Entry, ptr::NonNull};

use ahash::AHashMap;

use crate::{
    Heap,
    ecmascript::{
        builtins::ordinary::caches::Caches,
        execution::{Agent, JsResult, PrivateField, RealmRecord, agent::ExceptionType},
        types::{IntoValue, PrivateName, PropertyDescriptor, Value},
    },
    engine::context::{Bindable, NoGcScope},
    heap::{
        element_array::{
            ElementDescriptor, ElementStorageUninit, PropertyStorageMut, PropertyStorageRef,
        },
        indexes::ElementIndex,
    },
};

use super::{IntoObject, Object, OrdinaryObject, PropertyKey};

#[derive(Debug, Clone, Copy)]
pub struct PropertyStorage<'a>(OrdinaryObject<'a>);

fn verify_writable(
    descriptors: Entry<'_, ElementIndex<'static>, AHashMap<u32, ElementDescriptor<'static>>>,
    index: u32,
) -> bool {
    let Entry::Occupied(e) = descriptors else {
        // Note: no descriptors means all values are plain data properties.
        return true;
    };
    let descriptors = e.into_mut();
    // Note: no descriptor means value is a plain data property.
    descriptors.get(&index).is_none_or(|d| {
        d.is_writable()
            // Note: if we have an accessor property here, it is not writable.
            .unwrap_or(false)
    })
}

impl<'a> PropertyStorage<'a> {
    pub fn new(object: OrdinaryObject<'a>) -> Self {
        Self(object)
    }

    fn into_object(self) -> Object<'a> {
        self.0.into_object()
    }

    fn into_value(self) -> Value<'a> {
        self.0.into_value()
    }

    /// Adds an uninitialized PrivateName field to the object.
    pub(crate) fn add_private_field_slot(self, agent: &mut Agent, private_name: PrivateName) {
        // SAFETY: Private fields are backed by on-stack data; mutating Agent
        // is totally okay.
        unsafe {
            Self::insert_private_fields(
                agent,
                self.0,
                NonNull::from(&[PrivateField::Field { key: private_name }]),
            )
        };
    }

    /// Copy all PrivateMethods and reserve PrivateName fields from the current
    /// private environment to this object property storage.
    pub(crate) fn initialize_private_elements<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        let private_env = agent
            .current_private_environment(gc)
            .expect("Expected PrivateEnvironment to be set");
        let object = self.0;
        let Heap {
            environments,
            elements,
            objects,
            object_shapes,
            ..
        } = &agent.heap;
        let private_env = environments.get_private_environment(private_env);
        let private_fields = private_env.get_instance_private_fields(gc);
        if objects[object]
            .get_shape()
            .keys(object_shapes, elements)
            .contains(&private_fields[0].get_key().into())
        {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "attempted to initialize private elements twice",
                gc,
            ));
        }
        let private_fields = NonNull::from(private_fields);
        // SAFETY: insert_private_fields does not touch environments, and it
        // cannot push into the private_fields list. Hence the pointer here is
        // always valid.
        unsafe { Self::insert_private_fields(agent, self.0, private_fields) };
        Ok(())
    }

    /// ## Safety
    ///
    /// TODO
    unsafe fn insert_private_fields(
        agent: &mut Agent,
        object: OrdinaryObject,
        private_fields: NonNull<[PrivateField]>,
    ) {
        let original_len = object.len(agent);
        let original_shape = object.get_shape(agent);
        // SAFETY: User says so.
        let (new_shape, insertion_index) =
            unsafe { original_shape.add_private_fields(agent, private_fields) };
        let insertion_end_index = insertion_index.wrapping_add(private_fields.len());
        // Note: use saturating_mul to avoid a panic site.
        agent.heap.alloc_counter +=
            core::mem::size_of::<Option<Value>>().saturating_mul(private_fields.len());
        agent[object].set_shape(new_shape);
        object.reserve(agent, original_len + private_fields.len() as u32);
        let ElementStorageUninit {
            values,
            mut descriptors,
        } = object.get_elements_storage_uninit(agent);

        if insertion_index != original_len as usize {
            // Shift keys over by necessary amount.
            // Then do the same for values.
            values.copy_within(insertion_index..original_len as usize, insertion_end_index);
            // Finally, shift descriptors over if we have any.
            if let Entry::Occupied(d) = &mut descriptors {
                let d = d.get_mut();
                let lower_bound = insertion_index as u32;
                let upper_bound = original_len;
                let range = lower_bound..upper_bound;
                let keys_to_shift = d.extract_if(|k, _| range.contains(k)).collect::<Vec<_>>();
                for (k, v) in keys_to_shift {
                    d.insert(k + private_fields.len() as u32, v);
                }
            }
        }

        // SAFETY: User guarantees that the fields are not backed by memory
        // that we're going to be mutating.
        let private_fields = unsafe { private_fields.as_ref() };

        // Fill the keys and values with our PrivateNames starting at our found
        // index and ending at found index + number of private elements.
        let mut methods_count = 0;
        for (value_slot, private_element) in values[insertion_index..insertion_end_index]
            .iter_mut()
            .zip(private_fields)
        {
            *value_slot = private_element.get_value().unbind();
            if private_element.is_method() {
                methods_count += 1;
            }
        }

        // If we found some methods then we'll want to put their descriptors
        // in.
        if methods_count > 0 {
            let descriptors = descriptors.or_insert_with(|| AHashMap::with_capacity(methods_count));
            for (i, private_field) in private_fields.iter().enumerate() {
                if private_field.is_method() {
                    let k = (insertion_index + i) as u32;
                    descriptors.insert(k, private_field.into_element_descriptor().unbind());
                }
            }
        }
        agent[object].set_len(original_len.wrapping_add(private_fields.len() as u32));
    }

    pub(crate) fn set_private_field_value(
        self,
        agent: &mut Agent,
        private_name: PrivateName,
        offset_hint: usize,
        value: Value,
    ) -> bool {
        let object = self.0;
        let Some(PropertyStorageMut {
            keys,
            values,
            descriptors,
        }) = object.get_property_storage_mut(agent)
        else {
            // If the storage is empty, we cannot set a private field value.
            return false;
        };
        let offset = if keys.get(offset_hint) == Some(&private_name.into()) {
            offset_hint
        } else {
            let key = private_name.into();
            let result = keys
                .iter()
                .enumerate()
                .find(|(_, k)| **k == key)
                .map(|res| res.0);
            let Some(result) = result else {
                // Didn't find the property.
                return false;
            };
            result
        };
        let writable = verify_writable(descriptors, offset as u32);
        if !writable {
            return false;
        }
        values[offset] = Some(value.unbind());
        true
    }

    /// ### [7.3.26 PrivateElementFind ( O, P )](https://tc39.es/ecma262/#sec-privateelementfind)
    ///
    /// The abstract operation PrivateElementFind takes arguments O (an Object)
    /// and P (a Private Name) and returns a PrivateElement or empty.
    pub(crate) fn private_element_find(
        self,
        agent: &Agent,
        private_name: PrivateName,
    ) -> Option<(Option<Value<'a>>, Option<&ElementDescriptor<'a>>)> {
        // 1. If O.[[PrivateElements]] contains a PrivateElement pe such that
        //    pe.[[Key]] is P, then
        let PropertyStorageRef {
            keys,
            values,
            descriptors,
        } = self.0.get_property_storage(agent);
        let key = private_name.into();
        let index = keys
            .iter()
            .enumerate()
            .find(|(_, k)| **k == key)
            .map(|res| res.0);
        let Some(index) = index else {
            // Didn't find the private name.
            return None;
        };
        let value = values[index];
        let descriptor = descriptors.and_then(|d| d.get(&(index as u32)));
        // a. Return pe.
        Some((value, descriptor))
    }

    /// ### [7.3.26 PrivateElementFind ( O, P )](https://tc39.es/ecma262/#sec-privateelementfind)
    ///
    /// The abstract operation PrivateElementFind takes arguments O (an Object)
    /// and P (a Private Name) and returns a PrivateElement or empty.
    pub(crate) fn private_element_find_mut(
        self,
        agent: &mut Agent,
        private_name: PrivateName,
    ) -> Option<(
        Option<&mut Value<'static>>,
        Option<&ElementDescriptor<'static>>,
    )> {
        // 1. If O.[[PrivateElements]] contains a PrivateElement pe such that
        //    pe.[[Key]] is P, then
        let object = self.0;
        // If the storage is empty, no entry for the PrivateName can exist.
        let PropertyStorageMut {
            keys,
            values,
            descriptors,
        } = object.unbind().get_property_storage_mut(agent)?;
        let key = private_name.into();
        let index = keys
            .iter()
            .enumerate()
            .find(|(_, k)| **k == key)
            .map(|res| res.0);
        let Some(index) = index else {
            // Didn't find the private name.
            return None;
        };
        let value = values[index].as_mut();
        let descriptor = if let Entry::Occupied(e) = descriptors {
            e.into_mut().get(&(index as u32))
        } else {
            None
        };
        // a. Return pe.
        Some((value, descriptor))
    }

    pub fn get(self, agent: &Agent, key: PropertyKey) -> Option<(PropertyDescriptor<'a>, u32)> {
        let object = self.0;
        let PropertyStorageRef {
            keys,
            values,
            descriptors,
        } = object.get_property_storage(agent);
        let result = keys
            .iter()
            .enumerate()
            .find(|(_, k)| **k == key)
            .map(|res| res.0);
        result.map(|index| {
            let value = values[index].unbind();
            let index = index as u32;
            let descriptor = descriptors.and_then(|d| d.get(&index)).cloned();
            let result = ElementDescriptor::to_property_descriptor(descriptor, value);
            (result, index)
        })
    }

    pub fn set(
        self,
        agent: &mut Agent,
        o: Object<'a>,
        key: PropertyKey<'a>,
        descriptor: PropertyDescriptor<'a>,
        gc: NoGcScope,
    ) {
        let object = self.0;

        let value = descriptor.value;
        let element_descriptor = ElementDescriptor::from_property_descriptor(descriptor);

        let cur_len = if let Some(PropertyStorageMut {
            keys,
            values,
            descriptors,
        }) = object.unbind().get_property_storage_mut(agent)
        {
            let result = keys
                .iter()
                .enumerate()
                .find(|(_, k)| **k == key)
                .map(|res| res.0 as u32);
            if let Some(index) = result {
                // Mutating existing property.
                let value_entry = values.get_mut(index as usize).unwrap();
                *value_entry = value.unbind();
                match (descriptors, element_descriptor) {
                    (e, Some(element_descriptor)) => {
                        let e = e.or_insert_with(|| AHashMap::with_capacity(1));
                        e.insert(index, element_descriptor.unbind());
                    }
                    (Entry::Occupied(mut e), None) => {
                        let descs = e.get_mut();
                        descs.remove(&index);
                        if descs.is_empty() {
                            e.remove();
                        }
                    }
                    _ => {}
                }
                return;
            }
            keys.len() as u32
        } else {
            0
        };
        let new_len = cur_len.checked_add(1).unwrap();
        let old_shape = object.get_shape(agent);
        let new_shape = old_shape.get_child_shape(agent, key);
        agent.heap.alloc_counter += core::mem::size_of::<Option<Value>>()
            + if element_descriptor.is_some() {
                core::mem::size_of::<(u32, ElementDescriptor)>()
            } else {
                0
            };
        if new_shape != old_shape {
            agent[object].set_shape(new_shape);
        }
        object.reserve(agent, new_len);
        agent[object].set_len(new_len);
        let PropertyStorageMut {
            keys: _,
            values,
            descriptors,
        } = object.get_property_storage_mut(agent).unwrap();
        debug_assert!(
            values[cur_len as usize].is_none()
                && match &descriptors {
                    Entry::Occupied(e) => {
                        !e.get().contains_key(&cur_len)
                    }
                    Entry::Vacant(_) => true,
                }
        );
        values[cur_len as usize] = value.unbind();
        if let Some(element_descriptor) = element_descriptor {
            let descriptors = descriptors.or_insert_with(|| AHashMap::with_capacity(1));
            descriptors.insert(cur_len, element_descriptor.unbind());
        }
        if old_shape == new_shape {
            // Intrinsic shape! Adding a new property to an intrinsic needs to
            // invalidate any NOT_FOUND caches for the added key.
            Caches::invalidate_caches_on_intrinsic_shape_property_addition(
                agent, o, old_shape, key, cur_len, gc,
            );
        }
    }

    pub fn remove(self, agent: &mut Agent, o: Object, key: PropertyKey<'a>) {
        let object = self.0;

        let PropertyStorageMut {
            keys,
            values,
            descriptors,
        } = object.unbind().get_property_storage_mut(agent).unwrap();

        let result = keys
            .iter()
            .enumerate()
            .find(|(_, k)| **k == key)
            .map(|res| res.0);
        let Some(index) = result else {
            // No match; nothing to delete.
            return;
        };
        // Note: keys can only go up to 2^32.
        let index = index as u32;
        let old_len = keys.len() as u32;
        let new_len = old_len - 1;
        if index == new_len {
            // Removing last property.
            values[index as usize] = None;
            if let Entry::Occupied(mut e) = descriptors {
                let descs = e.get_mut();
                descs.remove(&index);
                if descs.is_empty() {
                    e.remove();
                }
            }
        } else {
            // Removing indexed property.
            // First overwrite the noted index with subsequent values.
            values.copy_within((index as usize) + 1.., index as usize);
            *values.last_mut().unwrap() = None;
            // Then move our descriptors if found.
            if let Entry::Occupied(mut e) = descriptors {
                let descs = e.get_mut();
                descs.remove(&(index as u32));
                if descs.is_empty() {
                    e.remove();
                } else {
                    let extracted = descs
                        .extract_if(|k, _| *k > index as u32)
                        .collect::<Vec<_>>();
                    for (k, v) in extracted {
                        descs.insert(k.wrapping_sub(1), v);
                    }
                }
            }
        }
        let old_shape = object.get_shape(agent);
        let new_shape = old_shape.get_shape_with_removal(agent, index);
        agent[object].set_len(new_len);
        if old_shape == new_shape {
            // Shape did not change with removal: this is an intrinsic shape!
            // We must invalidate any property lookup cahces associated with
            // the removed and subsequent property indexes.
            agent
                .heap
                .caches
                .invalidate_caches_on_intrinsic_shape_property_removal(o, old_shape, index);
        } else {
            agent[object].set_shape(new_shape);
        }
    }
}

#[derive(Debug)]
pub struct Entries<'a> {
    pub realm: Ref<'a, RealmRecord<'static>>,
}

impl<'a> Entries<'a> {
    fn new(realm: Ref<'a, RealmRecord<'static>>) -> Self {
        Self { realm }
    }
}
