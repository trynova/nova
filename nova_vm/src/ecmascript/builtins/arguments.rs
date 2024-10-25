// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [10.4.4 Arguments Exotic Objects](https://tc39.es/ecma262/#sec-arguments-exotic-objects)
//!
//! Most ECMAScript functions make an arguments object available to their code. Depending upon the characteristics of the function definition, its arguments object is either an ordinary object or an arguments exotic object. An arguments exotic object is an exotic object whose array index properties map to the formal parameters bindings of an invocation of its associated ECMAScript function.
//!
//! An object is an arguments exotic object if its internal methods use the following implementations, with the ones not specified here using those found in 10.1. These methods are installed in CreateMappedArgumentsObject.
//!
//! #### Note 1
//!
//! While CreateUnmappedArgumentsObject is grouped into this clause, it creates an ordinary object, not an arguments exotic object.
//!
//! Arguments exotic objects have the same internal slots as ordinary objects. They also have a [[ParameterMap]] internal slot. Ordinary arguments objects also have a [[ParameterMap]] internal slot whose value is always undefined. For ordinary argument objects the [[ParameterMap]] internal slot is only used by Object.prototype.toString (20.1.3.6) to identify them as such.
//! #### Note 2
//!
//! The integer-indexed data properties of an arguments exotic object whose numeric name values are less than the number of formal parameters of the corresponding function object initially share their values with the corresponding argument bindings in the function's execution context. This means that changing the property changes the corresponding value of the argument binding and vice-versa. This correspondence is broken if such a property is deleted and then redefined or if the property is changed into an accessor property. If the arguments object is an ordinary object, the values of its properties are simply a copy of the arguments passed to the function and there is no dynamic linkage between the property values and the formal parameter values.
//! #### Note 3
//!
//! The ParameterMap object and its property values are used as a device for specifying the arguments object correspondence to argument bindings. The ParameterMap object and the objects that are the values of its properties are not directly observable from ECMAScript code. An ECMAScript implementation does not need to actually create or use such objects to implement the specified semantics.
//! #### Note 4
//!
//! Ordinary arguments objects define a non-configurable accessor property named "callee" which throws a TypeError exception on access. The "callee" property has a more specific meaning for arguments exotic objects, which are created only for some class of non-strict functions. The definition of this property in the ordinary variant exists to ensure that it is not defined in any other manner by conforming ECMAScript implementations.
//! #### Note 5
//!
//! ECMAScript implementations of arguments exotic objects have historically contained an accessor property named "caller". Prior to ECMAScript 2017, this specification included the definition of a throwing "caller" property on ordinary arguments objects. Since implementations do not contain this extension any longer, ECMAScript 2017 dropped the requirement for a throwing "caller" accessor.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::{
            create_data_property_or_throw, define_property_or_throw,
        },
        execution::{agent::Agent, ProtoIntrinsics},
        types::{
            IntoFunction, IntoValue, Number, Object, PropertyDescriptor, PropertyKey, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::WellKnownSymbolIndexes,
};

use super::ordinary::ordinary_object_create_with_intrinsics;

// 10.4.4.1 [[GetOwnProperty]] ( P )

// The [[GetOwnProperty]] internal method of an arguments exotic object args takes argument P (a property key) and returns a normal completion containing either a Property Descriptor or undefined. It performs the following steps when called:

//     1. Let desc be OrdinaryGetOwnProperty(args, P).
//     2. If desc is undefined, return undefined.
//     3. Let map be args.[[ParameterMap]].
//     4. Let isMapped be ! HasOwnProperty(map, P).
//     5. If isMapped is true, then
//         a. Set desc.[[Value]] to ! Get(map, P).
//     6. Return desc.

// 10.4.4.2 [[DefineOwnProperty]] ( P, Desc )

// The [[DefineOwnProperty]] internal method of an arguments exotic object args takes arguments P (a property key) and Desc (a Property Descriptor) and returns a normal completion containing a Boolean. It performs the following steps when called:

//     1. Let map be args.[[ParameterMap]].
//     2. Let isMapped be ! HasOwnProperty(map, P).
//     3. Let newArgDesc be Desc.
//     4. If isMapped is true and IsDataDescriptor(Desc) is true, then
//         a. If Desc does not have a [[Value]] field, Desc has a [[Writable]] field, and Desc.[[Writable]] is false, then
//             i. Set newArgDesc to a copy of Desc.
//             ii. Set newArgDesc.[[Value]] to ! Get(map, P).
//     5. Let allowed be ! OrdinaryDefineOwnProperty(args, P, newArgDesc).
//     6. If allowed is false, return false.
//     7. If isMapped is true, then
//         a. If IsAccessorDescriptor(Desc) is true, then
//             i. Perform ! map.[[Delete]](P).
//         b. Else,
//             i. If Desc has a [[Value]] field, then
//                 1. Assert: The following Set will succeed, since formal parameters mapped by arguments objects are always writable.
//                 2. Perform ! Set(map, P, Desc.[[Value]], false).
//             ii. If Desc has a [[Writable]] field and Desc.[[Writable]] is false, then
//                 1. Perform ! map.[[Delete]](P).
//     8. Return true.

