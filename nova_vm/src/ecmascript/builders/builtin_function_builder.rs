use crate::{
    ecmascript::{
        builtins::{Behaviour, Builtin, BuiltinFunction},
        execution::{Agent, RealmIdentifier},
        types::{
            BuiltinFunctionHeapData, IntoObject, Object, ObjectHeapData, PropertyKey, String, Value,
        },
    },
    heap::{
        element_array::ElementDescriptor,
        indexes::{BuiltinFunctionIndex, ObjectIndex},
    },
};

use super::property_builder::{self, PropertyBuilder};

#[derive(Default, Clone, Copy)]
pub struct NoPrototype;

#[derive(Clone, Copy)]
pub struct CreatorPrototype(Object);

#[derive(Default, Clone, Copy)]
pub struct NoLength;

#[derive(Clone, Copy)]
pub struct CreatorLength(u8);

#[derive(Default, Clone, Copy)]
pub struct NoName;

#[derive(Clone, Copy)]
pub struct CreatorName(String);

#[derive(Default, Clone, Copy)]
pub struct NoBehaviour;

#[derive(Clone, Copy)]
pub struct CreatorBehaviour(Behaviour);

#[derive(Default, Clone, Copy)]
pub struct NoProperties;

#[derive(Clone)]
pub struct CreatorProperties(Vec<(PropertyKey, Option<ElementDescriptor>, Option<Value>)>);

pub struct BuiltinFunctionBuilder<'agent, P, L, N, B, Pr> {
    pub(crate) agent: &'agent mut Agent,
    this: BuiltinFunction,
    object_index: Option<ObjectIndex>,
    realm: RealmIdentifier,
    prototype: P,
    length: L,
    name: N,
    behaviour: B,
    properties: Pr,
}

impl<'agent>
    BuiltinFunctionBuilder<'agent, NoPrototype, NoLength, NoName, NoBehaviour, NoProperties>
{
    pub fn new<T: Builtin>(
        agent: &'agent mut Agent,
        realm: RealmIdentifier,
    ) -> BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    > {
        agent.heap.builtin_functions.push(None);
        let this = BuiltinFunctionIndex::last(&agent.heap.builtin_functions).into();
        let name = String::from_str(agent, T::NAME);
        BuiltinFunctionBuilder {
            agent,
            this,
            object_index: None,
            realm,
            prototype: Default::default(),
            length: CreatorLength(T::LENGTH),
            name: CreatorName(name),
            behaviour: CreatorBehaviour(T::BEHAVIOUR),
            properties: Default::default(),
        }
    }

    pub(crate) fn new_intrinsic_constructor<T: Builtin>(
        agent: &'agent mut Agent,
        realm: RealmIdentifier,
        this: BuiltinFunction,
        base_object: Option<ObjectIndex>,
    ) -> BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    > {
        let name = String::from_str(agent, T::NAME);
        BuiltinFunctionBuilder {
            agent,
            this,
            object_index: base_object,
            realm,
            prototype: Default::default(),
            length: CreatorLength(T::LENGTH),
            name: CreatorName(name),
            behaviour: CreatorBehaviour(T::BEHAVIOUR),
            properties: Default::default(),
        }
    }
}

impl<'agent, P, L, N, Pr> BuiltinFunctionBuilder<'agent, P, L, N, NoBehaviour, Pr> {
    pub fn with_behaviour(
        self,
        behaviour: Behaviour,
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, CreatorBehaviour, Pr> {
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: CreatorBehaviour(behaviour),
            properties: self.properties,
        }
    }
}

