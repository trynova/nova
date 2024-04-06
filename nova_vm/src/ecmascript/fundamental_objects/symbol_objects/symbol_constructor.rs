use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::SymbolHeapData;
use crate::ecmascript::types::Value;
use crate::heap::indexes::SymbolIndex;
use crate::heap::WellKnownSymbolIndexes;

pub(crate) struct SymbolConstructor;

impl Builtin for SymbolConstructor {
    const NAME: &'static str = "Symbol";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}

struct SymbolFor;

impl Builtin for SymbolFor {
    const NAME: &'static str = "for";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolConstructor::r#for);
}

struct SymbolKeyFor;

impl Builtin for SymbolKeyFor {
    const NAME: &'static str = "keyFor";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolConstructor::key_for);
}

impl SymbolConstructor {
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        if new_target.is_some() {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Symbol is not a constructor")
            );
        }
        let description = arguments.get(0);
        let desc_string = if description.is_undefined() {
            None
        } else {
            Some(to_string(agent, description)?)
        };
        agent.heap.symbols.push(Some(SymbolHeapData {
            descriptor: desc_string,
        }));
        Ok(Value::Symbol(SymbolIndex::last(&agent.heap.symbols)))
    }

    fn r#for(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn key_for(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.symbol();
        let this_object_index = intrinsics.symbol_base_object();
        let symbol_prototype = intrinsics.symbol_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<SymbolConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(16)
        .with_property(|builder| {
            builder
                .with_key_from_str("asyncIterator")
                .with_value_readonly(WellKnownSymbolIndexes::AsyncIterator.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(SymbolFor::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<SymbolFor>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("hasInstance")
                .with_value_readonly(WellKnownSymbolIndexes::HasInstance.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("isConcatSpreadable")
                .with_value_readonly(WellKnownSymbolIndexes::IsConcatSpreadable.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("iterator")
                .with_value_readonly(WellKnownSymbolIndexes::Iterator.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(SymbolKeyFor::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<SymbolKeyFor>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("match")
                .with_value_readonly(WellKnownSymbolIndexes::Match.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("matchAll")
                .with_value_readonly(WellKnownSymbolIndexes::MatchAll.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("prototype")
                .with_value_readonly(symbol_prototype.into_value())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("replace")
                .with_value_readonly(WellKnownSymbolIndexes::Replace.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("search")
                .with_value_readonly(WellKnownSymbolIndexes::Search.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("species")
                .with_value_readonly(WellKnownSymbolIndexes::Species.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("split")
                .with_value_readonly(WellKnownSymbolIndexes::Split.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("toPrimitive")
                .with_value_readonly(WellKnownSymbolIndexes::ToPrimitive.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("toStringTag")
                .with_value_readonly(WellKnownSymbolIndexes::ToStringTag.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("unscopables")
                .with_value_readonly(WellKnownSymbolIndexes::Unscopables.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .build();
    }
}
