// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cell::Ref;
use std::{cmp::Ordering, collections::hash_map::Entry};

use ahash::AHashMap;

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, PrivateField, RealmRecord},
        types::{IntoValue, PrivateName, PropertyDescriptor, Value},
    },
    engine::context::{Bindable, NoGcScope},
    heap::element_array::{ElementArrays, ElementDescriptor, PropertyStorageVector},
};

use super::{IntoObject, Object, OrdinaryObject, PropertyKey};

#[derive(Debug, Clone, Copy)]
pub struct PropertyStorage<'a>(OrdinaryObject<'a>);

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
        let object = self.0;
        let Heap {
            elements,
            objects,
            alloc_counter,
            ..
        } = &mut agent.heap;
        let props = &mut objects[object].property_storage;

        if props.is_empty() {
            println!("Pushing one private element");
            *alloc_counter += core::mem::size_of::<Option<Value>>() * 2;
            props.push(elements, private_name.into(), None, None);
            return;
        }

        Self::insert_private_fields(
            props,
            elements,
            alloc_counter,
            &[PrivateField::Field { key: private_name }],
        );
    }

    /// Copy all PrivateMethods and reserve PrivateName fields from the current
    /// private environment to this object property storage.
    pub(crate) fn initialize_private_elements(self, agent: &mut Agent, gc: NoGcScope) {
        let private_env = agent
            .current_private_environment(gc)
            .expect("Expected PrivateEnvironment to be set");
        let object = self.0;
        let Heap {
            environments,
            elements,
            objects,
            alloc_counter,
            ..
        } = &mut agent.heap;
        let private_env = environments.get_private_environment(private_env);
        let props = &mut objects[object].property_storage;
        let private_methods = private_env.get_instance_private_methods(gc);
        Self::insert_private_fields(props, elements, alloc_counter, private_methods);
    }

    fn insert_private_fields(
        props: &mut PropertyStorageVector,
        elements: &mut ElementArrays,
        alloc_counter: &mut usize,
        private_fields: &[PrivateField<'_>],
    ) {
        let len = props.len();
        // Note: use saturating_mul to avoid a panic site.
        *alloc_counter +=
            core::mem::size_of::<Option<Value>>().saturating_mul(private_fields.len());
        props.reserve(elements, len + private_fields.len() as u32);
        let (keys, values, mut descriptors) = props.get_storage_uninit(elements);

        let start_index = if len == 0 {
            // Property storage is currently empty: We don't need to do any
            // shifting of existing properties.
            0
        } else {
            let first_normal_property_index = keys[..len as usize]
                .binary_search_by(|k| {
                    if k.is_some_and(|k| k.is_private_key()) {
                        Ordering::Greater
                    } else {
                        // Nones appear at the end of the keys list. Our PrivateName
                        // should be inserted before the first normal property.
                        Ordering::Less
                    }
                })
                .unwrap();
            // Shift keys over by necessary amount.
            let end_index = first_normal_property_index + private_fields.len();
            keys.copy_within(first_normal_property_index..len as usize, end_index);
            // Then do the same for values.
            values.copy_within(first_normal_property_index..len as usize, end_index);
            // Finally, shift descriptors over if we have any.
            if let Entry::Occupied(d) = &mut descriptors {
                let d = d.get_mut();
                let lower_bound = first_normal_property_index as u32;
                let upper_bound = len;
                let range = lower_bound..upper_bound;
                let keys_to_shift = d
                    .keys()
                    .filter(|k| range.contains(k))
                    .copied()
                    .collect::<Vec<_>>();
                for i in keys_to_shift {
                    let desc = d.remove(&i).unwrap();
                    d.insert(i + private_fields.len() as u32, desc);
                }
            }
            first_normal_property_index
        };

        // Fill the keys and values with our PrivateNames starting at our found
        // index and ending at found index + number of private elements.
        let mut methods_count = 0;
        for ((key_slot, value_slot), private_element) in keys
            [start_index..start_index + private_fields.len()]
            .iter_mut()
            .zip(&mut values[start_index..start_index + private_fields.len()])
            .zip(private_fields)
        {
            *key_slot = Some(private_element.get_key().into());
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
                    let k = (start_index + i) as u32;
                    descriptors.insert(k, private_field.into_element_descriptor().unbind());
                }
            }
        }
        props.len = len + 1;
    }

    pub(crate) fn set_private_field_value(
        self,
        agent: &mut Agent,
        private_name: PrivateName,
        value: Value,
    ) {
        let object = self.0;
        let Heap {
            elements, objects, ..
        } = &mut agent.heap;
        let props = &mut objects[object].property_storage;
        let (keys, values, mut descriptors) = props.get_storage_uninit(elements);
    }

    pub fn get(self, agent: &Agent, key: PropertyKey) -> Option<PropertyDescriptor<'a>> {
        let object = self.0;
        let props = &agent[object].property_storage;
        let result = props
            .keys(agent)
            .iter()
            .enumerate()
            .find(|(_, k)| **k == key)
            .map(|res| res.0);
        result.map(|index| {
            let value = props.values(agent).get(index).unwrap().unbind();
            let descriptor = agent.heap.elements.get_descriptor(props, index).unbind();
            println!(
                "Keys, {:?}, Values and Descriptors: {:?}",
                props.keys(agent),
                agent.heap.elements.get_descriptors_and_values(props)
            );
            ElementDescriptor::to_property_descriptor(descriptor, value)
        })
    }

    pub fn set(self, agent: &mut Agent, key: PropertyKey, descriptor: PropertyDescriptor) {
        let object = self.0;
        let Heap {
            elements,
            objects,
            alloc_counter,
            ..
        } = &mut agent.heap;
        let props = &mut objects[object].property_storage;

        let value = descriptor.value;
        let element_descriptor = ElementDescriptor::from_property_descriptor(descriptor);

        let result = props
            .keys(elements)
            .iter()
            .enumerate()
            .find(|(_, k)| **k == key)
            .map(|res| res.0);
        if let Some(index) = result {
            let key_entry = props.keys_mut(elements).get_mut(index).unwrap();
            *key_entry = Some(key.unbind());
            let value_entry = props.values_mut(elements).get_mut(index).unwrap();
            *value_entry = value.unbind();
            elements.set_descriptor(props, index, element_descriptor);
        } else {
            *alloc_counter += core::mem::size_of::<Option<Value>>() * 2;
            props.push(elements, key, value, element_descriptor);
        };
    }

    pub fn remove(self, agent: &mut Agent, key: PropertyKey) {
        let object = self.0;

        let Heap {
            elements, objects, ..
        } = &mut agent.heap;
        let props = &mut objects[object].property_storage;

        let result = props
            .keys(elements)
            .iter()
            .enumerate()
            .find(|(_, k)| **k == key)
            .map(|res| res.0);
        if let Some(index) = result {
            props.remove(elements, index);
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
