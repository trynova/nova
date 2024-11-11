// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod bigint;
mod function;
mod into_numeric;
mod into_primitive;
mod into_value;
mod number;
mod numeric;
mod object;
mod primitive;
mod string;
mod symbol;
mod value;

pub use bigint::{BigInt, BigIntHeapData};
pub(crate) use function::{
    function_create_backing_object, function_internal_define_own_property,
    function_internal_delete, function_internal_get, function_internal_get_own_property,
    function_internal_has_property, function_internal_own_property_keys, function_internal_set,
    BoundFunctionHeapData, BuiltinConstructorHeapData, BuiltinFunctionHeapData,
    ECMAScriptFunctionHeapData, FunctionInternalProperties,
};
pub use function::{Function, IntoFunction};
pub use into_numeric::IntoNumeric;
pub use into_primitive::IntoPrimitive;
pub use into_value::IntoValue;
pub use number::{HeapNumber, Number, NumberHeapData};
pub use numeric::Numeric;
pub use object::{
    InternalMethods, InternalSlots, IntoObject, Object, ObjectHeapData, OrdinaryObject, PropertyKey,
};
pub(crate) use primitive::HeapPrimitive;
pub use primitive::Primitive;
pub use string::{HeapString, String, StringHeapData, BUILTIN_STRINGS_LIST, BUILTIN_STRING_MEMORY};
pub use symbol::{Symbol, SymbolHeapData};
pub use value::Value;
#[cfg(feature = "date")]
pub(crate) use value::DATE_DISCRIMINANT;
#[cfg(feature = "shared-array-buffer")]
pub(crate) use value::SHARED_ARRAY_BUFFER_DISCRIMINANT;
pub(crate) use value::{
    ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
    ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT, ASYNC_ITERATOR_DISCRIMINANT, BIGINT_DISCRIMINANT,
    BOOLEAN_DISCRIMINANT, BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
    ECMASCRIPT_FUNCTION_DISCRIMINANT, EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT,
    FINALIZATION_REGISTRY_DISCRIMINANT, FLOAT_DISCRIMINANT, GENERATOR_DISCRIMINANT,
    INTEGER_DISCRIMINANT, ITERATOR_DISCRIMINANT, MAP_DISCRIMINANT, MAP_ITERATOR_DISCRIMINANT,
    MODULE_DISCRIMINANT, NUMBER_DISCRIMINANT, OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT,
    PROXY_DISCRIMINANT, REGEXP_DISCRIMINANT, SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT,
    SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
};
#[cfg(feature = "array-buffer")]
pub(crate) use value::{
    ARRAY_BUFFER_DISCRIMINANT, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
    DATA_VIEW_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
    INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT,
    UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
    UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
};
#[cfg(feature = "weak-refs")]
pub(crate) use value::{WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT};
