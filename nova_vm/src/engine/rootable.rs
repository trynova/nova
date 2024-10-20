// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod global;
mod scoped;

use private::RootableSealed;

#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{data_view::DataView, ArrayBuffer};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
#[cfg(feature = "date")]
use crate::ecmascript::types::DATE_DISCRIMINANT;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::types::SHARED_ARRAY_BUFFER_DISCRIMINANT;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::types::{
    ARRAY_BUFFER_DISCRIMINANT, BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
    DATA_VIEW_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT,
    INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT,
    UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
    UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::types::{
    WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT,
};
#[cfg(feature = "array-buffer")]
use crate::heap::indexes::TypedArrayIndex;
use crate::{
    ecmascript::{
        builtins::{
            bound_function::BoundFunction,
            embedder_object::EmbedderObject,
            error::Error,
            finalization_registry::FinalizationRegistry,
            generator_objects::Generator,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator,
            keyed_collections::{
                map_objects::map_iterator_objects::map_iterator::MapIterator,
                set_objects::set_iterator_objects::set_iterator::SetIterator,
            },
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction,
            proxy::Proxy,
            regexp::RegExp,
            set::Set,
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
        },
        types::{
            bigint::HeapBigInt, HeapNumber, HeapString, OrdinaryObject, Symbol,
            ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
            ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT, ASYNC_ITERATOR_DISCRIMINANT,
            BIGINT_DISCRIMINANT, BOUND_FUNCTION_DISCRIMINANT,
            BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT, BUILTIN_FUNCTION_DISCRIMINANT,
            BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
            BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
            BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
            ECMASCRIPT_FUNCTION_DISCRIMINANT, EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT,
            FINALIZATION_REGISTRY_DISCRIMINANT, GENERATOR_DISCRIMINANT, ITERATOR_DISCRIMINANT,
            MAP_DISCRIMINANT, MAP_ITERATOR_DISCRIMINANT, MODULE_DISCRIMINANT, NUMBER_DISCRIMINANT,
            OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT, REGEXP_DISCRIMINANT,
            SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT, STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
        },
    },
    heap::HeapMarkAndSweep,
};

mod private {
    #[cfg(feature = "date")]
    use crate::ecmascript::builtins::date::Date;
    #[cfg(feature = "shared-array-buffer")]
    use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
    #[cfg(feature = "array-buffer")]
    use crate::ecmascript::builtins::{data_view::DataView, typed_array::TypedArray, ArrayBuffer};
    #[cfg(feature = "weak-refs")]
    use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
    use crate::ecmascript::{
        builtins::{
            bound_function::BoundFunction,
            embedder_object::EmbedderObject,
            error::Error,
            finalization_registry::FinalizationRegistry,
            generator_objects::Generator,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator,
            keyed_collections::{
                map_objects::map_iterator_objects::map_iterator::MapIterator,
                set_objects::set_iterator_objects::set_iterator::SetIterator,
            },
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction,
            proxy::Proxy,
            regexp::RegExp,
            set::Set,
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
        },
        types::{
            BigInt, Function, Number, Numeric, Object, OrdinaryObject, Primitive, String, Symbol,
            Value,
        },
    };

    /// Marker trait to make Rootable not implementable outside of nova_vm.
    pub trait RootableSealed {}
    impl RootableSealed for Array {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for ArrayBuffer {}
    impl RootableSealed for ArrayIterator {}
    impl RootableSealed for BigInt {}
    impl RootableSealed for BoundFunction {}
    impl RootableSealed for BuiltinConstructorFunction {}
    impl RootableSealed for BuiltinFunction {}
    impl RootableSealed for BuiltinPromiseResolvingFunction {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for DataView {}
    #[cfg(feature = "date")]
    impl RootableSealed for Date {}
    impl RootableSealed for ECMAScriptFunction {}
    impl RootableSealed for EmbedderObject {}
    impl RootableSealed for Error {}
    impl RootableSealed for FinalizationRegistry {}
    impl RootableSealed for Function {}
    impl RootableSealed for Generator {}
    impl RootableSealed for Map {}
    impl RootableSealed for MapIterator {}
    impl RootableSealed for Module {}
    impl RootableSealed for Number {}
    impl RootableSealed for Numeric {}
    impl RootableSealed for Object {}
    impl RootableSealed for OrdinaryObject {}
    impl RootableSealed for Primitive {}
    impl RootableSealed for PrimitiveObject {}
    impl RootableSealed for Promise {}
    impl RootableSealed for Proxy {}
    impl RootableSealed for RegExp {}
    impl RootableSealed for Set {}
    impl RootableSealed for SetIterator {}
    #[cfg(feature = "shared-array-buffer")]
    impl RootableSealed for SharedArrayBuffer {}
    impl RootableSealed for String {}
    impl RootableSealed for Symbol {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for TypedArray {}
    impl RootableSealed for Value {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakMap {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakRef {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakSet {}
}

pub use global::Global;
pub use scoped::Scoped;

pub trait Rootable: std::fmt::Debug + Copy + RootableSealed {
    type RootRepr: Sized + std::fmt::Debug;

    /// Convert a rootable value to a root representation directly if the value
    /// does not need to be rooted, or return its heap root representation as
    /// the error value.
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData>;

    /// Convert a rootable value's root representation to the value type
    /// directly if it didn't need to be rooted in the first place, or return
    /// its heap root reference as the error value.
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef>;

    /// Convert a heap root reference to a root representation.
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr;

    /// Convert the rooted type's heap data value to the type itself. A failure
    /// to convert indicates that the heap is corrupted or the value's root
    /// representation was misused and points to a reused heap root data slot.
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self>;
}

/// Internal type that enables rooting any heap-allocated type mentioned here.
///
/// This is used by Global and Local references. Adding a variant here requires
/// also implementing `TryFrom<InnerHeapRef> for T` and
/// `TryFrom<T> for InnerHeapRef` to handle conversions to-and-from heap
/// reference format, and finally implementing `trait Rootable` to define the
/// root representation of the type.
///
/// For a type that always refers to the heap, the root representation should
/// simply be `InnerHeapRef`. Types that have stack-value representations can
/// define their own root representation enum that switches between stack
/// values and the `InnerHeapRef` representation.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum HeapRootData {
    // First the Value variants: This list should match 1-to-1 the list in
    // value.rs, but with the
    String(HeapString) = STRING_DISCRIMINANT,
    Symbol(Symbol) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    Object(OrdinaryObject) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject),
    Arguments(OrdinaryObject) = ARGUMENTS_DISCRIMINANT,
    Array(Array) = ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    DataView(DataView) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "date")]
    Date(Date) = DATE_DISCRIMINANT,
    Error(Error) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistry) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(Map) = MAP_DISCRIMINANT,
    Promise(Promise) = PROMISE_DISCRIMINANT,
    Proxy(Proxy) = PROXY_DISCRIMINANT,
    RegExp(RegExp) = REGEXP_DISCRIMINANT,
    Set(Set) = SET_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap) = WEAK_MAP_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef) = WEAK_REF_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet) = WEAK_SET_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int8Array(TypedArrayIndex) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(TypedArrayIndex) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(TypedArrayIndex) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(TypedArrayIndex) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(TypedArrayIndex) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(TypedArrayIndex) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(TypedArrayIndex) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(TypedArrayIndex) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(TypedArrayIndex) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(TypedArrayIndex) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(TypedArrayIndex) = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncIterator = ASYNC_ITERATOR_DISCRIMINANT,
    Iterator = ITERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator) = ARRAY_ITERATOR_DISCRIMINANT,
    SetIterator(SetIterator) = SET_ITERATOR_DISCRIMINANT,
    MapIterator(MapIterator) = MAP_ITERATOR_DISCRIMINANT,
    Generator(Generator) = GENERATOR_DISCRIMINANT,
    Module(Module) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject) = EMBEDDER_OBJECT_DISCRIMINANT,
    // Non-Value types go here. If the 128 variants here are not enough, we can
    // eventually take into use leftover "on-stack" discriminants but that has
    // to be done carefully, accepting that Value's TryFrom<InnerHeapRoot> must
    // not accept those "recirculated" values.
    //
    // The order here shouldn't be important at all, feel free to eg. keep
    // these in alphabetical order.
}

