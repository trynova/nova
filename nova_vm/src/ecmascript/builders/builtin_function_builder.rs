use crate::{
    ecmascript::{
        builtins::{
            Behaviour, Builtin, BuiltinFunction, BuiltinGetter, BuiltinIntrinsic,
            BuiltinIntrinsicConstructor,
        },
        execution::{Agent, RealmIdentifier},
        types::{
            BuiltinFunctionHeapData, IntoFunction, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
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
pub struct CreatorPrototype(Option<Object>);

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
    object_index: Option<OrdinaryObject>,
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
    #[must_use]
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
        let name = T::NAME;
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

    #[must_use]
    pub(crate) fn new_intrinsic_constructor<T: BuiltinIntrinsicConstructor>(
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
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.intrinsic_constructor_index_to_builtin_function(T::INDEX);
        let object_index = Some(OrdinaryObject(
            intrinsics.intrinsic_constructor_index_to_object_index(T::INDEX),
        ));
        let name = T::NAME;
        BuiltinFunctionBuilder {
            agent,
            this,
            object_index,
            realm,
            prototype: Default::default(),
            length: CreatorLength(T::LENGTH),
            name: CreatorName(name),
            behaviour: CreatorBehaviour(T::BEHAVIOUR),
            properties: Default::default(),
        }
    }

    #[must_use]
    pub(crate) fn new_intrinsic_function<T: BuiltinIntrinsic>(
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
        let name = T::NAME;
        let this = agent
            .get_realm(realm)
            .intrinsics()
            .intrinsic_function_index_to_builtin_function(T::INDEX);
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
}

impl<'agent, P, L, N, Pr> BuiltinFunctionBuilder<'agent, P, L, N, NoBehaviour, Pr> {
    #[must_use]
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
    #[must_use]
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
            Some(ObjectIndex::last(&self.agent.heap.objects).into())
        } else {
            self.object_index
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: CreatorPrototype(Some(prototype)),
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    #[must_use]
    pub fn with_null_prototype(
        self,
    ) -> BuiltinFunctionBuilder<'agent, CreatorPrototype, L, N, B, Pr> {
        let object_index = if self.object_index.is_none() {
            self.agent.heap.objects.push(None);
            Some(ObjectIndex::last(&self.agent.heap.objects).into())
        } else {
            self.object_index
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: CreatorPrototype(None),
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<'agent, P, N, B, Pr> BuiltinFunctionBuilder<'agent, P, NoLength, N, B, Pr> {
    #[must_use]
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
    #[must_use]
    pub fn with_name(
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

impl<'agent, P, L, B, Pr> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
    pub(crate) fn get_name(&self) -> String {
        self.name.0
    }
}

impl<'agent, P, B> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, NoProperties> {
    #[must_use]
    pub fn with_property_capacity(
        self,
        cap: usize,
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, CreatorProperties> {
        let object_index = Some(self.object_index.unwrap_or_else(|| {
            self.agent.heap.objects.push(None);
            ObjectIndex::last(&self.agent.heap.objects).into()
        }));
        let mut property_vector = Vec::with_capacity(cap + 2);
        property_vector.push((
            PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
            Some(self.length.0.into()),
        ));
        property_vector.push((
            PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
            Some(self.name.0.into()),
        ));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: CreatorProperties(property_vector),
        }
    }

    #[must_use]
    pub fn with_data_property(
        self,
        key: PropertyKey,
        value: Value,
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, CreatorProperties> {
        let object_index = Some(self.object_index.unwrap_or_else(|| {
            self.agent.heap.objects.push(None);
            ObjectIndex::last(&self.agent.heap.objects).into()
        }));
        let property_vector = vec![
            (
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                Some(self.length.0.into()),
            ),
            (
                PropertyKey::from(BUILTIN_STRING_MEMORY.name),
                Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                Some(self.name.0.into()),
            ),
            (key, None, Some(value)),
        ];
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: CreatorProperties(property_vector),
        }
    }

    #[must_use]
    pub fn with_property(
        self,
        creator: impl FnOnce(
            PropertyBuilder<'_, property_builder::NoKey, property_builder::NoDefinition>,
        ) -> (PropertyKey, Option<ElementDescriptor>, Option<Value>),
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, CreatorProperties> {
        let object_index = Some(self.object_index.unwrap_or_else(|| {
            self.agent.heap.objects.push(None);
            ObjectIndex::last(&self.agent.heap.objects).into()
        }));
        let property = {
            let builder = PropertyBuilder::new(self.agent);
            creator(builder)
        };
        let property_vector = vec![
            (
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                Some(self.length.0.into()),
            ),
            (
                PropertyKey::from(BUILTIN_STRING_MEMORY.name),
                Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                Some(self.name.0.into()),
            ),
            property,
        ];
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            object_index,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: CreatorProperties(property_vector),
        }
    }
}

impl<'agent, P, L, N, B> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
    #[must_use]
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

    #[must_use]
    pub fn with_property(
        mut self,
        creator: impl FnOnce(
            PropertyBuilder<'_, property_builder::NoKey, property_builder::NoDefinition>,
        ) -> (PropertyKey, Option<ElementDescriptor>, Option<Value>),
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let builder = PropertyBuilder::new(self.agent);
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

    #[must_use]
    pub fn with_prototype_property(mut self, prototype: Object) -> Self {
        let property = PropertyBuilder::new(self.agent)
            .with_configurable(false)
            .with_enumerable(false)
            .with_value_readonly(prototype.into_value())
            .with_key(BUILTIN_STRING_MEMORY.prototype.into())
            .build();
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

    #[must_use]
    pub fn with_builtin_function_getter_property<T: BuiltinGetter>(mut self) -> Self {
        let getter_function = BuiltinFunctionBuilder::new::<T>(self.agent, self.realm)
            .build()
            .into_function();
        let property = PropertyBuilder::new(self.agent)
            .with_key(T::KEY)
            .with_configurable(T::CONFIGURABLE)
            .with_enumerable(T::ENUMERABLE)
            .with_getter_function(getter_function)
            .build();
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
            .get_mut(self.this.get_index())
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
        assert_eq!(properties.len(), properties.capacity());
        {
            let slice = properties.as_slice();
            let duplicate = (1..slice.len()).find(|first_index| {
                slice[*first_index..]
                    .iter()
                    .any(|(key, _, _)| *key == slice[first_index - 1].0)
            });
            if let Some(index) = duplicate {
                panic!("Duplicate key found: {:?}", slice[index].0);
            }
        }

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
            .get_mut(object_index.unwrap().get_index())
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
            .get_mut(self.this.get_index())
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
        assert_eq!(properties.len(), properties.capacity());
        {
            let slice = properties.as_slice();
            let duplicate = (1..slice.len()).find(|first_index| {
                slice[*first_index..]
                    .iter()
                    .any(|(key, _, _)| *key == slice[first_index - 1].0)
            });
            if let Some(index) = duplicate {
                panic!("Duplicate key found: {:?}", slice[index].0);
            }
        }

        let (keys, values) = agent.heap.elements.create_with_stuff(properties);

        let slot = agent
            .heap
            .objects
            .get_mut(object_index.unwrap().get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(ObjectHeapData {
            extensible: true,
            prototype: prototype.0,
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
            .get_mut(self.this.get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(data);
        self.this
    }
}
