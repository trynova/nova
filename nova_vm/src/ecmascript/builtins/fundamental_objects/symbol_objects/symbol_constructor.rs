use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::IntoObject;

use crate::ecmascript::types::Object;
use crate::ecmascript::types::String;
use crate::ecmascript::types::SymbolHeapData;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::heap::indexes::SymbolIndex;
use crate::heap::IntrinsicConstructorIndexes;
use crate::heap::WellKnownSymbolIndexes;

pub(crate) struct SymbolConstructor;

impl Builtin for SymbolConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Symbol;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}
impl BuiltinIntrinsicConstructor for SymbolConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Symbol;
}

struct SymbolFor;

impl Builtin for SymbolFor {
    const NAME: String = BUILTIN_STRING_MEMORY.r#for;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SymbolConstructor::r#for);
}

struct SymbolKeyFor;

impl Builtin for SymbolKeyFor {
    const NAME: String = BUILTIN_STRING_MEMORY.keyFor;

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
        let symbol_prototype = intrinsics.symbol_prototype();

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
            .with_builtin_function_property::<SymbolKeyFor>()
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
            })
            .with_prototype_property(symbol_prototype.into_object())
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
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.species.into())
                    .with_value_readonly(WellKnownSymbolIndexes::Species.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.split.into())
                    .with_value_readonly(WellKnownSymbolIndexes::Split.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
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
