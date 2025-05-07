// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cell::Ref;

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, RealmRecord},
        types::{IntoValue, PropertyDescriptor, Value},
    },
    engine::context::Bindable,
    heap::element_array::ElementDescriptor,
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
