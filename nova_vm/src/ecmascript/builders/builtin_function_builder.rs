// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::create_intrinsic_backing_object,
        builtins::{
            Behaviour, Builtin, BuiltinFunction, BuiltinGetter, BuiltinIntrinsic,
            BuiltinIntrinsicConstructor, BuiltinSetter,
        },
        execution::{Agent, Realm},
        types::{
            BUILTIN_STRING_MEMORY, BuiltinFunctionHeapData, Object, OrdinaryObject, PropertyKey,
            String, Value,
        },
    },
    engine::context::Bindable,
    heap::{element_array::ElementDescriptor, indexes::HeapIndexHandle},
};

use super::{
    ordinary_object_builder::PropertyDefinition,
    property_builder::{self, PropertyBuilder},
};

#[derive(Default, Clone, Copy)]
pub struct NoPrototype;

#[derive(Clone, Copy)]
pub struct CreatorPrototype(Option<Object<'static>>);

#[derive(Default, Clone, Copy)]
pub struct NoLength;

#[derive(Clone, Copy)]
pub struct CreatorLength(u8);

#[derive(Default, Clone, Copy)]
pub struct NoName;

#[derive(Clone, Copy)]
pub struct CreatorName(String<'static>);

#[derive(Default, Clone, Copy)]
pub struct NoBehaviour;

#[derive(Clone, Copy)]
pub struct CreatorBehaviour(Behaviour);

#[derive(Default, Clone, Copy)]
pub struct NoProperties;

#[derive(Clone)]
pub struct CreatorProperties(Vec<PropertyDefinition>);

pub struct BuiltinFunctionBuilder<'agent, P, L, N, B, Pr> {
    pub(crate) agent: &'agent mut Agent,
    this: BuiltinFunction<'static>,
    backing_object: Option<OrdinaryObject<'static>>,
    realm: Realm<'static>,
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
        realm: Realm<'static>,
    ) -> BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    > {
        let this = BuiltinFunction::new_uninitialised(agent);
        BuiltinFunctionBuilder {
            agent,
            this,
            backing_object: None,
            realm,
            prototype: Default::default(),
            length: CreatorLength(T::LENGTH),
            name: CreatorName(T::NAME),
            behaviour: CreatorBehaviour(T::BEHAVIOUR),
            properties: Default::default(),
        }
    }

    #[must_use]
    pub fn new_getter<T: BuiltinGetter>(
        agent: &'agent mut Agent,
        realm: Realm<'static>,
    ) -> BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    > {
        let this = BuiltinFunction::new_uninitialised(agent);
        BuiltinFunctionBuilder {
            agent,
            this,
            backing_object: None,
            realm,
            prototype: Default::default(),
            length: CreatorLength(0),
            name: CreatorName(T::GETTER_NAME),
            behaviour: CreatorBehaviour(T::GETTER_BEHAVIOUR),
            properties: Default::default(),
        }
    }

    #[must_use]
    pub fn new_setter<T: BuiltinSetter>(
        agent: &'agent mut Agent,
        realm: Realm<'static>,
    ) -> BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    > {
        let this = BuiltinFunction::new_uninitialised(agent);
        BuiltinFunctionBuilder {
            agent,
            this,
            backing_object: None,
            realm,
            prototype: Default::default(),
            length: CreatorLength(1),
            name: CreatorName(T::SETTER_NAME),
            behaviour: CreatorBehaviour(T::SETTER_BEHAVIOUR),
            properties: Default::default(),
        }
    }

    #[must_use]
    pub(crate) fn new_intrinsic_constructor<T: BuiltinIntrinsicConstructor>(
        agent: &'agent mut Agent,
        realm: Realm<'static>,
    ) -> BuiltinFunctionBuilder<
        'agent,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    > {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.intrinsic_constructor_index_to_builtin_function(T::INDEX);
        let backing_object = Some(intrinsics.get_intrinsic_constructor_backing_object(T::INDEX));
        let name = T::NAME;
        BuiltinFunctionBuilder {
            agent,
            this,
            backing_object,
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
        realm: Realm<'static>,
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
            .get_realm_record_by_id(realm)
            .intrinsics()
            .intrinsic_function_index_to_builtin_function(T::INDEX);
        BuiltinFunctionBuilder {
            agent,
            this,
            backing_object: None,
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
            backing_object: self.backing_object,
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
        prototype: Object<'static>,
    ) -> BuiltinFunctionBuilder<'agent, CreatorPrototype, L, N, B, Pr> {
        let backing_object = if prototype
            != self
                .agent
                .get_realm_record_by_id(self.realm)
                .intrinsics()
                .function_prototype()
                .into()
            && self.backing_object.is_none()
        {
            Some(OrdinaryObject::new_uninitialised(self.agent))
        } else {
            self.backing_object
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object,
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
        let backing_object = if self.backing_object.is_none() {
            Some(OrdinaryObject::new_uninitialised(self.agent))
        } else {
            self.backing_object
        };
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object,
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
            backing_object: self.backing_object,
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
        name: String<'static>,
    ) -> BuiltinFunctionBuilder<'agent, P, L, CreatorName, B, Pr> {
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object: self.backing_object,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: CreatorName(name),
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl<P, L, B, Pr> BuiltinFunctionBuilder<'_, P, L, CreatorName, B, Pr> {
    pub(crate) fn get_name(&self) -> String<'static> {
        self.name.0
    }
}

