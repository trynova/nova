// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{Builtin, BuiltinFunction, BuiltinGetter, BuiltinIntrinsic},
        execution::{Agent, Realm},
        types::{
            BUILTIN_STRING_MEMORY, IntoFunction, IntoObject, IntoValue, ObjectHeapData,
            OrdinaryObject, PropertyKey, Value,
        },
    },
    heap::{element_array::ElementDescriptor, indexes::ObjectIndex},
};

use super::{
    builtin_function_builder::BuiltinFunctionBuilder,
    property_builder::{self, PropertyBuilder},
};

#[derive(Default, Clone, Copy)]
pub struct NoPrototype;

#[derive(Clone, Copy)]
pub struct CreatorPrototype<T: IntoObject<'static>>(T);

#[derive(Default, Clone, Copy)]
pub struct NoProperties;

#[derive(Clone)]
pub struct CreatorProperties(
    Vec<(
        PropertyKey<'static>,
        Option<ElementDescriptor<'static>>,
        Option<Value<'static>>,
    )>,
);

pub struct OrdinaryObjectBuilder<'agent, P, Pr> {
    pub(crate) agent: &'agent mut Agent,
    this: OrdinaryObject<'static>,
    realm: Realm<'static>,
    prototype: P,
    extensible: bool,
    properties: Pr,
}

impl<'agent> OrdinaryObjectBuilder<'agent, NoPrototype, NoProperties> {
    #[must_use]
    pub fn new(agent: &'agent mut Agent, realm: Realm<'static>) -> Self {
        agent.heap.objects.push(None);
        let this = ObjectIndex::last(&agent.heap.objects).into();
        Self {
            agent,
            this,
            realm,
            prototype: NoPrototype,
            extensible: true,
            properties: NoProperties,
        }
    }

    #[must_use]
    pub(crate) fn new_intrinsic_object(
        agent: &'agent mut Agent,
        realm: Realm<'static>,
        this: OrdinaryObject<'static>,
    ) -> Self {
        Self {
            agent,
            this,
            realm,
            prototype: NoPrototype,
            extensible: true,
            properties: NoProperties,
        }
    }
}

impl<P, Pr> OrdinaryObjectBuilder<'_, P, Pr> {
    #[must_use]
    pub fn with_extensible(self, extensible: bool) -> Self {
        Self {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible,
            properties: self.properties,
        }
    }
}

impl<'agent, Pr> OrdinaryObjectBuilder<'agent, NoPrototype, Pr> {
    #[must_use]
    pub fn with_prototype<T: IntoObject<'static>>(
        self,
        prototype: T,
    ) -> OrdinaryObjectBuilder<'agent, CreatorPrototype<T>, Pr> {
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: CreatorPrototype(prototype),
            extensible: self.extensible,
            properties: self.properties,
        }
    }
}

impl<'agent, P> OrdinaryObjectBuilder<'agent, P, NoProperties> {
    #[must_use]
    pub fn with_property_capacity(
        self,
        cap: usize,
    ) -> OrdinaryObjectBuilder<'agent, P, CreatorProperties> {
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible: self.extensible,
            properties: CreatorProperties(Vec::with_capacity(cap)),
        }
    }
}

