// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_objects::is_prototype_of_loop;
use crate::ecmascript::types::IntoValue;
use crate::engine::context::{Bindable, GcScope};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{get, has_own_property, invoke},
            type_conversion::{to_object, to_property_key},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic,
            primitive_objects::PrimitiveObjectData,
        },
        execution::{Agent, JsResult, RealmIdentifier},
        types::{BUILTIN_STRING_MEMORY, InternalMethods, Object, PropertyKey, String, Value},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct ObjectPrototype;

struct ObjectPrototypeHasOwnProperty;
impl Builtin for ObjectPrototypeHasOwnProperty {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.hasOwnProperty;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::has_own_property);
}

struct ObjectPrototypeIsPrototypeOf;
impl Builtin for ObjectPrototypeIsPrototypeOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isPrototypeOf;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::is_prototype_of);
}

struct ObjectPrototypePropertyIsEnumerable;
impl Builtin for ObjectPrototypePropertyIsEnumerable {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.propertyIsEnumerable;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::property_is_enumerable);
}

struct ObjectPrototypeToLocaleString;
impl Builtin for ObjectPrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::to_locale_string);
}

struct ObjectPrototypeToString;
impl Builtin for ObjectPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::to_string);
}
impl BuiltinIntrinsic for ObjectPrototypeToString {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ObjectPrototypeToString;
}

struct ObjectPrototypeValueOf;
impl Builtin for ObjectPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::value_of);
}

