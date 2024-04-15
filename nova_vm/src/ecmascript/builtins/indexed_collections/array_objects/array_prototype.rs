use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct ArrayPrototype;

struct ArrayPrototypeAt;
impl Builtin for ArrayPrototypeAt {
    const NAME: String = BUILTIN_STRING_MEMORY.at;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::at);
}
struct ArrayPrototypeConcat;
impl Builtin for ArrayPrototypeConcat {
    const NAME: String = BUILTIN_STRING_MEMORY.concat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::concat);
}
struct ArrayPrototypeCopyWithin;
impl Builtin for ArrayPrototypeCopyWithin {
    const NAME: String = BUILTIN_STRING_MEMORY.copyWithin;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::copy_within);
}
struct ArrayPrototypeEntries;
impl Builtin for ArrayPrototypeEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::entries);
}
struct ArrayPrototypeEvery;
impl Builtin for ArrayPrototypeEvery {
    const NAME: String = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::every);
}
struct ArrayPrototypeFill;
impl Builtin for ArrayPrototypeFill {
    const NAME: String = BUILTIN_STRING_MEMORY.fill;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::fill);
}
struct ArrayPrototypeFilter;
impl Builtin for ArrayPrototypeFilter {
    const NAME: String = BUILTIN_STRING_MEMORY.filter;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::filter);
}
struct ArrayPrototypeFind;
impl Builtin for ArrayPrototypeFind {
    const NAME: String = BUILTIN_STRING_MEMORY.find;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find);
}
struct ArrayPrototypeFindIndex;
impl Builtin for ArrayPrototypeFindIndex {
    const NAME: String = BUILTIN_STRING_MEMORY.findIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_index);
}
struct ArrayPrototypeFindLast;
impl Builtin for ArrayPrototypeFindLast {
    const NAME: String = BUILTIN_STRING_MEMORY.findLast;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_last);
}
struct ArrayPrototypeFindLastIndex;
impl Builtin for ArrayPrototypeFindLastIndex {
    const NAME: String = BUILTIN_STRING_MEMORY.findLastIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::find_last_index);
}
struct ArrayPrototypeFlat;
impl Builtin for ArrayPrototypeFlat {
    const NAME: String = BUILTIN_STRING_MEMORY.flat;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::flat);
}
struct ArrayPrototypeFlatMap;
impl Builtin for ArrayPrototypeFlatMap {
    const NAME: String = BUILTIN_STRING_MEMORY.flatMap;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::flat_map);
}
struct ArrayPrototypeForEach;
impl Builtin for ArrayPrototypeForEach {
    const NAME: String = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::for_each);
}
struct ArrayPrototypeIncludes;
impl Builtin for ArrayPrototypeIncludes {
    const NAME: String = BUILTIN_STRING_MEMORY.includes;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::includes);
}
struct ArrayPrototypeIndexOf;
impl Builtin for ArrayPrototypeIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.indexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::index_of);
}
struct ArrayPrototypeJoin;
impl Builtin for ArrayPrototypeJoin {
    const NAME: String = BUILTIN_STRING_MEMORY.join;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::join);
}
struct ArrayPrototypeKeys;
impl Builtin for ArrayPrototypeKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::keys);
}
struct ArrayPrototypeLastIndexOf;
impl Builtin for ArrayPrototypeLastIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::last_index_of);
}
struct ArrayPrototypeMap;
impl Builtin for ArrayPrototypeMap {
    const NAME: String = BUILTIN_STRING_MEMORY.map;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::map);
}
struct ArrayPrototypePop;
impl Builtin for ArrayPrototypePop {
    const NAME: String = BUILTIN_STRING_MEMORY.pop;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::pop);
}
struct ArrayPrototypePush;
impl Builtin for ArrayPrototypePush {
    const NAME: String = BUILTIN_STRING_MEMORY.push;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::push);
}
struct ArrayPrototypeReduce;
impl Builtin for ArrayPrototypeReduce {
    const NAME: String = BUILTIN_STRING_MEMORY.reduce;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reduce);
}
struct ArrayPrototypeReduceRight;
impl Builtin for ArrayPrototypeReduceRight {
    const NAME: String = BUILTIN_STRING_MEMORY.reduceRight;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reduce_right);
}
struct ArrayPrototypeReverse;
impl Builtin for ArrayPrototypeReverse {
    const NAME: String = BUILTIN_STRING_MEMORY.reverse;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::reverse);
}
struct ArrayPrototypeShift;
impl Builtin for ArrayPrototypeShift {
    const NAME: String = BUILTIN_STRING_MEMORY.shift;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::shift);
}
struct ArrayPrototypeSlice;
impl Builtin for ArrayPrototypeSlice {
    const NAME: String = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::slice);
}
struct ArrayPrototypeSome;
impl Builtin for ArrayPrototypeSome {
    const NAME: String = BUILTIN_STRING_MEMORY.some;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::some);
}
struct ArrayPrototypeSort;
impl Builtin for ArrayPrototypeSort {
    const NAME: String = BUILTIN_STRING_MEMORY.sort;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::sort);
}
impl BuiltinIntrinsic for ArrayPrototypeSort {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeSort;
}
struct ArrayPrototypeSplice;
impl Builtin for ArrayPrototypeSplice {
    const NAME: String = BUILTIN_STRING_MEMORY.splice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::splice);
}
struct ArrayPrototypeToLocaleString;
impl Builtin for ArrayPrototypeToLocaleString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_locale_string);
}
struct ArrayPrototypeToReversed;
impl Builtin for ArrayPrototypeToReversed {
    const NAME: String = BUILTIN_STRING_MEMORY.toReversed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_reversed);
}
struct ArrayPrototypeToSorted;
impl Builtin for ArrayPrototypeToSorted {
    const NAME: String = BUILTIN_STRING_MEMORY.toSorted;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_sorted);
}
struct ArrayPrototypeToSpliced;
impl Builtin for ArrayPrototypeToSpliced {
    const NAME: String = BUILTIN_STRING_MEMORY.toSpliced;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_spliced);
}
struct ArrayPrototypeToString;
impl Builtin for ArrayPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::to_string);
}
impl BuiltinIntrinsic for ArrayPrototypeToString {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeToString;
}
struct ArrayPrototypeUnshift;
impl Builtin for ArrayPrototypeUnshift {
    const NAME: String = BUILTIN_STRING_MEMORY.unshift;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::unshift);
}
struct ArrayPrototypeValues;
impl Builtin for ArrayPrototypeValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::values);
}
impl BuiltinIntrinsic for ArrayPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ArrayPrototypeValues;
}
struct ArrayPrototypeWith;
impl Builtin for ArrayPrototypeWith {
    const NAME: String = BUILTIN_STRING_MEMORY.with;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayPrototype::with);
}

