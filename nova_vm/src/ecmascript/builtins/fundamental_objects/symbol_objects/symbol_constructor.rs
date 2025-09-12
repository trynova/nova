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
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        if new_target.is_some() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Symbol is not a constructor",
                gc.into_nogc(),
            ));
        }
        let description = arguments.get(0).bind(gc.nogc());
        let desc_string = if description.is_undefined() {
            None
        } else {
            Some(to_string(agent, description.unbind(), gc)?.unbind())
        };

        Ok(agent
            .heap
            .create(SymbolHeapData {
                descriptor: desc_string,
            })
            .into_value())
    }

    fn r#for<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(arguments.get(0).unbind())
    }

    fn key_for<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(arguments.get(0).unbind())
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
