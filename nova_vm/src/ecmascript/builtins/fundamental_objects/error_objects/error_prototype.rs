// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{operations_on_objects::get, type_conversion::to_string},
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, Object, PropertyKey, String, Value},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
};

pub(crate) struct ErrorPrototype;

struct ErrorPrototypeToString;

impl Builtin for ErrorPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ErrorPrototype::to_string);
}

impl ErrorPrototype {
    /// ### [20.5.3.4 Error.prototype.toString ( )](https://tc39.es/ecma262/#sec-error.prototype.tostring)
    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                gc.into_nogc(),
            ));
        };
        let scoped_o = o.scope(agent, gc.nogc());
        // 3. Let name be ? Get(O, "name").
        let name = get(
            agent,
            o.unbind(),
            PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 4. If name is undefined, set name to "Error"; otherwise set name to ? ToString(name).
        let name = if name.is_undefined() {
            None
        } else {
            Some(
                to_string(agent, name.unbind(), gc.reborrow())
                    .unbind()?
                    .scope(agent, gc.nogc()),
            )
        };
        // 5. Let msg be ? Get(O, "message").
        let msg = get(
            agent,
            scoped_o.get(agent),
            BUILTIN_STRING_MEMORY.message.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 6. If msg is undefined, set msg to the empty String; otherwise set msg to ? ToString(msg).
        let msg = if msg.is_undefined() {
            String::EMPTY_STRING
        } else {
            to_string(agent, msg.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };
        // No more GC can be triggered.
        let msg = msg.unbind();
        let gc = gc.into_nogc();
        let msg = msg.bind(gc);
        // 4. If name is undefined, set name to "Error"
        let name = name
            .map_or(BUILTIN_STRING_MEMORY.Error, |name| name.get(agent))
            .bind(gc);
        if name.is_empty_string() {
            // 7. If name is the empty String, return msg.
            Ok(msg.into_value())
        } else if msg.is_empty_string() {
            // 8. If msg is the empty String, return name.
            Ok(name.into_value())
        } else {
            // 9. Return the string-concatenation of name, the code unit 0x003A (COLON), the code unit 0x0020 (SPACE), and msg.
            let result = format!("{}: {}", name.to_string_lossy(agent), msg.to_string_lossy(agent));
            Ok(String::from_string(agent, result, gc).into_value())
        }
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.error_prototype();
        let error_constructor = intrinsics.error();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(4)
            .with_prototype(object_prototype)
            .with_constructor_property(error_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.Error.into())
                    .build()
            })
            .with_builtin_function_property::<ErrorPrototypeToString>()
            .build();
    }
}
