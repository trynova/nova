// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cell::Ref;
use std::{cmp::Ordering, collections::hash_map::Entry};

use ahash::AHashMap;

use crate::{
    Heap,
    ecmascript::{
        builtins::ordinary::shape::ObjectShape,
        execution::{Agent, JsResult, PrivateField, RealmRecord, agent::ExceptionType},
        types::{IntoValue, PrivateName, PropertyDescriptor, Value},
    },
    engine::context::{Bindable, NoGcScope},
    heap::{
        element_array::{
            ElementArrayKey, ElementDescriptor, ElementStorageUninit, PropertyStorageMut,
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
    pub(crate) fn add_private_field_slot(self, agent: &mut Agent, private_name: PrivateName) {
        Self::insert_private_fields(agent, self.0, &[PrivateField::Field { key: private_name }]);
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
        let private_fields = private_fields as *const _;
        // SAFETY: insert_private_fields does not touch environments, and it
        // cannot push into the private_fields list. Hence the pointer here is
        // always valid.
        let private_fields = unsafe { &*private_fields };
        Self::insert_private_fields(agent, self.0, private_fields);
        Ok(())
    }

    fn insert_private_fields(
        agent: &mut Agent,
        object: OrdinaryObject,
        private_fields: &[PrivateField],
    ) {
        let original_len = object.len(agent);
        let original_shape = object.get_shape(agent);
        let insertion_index = if original_len == 0 {
            // Property storage is currently empty: We don't need to do any
            // shifting of existing properties.
            0
        } else {
            original_shape.keys(&agent.heap.object_shapes, &agent.heap.elements)
                [..original_len as usize]
                .binary_search_by(|k| {
                    if k.is_private_name() {
                        Ordering::Less
                    } else {
                        // Our PrivateName should be inserted before the first
                        // normal property.
                        Ordering::Greater
                    }
                })
                .unwrap_err()
        };
        let insertion_end_index = insertion_index.wrapping_add(private_fields.len());
        let prototype = original_shape.get_prototype(agent);
        let root_shape = ObjectShape::get_or_create_shape_for_prototype(agent, prototype);
        // Note: use saturating_mul to avoid a panic site.
        agent.heap.alloc_counter +=
            core::mem::size_of::<Option<Value>>().saturating_mul(private_fields.len());
        let cap = ElementArrayKey::from(original_len);
        let keys_index = original_shape.get_keys(agent);
        let new_shape = root_shape.get_or_create_child_shape(
            agent,
            prototype,
            original_len
                .checked_add(private_fields.len() as u32)
                .expect("Unreasonable amount of fields") as usize,
            |elements, i| {
                if i < insertion_index {
                    elements.get_keys_raw(cap, keys_index, original_len)[i].unbind()
                } else if i < insertion_index + private_fields.len() {
                    private_fields[i.wrapping_sub(insertion_index)]
                        .get_key()
                        .into()
                } else {
                    elements.get_keys_raw(cap, keys_index, original_len)[i - private_fields.len()]
                        .unbind()
                }
            },
            |elements, new_len| {
                let (cap, index) = elements.allocate_keys_with_capacity(new_len);
                let keys_memory =
                    elements.get_keys_uninit_raw(cap, index) as *mut [Option<PropertyKey>];
                let original_keys = elements.get_keys_raw(cap, keys_index, original_len);
                // SAFETY: original_keys and keys_memory cannot overlap as
                // keys_memory was just allocated; accessing original_keys
                // cannot invalidate the keys_memory pointer.
                let keys_memory = unsafe { &mut *keys_memory };
                // Previous existing private keys in original_keys
                for (slot, key) in keys_memory[0..insertion_index]
                    .iter_mut()
                    .zip(&original_keys[0..insertion_index])
                {
                    *slot = Some(key.unbind())
                }
                // Added private keys
                for (slot, key) in keys_memory[insertion_index..insertion_end_index]
                    .iter_mut()
                    .zip(private_fields.iter().map(|e| e.get_key()))
                {
                    *slot = Some(key.into())
                }
                // Previous existing normal keys in original_keys
                for (slot, key) in keys_memory[insertion_end_index..]
                    .iter_mut()
                    .zip(&original_keys[insertion_index..])
                {
                    *slot = Some(key.unbind())
                }
                (cap, index)
            },
        );
        agent[object].set_shape(new_shape);
        // SAFETY: We do set the shape after this.
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
    pub(crate) fn private_element_find_mut<'b>(
        self,
        agent: &'b mut Agent,
        private_name: PrivateName,
    ) -> Option<(
        Option<&'b mut Value<'static>>,
        Option<&'b ElementDescriptor<'static>>,
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

    pub fn get(self, agent: &Agent, key: PropertyKey) -> Option<PropertyDescriptor<'a>> {
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
            let value = values.get(index).unwrap().unbind();
            let descriptor = descriptors.and_then(|d| d.get(&(index as u32))).cloned();
            ElementDescriptor::to_property_descriptor(descriptor, value)
        })
    }

    pub fn set(self, agent: &mut Agent, key: PropertyKey<'a>, descriptor: PropertyDescriptor<'a>) {
        let object = self.0;
        if let Some(PropertyStorageMut {
            keys,
            values,
            descriptors,
        }) = object.unbind().get_property_storage_mut(agent)
        {
            let value = descriptor.value;
            let element_descriptor = ElementDescriptor::from_property_descriptor(descriptor);

            let result = keys
                .iter()
                .enumerate()
                .find(|(_, k)| **k == key)
                .map(|res| res.0);
            if let Some(index) = result {
                let value_entry = values.get_mut(index).unwrap();
                *value_entry = value.unbind();
                match (descriptors, element_descriptor) {
                    (e, Some(element_descriptor)) => {
                        let e = e.or_insert_with(|| AHashMap::with_capacity(1));
                        e.insert(index as u32, element_descriptor.unbind());
                    }
                    (Entry::Occupied(mut e), None) => {
                        let descs = e.get_mut();
                        descs.remove(&(index as u32));
                        if descs.is_empty() {
                            e.remove();
                        }
                    }
                    _ => {}
                }
            } else {
                let cur_len = keys.len() as u32;
                let new_len = cur_len.checked_add(1).expect("Absurd number of properties");
                object.reserve(agent, new_len);
                let prototype = object.internal_prototype(agent);
                let shape = object.get_shape(agent);
                let shape_cap = shape.get_cap(agent);
                let shape_keys = shape.get_keys(agent);
                let shape = shape.get_or_create_child_shape(
                    agent,
                    prototype,
                    new_len as usize,
                    |_, _| key.unbind(),
                    |elements, len| {
                        let last_index = len.wrapping_sub(1);
                        let keys_memory = elements.get_keys_uninit_raw(shape_cap, shape_keys);
                        if keys_memory.get(last_index) == Some(&None) {
                            // We can just extend the current keys memory with a
                            // new key.
                            keys_memory[last_index] = Some(key.unbind());
                            return (shape_cap, shape_keys.unbind());
                        }
                        let (new_keys_cap, new_keys_index) =
                            elements.allocate_keys_with_capacity(len);
                        let new_keys_memory =
                            elements.get_keys_uninit_raw(new_keys_cap, new_keys_index);
                        new_keys_memory[last_index] = Some(key.unbind());
                        let new_keys_memory =
                            new_keys_memory as *mut [Option<PropertyKey<'static>>];
                        let keys_memory =
                            elements.get_keys_raw(shape_cap, shape_keys, last_index as u32);
                        // SAFETY: keys_memory and new_keys_memory necessarily
                        // point to different slices; getting keys_memory cannot
                        // invalidate new_keys_memory.
                        let new_keys_memory = unsafe { &mut *new_keys_memory };
                        for (slot, key) in new_keys_memory[..last_index].iter_mut().zip(keys_memory)
                        {
                            *slot = Some(key.unbind());
                        }
                        (new_keys_cap, new_keys_index)
                    },
                );
                agent.heap.alloc_counter += core::mem::size_of::<Option<Value>>();
                agent[object].set_len(new_len);
                agent[object].set_shape(shape);
                let PropertyStorageMut {
                    keys: _,
                    values,
                    descriptors,
                } = object.get_property_storage_mut(agent).unwrap();
                let index = cur_len;
                values[index as usize] = value.unbind();
                if let Some(element_descriptor) = element_descriptor {
                    let descriptors = descriptors.or_insert_with(|| AHashMap::with_capacity(1));
                    descriptors.insert(index, element_descriptor.unbind());
                }
            };
        } else {
            let prototype = object.internal_prototype(agent);
            let shape = object.get_shape(agent);
            let shape = shape.get_or_create_child_shape(
                agent,
                prototype,
                1,
                |_, _| key.unbind(),
                |elements, _| {
                    let (new_keys_cap, new_keys_index) = elements.allocate_keys_with_capacity(1);
                    let new_keys_memory =
                        elements.get_keys_uninit_raw(new_keys_cap, new_keys_index);
                    new_keys_memory[0] = Some(key.unbind());
                    (new_keys_cap, new_keys_index)
                },
            );
            let value = descriptor.value;
            let element_descriptor = ElementDescriptor::from_property_descriptor(descriptor);

            object.reserve(agent, 1);
            agent[object].set_shape(shape);
            agent[object].set_len(1);
            let PropertyStorageMut {
                keys: _,
                values,
                descriptors,
            } = object.get_property_storage_mut(agent).unwrap();
            values[0] = value.unbind();
            if let Some(element_descriptor) = element_descriptor {
                let descriptors = descriptors.or_insert_with(|| AHashMap::with_capacity(1));
                descriptors.insert(0, element_descriptor.unbind());
            }
        }
    }

    pub fn remove(self, agent: &mut Agent, key: PropertyKey<'a>) {
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
        let old_len = keys.len();
        let new_len = old_len.wrapping_sub(1);
        if index == new_len {
            // Removing last property.
            values[index] = None;
            if let Entry::Occupied(mut e) = descriptors {
                let descs = e.get_mut();
                descs.remove(&(index as u32));
                if descs.is_empty() {
                    e.remove();
                }
            }
            // Fix the shape: if we have a parent shape then that's what we go
            // to. Otherwise, we need to create a new shape like that.
            let parent_shape = object.get_shape(agent).get_parent(agent);
            if let Some(parent_shape) = parent_shape {
                agent[object].set_shape(parent_shape);
                agent[object].set_len(index as u32);
                return;
            } else {
                // No direct parent shape; need to create one. We'll let this
                // fall into the general case.
            }
        } else {
            // Removing indexed property.
            // First overwrite the noted index with subsequent values.
            values.copy_within(index.wrapping_add(1).., index);
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
        let prototype = object.internal_prototype(agent);
        let base_shape = ObjectShape::get_or_create_shape_for_prototype(agent, prototype);
        let shape = object.get_shape(agent);
        let shape_cap = shape.get_cap(agent);
        let shape_keys = shape.get_keys(agent);
        let old_len = shape.get_length(agent);
        let new_shape = base_shape.get_or_create_child_shape(
            agent,
            prototype,
            new_len,
            |elements, i| {
                // When we reach our removal index, we start
                // indexing off-by-one.
                let i = if i < index { i } else { i.wrapping_add(1) };
                elements.get_keys_raw(shape_cap, shape_keys, old_len)[i].unbind()
            },
            |elements, len| {
                let (new_keys_cap, new_keys_index) = elements.allocate_keys_with_capacity(len);
                let keys_memory =
                    elements.get_keys_raw(shape_cap, shape_keys, old_len) as *const [PropertyKey];
                let new_keys_memory = elements.get_keys_uninit_raw(new_keys_cap, new_keys_index);
                // SAFETY: keys_memory and new_keys_memory necessarily
                // point to different slices; getting new_keys_memory cannot
                // invalidate keys_memory.
                let keys_memory = unsafe { &*keys_memory };
                assert_eq!(keys_memory.len(), len.wrapping_add(1));
                assert!(new_keys_memory.len() >= len);
                for i in 0..index {
                    new_keys_memory[i] = Some(keys_memory[i].unbind());
                }
                for i in index..len {
                    new_keys_memory[i] = Some(keys_memory[i.wrapping_add(1)].unbind());
                }
                (new_keys_cap, new_keys_index)
            },
        );
        agent[object].set_shape(new_shape);
        agent[object].set_len(new_len as u32);
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
