// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Builtin, BuiltinGetter},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{
            IntoValue, PropertyKey, String, Symbol, SymbolHeapData, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SymbolPrototype;

struct SymbolPrototypeGetDescription;
impl Builtin for SymbolPrototypeGetDescription {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_description;

    const KEY: Option<PropertyKey> = Some(BUILTIN_STRING_MEMORY.description.to_property_key());

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::get_description);
}
impl BuiltinGetter for SymbolPrototypeGetDescription {}

struct SymbolPrototypeToString;
impl Builtin for SymbolPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::to_string);
}

struct SymbolPrototypeValueOf;
impl Builtin for SymbolPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::value_of);
}

struct SymbolPrototypeToPrimitive;
impl Builtin for SymbolPrototypeToPrimitive {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_toPrimitive_;

    const KEY: Option<PropertyKey> = Some(WellKnownSymbolIndexes::ToPrimitive.to_property_key());

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::value_of);

    const WRITABLE: bool = false;
}

impl SymbolPrototype {
    /// ### [20.4.3.2 get Symbol.prototype.description](https://tc39.es/ecma262/multipage/fundamental-objects.html#sec-symbol.prototype.description)
    ///
    /// Symbol.prototype.description is an accessor property whose set accessor
    /// function is undefined.
    fn get_description(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let s be the this value.
        // 2. Let sym be ? ThisSymbolValue(s).
        let sym = this_symbol_value(agent, gc.nogc(), this_value)?;
        // 3. Return sym.[[Description]].
        agent[sym]
            .descriptor
            .map_or_else(|| Ok(Value::Undefined), |desc| Ok(desc.into_value()))
    }

    fn to_string(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        let symb = this_symbol_value(agent, gc.nogc(), this_value)?;
        Ok(symbol_descriptive_string(agent, gc.nogc(), symb).into_value())
    }

    fn value_of(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        this_symbol_value(agent, gc.nogc(), this_value).map(|res| res.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.symbol_prototype();
        let symbol_constructor = intrinsics.symbol();

        agent.heap.symbols.extend_from_slice(
            &[
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_asyncIterator),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_hasInstance),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_isConcatSpreadable),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_iterator),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_match),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_matchAll),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_replace),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_search),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_species),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_split),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_toPrimitive),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_toStringTag),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_unscopables),
                },
            ]
            .map(Some),
        );

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
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Symbol.into_value())
                    .with_enumerable(false)
                    .build()
            })
            .build();
    }
}

#[inline(always)]
fn this_symbol_value<'a>(
    agent: &mut Agent,
    gc: NoGcScope<'a, '_>,
    value: Value,
) -> JsResult<Symbol<'a>> {
    match value {
        Value::Symbol(symbol) => Ok(symbol),
        Value::PrimitiveObject(object) if object.is_symbol_object(agent) => {
            let s: Symbol = agent[object].data.try_into().unwrap();
            Ok(s)
        }
        _ => Err(agent.throw_exception_with_static_message(
            gc,
            ExceptionType::TypeError,
            "this is not a symbol",
        )),
    }
}

/// ### [20.4.3.3.1 SymbolDescriptiveString ( sym )](https://tc39.es/ecma262/#sec-symboldescriptivestring)
///
/// The abstract operation SymbolDescriptiveString takes argument sym (a Symbol)
/// and returns a String.
fn symbol_descriptive_string<'gc>(
    agent: &mut Agent,
    gc: NoGcScope<'gc, '_>,
    sym: Symbol,
) -> String<'gc> {
    // 1. Let desc be sym's [[Description]] value.
    let desc = agent[sym].descriptor;
    // 2. If desc is undefined, set desc to the empty String.
    if let Some(desc) = desc {
        // 3. Assert: desc is a String.
        // 4. Return the string-concatenation of "Symbol(", desc, and ")".
        let result = format!("Symbol({})", desc.as_str(agent));
        String::from_string(agent, gc, result)
    } else {
        BUILTIN_STRING_MEMORY.Symbol__
    }
}
