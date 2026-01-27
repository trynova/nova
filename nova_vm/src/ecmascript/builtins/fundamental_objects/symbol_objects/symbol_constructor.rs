// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinFunctionBuilder,
        BuiltinIntrinsicConstructor, ExceptionType, JsResult, Object, Realm, String, Symbol,
        SymbolHeapData, Value, to_string,
    },
    engine::context::{Bindable, GcScope},
    heap::{CreateHeapData, IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct SymbolConstructor;

impl Builtin for SymbolConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Symbol;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for SymbolConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Symbol;
}

struct SymbolFor;

impl Builtin for SymbolFor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.r#for;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolConstructor::r#for);
}

struct SymbolKeyFor;

impl Builtin for SymbolKeyFor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.keyFor;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolConstructor::key_for);
}

impl SymbolConstructor {
    /// ### [20.4.1.1 Symbol ( \[ description \] )](https://tc39.es/ecma262/#sec-symbol-description)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let description = arguments.get(0).bind(gc.nogc());
        // 1. If NewTarget is not undefined, throw a TypeError exception.
        if new_target.is_some() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Symbol is not a constructor",
                gc.into_nogc(),
            ));
        }
        // 2. If description is undefined,
        let desc_string = if description.is_undefined() {
            // let descString be undefined.
            None
        } else {
            // 3. Else, let descString be ? ToString(description).
            Some(to_string(agent, description.unbind(), gc)?.unbind())
        };

        // 4. Return a new Symbol whose [[Description]] is descString.
        Ok(agent
            .heap
            .create(SymbolHeapData {
                descriptor: desc_string,
            })
            .into())
    }

    /// ### [20.4.2.2 Symbol.for ( key )](https://tc39.es/ecma262/#sec-symbol.for)
    fn r#for<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let key = arguments.get(0).bind(gc.nogc());
        // 1. Let stringKey be ? ToString(key).
        let string_key = to_string(agent, key.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 2. For each element e of the GlobalSymbolRegistry List, do
        //        a. If e.[[Key]] is stringKey, return e.[[Symbol]].
        if let Some(&symbol) = agent.global_symbol_registry.get(&string_key.unbind()) {
            return Ok(symbol.into());
        }

        // 3. Assert: The GlobalSymbolRegistry List does not currently contain an entry for stringKey.
        // 4. Let newSymbol be a new Symbol whose [[Description]] is stringKey.
        let new_symbol = agent.heap.create(SymbolHeapData {
            descriptor: Some(string_key.unbind()),
        });

        // 5. Append the GlobalSymbolRegistry Record { [[Key]]: stringKey, [[Symbol]]: newSymbol } to the GlobalSymbolRegistry List.
        agent
            .global_symbol_registry
            .insert(string_key.unbind(), new_symbol);

        // 6. Return newSymbol.
        Ok(new_symbol.into())
    }

    /// ### [20.4.2.6 Symbol.keyFor ( sym )](https://tc39.es/ecma262/#sec-symbol.keyfor)
    fn key_for<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let sym = arguments.get(0).bind(gc.nogc());

        // 1. If sym is not a Symbol, throw a TypeError exception.
        let symbol = match sym.unbind().try_into() {
            Ok(symbol) => symbol,
            Err(_) => {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Symbol.keyFor argument is not a symbol",
                    gc.into_nogc(),
                ));
            }
        };

        // 2. Return KeyForSymbol(sym).
        Ok(key_for_symbol(agent, symbol).map_or(Value::Undefined, |key| key.into()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let symbol_prototype = intrinsics.symbol_prototype();

        let builder =
            BuiltinFunctionBuilder::new_intrinsic_constructor::<SymbolConstructor>(agent, realm)
                .with_property_capacity(16)
                .with_property(|builder| {
                    builder
                        .with_key(BUILTIN_STRING_MEMORY.asyncIterator.into())
                        .with_value_readonly(WellKnownSymbolIndexes::AsyncIterator.into())
                        .with_enumerable(false)
                        .with_configurable(false)
                        .build()
                })
                .with_builtin_function_property::<SymbolFor>()
                .with_property(|builder| {
                    builder
                        .with_key(BUILTIN_STRING_MEMORY.hasInstance.into())
                        .with_value_readonly(WellKnownSymbolIndexes::HasInstance.into())
                        .with_enumerable(false)
                        .with_configurable(false)
                        .build()
                })
                .with_property(|builder| {
                    builder
                        .with_key(BUILTIN_STRING_MEMORY.isConcatSpreadable.into())
                        .with_value_readonly(WellKnownSymbolIndexes::IsConcatSpreadable.into())
                        .with_enumerable(false)
                        .with_configurable(false)
                        .build()
                })
                .with_property(|builder| {
                    builder
                        .with_key(BUILTIN_STRING_MEMORY.iterator.into())
                        .with_value_readonly(WellKnownSymbolIndexes::Iterator.into())
                        .with_enumerable(false)
                        .with_configurable(false)
                        .build()
                })
                .with_builtin_function_property::<SymbolKeyFor>();
        #[cfg(feature = "regexp")]
        let builder = builder
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.r#match.into())
                    .with_value_readonly(WellKnownSymbolIndexes::Match.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.matchAll.into())
                    .with_value_readonly(WellKnownSymbolIndexes::MatchAll.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            });
        let builder = builder.with_prototype_property(symbol_prototype.into());
        #[cfg(feature = "regexp")]
        let builder = builder
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.replace.into())
                    .with_value_readonly(WellKnownSymbolIndexes::Replace.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.search.into())
                    .with_value_readonly(WellKnownSymbolIndexes::Search.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            });
        let builder = builder.with_property(|builder| {
            builder
                .with_key(BUILTIN_STRING_MEMORY.species.into())
                .with_value_readonly(WellKnownSymbolIndexes::Species.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        });
        #[cfg(feature = "regexp")]
        let builder = builder.with_property(|builder| {
            builder
                .with_key(BUILTIN_STRING_MEMORY.split.into())
                .with_value_readonly(WellKnownSymbolIndexes::Split.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        });
        builder
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.toPrimitive.into())
                    .with_value_readonly(WellKnownSymbolIndexes::ToPrimitive.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.toStringTag.into())
                    .with_value_readonly(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.unscopables.into())
                    .with_value_readonly(WellKnownSymbolIndexes::Unscopables.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .build();
    }
}

/// ### [20.4.5.1 KeyForSymbol ( sym )](https://tc39.es/ecma262/#sec-keyforsymbol)
///
/// The abstract operation KeyForSymbol takes argument sym (a Symbol) and returns a String or undefined.
pub(crate) fn key_for_symbol<'a>(agent: &Agent, sym: Symbol<'a>) -> Option<String<'a>> {
    // 1. For each element e of the GlobalSymbolRegistry List, do
    //        a. If SameValue(e.[[Symbol]], sym) is true, return e.[[Key]].
    for (key, &symbol) in &agent.global_symbol_registry {
        if symbol == sym {
            return Some(*key);
        }
    }
    // 2. Assert: The GlobalSymbolRegistry List does not currently contain an entry for sym.
    // 3. Return undefined.
    None
}
