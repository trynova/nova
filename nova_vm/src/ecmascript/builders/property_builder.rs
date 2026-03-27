// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{Function, PropertyKey, Value, execution::Agent},
    heap::ElementDescriptor,
};

#[doc(hidden)]
#[derive(Default, Clone, Copy)]
pub struct NoKey;

#[doc(hidden)]
#[derive(Default, Clone, Copy)]
pub struct NoEnumerable;

#[doc(hidden)]
#[derive(Default, Clone, Copy)]
pub struct NoConfigurable;

#[doc(hidden)]
#[derive(Default, Clone, Copy)]
pub struct NoDefinition;

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct CreatorKey(PropertyKey<'static>);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct CreatorGetAccessor(Function<'static>);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct CreatorSetAccess(Function<'static>);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct CreatorGetSetAccessor {
    get: Function<'static>,
    set: Function<'static>,
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct CreatorValue(Value<'static>);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct CreatorReadOnlyValue(Value<'static>);

/// Builder struct for creating object or function properties in embedders.
pub struct PropertyBuilder<'agent, K: 'static, D> {
    pub(crate) agent: &'agent mut Agent,
    key: K,
    definition: D,
    enumerable: bool,
    configurable: bool,
}

impl<'agent> PropertyBuilder<'agent, NoKey, NoDefinition> {
    /// Create a new property descriptor builder.
    pub(crate) fn new(agent: &'agent mut Agent) -> Self {
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
    /// Set the property descriptor key.
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
    /// Set the property descriptor value.
    pub fn with_value(self, value: Value<'static>) -> PropertyBuilder<'agent, K, CreatorValue> {
        PropertyBuilder {
            agent: self.agent,
            key: self.key,
            definition: CreatorValue(value),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    /// Set the property descriptor value and make it read-only.
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

    /// Create a value for the property descriptor.
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

    /// Create a value for the property descriptor and make it read-only.
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
    /// Set a getter function on the property descriptor.
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

    /// Create a getter function and set it on the property descriptor.
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

    /// Set a setter function on the property descriptor.
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

    /// Create a setter function and set it on the property descriptor.
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

    /// Set getter and setter functions on the property descriptor.
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

    /// Create getter and setter functions and set them on the property descriptor.
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
    /// Set the `enumerable` flag of the property descriptor.
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
    /// Set the `configurable` flag of the property descriptor.
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
    /// Builds the property descriptor.
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
    /// Builds the property descriptor.
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
    /// Builds the property descriptor.
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
    /// Builds the property descriptor.
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
    /// Builds the property descriptor.
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