impl<'agent, L, N, B, Pr> BuiltinFunctionBuilder<'agent, NoPrototype, L, N, B, Pr> {
    pub fn with_prototype(
        self,
        prototype: Object,
    ) -> BuiltinFunctionBuilder<'agent, CreatorPrototype, L, N, B, Pr> {
        let object_index = if prototype
            != self
                .agent
                .get_realm(self.realm)
                .intrinsics()
                .function_prototype()
                .into_object()
            && self.object_index.is_none()
        {
            self.agent.heap.objects.push(None);
            Some(ObjectIndex::last(&self.agent.heap.objects))
        } else {
            self.object_index
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: CreatorPrototype(prototype),
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent, P, N, B, Pr> BuiltinFunctionBuilder<'agent, P, NoLength, N, B, Pr> {
    pub fn with_length(
        self,
        length: u8,
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, N, B, Pr> {
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: CreatorLength(length),
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent, P, L, B, Pr> BuiltinFunctionBuilder<'agent, P, L, NoName, B, Pr> {
    pub fn with_name_from_str(
        self,
        str: &str,
    ) -> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
        let name = String::from_str(self.agent, str);
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: CreatorName(name),
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    pub fn with_prefixed_name_from_str(
        self,
        prefix: &str,
        name: &str,
    ) -> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
        let name = String::from_str(self.agent, &format!("{} {}", name, prefix));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: CreatorName(name),
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    pub fn with_name_from_string(
        self,
        name: String,
    ) -> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: CreatorName(name),
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent, P, L, N, B> BuiltinFunctionBuilder<'agent, P, L, N, B, NoProperties> {
    pub fn with_data_property(
        self,
        key: PropertyKey,
        value: Value,
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let object_index = Some(self.object_index.unwrap_or_else(|| {
            self.agent.heap.objects.push(None);
            ObjectIndex::last(&self.agent.heap.objects)
        }));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: CreatorProperties(vec![(key, None, Some(value))]),
        }
    }

    pub fn with_property(
        self,
        creator: impl FnOnce(
            PropertyBuilder<'_, property_builder::NoKey, property_builder::NoDefinition>,
        ) -> (PropertyKey, Option<ElementDescriptor>, Option<Value>),
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let object_index = Some(self.object_index.unwrap_or_else(|| {
            self.agent.heap.objects.push(None);
            ObjectIndex::last(&self.agent.heap.objects)
        }));
        let property = {
            let builder = PropertyBuilder::new(self.agent, self.this.into_object());
            creator(builder)
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: CreatorProperties(vec![property]),
        }
    }
}

impl<'agent, P, L, N, B> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
    pub fn with_data_property(
        mut self,
        key: PropertyKey,
        value: Value,
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        self.properties.0.push((key, None, Some(value)));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    pub fn with_property(
        mut self,
        creator: impl FnOnce(
            PropertyBuilder<'_, property_builder::NoKey, property_builder::NoDefinition>,
        ) -> (PropertyKey, Option<ElementDescriptor>, Option<Value>),
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let builder = PropertyBuilder::new(self.agent, self.this.into_object());
        let property = creator(builder);
        self.properties.0.push(property);
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index: self.object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent>
    BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    >
{
    pub fn build(&mut self) -> BuiltinFunction {
        let data = BuiltinFunctionHeapData {
            object_index: None,
            length: self.length.0,
            realm: self.realm,
            initial_name: Some(self.name.0),
            behaviour: self.behaviour.0,
        };

        let slot = self
            .agent
            .heap
            .builtin_functions
            .get_mut(self.this.0.into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(data);
        self.this
    }
}

impl<'agent>
    BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        CreatorProperties,
    >
{
    pub fn build(self) -> BuiltinFunction {
        let Self {
            agent,
            length,
            name,
            realm,
            behaviour,
            properties,
            object_index,
            ..
        } = self;
        let properties = properties.0;

        let (keys, values) = agent.heap.elements.create_with_stuff(properties);

        let prototype = Some(
            agent
                .get_realm(realm)
                .intrinsics()
                .function_prototype()
                .into_object(),
        );
        let slot = agent
            .heap
            .objects
            .get_mut(object_index.unwrap().into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: true,
            prototype,
            keys,
            values,
        });

        let data = BuiltinFunctionHeapData {
            object_index,
            length: length.0,
            realm,
            initial_name: Some(name.0),
            behaviour: behaviour.0,
        };

        let slot = agent
            .heap
            .builtin_functions
            .get_mut(self.this.0.into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(data);
        self.this
    }
}

impl<'agent>
    BuiltinFunctionBuilder<
        'agent,
        CreatorPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        CreatorProperties,
    >
{
    pub fn build(self) -> BuiltinFunction {
        let Self {
            agent,
            length,
            name,
            behaviour,
            realm,
            properties,
            object_index,
            prototype,
            ..
        } = self;
        let properties = properties.0;

        let (keys, values) = agent.heap.elements.create_with_stuff(properties);

        let slot = agent
            .heap
            .objects
            .get_mut(object_index.unwrap().into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype.0),
            keys,
            values,
        });

        let data = BuiltinFunctionHeapData {
            object_index,
            length: length.0,
            realm,
            initial_name: Some(name.0),
            behaviour: behaviour.0,
        };

        let slot = agent
            .heap
            .builtin_functions
            .get_mut(self.this.0.into_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(data);
        self.this
    }
}