impl ArrayPrototype {
    fn at(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn concat(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn copy_within(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn entries(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn every(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn fill(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn filter(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn find(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn find_index(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn find_last(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn find_last_index(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn flat(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn flat_map(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn for_each(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn includes(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn index_of(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn join(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn keys(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn last_index_of(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn map(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_locale_lower_case(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn pop(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn push(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn reduce(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn reduce_right(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn reverse(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn shift(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn slice(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn some(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn sort(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn splice(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn to_locale_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn to_reversed(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn to_sorted(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn to_spliced(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn to_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn unshift(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn values(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn with(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.array_prototype();
        let array_constructor = intrinsics.array();
        let array_prototype_values = intrinsics.array_prototype_values();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(41)
            .with_builtin_function_property::<ArrayPrototypeAt>()
            .with_builtin_function_property::<ArrayPrototypeConcat>()
            .with_constructor_property(array_constructor)
            .with_builtin_function_property::<ArrayPrototypeCopyWithin>()
            .with_builtin_function_property::<ArrayPrototypeEntries>()
            .with_builtin_function_property::<ArrayPrototypeEvery>()
            .with_builtin_function_property::<ArrayPrototypeFill>()
            .with_builtin_function_property::<ArrayPrototypeFilter>()
            .with_builtin_function_property::<ArrayPrototypeFind>()
            .with_builtin_function_property::<ArrayPrototypeFindIndex>()
            .with_builtin_function_property::<ArrayPrototypeFindLast>()
            .with_builtin_function_property::<ArrayPrototypeFindLastIndex>()
            .with_builtin_function_property::<ArrayPrototypeFlat>()
            .with_builtin_function_property::<ArrayPrototypeFlatMap>()
            .with_builtin_function_property::<ArrayPrototypeForEach>()
            .with_builtin_function_property::<ArrayPrototypeIncludes>()
            .with_builtin_function_property::<ArrayPrototypeIndexOf>()
            .with_builtin_function_property::<ArrayPrototypeJoin>()
            .with_builtin_function_property::<ArrayPrototypeKeys>()
            .with_builtin_function_property::<ArrayPrototypeLastIndexOf>()
            .with_builtin_function_property::<ArrayPrototypeMap>()
            .with_builtin_function_property::<ArrayPrototypePop>()
            .with_builtin_function_property::<ArrayPrototypePush>()
            .with_builtin_function_property::<ArrayPrototypeReduce>()
            .with_builtin_function_property::<ArrayPrototypeReduceRight>()
            .with_builtin_function_property::<ArrayPrototypeReverse>()
            .with_builtin_function_property::<ArrayPrototypeShift>()
            .with_builtin_function_property::<ArrayPrototypeSlice>()
            .with_builtin_function_property::<ArrayPrototypeSome>()
            .with_builtin_intrinsic_function_property::<ArrayPrototypeSort>()
            .with_builtin_function_property::<ArrayPrototypeSplice>()
            .with_builtin_function_property::<ArrayPrototypeToLocaleString>()
            .with_builtin_function_property::<ArrayPrototypeToReversed>()
            .with_builtin_function_property::<ArrayPrototypeToSorted>()
            .with_builtin_function_property::<ArrayPrototypeToSpliced>()
            .with_builtin_intrinsic_function_property::<ArrayPrototypeToString>()
            .with_builtin_function_property::<ArrayPrototypeUnshift>()
            .with_builtin_intrinsic_function_property::<ArrayPrototypeValues>()
            .with_builtin_function_property::<ArrayPrototypeWith>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(array_prototype_values.into_value())
                    .with_enumerable(ArrayPrototypeValues::ENUMERABLE)
                    .with_configurable(ArrayPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Unscopables.into())
                    .with_value_creator_readonly(|agent| {
                        OrdinaryObjectBuilder::new(agent, realm)
                            .with_property_capacity(16)
                            .with_data_property(BUILTIN_STRING_MEMORY.at.into(), true.into())
                            .with_data_property(
                                BUILTIN_STRING_MEMORY.copyWithin.into(),
                                true.into(),
                            )
                            .with_data_property(BUILTIN_STRING_MEMORY.entries.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.fill.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.find.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.findIndex.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.findLast.into(), true.into())
                            .with_data_property(
                                BUILTIN_STRING_MEMORY.findLastIndex.into(),
                                true.into(),
                            )
                            .with_data_property(BUILTIN_STRING_MEMORY.flat.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.flatMap.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.includes.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.keys.into(), true.into())
                            .with_data_property(
                                BUILTIN_STRING_MEMORY.toReversed.into(),
                                true.into(),
                            )
                            .with_data_property(BUILTIN_STRING_MEMORY.toSorted.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.toSpliced.into(), true.into())
                            .with_data_property(BUILTIN_STRING_MEMORY.values.into(), true.into())
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .build();
    }
}