impl<'agent, P, B> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, NoProperties> {
    #[must_use]
    pub fn with_property_capacity(
        self,
        cap: usize,
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, CreatorProperties> {
        let backing_object = Some(
            self.backing_object
                .unwrap_or_else(|| OrdinaryObject::new_uninitialised(self.agent)),
        );
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
            backing_object,
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
        key: PropertyKey<'static>,
        value: Value<'static>,
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, CreatorProperties> {
        let backing_object = Some(
            self.backing_object
                .unwrap_or_else(|| OrdinaryObject::new_uninitialised(self.agent)),
        );
        let property_vector = vec![
            (
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                Some(self.length.0.into()),
            ),
            (
                PropertyKey::from(BUILTIN_STRING_MEMORY.name),
                Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                Some(self.name.0.unbind().into()),
            ),
            (key.unbind(), None, Some(value.unbind())),
        ];
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object,
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
        ) -> (
            PropertyKey<'static>,
            Option<ElementDescriptor<'static>>,
            Option<Value<'static>>,
        ),
    ) -> BuiltinFunctionBuilder<'agent, P, CreatorLength, CreatorName, B, CreatorProperties> {
        let backing_object = Some(
            self.backing_object
                .unwrap_or_else(|| OrdinaryObject::new_uninitialised(self.agent)),
        );
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
                Some(self.name.0.unbind().into()),
            ),
            property,
        ];
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object,
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
        key: PropertyKey<'static>,
        value: Value<'static>,
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        self.properties.0.push((key, None, Some(value.unbind())));
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object: self.backing_object,
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
        ) -> (
            PropertyKey<'static>,
            Option<ElementDescriptor<'static>>,
            Option<Value<'static>>,
        ),
    ) -> BuiltinFunctionBuilder<'agent, P, L, N, B, CreatorProperties> {
        let builder = PropertyBuilder::new(self.agent);
        let property = creator(builder);
        self.properties.0.push(property);
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object: self.backing_object,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }

    #[must_use]
    pub fn with_prototype_property(mut self, prototype: Object<'static>) -> Self {
        let property = PropertyBuilder::new(self.agent)
            .with_configurable(false)
            .with_enumerable(false)
            .with_value_readonly(prototype.into())
            .with_key(BUILTIN_STRING_MEMORY.prototype.into())
            .build();
        self.properties.0.push(property);
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object: self.backing_object,
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
            let name = T::KEY.unwrap_or_else(|| PropertyKey::from(builder.get_name()));
            (builder.build().into(), name)
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
            backing_object: self.backing_object,
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
            .into();
        let property = PropertyBuilder::new(self.agent)
            .with_key(T::KEY.unwrap())
            .with_configurable(T::CONFIGURABLE)
            .with_enumerable(T::ENUMERABLE)
            .with_getter_function(getter_function)
            .build();
        self.properties.0.push(property);
        BuiltinFunctionBuilder {
            agent: self.agent,
            this: self.this,
            backing_object: self.backing_object,
            realm: self.realm,
            prototype: self.prototype,
            length: self.length,
            name: self.name,
            behaviour: self.behaviour,
            properties: self.properties,
        }
    }
}

impl
    BuiltinFunctionBuilder<
        '_,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        NoProperties,
    >
{
    pub fn build(&mut self) -> BuiltinFunction<'static> {
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
        *slot = data;
        self.this
    }
}

impl
    BuiltinFunctionBuilder<
        '_,
        NoPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        CreatorProperties,
    >
{
    pub fn build(self) -> BuiltinFunction<'static> {
        let Self {
            agent,
            length: CreatorLength(length),
            name: CreatorName(name),
            realm,
            behaviour: CreatorBehaviour(behaviour),
            properties: CreatorProperties(properties),
            backing_object,
            ..
        } = self;

        let prototype = agent
            .get_realm_record_by_id(realm)
            .intrinsics()
            .function_prototype()
            .into();
        create_function_intrinsic_backing_object(
            agent,
            backing_object,
            Some(prototype),
            properties,
        );

        let data = BuiltinFunctionHeapData {
            object_index: backing_object,
            length,
            realm,
            initial_name: Some(name),
            behaviour,
        };

        let slot = agent
            .heap
            .builtin_functions
            .get_mut(self.this.get_index())
            .unwrap();
        *slot = data;
        self.this
    }
}

impl
    BuiltinFunctionBuilder<
        '_,
        CreatorPrototype,
        CreatorLength,
        CreatorName,
        CreatorBehaviour,
        CreatorProperties,
    >
{
    pub fn build(self) -> BuiltinFunction<'static> {
        let Self {
            agent,
            length: CreatorLength(length),
            name: CreatorName(name),
            realm,
            behaviour: CreatorBehaviour(behaviour),
            properties: CreatorProperties(properties),
            backing_object,
            prototype: CreatorPrototype(prototype),
            ..
        } = self;

        create_function_intrinsic_backing_object(agent, backing_object, prototype, properties);

        let data = BuiltinFunctionHeapData {
            object_index: backing_object,
            length,
            realm,
            initial_name: Some(name),
            behaviour,
        };

        let slot = agent
            .heap
            .builtin_functions
            .get_mut(self.this.get_index())
            .unwrap();
        *slot = data;
        self.this
    }
}

#[inline]
fn create_function_intrinsic_backing_object(
    agent: &mut Agent,
    backing_object: Option<OrdinaryObject>,
    prototype: Option<Object>,
    properties: Vec<PropertyDefinition>,
) {
    debug_assert_eq!(properties[0].0, BUILTIN_STRING_MEMORY.length.into());
    debug_assert_eq!(properties[1].0, BUILTIN_STRING_MEMORY.name.into());
    let backing_object = backing_object
        .expect("Cannot create a BuiltinFunction backing object if a slot has not been defined");

    create_intrinsic_backing_object(agent, backing_object, prototype, properties, true);
}
