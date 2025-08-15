// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::ControlFlow;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{has_own_property, invoke, is_prototype_of_loop, try_get},
            testing_and_comparison::is_array,
            type_conversion::{to_object, to_property_key},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic,
            ordinary::caches::PropertyLookupCache, primitive_objects::PrimitiveObjectData,
        },
        execution::{Agent, JsResult, Realm},
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, IntoObject, IntoValue, Object, PropertyKey,
            String, TryGetResult, Value, handle_try_get_result,
        },
    },
    engine::{
        TryError,
        context::{Bindable, GcScope},
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
    ) -> JsResult<'gc, Value<'gc>> {
        let p = to_property_key(agent, arguments.get(0), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        has_own_property(agent, o.unbind(), p.unbind(), gc).map(|result| result.into_value())
    }

    fn is_prototype_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let v = arguments.get(0).bind(gc.nogc());
        let Ok(v) = Object::try_from(v) else {
            return Ok(false.into());
        };
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let result = is_prototype_of_loop(agent, o.unbind(), v.unbind(), gc)?;
        Ok(result.into_value())
    }

    fn property_is_enumerable<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let p = to_property_key(agent, arguments.get(0), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let o = to_object(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let desc = o
            .unbind()
            .internal_get_own_property(agent, p.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
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
    ) -> JsResult<'gc, Value<'gc>> {
        let o = this_value;
        let p = PropertyKey::from(BUILTIN_STRING_MEMORY.toString);
        invoke(agent, o, p, None, gc)
    }

    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let builtin_tag_with_object_concatenation = match this_value.bind(gc.nogc()) {
            // 1. If the this value is undefined, return "[object Undefined]".
            Value::Undefined => return Ok(BUILTIN_STRING_MEMORY._object_Undefined_.into_value()),
            // 2. If the this value is null, return "[object Null]".
            Value::Null => return Ok(BUILTIN_STRING_MEMORY._object_Null_.into_value()),
            // 9. Else if O has a [[BooleanData]] internal slot, let builtinTag be "Boolean".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::Boolean(_) => BUILTIN_STRING_MEMORY._object_Boolean_,
            // 6. Else if O has a [[ParameterMap]] internal slot, let builtinTag be "Arguments".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::Arguments(_) => BUILTIN_STRING_MEMORY._object_Arguments_,
            // 11. Else if O has a [[StringData]] internal slot, let builtinTag be "String".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::String(_) | Value::SmallString(_) => BUILTIN_STRING_MEMORY._object_String_,
            // 10. Else if O has a [[NumberData]] internal slot, let builtinTag be "Number".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::Number(_) | Value::Integer(_) | Value::SmallF64(_) => {
                BUILTIN_STRING_MEMORY._object_Number_
            }
            // 4. Let isArray be ? IsArray(O).
            // 5. If isArray is true, let builtinTag be "Array".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::Array(_) | Value::Proxy(_)
                if is_array(agent, this_value, gc.nogc()).unbind()? =>
            {
                BUILTIN_STRING_MEMORY._object_Array_
            }
            // 12. Else if O has a [[DateValue]] internal slot, let builtinTag be "Date".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            #[cfg(feature = "date")]
            Value::Date(_) => BUILTIN_STRING_MEMORY._object_Date_,
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            // 8. Else if O has an [[ErrorData]] internal slot, let builtinTag be "Error".
            Value::Error(_) => BUILTIN_STRING_MEMORY._object_Error_,
            // 7. Else if O has a [[Call]] internal method, let builtinTag be "Function".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::BoundFunction(_)
            | Value::BuiltinFunction(_)
            | Value::ECMAScriptFunction(_)
            | Value::BuiltinConstructorFunction(_)
            | Value::BuiltinPromiseResolvingFunction(_) => BUILTIN_STRING_MEMORY._object_Function_,
            Value::Proxy(proxy) if proxy.is_callable(agent, gc.nogc()) => {
                BUILTIN_STRING_MEMORY._object_Function_
            }
            // TODO: Check for [[Call]] slot of EmbedderObject
            Value::EmbedderObject(_) => todo!(),
            // 13. Else if O has a [[RegExpMatcher]] internal slot, let builtinTag be "RegExp".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            #[cfg(feature = "regexp")]
            Value::RegExp(_) => BUILTIN_STRING_MEMORY._object_RegExp_,
            Value::PrimitiveObject(idx) => match &agent[idx].data {
                // 9. Else if O has a [[BooleanData]] internal slot, let builtinTag be "Boolean".
                // 17. Return the string-concatenation of "[object ", tag, and "]".
                PrimitiveObjectData::Boolean(_) => BUILTIN_STRING_MEMORY._object_Boolean_,
                // 11. Else if O has a [[StringData]] internal slot, let builtinTag be "String".
                // 17. Return the string-concatenation of "[object ", tag, and "]".
                PrimitiveObjectData::String(_) | PrimitiveObjectData::SmallString(_) => {
                    BUILTIN_STRING_MEMORY._object_String_
                }
                // 10. Else if O has a [[NumberData]] internal slot, let builtinTag be "Number".
                // 17. Return the string-concatenation of "[object ", tag, and "]".
                PrimitiveObjectData::Number(_)
                | PrimitiveObjectData::Integer(_)
                | PrimitiveObjectData::SmallF64(_) => BUILTIN_STRING_MEMORY._object_Number_,
                PrimitiveObjectData::Symbol(_)
                | PrimitiveObjectData::BigInt(_)
                // 14. Else, let builtinTag be "Object".
                // 17. Return the string-concatenation of "[object ", tag, and "]".
                | PrimitiveObjectData::SmallBigInt(_) => BUILTIN_STRING_MEMORY._object_Object_,
            },
            // 14. Else, let builtinTag be "Object".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            _ => BUILTIN_STRING_MEMORY._object_Object_,
        };
        // 3. Let O be ! ToObject(this value).
        let o_or_prototype = if let Ok(o) = Object::try_from(this_value) {
            o
        } else {
            // Our value is a primitive; this means we'd normally have to
            // create a PrimitiveObject. Usually it's not needed, though, so
            // we'll try to get a tag without calling into JavaScript by
            // accessing @@toStringTag from the prototype directly.
            let intrinsics = agent.current_realm_record().intrinsics();
            match this_value {
                Value::Boolean(_) => intrinsics.boolean_prototype().into_object(),
                Value::String(_) | Value::SmallString(_) => {
                    intrinsics.string_prototype().into_object()
                }
                Value::Symbol(_) => intrinsics.symbol_prototype().into_object(),
                Value::Number(_) | Value::Integer(_) | Value::SmallF64(_) => {
                    intrinsics.number_prototype().into_object()
                }
                Value::BigInt(_) | Value::SmallBigInt(_) => {
                    intrinsics.big_int_prototype().into_object()
                }
                _ => unreachable!(),
            }
        };
        // 15. Let tag be ? Get(O, @@toStringTag).
        let key = WellKnownSymbolIndexes::ToStringTag.into();
        let tag = try_get(
            agent,
            o_or_prototype,
            key,
            PropertyLookupCache::get(agent, key),
            gc.nogc(),
        );
        let tag = match tag {
            // We got a result without creating a primitive object! Good!
            ControlFlow::Continue(TryGetResult::Unset) => Value::Undefined,
            ControlFlow::Continue(TryGetResult::Value(v)) => v,
            ControlFlow::Break(TryError::Err(e)) => {
                return Err(e.unbind().bind(gc.into_nogc()));
            }
            _ => {
                // No such luck: Getting @@toStringTag would call a getter,
                // someone wants to observe the engine at work. We'll have to
                // make pretend for their sake.
                // 3. Let O be ! ToObject(this value).
                let o = to_object(agent, this_value, gc.nogc()).unwrap();
                handle_try_get_result(
                    agent,
                    o.unbind(),
                    WellKnownSymbolIndexes::ToStringTag.into(),
                    tag.unbind(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc())
            }
        };
        if let Ok(tag) = String::try_from(tag) {
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            let str = format!("[object {}]", tag.to_string_lossy(agent));
            Ok(Value::from_string(agent, str, gc.into_nogc()))
        } else {
            // 16. If tag is not a String, set tag to builtinTag.
            Ok(builtin_tag_with_object_concatenation.into_value())
        }
    }

    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        to_object(agent, this_value, gc.into_nogc()).map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
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
