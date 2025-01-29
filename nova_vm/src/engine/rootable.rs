// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod global;
mod scoped;

use private::RootableSealed;

#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExp;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{data_view::DataView, ArrayBuffer};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
#[cfg(feature = "date")]
use crate::ecmascript::types::DATE_DISCRIMINANT;
#[cfg(feature = "regexp")]
use crate::ecmascript::types::REGEXP_DISCRIMINANT;
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
#[cfg(feature = "set")]
use crate::ecmascript::{
    builtins::{
        keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
    },
    types::{SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT},
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
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIterator,
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            promise_objects::promise_abstract_operations::{
                promise_reaction_records::PromiseReaction,
                promise_resolving_functions::BuiltinPromiseResolvingFunction,
            },
            proxy::Proxy,
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
        },
        types::{
            bigint::HeapBigInt, HeapNumber, HeapString, IntoObject, Object, OrdinaryObject, Symbol,
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
            OBJECT_DISCRIMINANT, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT, STRING_DISCRIMINANT,
            SYMBOL_DISCRIMINANT,
        },
    },
    heap::HeapMarkAndSweep,
};

mod private {
    #[cfg(feature = "date")]
    use crate::ecmascript::builtins::date::Date;
    #[cfg(feature = "regexp")]
    use crate::ecmascript::builtins::regexp::RegExp;
    #[cfg(feature = "shared-array-buffer")]
    use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
    #[cfg(feature = "array-buffer")]
    use crate::ecmascript::builtins::{data_view::DataView, typed_array::TypedArray, ArrayBuffer};
    #[cfg(feature = "set")]
    use crate::ecmascript::builtins::{
        keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
    };
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
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIterator,
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            promise_objects::promise_abstract_operations::{
                promise_reaction_records::PromiseReaction,
                promise_resolving_functions::BuiltinPromiseResolvingFunction,
            },
            proxy::Proxy,
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
        },
        types::{
            BigInt, Function, Number, Numeric, Object, OrdinaryObject, Primitive, PropertyKey,
            String, Symbol, Value,
        },
    };

    /// Marker trait to make Rootable not implementable outside of nova_vm.
    pub trait RootableSealed {}
    impl RootableSealed for Array<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for ArrayBuffer<'_> {}
    impl RootableSealed for ArrayIterator<'_> {}
    impl RootableSealed for BigInt<'_> {}
    impl RootableSealed for BoundFunction<'_> {}
    impl RootableSealed for BuiltinConstructorFunction<'_> {}
    impl RootableSealed for BuiltinFunction<'_> {}
    impl RootableSealed for BuiltinPromiseResolvingFunction<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for DataView<'_> {}
    #[cfg(feature = "date")]
    impl RootableSealed for Date<'_> {}
    impl RootableSealed for ECMAScriptFunction<'_> {}
    impl RootableSealed for EmbedderObject<'_> {}
    impl RootableSealed for Error<'_> {}
    impl RootableSealed for FinalizationRegistry<'_> {}
    impl RootableSealed for Function<'_> {}
    impl RootableSealed for Generator<'_> {}
    impl RootableSealed for Map<'_> {}
    impl RootableSealed for MapIterator<'_> {}
    impl RootableSealed for Module<'_> {}
    impl RootableSealed for Number<'_> {}
    impl RootableSealed for Numeric<'_> {}
    impl RootableSealed for Object<'_> {}
    impl RootableSealed for OrdinaryObject<'_> {}
    impl RootableSealed for Primitive<'_> {}
    impl RootableSealed for PrimitiveObject<'_> {}
    impl RootableSealed for Promise<'_> {}
    impl RootableSealed for PromiseReaction {}
    impl RootableSealed for PropertyKey<'_> {}
    impl RootableSealed for Proxy<'_> {}
    #[cfg(feature = "regexp")]
    impl RootableSealed for RegExp<'_> {}
    #[cfg(feature = "set")]
    impl RootableSealed for Set<'_> {}
    #[cfg(feature = "set")]
    impl RootableSealed for SetIterator<'_> {}
    #[cfg(feature = "shared-array-buffer")]
    impl RootableSealed for SharedArrayBuffer<'_> {}
    impl RootableSealed for String<'_> {}
    impl RootableSealed for Symbol<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for TypedArray<'_> {}
    impl RootableSealed for Value {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakMap<'_> {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakRef<'_> {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakSet<'_> {}
}

pub use global::Global;
pub use scoped::Scoped;

pub trait Rootable: std::fmt::Debug + Copy + RootableSealed {
    type RootRepr: Sized + Clone + std::fmt::Debug;

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

// Blanket impl for Objects
impl<'a, T: std::fmt::Debug + RootableSealed + IntoObject<'a> + TryFrom<HeapRootData>> Rootable
    for T
{
    type RootRepr = HeapRootRef;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(value.into_object().unbind().into())
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        Self::try_from(heap_data).ok()
    }
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
    String(HeapString<'static>) = STRING_DISCRIMINANT,
    Symbol(Symbol<'static>) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber<'static>) = NUMBER_DISCRIMINANT,
    BigInt(HeapBigInt<'static>) = BIGINT_DISCRIMINANT,
    Object(OrdinaryObject<'static>) = OBJECT_DISCRIMINANT,
    BoundFunction(BoundFunction<'static>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'static>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'static>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction<'static>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'static>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject<'static>),
    Arguments(OrdinaryObject<'static>) = ARGUMENTS_DISCRIMINANT,
    Array(Array<'static>) = ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'static>) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'static>) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "date")]
    Date(Date<'static>) = DATE_DISCRIMINANT,
    Error(Error<'static>) = ERROR_DISCRIMINANT,
    FinalizationRegistry(FinalizationRegistry<'static>) = FINALIZATION_REGISTRY_DISCRIMINANT,
    Map(Map<'static>) = MAP_DISCRIMINANT,
    Promise(Promise<'static>) = PROMISE_DISCRIMINANT,
    Proxy(Proxy<'static>) = PROXY_DISCRIMINANT,
    #[cfg(feature = "regexp")]
    RegExp(RegExp<'static>) = REGEXP_DISCRIMINANT,
    #[cfg(feature = "set")]
    Set(Set<'static>) = SET_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'static>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'static>) = WEAK_MAP_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'static>) = WEAK_REF_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'static>) = WEAK_SET_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int8Array(TypedArrayIndex<'static>) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(TypedArrayIndex<'static>) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(TypedArrayIndex<'static>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(TypedArrayIndex<'static>) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(TypedArrayIndex<'static>) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(TypedArrayIndex<'static>) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(TypedArrayIndex<'static>) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(TypedArrayIndex<'static>) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(TypedArrayIndex<'static>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(TypedArrayIndex<'static>) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(TypedArrayIndex<'static>) = FLOAT_64_ARRAY_DISCRIMINANT,
    AsyncFromSyncIterator = ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT,
    AsyncIterator = ASYNC_ITERATOR_DISCRIMINANT,
    Iterator = ITERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator<'static>) = ARRAY_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'static>) = SET_ITERATOR_DISCRIMINANT,
    MapIterator(MapIterator<'static>) = MAP_ITERATOR_DISCRIMINANT,
    Generator(Generator<'static>) = GENERATOR_DISCRIMINANT,
    Module(Module<'static>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject<'static>) = EMBEDDER_OBJECT_DISCRIMINANT,
    // Non-Value types go here. If the 128 variants here are not enough, we can
    // eventually take into use leftover "on-stack" discriminants but that has
    // to be done carefully, accepting that Value's TryFrom<InnerHeapRoot> must
    // not accept those "recirculated" values.
    //
    // The order here shouldn't be important at all, feel free to eg. keep
    // these in alphabetical order.
    PromiseReaction(PromiseReaction),
}

impl From<Object<'static>> for HeapRootData {
    #[inline]
    fn from(value: Object<'static>) -> Self {
        match value {
            Object::Object(ordinary_object) => Self::Object(ordinary_object),
            Object::BoundFunction(bound_function) => Self::BoundFunction(bound_function),
            Object::BuiltinFunction(builtin_function) => Self::BuiltinFunction(builtin_function),
            Object::ECMAScriptFunction(ecmascript_function) => {
                Self::ECMAScriptFunction(ecmascript_function)
            }
            Object::BuiltinGeneratorFunction => Self::BuiltinGeneratorFunction,
            Object::BuiltinConstructorFunction(builtin_constructor_function) => {
                Self::BuiltinConstructorFunction(builtin_constructor_function)
            }
            Object::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Self::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function)
            }
            Object::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            Object::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
            Object::PrimitiveObject(primitive_object) => Self::PrimitiveObject(primitive_object),
            Object::Arguments(ordinary_object) => Self::Arguments(ordinary_object),
            Object::Array(array) => Self::Array(array),
            Object::ArrayBuffer(array_buffer) => Self::ArrayBuffer(array_buffer),
            Object::DataView(data_view) => Self::DataView(data_view),
            Object::Date(date) => Self::Date(date),
            Object::Error(error) => Self::Error(error),
            Object::FinalizationRegistry(finalization_registry) => {
                Self::FinalizationRegistry(finalization_registry)
            }
            Object::Map(map) => Self::Map(map),
            Object::Promise(promise) => Self::Promise(promise),
            Object::Proxy(proxy) => Self::Proxy(proxy),
            Object::RegExp(reg_exp) => Self::RegExp(reg_exp),
            #[cfg(feature = "set")]
            Object::Set(set) => Self::Set(set),
            Object::SharedArrayBuffer(shared_array_buffer) => {
                Self::SharedArrayBuffer(shared_array_buffer)
            }
            Object::WeakMap(weak_map) => Self::WeakMap(weak_map),
            Object::WeakRef(weak_ref) => Self::WeakRef(weak_ref),
            Object::WeakSet(weak_set) => Self::WeakSet(weak_set),
            Object::Int8Array(base_index) => Self::Int8Array(base_index),
            Object::Uint8Array(base_index) => Self::Uint8Array(base_index),
            Object::Uint8ClampedArray(base_index) => Self::Uint8ClampedArray(base_index),
            Object::Int16Array(base_index) => Self::Int16Array(base_index),
            Object::Uint16Array(base_index) => Self::Uint16Array(base_index),
            Object::Int32Array(base_index) => Self::Int32Array(base_index),
            Object::Uint32Array(base_index) => Self::Uint32Array(base_index),
            Object::BigInt64Array(base_index) => Self::BigInt64Array(base_index),
            Object::BigUint64Array(base_index) => Self::BigUint64Array(base_index),
            Object::Float32Array(base_index) => Self::Float32Array(base_index),
            Object::Float64Array(base_index) => Self::Float64Array(base_index),
            Object::AsyncFromSyncIterator => Self::AsyncFromSyncIterator,
            Object::AsyncIterator => Self::AsyncIterator,
            Object::Iterator => Self::Iterator,
            Object::ArrayIterator(array_iterator) => Self::ArrayIterator(array_iterator),
            #[cfg(feature = "set")]
            Object::SetIterator(set_iterator) => Self::SetIterator(set_iterator),
            Object::MapIterator(map_iterator) => Self::MapIterator(map_iterator),
            Object::Generator(generator) => Self::Generator(generator),
            Object::Module(module) => Self::Module(module),
            Object::EmbedderObject(embedder_object) => Self::EmbedderObject(embedder_object),
        }
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            #[cfg(feature = "regexp")]
            HeapRootData::RegExp(reg_exp) => reg_exp.mark_values(queues),
            #[cfg(feature = "set")]
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
            #[cfg(feature = "set")]
            HeapRootData::SetIterator(set_iterator) => set_iterator.mark_values(queues),
            HeapRootData::MapIterator(map_iterator) => map_iterator.mark_values(queues),
            HeapRootData::Generator(generator) => generator.mark_values(queues),
            HeapRootData::Module(module) => module.mark_values(queues),
            HeapRootData::EmbedderObject(embedder_object) => embedder_object.mark_values(queues),
            HeapRootData::PromiseReaction(promise_reaction) => promise_reaction.mark_values(queues),
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
            #[cfg(feature = "regexp")]
            HeapRootData::RegExp(reg_exp) => reg_exp.sweep_values(compactions),
            #[cfg(feature = "set")]
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
            #[cfg(feature = "set")]
            HeapRootData::SetIterator(set_iterator) => set_iterator.sweep_values(compactions),
            HeapRootData::MapIterator(map_iterator) => map_iterator.sweep_values(compactions),
            HeapRootData::Generator(generator) => generator.sweep_values(compactions),
            HeapRootData::Module(module) => module.sweep_values(compactions),
            HeapRootData::EmbedderObject(embedder_object) => {
                embedder_object.sweep_values(compactions)
            }
            HeapRootData::PromiseReaction(promise_reaction) => {
                promise_reaction.sweep_values(compactions)
            }
        }
    }
}
