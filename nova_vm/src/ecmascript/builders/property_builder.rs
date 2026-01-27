// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::Agent,
        Function, PropertyKey, Value,
    },
    heap::element_array::ElementDescriptor,
};

#[derive(Default, Clone, Copy)]
pub struct NoKey;

#[derive(Default, Clone, Copy)]
pub struct NoEnumerable;

#[derive(Default, Clone, Copy)]
pub struct NoConfigurable;

#[derive(Default, Clone, Copy)]
pub struct NoDefinition;

#[derive(Clone, Copy)]
pub struct CreatorKey(PropertyKey<'static>);

#[derive(Clone, Copy)]
pub struct CreatorGetAccessor(Function<'static>);

#[derive(Clone, Copy)]
pub struct CreatorSetAccess(Function<'static>);

#[derive(Clone, Copy)]
pub struct CreatorGetSetAccessor {
    get: Function<'static>,
    set: Function<'static>,
}

#[derive(Clone, Copy)]
pub struct CreatorValue(Value<'static>);

#[derive(Clone, Copy)]
pub struct CreatorReadOnlyValue(Value<'static>);

pub struct PropertyBuilder<'agent, K: 'static, D> {
    pub(crate) agent: &'agent mut Agent,
    key: K,
    definition: D,
    enumerable: bool,
    configurable: bool,
}

impl<'agent> PropertyBuilder<'agent, NoKey, NoDefinition> {
    pub fn new(agent: &'agent mut Agent) -> Self {
        PropertyBuilder {
            agent,
            key: NoKey,
            definition: NoDefinition,
            enumerable: true,
            configurable: true,
        }
    }
}

impl<'agent, D> PropertyBuilder<'agent, NoKey, D> {
    pub fn with_key(self, key: PropertyKey<'static>) -> PropertyBuilder<'agent, CreatorKey, D> {
        PropertyBuilder {
            agent: self.agent,
            key: CreatorKey(key),
            definition: self.definition,
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }
}

impl<'agent, K> PropertyBuilder<'agent, K, NoDefinition> {
    pub fn with_value(self, value: Value<'static>) -> PropertyBuilder<'agent, K, CreatorValue> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorValue(value),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_value_readonly(
        self,
        value: Value<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorReadOnlyValue> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorReadOnlyValue(value),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_value_creator(
        self,
        creator: impl FnOnce(&mut Agent) -> Value<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorValue> {
        let value = creator(self.agent);
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorValue(value),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_value_creator_readonly(
        self,
        creator: impl FnOnce(&mut Agent) -> Value<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorReadOnlyValue> {
        let value = creator(self.agent);
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorReadOnlyValue(value),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }
}

impl<'agent, K> PropertyBuilder<'agent, K, NoDefinition> {
    pub fn with_getter_function(
        self,
        getter: Function<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorGetAccessor> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorGetAccessor(getter),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_getter(
        self,
        creator: impl FnOnce(&mut Agent) -> Function<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorGetAccessor> {
        let getter = creator(self.agent);
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorGetAccessor(getter),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_setter_function(
        self,
        setter: Function<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorSetAccess> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorSetAccess(setter),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_setter(
        self,
        creator: impl FnOnce(&mut Agent) -> Function<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorSetAccess> {
        let setter = creator(self.agent);
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorSetAccess(setter),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_getter_and_setter_functions(
        self,
        getter: Function<'static>,
        setter: Function<'static>,
    ) -> PropertyBuilder<'agent, K, CreatorGetSetAccessor> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorGetSetAccessor {
                get: getter,
                set: setter,
            },
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn with_getter_and_setter(
        self,
        creator: impl FnOnce(&mut Agent) -> (Function<'static>, Function<'static>),
    ) -> PropertyBuilder<'agent, K, CreatorGetSetAccessor> {
        let (getter, setter) = creator(self.agent);
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorGetSetAccessor {
                get: getter,
                set: setter,
            },
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }
}

impl<'agent, K, D> PropertyBuilder<'agent, K, D> {
    pub fn with_enumerable(self, enumerable: bool) -> PropertyBuilder<'agent, K, D> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: self.definition,
            enumerable,
            configurable: self.configurable,
        }
    }
}

impl<'agent, K, D> PropertyBuilder<'agent, K, D> {
    pub fn with_configurable(self, configurable: bool) -> PropertyBuilder<'agent, K, D> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: self.definition,
            enumerable: self.enumerable,
            configurable,
        }
    }
}

impl PropertyBuilder<'_, CreatorKey, CreatorValue> {
    pub fn build(
        self,
    ) -> (
        PropertyKey<'static>,
        Option<ElementDescriptor<'static>>,
        Option<Value<'static>>,
    ) {
        (
            self.key.0,
            ElementDescriptor::new_with_wec(true, self.enumerable, self.configurable),
            Some(self.definition.0),
        )
    }
}

impl PropertyBuilder<'_, CreatorKey, CreatorReadOnlyValue> {
    pub fn build(
        self,
    ) -> (
        PropertyKey<'static>,
        Option<ElementDescriptor<'static>>,
        Option<Value<'static>>,
    ) {
        (
            self.key.0,
            ElementDescriptor::new_with_wec(false, self.enumerable, self.configurable),
            Some(self.definition.0),
        )
    }
}

impl PropertyBuilder<'_, CreatorKey, CreatorGetAccessor> {
    pub fn build(
        self,
    ) -> (
        PropertyKey<'static>,
        Option<ElementDescriptor<'static>>,
        Option<Value<'static>>,
    ) {
        (
            self.key.0,
            Some(ElementDescriptor::new_with_get_ec(
                self.definition.0,
                self.enumerable,
                self.configurable,
            )),
            None,
        )
    }
}

impl PropertyBuilder<'_, CreatorKey, CreatorSetAccess> {
    pub fn build(
        self,
    ) -> (
        PropertyKey<'static>,
        Option<ElementDescriptor<'static>>,
        Option<Value<'static>>,
    ) {
        (
            self.key.0,
            Some(ElementDescriptor::new_with_set_ec(
                self.definition.0,
                self.enumerable,
                self.configurable,
            )),
            None,
        )
    }
}

impl PropertyBuilder<'_, CreatorKey, CreatorGetSetAccessor> {
    pub fn build(
        self,
    ) -> (
        PropertyKey<'static>,
        Option<ElementDescriptor<'static>>,
        Option<Value<'static>>,
    ) {
        (
            self.key.0,
            Some(ElementDescriptor::new_with_get_set_ec(
                self.definition.get,
                self.definition.set,
                self.enumerable,
                self.configurable,
            )),
            None,
        )
    }
}
