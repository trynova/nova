// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::testing_and_comparison::is_array;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builders::ordinary_object_builder::OrdinaryObjectBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinGetter;
use crate::ecmascript::builtins::BuiltinIntrinsic;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::RealmIdentifier;

use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::PropertyKey;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::heap::IntrinsicConstructorIndexes;
use crate::heap::IntrinsicFunctionIndexes;
use crate::heap::WellKnownSymbolIndexes;

pub struct TypedArrayIntrinsicObject;

impl Builtin for TypedArrayIntrinsicObject {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.TypedArray;
}
impl BuiltinIntrinsicConstructor for TypedArrayIntrinsicObject {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TypedArray;
}

struct TypedArrayFrom;
impl Builtin for TypedArrayFrom {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayIntrinsicObject::from);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.from;
}
struct TypedArrayOf;
impl Builtin for TypedArrayOf {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayIntrinsicObject::of);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.fromCodePoint;
}
struct TypedArrayGetSpecies;
impl Builtin for TypedArrayGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayIntrinsicObject::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for TypedArrayGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}
impl TypedArrayIntrinsicObject {
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        Err(agent.throw_exception(
            crate::ecmascript::execution::agent::ExceptionType::TypeError,
            "Abstract class TypedArray not directly constructable",
        ))
    }

    fn from(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn is_array(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        is_array(agent, arguments.get(0)).map(Value::Boolean)
    }

    fn of(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn get_species(_: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let typed_array_prototype = intrinsics.typed_array_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TypedArrayIntrinsicObject>(
            agent, realm,
        )
        .with_property_capacity(4)
        .with_builtin_function_property::<TypedArrayFrom>()
        .with_builtin_function_property::<TypedArrayOf>()
        .with_prototype_property(typed_array_prototype.into_object())
        .with_builtin_function_getter_property::<TypedArrayGetSpecies>()
        .build();
    }
}

pub(crate) struct TypedArrayPrototype;

struct TypedArrayPrototypeAt;
impl Builtin for TypedArrayPrototypeAt {
    const NAME: String = BUILTIN_STRING_MEMORY.at;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::at);
}
struct TypedArrayPrototypeGetBuffer;
impl Builtin for TypedArrayPrototypeGetBuffer {
    const NAME: String = BUILTIN_STRING_MEMORY.get_buffer;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_buffer);
}
impl BuiltinGetter for TypedArrayPrototypeGetBuffer {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.buffer.to_property_key();
}
struct TypedArrayPrototypeGetByteLength;
impl Builtin for TypedArrayPrototypeGetByteLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_byteLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_byte_length);
}
impl BuiltinGetter for TypedArrayPrototypeGetByteLength {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.byteLength.to_property_key();
}
struct TypedArrayPrototypeGetByteOffset;
impl Builtin for TypedArrayPrototypeGetByteOffset {
    const NAME: String = BUILTIN_STRING_MEMORY.get_byteOffset;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_byte_offset);
}
impl BuiltinGetter for TypedArrayPrototypeGetByteOffset {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.byteOffset.to_property_key();
}
struct TypedArrayPrototypeCopyWithin;
impl Builtin for TypedArrayPrototypeCopyWithin {
    const NAME: String = BUILTIN_STRING_MEMORY.copyWithin;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::copy_within);
}
struct TypedArrayPrototypeEntries;
impl Builtin for TypedArrayPrototypeEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::entries);
}
struct TypedArrayPrototypeEvery;
impl Builtin for TypedArrayPrototypeEvery {
    const NAME: String = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::every);
}
struct TypedArrayPrototypeFill;
impl Builtin for TypedArrayPrototypeFill {
    const NAME: String = BUILTIN_STRING_MEMORY.fill;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::fill);
}
struct TypedArrayPrototypeFilter;
impl Builtin for TypedArrayPrototypeFilter {
    const NAME: String = BUILTIN_STRING_MEMORY.filter;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::filter);
}
struct TypedArrayPrototypeFind;
impl Builtin for TypedArrayPrototypeFind {
    const NAME: String = BUILTIN_STRING_MEMORY.find;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find);
}
struct TypedArrayPrototypeFindIndex;
impl Builtin for TypedArrayPrototypeFindIndex {
    const NAME: String = BUILTIN_STRING_MEMORY.findIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find_index);
}
struct TypedArrayPrototypeFindLast;
impl Builtin for TypedArrayPrototypeFindLast {
    const NAME: String = BUILTIN_STRING_MEMORY.findLast;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find_last);
}
struct TypedArrayPrototypeFindLastIndex;
impl Builtin for TypedArrayPrototypeFindLastIndex {
    const NAME: String = BUILTIN_STRING_MEMORY.findLastIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::find_last_index);
}
struct TypedArrayPrototypeForEach;
impl Builtin for TypedArrayPrototypeForEach {
    const NAME: String = BUILTIN_STRING_MEMORY.forEach;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::for_each);
}
struct TypedArrayPrototypeIncludes;
impl Builtin for TypedArrayPrototypeIncludes {
    const NAME: String = BUILTIN_STRING_MEMORY.includes;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::includes);
}
struct TypedArrayPrototypeIndexOf;
impl Builtin for TypedArrayPrototypeIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.indexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::index_of);
}
struct TypedArrayPrototypeJoin;
impl Builtin for TypedArrayPrototypeJoin {
    const NAME: String = BUILTIN_STRING_MEMORY.join;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::join);
}
struct TypedArrayPrototypeKeys;
impl Builtin for TypedArrayPrototypeKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::keys);
}
struct TypedArrayPrototypeLastIndexOf;
impl Builtin for TypedArrayPrototypeLastIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::last_index_of);
}
struct TypedArrayPrototypeGetLength;
impl Builtin for TypedArrayPrototypeGetLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_length;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_length);
}
impl BuiltinGetter for TypedArrayPrototypeGetLength {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.length.to_property_key();
}
struct TypedArrayPrototypeMap;
impl Builtin for TypedArrayPrototypeMap {
    const NAME: String = BUILTIN_STRING_MEMORY.map;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::map);
}
struct TypedArrayPrototypeReduce;
impl Builtin for TypedArrayPrototypeReduce {
    const NAME: String = BUILTIN_STRING_MEMORY.reduce;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::reduce);
}
struct TypedArrayPrototypeReduceRight;
impl Builtin for TypedArrayPrototypeReduceRight {
    const NAME: String = BUILTIN_STRING_MEMORY.reduceRight;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::reduce_right);
}
struct TypedArrayPrototypeReverse;
impl Builtin for TypedArrayPrototypeReverse {
    const NAME: String = BUILTIN_STRING_MEMORY.reverse;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::reverse);
}
struct TypedArrayPrototypeSet;
impl Builtin for TypedArrayPrototypeSet {
    const NAME: String = BUILTIN_STRING_MEMORY.set;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::set);
}
struct TypedArrayPrototypeSlice;
impl Builtin for TypedArrayPrototypeSlice {
    const NAME: String = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::slice);
}
struct TypedArrayPrototypeSome;
impl Builtin for TypedArrayPrototypeSome {
    const NAME: String = BUILTIN_STRING_MEMORY.some;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::some);
}
struct TypedArrayPrototypeSort;
impl Builtin for TypedArrayPrototypeSort {
    const NAME: String = BUILTIN_STRING_MEMORY.sort;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::sort);
}
struct TypedArrayPrototypeSubarray;
impl Builtin for TypedArrayPrototypeSubarray {
    const NAME: String = BUILTIN_STRING_MEMORY.subarray;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::subarray);
}
struct TypedArrayPrototypeToLocaleString;
impl Builtin for TypedArrayPrototypeToLocaleString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::to_locale_string);
}
struct TypedArrayPrototypeToReversed;
impl Builtin for TypedArrayPrototypeToReversed {
    const NAME: String = BUILTIN_STRING_MEMORY.toReversed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::to_reversed);
}
struct TypedArrayPrototypeToSorted;
impl Builtin for TypedArrayPrototypeToSorted {
    const NAME: String = BUILTIN_STRING_MEMORY.toSorted;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::to_sorted);
}
struct TypedArrayPrototypeToString;
impl Builtin for TypedArrayPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::to_string);
}
struct TypedArrayPrototypeValues;
impl Builtin for TypedArrayPrototypeValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::values);
}
impl BuiltinIntrinsic for TypedArrayPrototypeValues {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::TypedArrayPrototypeValues;
}
struct TypedArrayPrototypeWith;
impl Builtin for TypedArrayPrototypeWith {
    const NAME: String = BUILTIN_STRING_MEMORY.with;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::with);
}
struct TypedArrayPrototypeGetToStringTag;
impl Builtin for TypedArrayPrototypeGetToStringTag {
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_toStringTag_;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TypedArrayPrototype::get_to_string_tag);
}
impl BuiltinGetter for TypedArrayPrototypeGetToStringTag {
    const KEY: PropertyKey = WellKnownSymbolIndexes::ToStringTag.to_property_key();
}

