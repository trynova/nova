// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_string,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, IntoObject, IntoValue, Object, String, SymbolHeapData, Value,
        },
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
            .into_value())
    }

    /// ### [20.4.2.2 Symbol.for ( key )](https://tc39.es/ecma262/#sec-symbol.for)
    fn r#for<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let stringKey be ? ToString(key).
        // 2. For each element e of the GlobalSymbolRegistry List, do
        //        a. If e.[[Key]] is stringKey, return e.[[Symbol]].
        // 3. Assert: The GlobalSymbolRegistry List does not currently contain an entry for stringKey.
        // 4. Let newSymbol be a new Symbol whose [[Description]] is stringKey.
        // 5. Append the GlobalSymbolRegistry Record { [[Key]]: stringKey, [[Symbol]]: newSymbol } to the GlobalSymbolRegistry List.
        // 6. Return newSymbol.
        Err(agent.todo("Symbol.for", gc.into_nogc()))
    }

    /// ### [20.4.2.6 Symbol.keyFor ( sym )](https://tc39.es/ecma262/#sec-symbol.keyfor)
    fn key_for<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. If sym is not a Symbol, throw a TypeError exception.
        // 2. Return KeyForSymbol(sym).
        Err(agent.todo("Symbol.keyFor", gc.into_nogc()))
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
        let builder = builder.with_prototype_property(symbol_prototype.into_object());
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
