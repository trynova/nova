// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter,
        ExceptionType, JsResult, PropertyKey, Realm, String, Symbol, Value,
        builders::OrdinaryObjectBuilder,
    },
    engine::{Bindable, GcScope, NoGcScope},
    heap::{ArenaAccess, WellKnownSymbols},
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

    const KEY: Option<PropertyKey<'static>> = Some(WellKnownSymbols::ToPrimitive.to_property_key());

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
        let gc = gc.into_nogc();
        // 1. Let s be the this value.
        // 2. Let sym be ? ThisSymbolValue(s).
        let sym = this_symbol_value(agent, this_value.bind(gc), gc)?;
        // 3. Return sym.[[Description]].
        sym.description(agent)
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
        Ok(symb.descriptive_string(agent, gc).into())
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
                    .with_key(WellKnownSymbols::ToStringTag.into())
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