// 10.4.4.3 [[Get]] ( P, Receiver )

// The [[Get]] internal method of an arguments exotic object args takes arguments P (a property key) and Receiver (an ECMAScript language value) and returns either a normal completion containing an ECMAScript language value or a throw completion. It performs the following steps when called:

//     1. Let map be args.[[ParameterMap]].
//     2. Let isMapped be ! HasOwnProperty(map, P).
//     3. If isMapped is false, then
//         a. Return ? OrdinaryGet(args, P, Receiver).
//     4. Else,
//         a. Assert: map contains a formal parameter mapping for P.
//         b. Return ! Get(map, P).

// 10.4.4.4 [[Set]] ( P, V, Receiver )

// The [[Set]] internal method of an arguments exotic object args takes arguments P (a property key), V (an ECMAScript language value), and Receiver (an ECMAScript language value) and returns either a normal completion containing a Boolean or a throw completion. It performs the following steps when called:

//     1. If SameValue(args, Receiver) is false, then
//         a. Let isMapped be false.
//     2. Else,
//         a. Let map be args.[[ParameterMap]].
//         b. Let isMapped be ! HasOwnProperty(map, P).
//     3. If isMapped is true, then
//         a. Assert: The following Set will succeed, since formal parameters mapped by arguments objects are always writable.
//         b. Perform ! Set(map, P, V, false).
//     4. Return ? OrdinarySet(args, P, V, Receiver).

// 10.4.4.5 [[Delete]] ( P )

// The [[Delete]] internal method of an arguments exotic object args takes argument P (a property key) and returns either a normal completion containing a Boolean or a throw completion. It performs the following steps when called:

//     1. Let map be args.[[ParameterMap]].
//     2. Let isMapped be ! HasOwnProperty(map, P).
//     3. Let result be ? OrdinaryDelete(args, P).
//     4. If result is true and isMapped is true, then
//         a. Perform ! map.[[Delete]](P).
//     5. Return result.

