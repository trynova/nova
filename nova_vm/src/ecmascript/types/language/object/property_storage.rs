// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cell::Ref;
use std::{
    collections::{TryReserveError, hash_map::Entry},
    ptr::NonNull,
};

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
            ElementDescriptor, ElementStorageMut, ElementStorageUninit, PropertyStorageMut,
            PropertyStorageRef,
        },
        indexes::ElementIndex,
    },
};

use super::{InternalSlots, IntoObject, Object, OrdinaryObject, PropertyKey};

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
    pub(crate) fn add_private_field_slot(
        self,
        agent: &mut Agent,
        private_name: PrivateName,
    ) -> Result<(), TryReserveError> {
        // SAFETY: Private fields are backed by on-stack data; mutating Agent
        // is totally okay.
        unsafe {
            Self::insert_private_fields(
                agent,
                self.0,
                NonNull::from(&[PrivateField::Field { key: private_name }]),
            )
        }
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
        if object
            .get_direct(objects)
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
        if let Err(err) = unsafe { Self::insert_private_fields(agent, self.0, private_fields) } {
            return Err(agent.throw_allocation_exception(err, gc));
        };
        Ok(())
    }

    /// Insert a list of private fields to an object.
    ///
    /// ## Safety
    ///
    /// The private_fields must not be backed by memory in the Agent heap's
    /// Elements or Object Shape related vectors.
    ///
    /// The method will read from the private_fields parameter but does not
    /// mutate them. The method also does not touch the Agent's environments at
    /// all. As a result, it is safe to pass in private fields backed by a
    /// PrivateEnvironment held in the Agent.
    unsafe fn insert_private_fields(
        agent: &mut Agent,
        object: OrdinaryObject,
        private_fields: NonNull<[PrivateField]>,
    ) -> Result<(), TryReserveError> {
        let old_len = object.len(agent);
        let new_len = old_len.checked_add(private_fields.len() as u32).unwrap();
        let old_shape = object.object_shape(agent);
        let mut elements_vector = object.get_elements_vector(agent);
        elements_vector.reserve(&mut agent.heap.elements, new_len)?;
        // SAFETY: User says so.
        let (new_shape, insertion_index) =
            unsafe { old_shape.add_private_fields(agent, private_fields) };
        // SAFETY: insertion index is <= old_len; old_len + private_fields.len() was checked.
        let insertion_end_index = unsafe { insertion_index.unchecked_add(private_fields.len()) };
        // Note: use saturating_mul to avoid a panic site.
        agent.heap.alloc_counter +=
            core::mem::size_of::<Option<Value>>().saturating_mul(private_fields.len());
        elements_vector.len = new_len;
        let ElementStorageMut {
            values,
            mut descriptors,
        } = elements_vector.get_storage_mut(agent);

        if insertion_index != old_len as usize {
            // Shift keys over by necessary amount.
            // Then do the same for values.
            values.copy_within(insertion_index..old_len as usize, insertion_end_index);
            // Finally, shift descriptors over if we have any.
            if let Entry::Occupied(d) = &mut descriptors {
                let d = d.get_mut();
                let lower_bound = insertion_index as u32;
                let upper_bound = old_len;
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
            let descriptors = descriptors.or_default();
            descriptors.reserve(methods_count);
            for (i, private_field) in private_fields.iter().enumerate() {
                if private_field.is_method() {
                    let k = (insertion_index + i) as u32;
                    descriptors.insert(k, private_field.into_element_descriptor().unbind());
                }
            }
        }
        let data = object.get_mut(agent);
        data.set_shape(new_shape);
        data.set_values(elements_vector.elements_index.unbind());
        Ok(())
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

    pub fn get<'b>(
        self,
        agent: &'b Agent,
        key: PropertyKey,
    ) -> Option<(
        &'b Option<Value<'a>>,
        Option<&'b ElementDescriptor<'a>>,
        u32,
    )> {
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
            let value = &values[index];
            let index = index as u32;
            let descriptor = descriptors.and_then(|d| d.get(&index));
            (value, descriptor, index)
        })
    }

    pub fn set(
        self,
        agent: &mut Agent,
        o: Object<'a>,
        key: PropertyKey<'a>,
        descriptor: PropertyDescriptor<'a>,
        gc: NoGcScope,
    ) -> Result<(), TryReserveError> {
        let object = self.0;

        let value = descriptor.value;
        let element_descriptor = ElementDescriptor::from_property_descriptor(descriptor);

        if let Some(PropertyStorageMut {
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
                return Ok(());
            }
        }
        self.push(agent, o, key, value, element_descriptor, gc)
    }

    pub fn push(
        self,
        agent: &mut Agent,
        o: Object<'a>,
        key: PropertyKey<'a>,
        value: Option<Value<'a>>,
        desc: Option<ElementDescriptor<'a>>,
        gc: NoGcScope,
    ) -> Result<(), TryReserveError> {
        let object = self.0;

        let old_len = object.len(agent);
        let new_len = old_len.checked_add(1).unwrap();
        let old_shape = object.object_shape(agent);
        let mut elements_vector = object.get_elements_vector(agent);
        elements_vector.reserve(&mut agent.heap.elements, new_len)?;
        elements_vector.len = new_len;
        let new_shape = old_shape.get_child_shape(agent, key);
        agent.heap.alloc_counter += core::mem::size_of::<Option<Value>>()
            + if desc.is_some() {
                core::mem::size_of::<(u32, ElementDescriptor)>()
            } else {
                0
            };
        let ElementStorageMut {
            values,
            descriptors,
        } = elements_vector.get_storage_mut(agent);
        debug_assert!(
            values[old_len as usize].is_none()
                && match &descriptors {
                    Entry::Occupied(e) => {
                        !e.get().contains_key(&old_len)
                    }
                    Entry::Vacant(_) => true,
                }
        );
        values[old_len as usize] = value.unbind();
        if let Some(desc) = desc {
            let descriptors = descriptors.or_insert_with(|| AHashMap::with_capacity(1));
            descriptors.insert(old_len, desc.unbind());
        }
        if old_shape == new_shape {
            // Intrinsic shape! Adding a new property to an intrinsic needs to
            // invalidate any NOT_FOUND caches for the added key.
            Caches::invalidate_caches_on_intrinsic_shape_property_addition(
                agent, o, old_shape, key, old_len, gc,
            );
        }
        let data = object.get_mut(agent);
        data.set_shape(new_shape);
        data.set_values(elements_vector.elements_index.unbind());
        if cfg!(debug_assertions) {
            assert_eq!(object.len(agent), new_len);
            assert_eq!(object.len(agent), elements_vector.len());
            assert_eq!(
                object.object_shape(agent).values_capacity(agent).capacity(),
                elements_vector.cap(),
                "{}",
                key.as_display(agent)
            );
            assert_eq!(
                object.get(agent).get_values(),
                elements_vector.elements_index
            );
            let property_storage = object.get_property_storage(agent);
            assert_eq!(property_storage.keys.len(), new_len as usize);
            assert_eq!(property_storage.keys.last(), Some(&key));
            assert_eq!(property_storage.values.last(), Some(&value));
        }
        Ok(())
    }

    pub fn remove(self, agent: &mut Agent, o: Object, key: PropertyKey<'a>) {
        let object = self.0;

        let old_shape = object.object_shape(agent);
        let old_cap = old_shape.values_capacity(agent);

        let keys = old_shape.keys(&agent.heap.object_shapes, &agent.heap.elements);

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

        let new_shape = old_shape.get_shape_with_removal(agent, index);
        let new_cap = new_shape.values_capacity(agent);

        if new_cap != old_cap {
            // We need to perform a copy with this removal, as it changes the
            // capacity of the shape and thus the object.
            // Note: we purposefully check new_cap from shape after removal,
            // allowing intrinsic object to be temporarily invalid here. This
            // is because it's possible that the new shape is actually
            // allocated based on some much larger shape and overallocates a
            // bunch. That's intentional (at least for now).
            let new_values = agent.heap.elements.realloc_values_with_removal(
                old_cap,
                object.get(agent).get_values(),
                new_cap,
                old_len,
                index,
            );
            object.get_mut(agent).set_values(new_values);
        } else {
            // Capacity of the property storage isn't changing, so we can
            // perform the deletion directly in the storage.
            let ElementStorageUninit {
                values,
                descriptors,
            } = object.unbind().get_elements_storage_uninit(agent);

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
                    descs.remove(&index);
                    if descs.is_empty() {
                        e.remove();
                    } else {
                        let extracted = descs.extract_if(|k, _| *k > index).collect::<Vec<_>>();
                        for (k, v) in extracted {
                            descs.insert(k - 1, v);
                        }
                    }
                }
            }
        }

        if old_shape == new_shape {
            // Shape did not change with removal: this is an intrinsic shape!
            // We must invalidate any property lookup caches associated with
            // the removed and subsequent property indexes.
            Caches::invalidate_caches_on_intrinsic_shape_property_removal(
                agent, o, old_shape, index,
            );
        } else {
            object.get_mut(agent).set_shape(new_shape);
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
