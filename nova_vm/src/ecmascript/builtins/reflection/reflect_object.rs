// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::type_conversion::{
    to_property_key_complex, to_property_key_simple,
};
use crate::ecmascript::builtins::Behaviour;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::TryResult;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, construct, create_array_from_list, create_list_from_array_like,
            },
            testing_and_comparison::{is_callable, is_constructor},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Builtin},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{
            InternalMethods, IntoValue, Object, PropertyDescriptor, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct ReflectObject;

struct ReflectObjectApply;
impl Builtin for ReflectObjectApply {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.apply;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::apply);
}

struct ReflectObjectConstruct;
impl Builtin for ReflectObjectConstruct {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.construct;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::construct);
}
struct ReflectObjectDefineProperty;
impl Builtin for ReflectObjectDefineProperty {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.defineProperty;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::define_property);
}
struct ReflectObjectDeleteProperty;
impl Builtin for ReflectObjectDeleteProperty {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.deleteProperty;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::delete_property);
}
struct ReflectObjectGet;
impl Builtin for ReflectObjectGet {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::get);
}
struct ReflectObjectGetOwnPropertyDescriptor;
impl Builtin for ReflectObjectGetOwnPropertyDescriptor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::get_own_property_descriptor);
}
struct ReflectObjectGetPrototypeOf;
impl Builtin for ReflectObjectGetPrototypeOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getPrototypeOf;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::get_prototype_of);
}

struct ReflectObjectHas;
impl Builtin for ReflectObjectHas {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.has;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::has);
}
struct ReflectObjectIsExtensible;
impl Builtin for ReflectObjectIsExtensible {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isExtensible;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::is_extensible);
}
struct ReflectObjectOwnKeys;
impl Builtin for ReflectObjectOwnKeys {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.ownKeys;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::own_keys);
}
struct ReflectObjectPreventExtensions;
impl Builtin for ReflectObjectPreventExtensions {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.preventExtensions;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::prevent_extensions);
}
struct ReflectObjectSet;
impl Builtin for ReflectObjectSet {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.set;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::set);
}
struct ReflectObjectSetPrototypeOf;
impl Builtin for ReflectObjectSetPrototypeOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setPrototypeOf;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ReflectObject::set_prototype_of);
}

impl ReflectObject {
    /// [28.1.1 Reflect.apply ( target, thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-reflect.apply)
    fn apply<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let this_argument = arguments.get(1).bind(nogc);
        let arguments_list = arguments.get(2).bind(nogc);

