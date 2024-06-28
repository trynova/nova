use small_string::SmallString;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_data_property_or_throw, get, has_property,
                length_of_array_like, set,
            },
            testing_and_comparison::{is_array, is_callable},
            type_conversion::{to_boolean, to_integer_or_infinity, to_object, to_string},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{array_species_create, ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{
            Function, IntoFunction, IntoValue, Number, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
    SmallInteger,
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
    /// ### [23.1.3.1 Array.prototype.at ( index )](https://tc39.es/ecma262/#sec-array.prototype.at)
    fn at(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        let index = arguments.get(0);
        // 3. Let relativeIndex be ? ToIntegerOrInfinity(index).
        let relative_index = to_integer_or_infinity(agent, index)?;
        let relative_index = match relative_index {
            Number::Float(_) | Number::Number(_) => {
                // Heap number or f32 here means that the value is over the
                // safe integer limit, which is necessarily >= len
                return Ok(Value::Undefined);
            }
            Number::Integer(int) => int.into_i64(),
        };
        // 4. If relativeIndex ≥ 0, then
        let k = if relative_index >= 0 {
            // a. Let k be relativeIndex.
            relative_index
        } else {
            // 5. Else,
            // a. Let k be len + relativeIndex.
            len + relative_index
        };
        // 6. If k < 0 or k ≥ len, return undefined.
        if k < 0 || k >= len {
            Ok(Value::Undefined)
        } else {
            // 7. Return ? Get(O, ! ToString(𝔽(k))).
            get(agent, o, PropertyKey::Integer(k.try_into().unwrap()))
        }
    }

    /// ### [23.1.3.2 Array.prototype.concat ( ...items )](https://tc39.es/ecma262/#sec-array.prototype.concat)
    ///
    /// This method returns an array containing the array elements of the
    /// object followed by the array elements of each argument.
    ///
    /// > Note 1: The explicit setting of the "length" property in step 6 is
    /// > intended to ensure the length is correct when the final non-empty
    /// > element of items has trailing holes or when A is not a built-in
    /// > Array.
    ///
    /// > Note 2: This method is intentionally generic; it does not require
    /// > that its this value be an Array. Therefore it can be transferred to
    /// > other kinds of objects for use as a method.
    fn concat(agent: &mut Agent, this_value: Value, items: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let A be ? ArraySpeciesCreate(O, 0).
        let a = array_species_create(agent, o, 0)?;
        // 3. Let n be 0.
        let mut n = 0;
        // 4. Prepend O to items.
        let mut items = Vec::from(items.0);
        items.insert(0, o.into_value());
        // 5. For each element E of items, do
        for e in items {
            // a. Let spreadable be ? IsConcatSpreadable(E).
            let e_is_spreadable = is_concat_spreadable(agent, e)?;
            // b. If spreadable is true, then
            if let Some(e) = e_is_spreadable {
                // i. Let len be ? LengthOfArrayLike(E).
                let len = length_of_array_like(agent, e)?;
                // ii. If n + len > 2**53 - 1, throw a TypeError exception.
                if (n + len) > SmallInteger::MAX_NUMBER {
                    return Err(agent.throw_exception(ExceptionType::TypeError, "Array overflow"));
                }
                // iii. Let k be 0.
                let mut k = 0;
                // iv. Repeat, while k < len,
                while k < len {
                    // 1. Let Pk be ! ToString(𝔽(k)).
                    let pk = PropertyKey::Integer(k.try_into().unwrap());
                    // 2. Let exists be ? HasProperty(E, Pk).
                    let exists = has_property(agent, e, pk)?;
                    // 3. If exists is true, then
                    if exists {
                        // a. Let subElement be ? Get(E, Pk).
                        let sub_element = get(agent, e, pk)?;
                        // b. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), subElement).
                        create_data_property_or_throw(
                            agent,
                            a,
                            PropertyKey::Integer(n.try_into().unwrap()),
                            sub_element,
                        )?;
                    }
                    // 4. Set n to n + 1.
                    n = n + 1;
                    // 5. Set k to k + 1.
                    k = k + 1;
                }
            } else {
                // c. Else,
                // i. NOTE: E is added as a single item rather than spread.
                // ii. If n ≥ 2**53 - 1, throw a TypeError exception.
                if n >= SmallInteger::MAX_NUMBER {
                    return Err(agent.throw_exception(ExceptionType::TypeError, "Array overflow"));
                }
                // iii. Perform ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), E).
                create_data_property_or_throw(
                    agent,
                    a,
                    PropertyKey::Integer(n.try_into().unwrap()),
                    e,
                )?;
                // iv. Set n to n + 1.
                n = n + 1;
            }
        }
        // 6. Perform ? Set(A, "length", 𝔽(n), true).
        set(
            agent,
            a,
            BUILTIN_STRING_MEMORY.length.into(),
            Value::try_from(n).unwrap(),
            true,
        )?;
        // 7. Return A.
        Ok(a.into_value())
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

    /// ### [23.1.3.18 Array.prototype.join ( separator )](https://tc39.es/ecma262/#sec-array.prototype.join)
    ///
    /// This method converts the elements of the array to Strings, and then
    /// concatenates these Strings, separated by occurrences of the separator.
    /// If no separator is provided, a single comma is used as the separator.
    fn join(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let separator = arguments.get(0);

        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value)?;
        // 2. Let len be ? LengthOfArrayLike(O).
        let len = length_of_array_like(agent, o)?;
        if len == 0 {
            return Ok(String::EMPTY_STRING.into_value());
        }
        let len = len as usize;
        // 3. If separator is undefined, let sep be ",".
        let separator = if separator.is_undefined() {
            SmallString::from_str_unchecked(",").into()
        } else {
            // 4. Else, let sep be ? ToString(separator).
            to_string(agent, separator)?
        };
        // 5. Let R be the empty String.
        let mut r = std::string::String::with_capacity(len * 10);
        // 6. Let k be 0.
        // 7. Repeat, while k < len,
        // b. Let element be ? Get(O, ! ToString(𝔽(k))).
        {
            let element = get(agent, o, 0.into())?;
            // c. If element is neither undefined nor null, then
            if !element.is_undefined() && !element.is_null() {
                // i. Let S be ? ToString(element).
                let s = to_string(agent, element)?;
                // ii. Set R to the string-concatenation of R and S.
                r.push_str(s.as_str(agent));
            }
        }
        for k in 1..len {
            // a. If k > 0, set R to the string-concatenation of R and sep.
            r.push_str(separator.as_str(agent));
            // b. Let element be ? Get(O, ! ToString(𝔽(k))).
            let element = get(agent, o, SmallInteger::try_from(k as u64).unwrap().into())?;
            // c. If element is neither undefined nor null, then
            if !element.is_undefined() && !element.is_null() {
                // i. Let S be ? ToString(element).
                let s = to_string(agent, element)?;
                // ii. Set R to the string-concatenation of R and S.
                r.push_str(s.as_str(agent));
            }
            // d. Set k to k + 1.
        }
        // 8. Return R.
        Ok(Value::from_string(agent, r).into_value())
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

    /// ### [23.1.3.36 Array.prototype.toString ( )](https://tc39.es/ecma262/#sec-array.prototype.tostring)
    fn to_string(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let array be ? ToObject(this value).
        let array = to_object(agent, this_value)?;
        // 2. Let func be ? Get(array, "join").
        let func = get(agent, array, BUILTIN_STRING_MEMORY.join.into())?;
        // 3. If IsCallable(func) is false, set func to the intrinsic function %Object.prototype.toString%.
        let func = if !is_callable(func) {
            agent
                .current_realm()
                .intrinsics()
                .object_prototype_to_string()
                .into_function()
        } else {
            Function::try_from(func).unwrap()
        };
        // 4. Return ? Call(func, array).
        call_function(agent, func, array.into_value(), None)
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
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.array_prototype();
        let array_constructor = intrinsics.array();
        let array_prototype_values = intrinsics.array_prototype_values();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(41)
            .with_prototype(object_prototype)
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

/// ### [23.1.3.2.1 IsConcatSpreadable ( O )](https://tc39.es/ecma262/#sec-isconcatspreadable)
///
/// The abstract operation IsConcatSpreadable takes argument O (an ECMAScript
/// language value) and returns either a normal completion containing a Boolean
/// or a throw completion.
///
/// > Note: Instead of returning a bool, Nova returns an Option<Object>.

fn is_concat_spreadable(agent: &mut Agent, o: Value) -> JsResult<Option<Object>> {
    // 1. If O is not an Object, return false.
    if let Ok(o) = Object::try_from(o) {
        // 2. Let spreadable be ? Get(O, @@isConcatSpreadable).
        let spreadable = get(agent, o, WellKnownSymbolIndexes::IsConcatSpreadable.into())?;
        // 3. If spreadable is not undefined, return ToBoolean(spreadable).
        if !spreadable.is_undefined() {
            let spreadable = to_boolean(agent, spreadable);
            if spreadable {
                Ok(Some(o))
            } else {
                Ok(None)
            }
        } else {
            // 4. Return ? IsArray(O).
            let o_is_array = is_array(agent, o.into_value())?;
            if o_is_array {
                Ok(Some(o))
            } else {
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}