impl TypedArrayPrototype {
    fn at(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_buffer(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_byte_length(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_byte_offset(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
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

    fn get_length(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn map(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
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

    fn set(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
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

    fn subarray(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
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

    fn values(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn with(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn get_to_string_tag(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.typed_array_prototype();
        let typed_array_constructor = intrinsics.typed_array();
        let typed_array_prototype_values = intrinsics.typed_array_prototype_values();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(38)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<TypedArrayPrototypeAt>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetBuffer>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetByteLength>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetByteOffset>()
            .with_constructor_property(typed_array_constructor)
            .with_builtin_function_property::<TypedArrayPrototypeCopyWithin>()
            .with_builtin_function_property::<TypedArrayPrototypeEntries>()
            .with_builtin_function_property::<TypedArrayPrototypeEvery>()
            .with_builtin_function_property::<TypedArrayPrototypeFill>()
            .with_builtin_function_property::<TypedArrayPrototypeFilter>()
            .with_builtin_function_property::<TypedArrayPrototypeFind>()
            .with_builtin_function_property::<TypedArrayPrototypeFindIndex>()
            .with_builtin_function_property::<TypedArrayPrototypeFindLast>()
            .with_builtin_function_property::<TypedArrayPrototypeFindLastIndex>()
            .with_builtin_function_property::<TypedArrayPrototypeForEach>()
            .with_builtin_function_property::<TypedArrayPrototypeIncludes>()
            .with_builtin_function_property::<TypedArrayPrototypeIndexOf>()
            .with_builtin_function_property::<TypedArrayPrototypeJoin>()
            .with_builtin_function_property::<TypedArrayPrototypeKeys>()
            .with_builtin_function_property::<TypedArrayPrototypeLastIndexOf>()
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetLength>()
            .with_builtin_function_property::<TypedArrayPrototypeMap>()
            .with_builtin_function_property::<TypedArrayPrototypeReduce>()
            .with_builtin_function_property::<TypedArrayPrototypeReduceRight>()
            .with_builtin_function_property::<TypedArrayPrototypeReverse>()
            .with_builtin_function_property::<TypedArrayPrototypeSet>()
            .with_builtin_function_property::<TypedArrayPrototypeSlice>()
            .with_builtin_function_property::<TypedArrayPrototypeSome>()
            .with_builtin_function_property::<TypedArrayPrototypeSort>()
            .with_builtin_function_property::<TypedArrayPrototypeSubarray>()
            .with_builtin_function_property::<TypedArrayPrototypeToLocaleString>()
            .with_builtin_function_property::<TypedArrayPrototypeToReversed>()
            .with_builtin_function_property::<TypedArrayPrototypeToSorted>()
            .with_builtin_function_property::<TypedArrayPrototypeToString>()
            .with_builtin_intrinsic_function_property::<TypedArrayPrototypeValues>()
            .with_builtin_function_property::<TypedArrayPrototypeWith>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value(typed_array_prototype_values.into_value())
                    .with_enumerable(TypedArrayPrototypeValues::ENUMERABLE)
                    .with_configurable(TypedArrayPrototypeValues::CONFIGURABLE)
                    .build()
            })
            .with_builtin_function_getter_property::<TypedArrayPrototypeGetToStringTag>()
            .build();
    }
}