        // 1. If IsCallable(target) is false, throw a TypeError exception.
        let Some(target) = is_callable(target, nogc) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not callable",
                nogc,
            ));
        };
        let target = target.scope(agent, nogc);
        let this_argument = this_argument.scope(agent, nogc);
        // 2. Let args be ? CreateListFromArrayLike(argumentsList).
        let args = create_list_from_array_like(agent, arguments_list.unbind(), gc.reborrow())?;
        // TODO: 3. Perform PrepareForTailCall().
        // 4. Return ? Call(target, thisArgument, args)
        call_function(
            agent,
            target.get(agent),
            this_argument.get(agent),
            Some(ArgumentsList(
                &args.into_iter().map(|v| v.unbind()).collect::<Vec<_>>(),
            )),
            gc,
        )
    }

    /// [28.1.2 Reflect.construct ( target, argumentsList \[ , newTarget \] )](https://tc39.es/ecma262/#sec-reflect.construct)
    fn construct<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let arguments_list = arguments.get(1).bind(nogc);

        // 1. If IsConstructor(target) is false, throw a TypeError exception.
        let Some(target) = is_constructor(agent, target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not a constructor",
                nogc,
            ));
        };

        // 2. If newTarget is not present, set newTarget to target.
        // 3. Else if IsConstructor(newTarget) is false, throw a TypeError exception.
        let new_target = if arguments.len() > 2 {
            let new_target = arguments.get(2).bind(nogc);
            let Some(new_target) = is_constructor(agent, new_target) else {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Value is not a constructor",
                    nogc,
                ));
            };
            new_target
        } else {
            target
        };

        let target = target.scope(agent, nogc);
        let new_target = new_target.scope(agent, nogc);
        // 4. Let args be ? CreateListFromArrayLike(argumentsList).
        let args = create_list_from_array_like(agent, arguments_list.unbind(), gc.reborrow())?;
        // 5. Return ? Construct(target, args, newTarget)
        construct(
            agent,
            target.get(agent),
            Some(ArgumentsList(
                &args.into_iter().map(|v| v.unbind()).collect::<Vec<_>>(),
            )),
            Some(new_target.get(agent)),
            gc,
        )
        .map(|o| o.into_value())
    }

    /// [28.1.3 Reflect.defineProperty ( target, propertyKey, attributes )](https://tc39.es/ecma262/#sec-reflect.defineproperty)
    fn define_property<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let property_key = arguments.get(1).bind(nogc);
        let mut attributes = arguments.get(2).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };
        let mut target = target.bind(nogc);

        let mut scoped_target = None;

        // 2. Let key be ? ToPropertyKey(propertyKey).
        let mut key =
            if let TryResult::Continue(key) = to_property_key_simple(agent, property_key, nogc) {
                key
            } else {
                scoped_target = Some(target.scope(agent, nogc));
                let scoped_attributes = attributes.scope(agent, nogc);
                let key = to_property_key_complex(agent, property_key.unbind(), gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                target = scoped_target.as_ref().unwrap().get(agent);
                attributes = scoped_attributes.get(agent);
                key
            };

        // 3. Let desc be ? ToPropertyDescriptor(attributes).
        let desc = if let TryResult::Continue(desc) =
            PropertyDescriptor::try_to_property_descriptor(agent, attributes, gc.nogc())
        {
            desc?
        } else {
            if scoped_target.is_none() {
                scoped_target = Some(target.scope(agent, gc.nogc()));
            }
            let scoped_key = key.scope(agent, gc.nogc());
            let desc = PropertyDescriptor::to_property_descriptor(
                agent,
                attributes.unbind(),
                gc.reborrow(),
            )?;
            key = scoped_key.get(agent).bind(gc.nogc());
            target = scoped_target.unwrap().get(agent).bind(gc.nogc());
            desc
        };
        // 4. Return ? target.[[DefineOwnProperty]](key, desc).
        let ret = target.unbind().internal_define_own_property(
            agent,
            key.unbind(),
            desc,
            gc.reborrow(),
        )?;

        Ok(ret.into())
    }

    /// [28.1.4 Reflect.deleteProperty ( target, propertyKey )](https://tc39.es/ecma262/#sec-reflect.deleteproperty)
    fn delete_property<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let property_key = arguments.get(1).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(mut target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };

        // 2. Let key be ? ToPropertyKey(propertyKey).
        let key =
            if let TryResult::Continue(key) = to_property_key_simple(agent, property_key, nogc) {
                key
            } else {
                let scoped_target = target.scope(agent, nogc);
                let key = to_property_key_complex(agent, property_key.unbind(), gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                target = scoped_target.get(agent);
                key
            };
        // 3. Return ? target.[[Delete]](key).
        let ret = target
            .unbind()
            .internal_delete(agent, key.unbind(), gc.reborrow())?;

        Ok(ret.into())
    }

    /// [28.1.5 Reflect.get ( target, propertyKey \[ , receiver \] )](https://tc39.es/ecma262/#sec-reflect.get)
    fn get<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let property_key = arguments.get(1).bind(nogc);
        let mut receiver = if arguments.len() > 2 {
            Some(arguments.get(2).bind(nogc))
        } else {
            None
        };

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };
        let mut target = target.bind(nogc);

        // 2. Let key be ? ToPropertyKey(propertyKey).
        let key =
            if let TryResult::Continue(key) = to_property_key_simple(agent, property_key, nogc) {
                key
            } else {
                let scoped_target = target.scope(agent, nogc);
                let scoped_receiver = receiver.map(|receiver| receiver.scope(agent, nogc));
                let key = to_property_key_complex(agent, property_key.unbind(), gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                target = scoped_target.get(agent).bind(gc.nogc());
                receiver = scoped_receiver.map(|scoped_receiver| scoped_receiver.get(agent));
                key
            };
        // 3. If receiver is not present, then
        //   a. Set receiver to target.
        let receiver = receiver.unwrap_or(target.into_value());
        // 4. Return ? target.[[Get]](key, receiver).
        target
            .unbind()
            .internal_get(agent, key.unbind(), receiver.unbind(), gc)
    }

    /// [28.1.6 Reflect.getOwnPropertyDescriptor ( target, propertyKey )](https://tc39.es/ecma262/#sec-reflect.getownpropertydescriptor)
    fn get_own_property_descriptor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let property_key = arguments.get(1).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };
        let mut target = target.bind(nogc);

        // 2. Let key be ? ToPropertyKey(propertyKey).
        let key =
            if let TryResult::Continue(key) = to_property_key_simple(agent, property_key, nogc) {
                key
            } else {
                let scoped_target = target.scope(agent, nogc);
                let key = to_property_key_complex(agent, property_key.unbind(), gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                target = scoped_target.get(agent).bind(gc.nogc());
                key
            };
        // 3. Let desc be ? target.[[GetOwnProperty]](key).
        let desc = target
            .unbind()
            .internal_get_own_property(agent, key.unbind(), gc.reborrow())?;
        // 4. Return FromPropertyDescriptor(desc).
        match PropertyDescriptor::from_property_descriptor(desc, agent, gc.nogc()) {
            Some(ret) => Ok(ret.into_value().unbind()),
            None => Ok(Value::Undefined),
        }
    }

    /// [28.1.7 Reflect.getPrototypeOf ( target )](https://tc39.es/ecma262/#sec-reflect.getprototypeof)
    fn get_prototype_of<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                gc.into_nogc(),
            ));
        };

        // 2. Return ? target.[[GetPrototypeOf]]().
        match target.unbind().internal_get_prototype_of(agent, gc)? {
            Some(ret) => Ok(ret.into_value()),
            None => Ok(Value::Null),
        }
    }

    /// [28.1.8 Reflect.has ( target, propertyKey )](https://tc39.es/ecma262/#sec-reflect.has)
    fn has<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let property_key = arguments.get(1).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };
        let mut target = target.bind(nogc);

        // 2. Let key be ? ToPropertyKey(propertyKey).
        let key =
            if let TryResult::Continue(key) = to_property_key_simple(agent, property_key, nogc) {
                key
            } else {
                let scoped_target = target.scope(agent, nogc);
                let key = to_property_key_complex(agent, property_key.unbind(), gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                target = scoped_target.get(agent).bind(gc.nogc());
                key
            };
        // 3. Return ? target.[[HasProperty]](key).
        let ret = target
            .unbind()
            .internal_has_property(agent, key.unbind(), gc.reborrow())?;
        Ok(ret.into())
    }

    /// [28.1.9 Reflect.isExtensible ( target )](https://tc39.es/ecma262/#sec-reflect.isextensible)
    fn is_extensible<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };

        // 2. Return ? target.[[IsExtensible]]().
        let ret = target.unbind().internal_is_extensible(agent, gc)?;
        Ok(ret.into())
    }

    /// [28.1.10 Reflect.ownKeys ( target )](https://tc39.es/ecma262/#sec-reflect.ownkeys)
    fn own_keys<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };

        // 2. Let keys be ? target.[[OwnPropertyKeys]]().
        let keys: Vec<Value> = target
            .unbind()
            .internal_own_property_keys(agent, gc.reborrow())?
            .unbind()
            .bind(gc.nogc())
            .into_iter()
            .map(|key| key.convert_to_value(agent, gc.nogc()))
            .collect();
        // 3. Return CreateArrayFromList(keys).
        Ok(create_array_from_list(agent, &keys, gc.nogc())
            .into_value()
            .unbind())
    }

    /// [28.1.11 Reflect.preventExtensions ( target )](https://tc39.es/ecma262/#sec-reflect.preventextensions)
    fn prevent_extensions<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };

        // 2. Return ? target.[[PreventExtensions]]().
        let ret = target.unbind().internal_prevent_extensions(agent, gc)?;
        Ok(ret.into())
    }

    /// [28.1.12 Reflect.set ( target, propertyKey, V \[ , receiver \] )](https://tc39.es/ecma262/#sec-reflect.set)
    fn set<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let property_key = arguments.get(1).bind(nogc);
        let mut v = arguments.get(2).bind(nogc);
        let mut receiver = if arguments.len() > 3 {
            Some(arguments.get(3).bind(nogc))
        } else {
            None
        };

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(mut target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };

        // 2. Let key be ? ToPropertyKey(propertyKey).
        let key =
            if let TryResult::Continue(key) = to_property_key_simple(agent, property_key, nogc) {
                key.bind(nogc)
            } else {
                let scoped_target = target.scope(agent, nogc);
                let scoped_v = v.scope(agent, nogc);
                let scoped_receiver = receiver.map(|receiver| receiver.scope(agent, nogc));
                let key = to_property_key_complex(agent, property_key.unbind(), gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                target = scoped_target.get(agent);
                v = scoped_v.get(agent);
                receiver = scoped_receiver.map(|scoped_receiver| scoped_receiver.get(agent));
                key
            };

        // 3. If receiver is not present, then
        //   a. Set receiver to target.
        let receiver = receiver.unwrap_or(target.into_value());

        // 4. Return ? target.[[Set]](key, V, receiver).
        let ret =
            target
                .unbind()
                .internal_set(agent, key.unbind(), v.unbind(), receiver.unbind(), gc)?;
        Ok(ret.into())
    }

    /// [28.1.13 Reflect.setPrototypeOf ( target, proto )](https://tc39.es/ecma262/#sec-reflect.setprototypeof)
    fn set_prototype_of<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        let proto = arguments.get(1).bind(nogc);

        // 1. If target is not an Object, throw a TypeError exception.
        let Ok(target) = Object::try_from(target) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Value is not an object",
                nogc,
            ));
        };

        // 2. If proto is not an Object and proto is not null, throw a TypeError exception.
        let proto = if let Ok(proto) = Object::try_from(proto) {
            Some(proto)
        } else if proto.is_null() {
            None
        } else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Prototype must be an object or null",
                nogc,
            ));
        };

        // 3. Return ? target.[[SetPrototypeOf]](proto).
        let ret =
            target
                .unbind()
                .internal_set_prototype_of(agent, proto.map(|p| p.unbind()), gc)?;
        Ok(ret.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.reflect();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(14)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<ReflectObjectApply>()
            .with_builtin_function_property::<ReflectObjectConstruct>()
            .with_builtin_function_property::<ReflectObjectDefineProperty>()
            .with_builtin_function_property::<ReflectObjectDeleteProperty>()
            .with_builtin_function_property::<ReflectObjectGet>()
            .with_builtin_function_property::<ReflectObjectGetOwnPropertyDescriptor>()
            .with_builtin_function_property::<ReflectObjectGetPrototypeOf>()
            .with_builtin_function_property::<ReflectObjectHas>()
            .with_builtin_function_property::<ReflectObjectIsExtensible>()
            .with_builtin_function_property::<ReflectObjectOwnKeys>()
            .with_builtin_function_property::<ReflectObjectPreventExtensions>()
            .with_builtin_function_property::<ReflectObjectSet>()
            .with_builtin_function_property::<ReflectObjectSetPrototypeOf>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Reflect.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
