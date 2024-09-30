// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod bigint;
mod function;
mod global_value;
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
    BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
    FunctionInternalProperties,
};
pub use function::{Function, IntoFunction};
pub use global_value::Global;
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
pub(crate) use value::{
    BIGINT_64_ARRAY_DISCRIMINANT, BIGINT_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
    BOOLEAN_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
    FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
    INT_8_ARRAY_DISCRIMINANT, NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT,
    SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
    UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
    UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
};
