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

use std::collections::TryReserveError;

use ahash::AHashMap;

use crate::{
    ecmascript::{
        execution::agent::Agent,
        types::{
            BUILTIN_STRING_MEMORY, IntoFunction, IntoObject, IntoValue, Number, Object,
            OrdinaryObject, Value,
        },
    },
    engine::context::{Bindable, NoGcScope},
    heap::{WellKnownSymbolIndexes, element_array::ElementDescriptor},
};

use super::{ScopedArgumentsList, ordinary::shape::ObjectShape};

/// #### [10.4.4.6 CreateUnmappedArgumentsObject ( argumentsList )](https://tc39.es/ecma262/#sec-createunmappedargumentsobject)
///
/// The abstract operation CreateUnmappedArgumentsObject takes argument
/// argumentsList (a List of ECMAScript language values) and returns an
/// ordinary object.
pub(crate) fn create_unmapped_arguments_object<'a, 'b>(
    agent: &mut Agent,
    arguments_list: &ScopedArgumentsList<'b>,
    gc: NoGcScope<'a, 'b>,
) -> Result<Object<'a>, TryReserveError> {
    // 1. Let len be the number of elements in argumentsList.
    let len = arguments_list.len(agent);
    // SAFETY: GC is not allowed in this scope, and no other scoped values are
    // accessed during this call. The pointer is not held beyond the current call scope.
    let arguments_non_null_slice = unsafe { arguments_list.as_non_null_slice(agent) };
    debug_assert!(len < u32::MAX as usize);
    let len = len as u32;
    let len_value = Number::from(len).into_value();
    // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%, « [[ParameterMap]] »).
    let prototype = agent.current_realm_record().intrinsics().object_prototype();
    let mut shape = ObjectShape::get_shape_for_prototype(agent, Some(prototype.into_object()));
    shape = shape.get_child_shape(agent, BUILTIN_STRING_MEMORY.length.to_property_key())?;
    shape = shape.get_child_shape(agent, BUILTIN_STRING_MEMORY.callee.into())?;
    shape = shape.get_child_shape(agent, WellKnownSymbolIndexes::Iterator.into())?;
    for index in 0..len {
        shape = shape.get_child_shape(agent, index.into())?;
    }
    let obj = OrdinaryObject::create_object_with_shape(agent, shape)
        .expect("Failed to create Arguments object storage");
    let array_prototype_values = agent
        .current_realm_record()
        .intrinsics()
        .array_prototype_values()
        .bind(gc)
        .into_value();
    let throw_type_error = agent
        .current_realm_record()
        .intrinsics()
        .throw_type_error()
        .into_function()
        .bind(gc);
    let storage = obj.get_elements_storage_mut(agent);
    let values = storage.values;
    let descriptors = storage.descriptors.or_insert(AHashMap::with_capacity(3));

    // 3. Set obj.[[ParameterMap]] to undefined.
    // 4. Perform ! DefinePropertyOrThrow(obj, "length", PropertyDescriptor {
    // [[Value]]: 𝔽(len),
    // [[Writable]]: true,
    // [[Enumerable]]: false,
    // [[Configurable]]: true
    // }).

    // "length"
    values[0] = Some(len_value.unbind());
    // "callee"
    values[1] = None;
    // Iterator
    values[2] = Some(array_prototype_values.unbind());
    // "length"
    descriptors.insert(0, ElementDescriptor::WritableUnenumerableConfigurableData);
    // "callee"
    descriptors.insert(
        1,
        ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor {
            get: throw_type_error.unbind(),
            set: throw_type_error.unbind(),
        },
    );
    // Iterator
    descriptors.insert(2, ElementDescriptor::WritableUnenumerableConfigurableData);
    // 5. Let index be 0.
    // 6. Repeat, while index < len,
    for index in 0..len {
        // a. Let val be argumentsList[index].
        // b. Perform ! CreateDataPropertyOrThrow(obj, ! ToString(𝔽(index)), val).
        // SAFETY: arguments slice valid in this call stack and we've not
        // performed GC or touched other scoped data.
        let val = unsafe { arguments_non_null_slice.as_ref() }
            .get(index as usize)
            .cloned()
            .unwrap_or(Value::Undefined);
        values[index as usize + 3] = Some(val);
        // c. Set index to index + 1.
    }
    // 7. Perform ! DefinePropertyOrThrow(obj, @@iterator, PropertyDescriptor {
    // [[Value]]: %Array.prototype.values%,
    // [[Writable]]: true,
    // [[Enumerable]]: false,
    // [[Configurable]]: true
    // }).
    // 8. Perform ! DefinePropertyOrThrow(obj, "callee", PropertyDescriptor {
    // [[Get]]: %ThrowTypeError%,
    // [[Set]]: %ThrowTypeError%,
    // [[Enumerable]]: false,
    // [[Configurable]]: false
    // }).
    // 9. Return obj.
    Ok(Object::Arguments(obj))
}