/// Internal type that is used to refer from user-controlled memory (stack or
/// heap) into the Agent heap, indexing into some root list within. The exact
/// root list being referred to is determined by the wrapping type. Locals
/// refer to the locals list, globals refer to the corresponding Realm's
/// globals list.
///
/// ### Usage note
///
/// This type should never appear inside the heap and should never be used
/// as-is. It only make sense within some root list referring type,
/// specifically `Local<T>` and `Global<T>`, and then those types should never
/// appear within the heap directly.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct HeapRootRef(u32);

impl HeapRootRef {
    #[inline]
    pub(crate) fn from_index(index: usize) -> Self {
        let Ok(index) = u32::try_from(index) else {
            handle_heap_ref_overflow()
        };
        Self(index)
    }

    #[inline]
    pub(crate) fn to_index(self) -> usize {
        self.0 as usize
    }
}

#[cold]
#[inline(never)]
fn handle_heap_ref_overflow() -> ! {
    panic!("Heap references overflowed");
}

impl HeapMarkAndSweep for HeapRootData {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        match self {
            HeapRootData::String(heap_string) => heap_string.mark_values(queues),
            HeapRootData::Symbol(symbol) => symbol.mark_values(queues),
            HeapRootData::Number(heap_number) => heap_number.mark_values(queues),
            HeapRootData::BigInt(heap_big_int) => heap_big_int.mark_values(queues),
            HeapRootData::Object(ordinary_object) => ordinary_object.mark_values(queues),
            HeapRootData::BoundFunction(bound_function) => bound_function.mark_values(queues),
            HeapRootData::BuiltinFunction(builtin_function) => builtin_function.mark_values(queues),
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                ecmascript_function.mark_values(queues)
            }
            HeapRootData::BuiltinGeneratorFunction => todo!(),
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => {
                builtin_constructor_function.mark_values(queues)
            }
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                builtin_promise_resolving_function.mark_values(queues)
            }
            HeapRootData::BuiltinPromiseCollectorFunction => todo!(),
            HeapRootData::BuiltinProxyRevokerFunction => todo!(),
            HeapRootData::PrimitiveObject(primitive_object) => primitive_object.mark_values(queues),
            HeapRootData::Arguments(ordinary_object) => ordinary_object.mark_values(queues),
            HeapRootData::Array(array) => array.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(array_buffer) => array_buffer.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(data_view) => data_view.mark_values(queues),
            #[cfg(feature = "date")]
            HeapRootData::Date(date) => date.mark_values(queues),
            HeapRootData::Error(error) => error.mark_values(queues),
            HeapRootData::FinalizationRegistry(finalization_registry) => {
                finalization_registry.mark_values(queues)
            }
            HeapRootData::Map(map) => map.mark_values(queues),
            HeapRootData::Promise(promise) => promise.mark_values(queues),
            HeapRootData::Proxy(proxy) => proxy.mark_values(queues),
            HeapRootData::RegExp(reg_exp) => reg_exp.mark_values(queues),
            HeapRootData::Set(set) => set.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(shared_array_buffer) => {
                shared_array_buffer.mark_values(queues)
            }
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => weak_map.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => weak_ref.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => weak_set.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(base_index) => base_index.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(base_index) => base_index.mark_values(queues),
            HeapRootData::AsyncFromSyncIterator => todo!(),
            HeapRootData::AsyncIterator => todo!(),
            HeapRootData::Iterator => todo!(),
            HeapRootData::ArrayIterator(array_iterator) => array_iterator.mark_values(queues),
            HeapRootData::SetIterator(set_iterator) => set_iterator.mark_values(queues),
            HeapRootData::MapIterator(map_iterator) => map_iterator.mark_values(queues),
            HeapRootData::Generator(generator) => generator.mark_values(queues),
            HeapRootData::Module(module) => module.mark_values(queues),
            HeapRootData::EmbedderObject(embedder_object) => embedder_object.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        match self {
            HeapRootData::String(heap_string) => heap_string.sweep_values(compactions),
            HeapRootData::Symbol(symbol) => symbol.sweep_values(compactions),
            HeapRootData::Number(heap_number) => heap_number.sweep_values(compactions),
            HeapRootData::BigInt(heap_big_int) => heap_big_int.sweep_values(compactions),
            HeapRootData::Object(ordinary_object) => ordinary_object.sweep_values(compactions),
            HeapRootData::BoundFunction(bound_function) => bound_function.sweep_values(compactions),
            HeapRootData::BuiltinFunction(builtin_function) => {
                builtin_function.sweep_values(compactions)
            }
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                ecmascript_function.sweep_values(compactions)
            }
            HeapRootData::BuiltinGeneratorFunction => todo!(),
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => {
                builtin_constructor_function.sweep_values(compactions)
            }
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                builtin_promise_resolving_function.sweep_values(compactions)
            }
            HeapRootData::BuiltinPromiseCollectorFunction => todo!(),
            HeapRootData::BuiltinProxyRevokerFunction => todo!(),
            HeapRootData::PrimitiveObject(primitive_object) => {
                primitive_object.sweep_values(compactions)
            }
            HeapRootData::Arguments(ordinary_object) => ordinary_object.sweep_values(compactions),
            HeapRootData::Array(array) => array.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(array_buffer) => array_buffer.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(data_view) => data_view.sweep_values(compactions),
            #[cfg(feature = "date")]
            HeapRootData::Date(date) => date.sweep_values(compactions),
            HeapRootData::Error(error) => error.sweep_values(compactions),
            HeapRootData::FinalizationRegistry(finalization_registry) => {
                finalization_registry.sweep_values(compactions)
            }
            HeapRootData::Map(map) => map.sweep_values(compactions),
            HeapRootData::Promise(promise) => promise.sweep_values(compactions),
            HeapRootData::Proxy(proxy) => proxy.sweep_values(compactions),
            HeapRootData::RegExp(reg_exp) => reg_exp.sweep_values(compactions),
            HeapRootData::Set(set) => set.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(shared_array_buffer) => {
                shared_array_buffer.sweep_values(compactions)
            }
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => weak_map.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => weak_ref.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => weak_set.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(base_index) => base_index.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(base_index) => base_index.sweep_values(compactions),
            HeapRootData::AsyncFromSyncIterator => todo!(),
            HeapRootData::AsyncIterator => todo!(),
            HeapRootData::Iterator => todo!(),
            HeapRootData::ArrayIterator(array_iterator) => array_iterator.sweep_values(compactions),
            HeapRootData::SetIterator(set_iterator) => set_iterator.sweep_values(compactions),
            HeapRootData::MapIterator(map_iterator) => map_iterator.sweep_values(compactions),
            HeapRootData::Generator(generator) => generator.sweep_values(compactions),
            HeapRootData::Module(module) => module.sweep_values(compactions),
            HeapRootData::EmbedderObject(embedder_object) => {
                embedder_object.sweep_values(compactions)
            }
        }
    }
}
