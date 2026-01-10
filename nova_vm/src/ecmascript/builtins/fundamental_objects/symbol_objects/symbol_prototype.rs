// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter},
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, PropertyKey, String, Symbol, Value},
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::{ArenaAccess, WellKnownSymbolIndexes},
};

pub(crate) struct SymbolPrototype;

struct SymbolPrototypeGetDescription;
impl Builtin for SymbolPrototypeGetDescription {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_description;

    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.description.to_property_key());

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolPrototype::get_description);
}
impl BuiltinGetter for SymbolPrototypeGetDescription {}

struct SymbolPrototypeToString;
impl Builtin for SymbolPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolPrototype::to_string);
}

struct SymbolPrototypeValueOf;
impl Builtin for SymbolPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolPrototype::value_of);
}

struct SymbolPrototypeToPrimitive;
impl Builtin for SymbolPrototypeToPrimitive {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_toPrimitive_;

    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::ToPrimitive.to_property_key());

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolPrototype::value_of);

    const WRITABLE: bool = false;
}

impl SymbolPrototype {
    /// ### [20.4.3.2 get Symbol.prototype.description](https://tc39.es/ecma262/#sec-symbol.prototype.description)
    ///
    /// Symbol.prototype.description is an accessor property whose set accessor
    /// function is undefined.
    fn get_description<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let s be the this value.
        // 2. Let sym be ? ThisSymbolValue(s).
        let sym = this_symbol_value(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.into_nogc());
        // 3. Return sym.[[Description]].
        sym.get(agent)
            .descriptor
            .map_or_else(|| Ok(Value::Undefined), |desc| Ok(desc.into()))
    }

    /// ### [20.4.3.3 Symbol.prototype.toString ( )](https://tc39.es/ecma262/#sec-symbol.prototype.tostring)
    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Let sym be ? ThisSymbolValue(this value).
        let symb = this_symbol_value(agent, this_value, gc)?;
        // 2. Return SymbolDescriptiveString(sym).
        Ok(symbol_descriptive_string(agent, symb, gc).into())
    }

    /// ### [20.4.3.4 Symbol.prototype.valueOf ( )](https://tc39.es/ecma262/#sec-symbol.prototype.valueof)
    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Return ? ThisSymbolValue(this value).
        this_symbol_value(agent, this_value, gc).map(|res| res.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.symbol_prototype();
        let symbol_constructor = intrinsics.symbol();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(6)
            .with_prototype(object_prototype)
            .with_constructor_property(symbol_constructor)
            .with_builtin_function_getter_property::<SymbolPrototypeGetDescription>()
            .with_builtin_function_property::<SymbolPrototypeToString>()
            .with_builtin_function_property::<SymbolPrototypeValueOf>()
            .with_builtin_function_property::<SymbolPrototypeToPrimitive>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Symbol.into())
                    .with_enumerable(false)
                    .build()
            })
            .build();
    }
}

#[inline(always)]
fn this_symbol_value<'a>(
    agent: &mut Agent,
    value: Value<'a>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Symbol<'a>> {
    match value {
        Value::Symbol(symbol) => Ok(symbol.unbind()),
        Value::PrimitiveObject(object) if object.is_symbol_object(agent) => {
            let s: Symbol = object.get(agent).data.try_into().unwrap();
            Ok(s)
        }
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "this is not a symbol",
            gc,
        )),
    }
}

/// ### [20.4.3.3.1 SymbolDescriptiveString ( sym )](https://tc39.es/ecma262/#sec-symboldescriptivestring)
///
/// The abstract operation SymbolDescriptiveString takes argument sym (a Symbol)
/// and returns a String.
fn symbol_descriptive_string<'gc>(
    agent: &mut Agent,
    sym: Symbol,
    gc: NoGcScope<'gc, '_>,
) -> String<'gc> {
    // 1. Let desc be sym's [[Description]] value.
    let desc = sym.get(agent).descriptor;
    // 2. If desc is undefined, set desc to the empty String.
    if let Some(desc) = desc {
        // 3. Assert: desc is a String.
        // 4. Return the string-concatenation of "Symbol(", desc, and ")".
        let result = format!("Symbol({})", desc.to_string_lossy(agent));
        String::from_string(agent, result, gc)
    } else {
        BUILTIN_STRING_MEMORY.Symbol__
    }
}
