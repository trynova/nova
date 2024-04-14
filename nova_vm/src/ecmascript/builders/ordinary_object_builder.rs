use crate::{
    ecmascript::{
        builtins::{Builtin, BuiltinFunction},
        execution::{Agent, RealmIdentifier},
        types::{
            IntoObject, IntoValue, ObjectHeapData, OrdinaryObject, PropertyKey, Value,
            BUILTIN_STRING_MEMORY,
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
pub struct CreatorPrototype<T: IntoObject>(T);

#[derive(Default, Clone, Copy)]
pub struct NoProperties;

#[derive(Clone)]
pub struct CreatorProperties(Vec<(PropertyKey, Option<ElementDescriptor>, Option<Value>)>);

pub struct OrdinaryObjectBuilder<'agent, P, Pr> {
    pub(crate) agent: &'agent mut Agent,
    this: OrdinaryObject,
    realm: RealmIdentifier,
    prototype: P,
    extensible: bool,
    properties: Pr,
}

impl<'agent> OrdinaryObjectBuilder<'agent, NoPrototype, NoProperties> {
    #[must_use]
    pub fn new(agent: &'agent mut Agent, realm: RealmIdentifier) -> Self {
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
        realm: RealmIdentifier,
        this: OrdinaryObject,
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

impl<'agent, P, Pr> OrdinaryObjectBuilder<'agent, P, Pr> {
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
    pub fn with_prototype<T: IntoObject>(
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

impl<'agent, P> OrdinaryObjectBuilder<'agent, P, CreatorProperties> {
    #[must_use]
    pub fn with_data_property(mut self, key: PropertyKey, value: Value) -> Self {
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
        ) -> (PropertyKey, Option<ElementDescriptor>, Option<Value>),
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
    pub fn with_constructor_property(mut self, constructor: BuiltinFunction) -> Self {
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
            let name = PropertyKey::from(builder.get_name());
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
}

impl<'agent> OrdinaryObjectBuilder<'agent, NoPrototype, NoProperties> {
    pub fn build(self) -> OrdinaryObject {
        let (keys, values) = self.agent.heap.elements.create_with_stuff(vec![]);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.into_index())
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

impl<'agent, T: IntoObject> OrdinaryObjectBuilder<'agent, CreatorPrototype<T>, NoProperties> {
    pub fn build(self) -> OrdinaryObject {
        let (keys, values) = self.agent.heap.elements.create_with_stuff(vec![]);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.into_index())
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

impl<'agent> OrdinaryObjectBuilder<'agent, NoPrototype, CreatorProperties> {
    pub fn build(self) -> OrdinaryObject {
        assert_eq!(self.properties.0.len(), self.properties.0.capacity());
        let (keys, values) = self
            .agent
            .heap
            .elements
            .create_with_stuff(self.properties.0);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.into_index())
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

impl<'agent, T: IntoObject> OrdinaryObjectBuilder<'agent, CreatorPrototype<T>, CreatorProperties> {
    pub fn build(self) -> OrdinaryObject {
        assert_eq!(self.properties.0.len(), self.properties.0.capacity());
        let (keys, values) = self
            .agent
            .heap
            .elements
            .create_with_stuff(self.properties.0);
        let slot = self
            .agent
            .heap
            .objects
            .get_mut(self.this.into_index())
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