/// ### [10.4.4.6 CreateUnmappedArgumentsObject ( argumentsList )](https://tc39.es/ecma262/#sec-createunmappedargumentsobject)
///
/// The abstract operation CreateUnmappedArgumentsObject takes argument
/// argumentsList (a List of ECMAScript language values) and returns an
/// ordinary object.
pub(crate) fn create_unmapped_arguments_object(
    agent: &mut Agent,
    mut gc: GcScope<'_, '_>,

    arguments_list: &[Value],
) -> Object {
    // 1. Let len be the number of elements in argumentsList.
    let len = arguments_list.len();
    let len = Number::from_f64(agent, len as f64).into_value();
    // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%, « [[ParameterMap]] »).
    let obj = ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None);
    let Object::Object(obj) = obj else {
        unreachable!()
    };
    // 3. Set obj.[[ParameterMap]] to undefined.
    // 4. Perform ! DefinePropertyOrThrow(obj, "length", PropertyDescriptor {
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.length);
    define_property_or_throw(
        agent,
        gc.reborrow(),
        obj,
        key,
        PropertyDescriptor {
            // [[Value]]: 𝔽(len),
            value: Some(len),
            // [[Writable]]: true,
            writable: Some(true),
            // [[Enumerable]]: false,
            enumerable: Some(false),
            // [[Configurable]]: true }).
            configurable: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    // 5. Let index be 0.
    // 6. Repeat, while index < len,
    for (index, val) in arguments_list.iter().enumerate() {
        // a. Let val be argumentsList[index].
        // b. Perform ! CreateDataPropertyOrThrow(obj, ! ToString(𝔽(index)), val).
        debug_assert!(index < u32::MAX as usize);
        let index = index as u32;
        let key = PropertyKey::Integer(index.into());
        create_data_property_or_throw(agent, gc.reborrow(), obj, key, *val).unwrap();
        // c. Set index to index + 1.
    }
    // 7. Perform ! DefinePropertyOrThrow(obj, @@iterator, PropertyDescriptor {
    let key = PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into());
    // TODO: "as if the property was accessed prior to any ECMAScript code being evaluated"
    // let array_prototype_values = agent.current_realm().intrinsics().array_prototype();
    define_property_or_throw(
        agent,
        gc.reborrow(),
        obj,
        key,
        PropertyDescriptor {
            // [[Value]]: %Array.prototype.values%,
            value: Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .array_prototype_values()
                    .into_value(),
            ),
            // [[Writable]]: true,
            writable: Some(true),
            // [[Enumerable]]: false,
            enumerable: Some(false),
            // [[Configurable]]: true }).
            configurable: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    let throw_type_error = agent.current_realm().intrinsics().throw_type_error();
    // 8. Perform ! DefinePropertyOrThrow(obj, "callee", PropertyDescriptor {
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.callee);
    define_property_or_throw(
        agent,
        gc.reborrow(),
        obj,
        key,
        PropertyDescriptor {
            // [[Get]]: %ThrowTypeError%,
            get: Some(throw_type_error.into_function()),
            // [[Set]]: %ThrowTypeError%,
            set: Some(throw_type_error.into_function()),
            // [[Enumerable]]: false,
            enumerable: Some(false),
            // [[Configurable]]: false }).
            configurable: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    // 9. Return obj.
    Object::Arguments(obj)
}

// 10.4.4.7 CreateMappedArgumentsObject ( func, formals, argumentsList, env )

// The abstract operation CreateMappedArgumentsObject takes arguments func (an Object), formals (a Parse Node), argumentsList (a List of ECMAScript language values), and env (an Environment Record) and returns an arguments exotic object. It performs the following steps when called:

//     1. Assert: formals does not contain a rest parameter, any binding patterns, or any initializers. It may contain duplicate identifiers.
//     2. Let len be the number of elements in argumentsList.
//     3. Let obj be MakeBasicObject(« [[Prototype]], [[Extensible]], [[ParameterMap]] »).
//     4. Set obj.[[GetOwnProperty]] as specified in 10.4.4.1.
//     5. Set obj.[[DefineOwnProperty]] as specified in 10.4.4.2.
//     6. Set obj.[[Get]] as specified in 10.4.4.3.
//     7. Set obj.[[Set]] as specified in 10.4.4.4.
//     8. Set obj.[[Delete]] as specified in 10.4.4.5.
//     9. Set obj.[[Prototype]] to %Object.prototype%.
//     10. Let map be OrdinaryObjectCreate(null).
//     11. Set obj.[[ParameterMap]] to map.
//     12. Let parameterNames be the BoundNames of formals.
//     13. Let numberOfParameters be the number of elements in parameterNames.
//     14. Let index be 0.
//     15. Repeat, while index < len,
//         a. Let val be argumentsList[index].
//         b. Perform ! CreateDataPropertyOrThrow(obj, ! ToString(𝔽(index)), val).
//         c. Set index to index + 1.
//     16. Perform ! DefinePropertyOrThrow(obj, "length", PropertyDescriptor { [[Value]]: 𝔽(len), [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }).
//     17. Let mappedNames be a new empty List.
//     18. Set index to numberOfParameters - 1.
//     19. Repeat, while index ≥ 0,
//         a. Let name be parameterNames[index].
//         b. If mappedNames does not contain name, then
//             i. Append name to mappedNames.
//             ii. If index < len, then
//                 1. Let g be MakeArgGetter(name, env).
//                 2. Let p be MakeArgSetter(name, env).
//                 3. Perform ! map.[[DefineOwnProperty]](! ToString(𝔽(index)), PropertyDescriptor { [[Set]]: p, [[Get]]: g, [[Enumerable]]: false, [[Configurable]]: true }).
//         c. Set index to index - 1.
//     20. Perform ! DefinePropertyOrThrow(obj, @@iterator, PropertyDescriptor { [[Value]]: %Array.prototype.values%, [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }).
//     21. Perform ! DefinePropertyOrThrow(obj, "callee", PropertyDescriptor { [[Value]]: func, [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }).
//     22. Return obj.

// 10.4.4.7.1 MakeArgGetter ( name, env )

// The abstract operation MakeArgGetter takes arguments name (a String) and env (an Environment Record) and returns a function object. It creates a built-in function object that when executed returns the value bound for name in env. It performs the following steps when called:

//     1. Let getterClosure be a new Abstract Closure with no parameters that captures name and env and performs the following steps when called:
//         a. Return env.GetBindingValue(name, false).
//     2. Let getter be CreateBuiltinFunction(getterClosure, 0, "", « »).
//     3. NOTE: getter is never directly accessible to ECMAScript code.
//     4. Return getter.

// 10.4.4.7.2 MakeArgSetter ( name, env )

// The abstract operation MakeArgSetter takes arguments name (a String) and env (an Environment Record) and returns a function object. It creates a built-in function object that when executed sets the value bound for name in env. It performs the following steps when called:

//     1. Let setterClosure be a new Abstract Closure with parameters (value) that captures name and env and performs the following steps when called:
//         a. Return ! env.SetMutableBinding(name, value, false).
//     2. Let setter be CreateBuiltinFunction(setterClosure, 1, "", « »).
//     3. NOTE: setter is never directly accessible to ECMAScript code.
//     4. Return setter.