impl ObjectPrototype {
    fn has_own_property<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let p = to_property_key(agent, arguments.get(0), gc.reborrow())?
            .unbind()
            .bind(gc.nogc());
        let o = to_object(agent, this_value, gc.nogc())?;
        has_own_property(agent, o.unbind(), p.unbind(), gc.reborrow()).map(|result| result.into())
    }

    fn is_prototype_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let v = arguments.get(0).bind(gc.nogc());
        let Ok(v) = Object::try_from(v) else {
            return Ok(false.into());
        };
        let o = to_object(agent, this_value, gc.nogc())?;
        let result = is_prototype_of_loop(agent, o.unbind(), v.unbind(), gc)?;
        Ok(result.into())
    }

    fn property_is_enumerable<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let p = to_property_key(agent, arguments.get(0), gc.reborrow())?
            .unbind()
            .bind(gc.nogc());
        let o = to_object(agent, this_value, gc.nogc())?;
        let desc = o
            .unbind()
            .internal_get_own_property(agent, p.unbind(), gc.reborrow())?;
        if let Some(desc) = desc {
            Ok(desc.enumerable.unwrap_or(false).into())
        } else {
            Ok(false.into())
        }
    }

    fn to_locale_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = this_value;
        let p = PropertyKey::from(BUILTIN_STRING_MEMORY.toString);
        invoke(agent, o, p, None, gc)
    }

    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        match this_value.bind(gc.nogc()) {
            // 1. If the this value is undefined, return "[object Undefined]".
            Value::Undefined => Ok(BUILTIN_STRING_MEMORY._object_Undefined_.into_value()),
            // 2. If the this value is null, return "[object Null]".
            Value::Null => Ok(BUILTIN_STRING_MEMORY._object_Null_.into_value()),
            // 9. Else if O has a [[BooleanData]] internal slot, let builtinTag be "Boolean".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::Boolean(_) => Ok(BUILTIN_STRING_MEMORY._object_Boolean_.into_value()),
            // 6. Else if O has a [[ParameterMap]] internal slot, let builtinTag be "Arguments".
            Value::Arguments(_) => Ok(BUILTIN_STRING_MEMORY._object_Arguments_.into_value()),
            // 11. Else if O has a [[StringData]] internal slot, let builtinTag be "String".
            Value::String(_) | Value::SmallString(_) => {
                Ok(BUILTIN_STRING_MEMORY._object_String_.into_value())
            }
            // 10. Else if O has a [[NumberData]] internal slot, let builtinTag be "Number".
            Value::Number(_) | Value::Integer(_) | Value::SmallF64(_) => {
                Ok(BUILTIN_STRING_MEMORY._object_Number_.into_value())
            }
            // 4. Let isArray be ? IsArray(O).
            // 5. If isArray is true, let builtinTag be "Array".
            Value::Array(_) => Ok(BUILTIN_STRING_MEMORY._object_Array_.into_value()),
            // 12. Else if O has a [[DateValue]] internal slot, let builtinTag be "Date".
            #[cfg(feature = "date")]
            Value::Date(_) => Ok(BUILTIN_STRING_MEMORY._object_Date_.into_value()),
            // 8. Else if O has an [[ErrorData]] internal slot, let builtinTag be "Error".
            Value::Error(_) => Ok(BUILTIN_STRING_MEMORY._object_Error_.into_value()),
            // 7. Else if O has a [[Call]] internal method, let builtinTag be "Function".
            Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_) => {
                Ok(BUILTIN_STRING_MEMORY._object_Function_.into_value())
            }
            // TODO: Check for [[Call]] slot of Proxy
            Value::Proxy(_) => todo!(),
            // TODO: Check for [[Call]] slot of EmbedderObject
            Value::EmbedderObject(_) => todo!(),
            // 13. Else if O has a [[RegExpMatcher]] internal slot, let builtinTag be "RegExp".
            #[cfg(feature = "regexp")]
            Value::RegExp(_) => Ok(BUILTIN_STRING_MEMORY._object_RegExp_.into_value()),
            Value::PrimitiveObject(idx) => match &agent[idx].data {
                PrimitiveObjectData::Boolean(_) => {
                    Ok(BUILTIN_STRING_MEMORY._object_Boolean_.into_value())
                }
                PrimitiveObjectData::String(_) => {
                    Ok(BUILTIN_STRING_MEMORY._object_String_.into_value())
                }
                PrimitiveObjectData::SmallString(_) => {
                    Ok(BUILTIN_STRING_MEMORY._object_String_.into_value())
                }
                PrimitiveObjectData::Number(_)
                | PrimitiveObjectData::Integer(_)
                | PrimitiveObjectData::SmallF64(_) => {
                    Ok(BUILTIN_STRING_MEMORY._object_Number_.into_value())
                }
                PrimitiveObjectData::Symbol(_)
                | PrimitiveObjectData::BigInt(_)
                | PrimitiveObjectData::SmallBigInt(_) => {
                    let o = to_object(agent, this_value, gc.nogc()).unwrap();
                    let tag = get(
                        agent,
                        o.unbind(),
                        WellKnownSymbolIndexes::ToStringTag.into(),
                        gc.reborrow(),
                    )?;
                    if let Ok(tag) = String::try_from(tag) {
                        let str = format!("[object {}]", tag.as_str(agent));
                        Ok(Value::from_string(agent, str, gc.into_nogc()))
                    } else {
                        let str =
                            format!("[object {}]", BUILTIN_STRING_MEMORY.Object.as_str(agent));
                        Ok(Value::from_string(agent, str, gc.into_nogc()))
                    }
                }
            },
            _ => {
                // 3. Let O be ! ToObject(this value).
                // 15. Let tag be ? Get(O, @@toStringTag).
                // 16. If tag is not a String, set tag to builtinTag.
                let o = to_object(agent, this_value, gc.nogc()).unwrap();
                let tag = get(
                    agent,
                    o.unbind(),
                    WellKnownSymbolIndexes::ToStringTag.into(),
                    gc.reborrow(),
                )?;
                if let Ok(tag) = String::try_from(tag) {
                    let str = format!("[object {}]", tag.as_str(agent));
                    Ok(Value::from_string(agent, str, gc.into_nogc()))
                } else {
                    // 14. Else, let builtinTag be "Object".
                    let str = format!("[object {}]", BUILTIN_STRING_MEMORY.Object.as_str(agent));
                    Ok(Value::from_string(agent, str, gc.into_nogc()))
                }
            }
        }
    }

    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        to_object(agent, this_value, gc.into_nogc()).map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier<'static>) {
        // The Object prototype object:
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        // is %Object.prototype%.
        let this = intrinsics.object_prototype();
        let _to_string_index = intrinsics.object_prototype_to_string();
        let object_constructor = intrinsics.object();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            // has an [[Extensible]] internal slot whose value is true.
            .with_extensible(true)
            // has a [[Prototype]] internal slot whose value is null.
            // .with_prototype(None)
            .with_property_capacity(7)
            .with_constructor_property(object_constructor)
            .with_builtin_function_property::<ObjectPrototypeHasOwnProperty>()
            .with_builtin_function_property::<ObjectPrototypeIsPrototypeOf>()
            .with_builtin_function_property::<ObjectPrototypePropertyIsEnumerable>()
            .with_builtin_function_property::<ObjectPrototypeToLocaleString>()
            .with_builtin_intrinsic_function_property::<ObjectPrototypeToString>()
            .with_builtin_function_property::<ObjectPrototypeValueOf>()
            .build();
    }
}