impl<P> OrdinaryObjectBuilder<'_, P, CreatorProperties> {
    #[must_use]
    pub fn with_data_property(mut self, key: PropertyKey<'static>, value: Value<'static>) -> Self {
        self.properties.0.push((key, None, Some(value)));
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible: self.extensible,
            properties: self.properties,
        }
    }

    #[must_use]
    pub fn with_property(
        mut self,
        creator: impl FnOnce(
            PropertyBuilder<'_, property_builder::NoKey, property_builder::NoDefinition>,
        ) -> (
            PropertyKey<'static>,
            Option<ElementDescriptor<'static>>,
            Option<Value<'static>>,
        ),
    ) -> Self {
        let builder = PropertyBuilder::new(self.agent);
        let property = creator(builder);
        self.properties.0.push(property);
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible: self.extensible,
            properties: self.properties,
        }
    }

    #[must_use]
    pub fn with_constructor_property(mut self, constructor: BuiltinFunction<'static>) -> Self {
        let property = PropertyBuilder::new(self.agent)
            .with_enumerable(false)
            .with_key(BUILTIN_STRING_MEMORY.constructor.into())
            .with_value(constructor.into_value())
            .build();
        self.properties.0.push(property);
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible: self.extensible,
            properties: self.properties,
        }
    }

    #[must_use]
    pub fn with_builtin_function_property<T: Builtin>(mut self) -> Self {
        let (value, key) = {
            let mut builder = BuiltinFunctionBuilder::new::<T>(self.agent, self.realm);
            let name = T::KEY.unwrap_or_else(|| PropertyKey::from(builder.get_name()));
            (builder.build().into_value(), name)
        };
        let builder = PropertyBuilder::new(self.agent)
            .with_key(key)
            .with_configurable(T::CONFIGURABLE)
            .with_enumerable(T::ENUMERABLE);
        let property = if T::WRITABLE {
            builder.with_value(value).build()
        } else {
            builder.with_value_readonly(value).build()
        };
        self.properties.0.push(property);
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible: self.extensible,
            properties: self.properties,
        }
    }

    #[must_use]
    pub(crate) fn with_builtin_intrinsic_function_property<T: BuiltinIntrinsic>(mut self) -> Self {
        let (value, key) = {
            let mut builder =
                BuiltinFunctionBuilder::new_intrinsic_function::<T>(self.agent, self.realm);
            let name = T::KEY.unwrap_or_else(|| PropertyKey::from(builder.get_name()));
            (builder.build().into_value(), name)
        };
        let builder = PropertyBuilder::new(self.agent)
            .with_key(key)
            .with_configurable(T::CONFIGURABLE)
            .with_enumerable(T::ENUMERABLE);
        let property = if T::WRITABLE {
            builder.with_value(value).build()
        } else {
            builder.with_value_readonly(value).build()
        };
        self.properties.0.push(property);
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible: self.extensible,
            properties: self.properties,
        }
    }

    #[must_use]
    pub(crate) fn with_builtin_function_getter_property<T: BuiltinGetter>(mut self) -> Self {
        let getter_function = BuiltinFunctionBuilder::new::<T>(self.agent, self.realm)
            .build()
            .into_function();
        let property = PropertyBuilder::new(self.agent)
            .with_key(T::KEY.unwrap())
            .with_getter_function(getter_function)
            .with_configurable(T::CONFIGURABLE)
            .with_enumerable(T::ENUMERABLE)
            .build();
        self.properties.0.push(property);
        OrdinaryObjectBuilder {
            agent: self.agent,
            this: self.this,
            realm: self.realm,
            prototype: self.prototype,
            extensible: self.extensible,
            properties: self.properties,
        }
    }
}

impl OrdinaryObjectBuilder<'_, NoPrototype, NoProperties> {
    pub fn build(self) -> OrdinaryObject<'static> {
        let (keys, values) = self.agent.heap.elements.create_with_stuff(vec![]);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: self.extensible,
            prototype: None,
            keys,
            values,
        });
        self.this
    }
}

impl<T: IntoObject<'static>> OrdinaryObjectBuilder<'_, CreatorPrototype<T>, NoProperties> {
    pub fn build(self) -> OrdinaryObject<'static> {
        let (keys, values) = self.agent.heap.elements.create_with_stuff(vec![]);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: self.extensible,
            prototype: Some(self.prototype.0.into_object()),
            keys,
            values,
        });
        self.this
    }
}

impl OrdinaryObjectBuilder<'_, NoPrototype, CreatorProperties> {
    pub fn build(self) -> OrdinaryObject<'static> {
        assert_eq!(self.properties.0.len(), self.properties.0.capacity());
        {
            let slice = self.properties.0.as_slice();
            let duplicate = (1..slice.len()).find(|first_index| {
                slice[*first_index..]
                    .iter()
                    .any(|(key, _, _)| *key == slice[first_index - 1].0)
            });
            if let Some(index) = duplicate {
                panic!("Duplicate key found: {:?}", slice[index].0);
            }
        }
        let (keys, values) = self
            .agent
            .heap
            .elements
            .create_with_stuff(self.properties.0);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: self.extensible,
            prototype: None,
            keys,
            values,
        });
        self.this
    }
}

impl<T: IntoObject<'static>> OrdinaryObjectBuilder<'_, CreatorPrototype<T>, CreatorProperties> {
    pub fn build(self) -> OrdinaryObject<'static> {
        assert_eq!(self.properties.0.len(), self.properties.0.capacity());
        {
            let slice = self.properties.0.as_slice();
            let duplicate = (1..slice.len()).find(|first_index| {
                slice[*first_index..]
                    .iter()
                    .any(|(key, _, _)| *key == slice[first_index - 1].0)
            });
            if let Some(index) = duplicate {
                panic!("Duplicate key found: {:?}", slice[index].0);
            }
        }
        let (keys, values) = self
            .agent
            .heap
            .elements
            .create_with_stuff(self.properties.0);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: self.extensible,
            prototype: Some(self.prototype.0.into_object()),
            keys,
            values,
        });
        self.this
    }
}
