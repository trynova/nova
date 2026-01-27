// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod global;
mod scoped;

pub(crate) use private::{HeapRootCollectionData, RootableCollectionSealed, RootableSealed};

#[cfg(feature = "date")]
use crate::ecmascript::DATE_DISCRIMINANT;
#[cfg(feature = "date")]
use crate::ecmascript::Date;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::{
    ARRAY_BUFFER_DISCRIMINANT, ArrayBuffer, BIGINT_64_ARRAY_DISCRIMINANT,
    BIGUINT_64_ARRAY_DISCRIMINANT, BigInt64Array, BigUint64Array, DATA_VIEW_DISCRIMINANT, DataView,
    FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, Float32Array, Float64Array,
    INT_8_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT, Int8Array,
    Int16Array, Int32Array, UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT, Uint8Array, Uint8ClampedArray,
    Uint16Array, Uint32Array,
};
#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::{FLOAT_16_ARRAY_DISCRIMINANT, Float16Array};
#[cfg(feature = "regexp")]
use crate::ecmascript::{
    REGEXP_DISCRIMINANT, REGEXP_STRING_ITERATOR_DISCRIMINANT, RegExp, RegExpStringIterator,
};
#[cfg(feature = "set")]
use crate::ecmascript::{SET_DISCRIMINANT, SET_ITERATOR_DISCRIMINANT, Set, SetIterator};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::{
    SHARED_ARRAY_BUFFER_DISCRIMINANT, SHARED_BIGINT_64_ARRAY_DISCRIMINANT,
    SHARED_BIGUINT_64_ARRAY_DISCRIMINANT, SHARED_DATA_VIEW_DISCRIMINANT,
    SHARED_FLOAT_32_ARRAY_DISCRIMINANT, SHARED_FLOAT_64_ARRAY_DISCRIMINANT,
    SHARED_INT_8_ARRAY_DISCRIMINANT, SHARED_INT_16_ARRAY_DISCRIMINANT,
    SHARED_INT_32_ARRAY_DISCRIMINANT, SHARED_UINT_8_ARRAY_DISCRIMINANT,
    SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT, SHARED_UINT_16_ARRAY_DISCRIMINANT,
    SHARED_UINT_32_ARRAY_DISCRIMINANT, SharedArrayBuffer, SharedBigInt64Array,
    SharedBigUint64Array, SharedDataView, SharedFloat32Array, SharedFloat64Array, SharedInt8Array,
    SharedInt16Array, SharedInt32Array, SharedUint8Array, SharedUint8ClampedArray,
    SharedUint16Array, SharedUint32Array,
};
#[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
use crate::ecmascript::{SHARED_FLOAT_16_ARRAY_DISCRIMINANT, SharedFloat16Array};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WEAK_MAP_DISCRIMINANT, WEAK_REF_DISCRIMINANT, WEAK_SET_DISCRIMINANT};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WeakMap, WeakRef, WeakSet};
use crate::{
    ecmascript::{
        ARGUMENTS_DISCRIMINANT, ARRAY_DISCRIMINANT, ARRAY_ITERATOR_DISCRIMINANT,
        ASYNC_GENERATOR_DISCRIMINANT, Array, ArrayIterator, AsyncGenerator, AwaitReaction,
        BIGINT_DISCRIMINANT, BOUND_FUNCTION_DISCRIMINANT,
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT, BUILTIN_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        BoundFunction, BuiltinConstructorFunction, BuiltinFunction, BuiltinPromiseFinallyFunction,
        BuiltinPromiseResolvingFunction, DeclarativeEnvironment, ECMASCRIPT_FUNCTION_DISCRIMINANT,
        ECMAScriptFunction, EMBEDDER_OBJECT_DISCRIMINANT, ERROR_DISCRIMINANT, EmbedderObject,
        Error, FINALIZATION_REGISTRY_DISCRIMINANT, FinalizationRegistry, FunctionEnvironment,
        GENERATOR_DISCRIMINANT, Generator, GlobalEnvironment, HeapBigInt, HeapNumber, HeapString,
        KeyedGroup, MAP_DISCRIMINANT, MAP_ITERATOR_DISCRIMINANT, MODULE_DISCRIMINANT, Map,
        MapIterator, Module, ModuleEnvironment, NUMBER_DISCRIMINANT, OBJECT_DISCRIMINANT,
        ObjectEnvironment, OrdinaryObject, PROMISE_DISCRIMINANT, PROXY_DISCRIMINANT,
        PrimitiveObject, PrivateEnvironment, Promise, PromiseGroup, PromiseReaction, PropertyKey,
        PropertyKeySet, PropertyLookupCache, Proxy, Realm, STRING_DISCRIMINANT,
        STRING_ITERATOR_DISCRIMINANT, SYMBOL_DISCRIMINANT, Script, SourceCode, SourceTextModule,
        StringIterator, Symbol, Value,
    },
    heap::HeapMarkAndSweep,
};

pub(crate) mod private {
    use std::ptr::NonNull;

    #[cfg(feature = "date")]
    use crate::ecmascript::Date;
    #[cfg(feature = "array-buffer")]
    use crate::ecmascript::{
        AnyArrayBuffer, AnyDataView, AnyTypedArray, ArrayBuffer, DataView, GenericTypedArray,
        TypedArray, Viewable,
    };
    #[cfg(feature = "shared-array-buffer")]
    use crate::ecmascript::{
        GenericSharedTypedArray, SharedArrayBuffer, SharedDataView, SharedTypedArray,
    };
    #[cfg(feature = "regexp")]
    use crate::ecmascript::{RegExp, RegExpStringIterator};
    #[cfg(feature = "set")]
    use crate::ecmascript::{Set, SetIterator};
    #[cfg(feature = "weak-refs")]
    use crate::ecmascript::{WeakKey, WeakMap, WeakRef, WeakSet};
    use crate::{
        ecmascript::{
            AbstractModule, ArgumentsList, Array, ArrayIterator, AsyncGenerator, AwaitReaction,
            BigInt, BoundFunction, BuiltinConstructorFunction, BuiltinFunction,
            BuiltinPromiseFinallyFunction, BuiltinPromiseResolvingFunction, CyclicModule,
            DeclarativeEnvironment, ECMAScriptFunction, EmbedderObject, Environment, Error,
            FinalizationRegistry, Function, FunctionEnvironment, Generator, GlobalEnvironment,
            InnerAbstractModule, InnerCyclicModule, InnerReferrer, JsError, KeyedGroup, Map,
            MapIterator, Module, ModuleEnvironment, Number, Numeric, Object, ObjectEnvironment,
            OrdinaryObject, Primitive, PrimitiveObject, PrivateEnvironment, Promise, PromiseGroup,
            PromiseReaction, PropertyKey, PropertyKeySet, PropertyLookupCache, Proxy, Realm,
            Referrer, Script, SourceCode, SourceTextModule, String, Symbol, Value,
        },
        engine::{Executable, context::Bindable},
        heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
    };

    /// Marker trait to make Rootable not implementable outside of nova_vm.
    pub trait RootableSealed {}
    impl RootableSealed for Array<'_> {}
    impl RootableSealed for ArrayIterator<'_> {}
    impl RootableSealed for AsyncGenerator<'_> {}
    impl RootableSealed for AwaitReaction<'_> {}
    impl RootableSealed for BigInt<'_> {}
    impl RootableSealed for BoundFunction<'_> {}
    impl RootableSealed for BuiltinConstructorFunction<'_> {}
    impl RootableSealed for BuiltinFunction<'_> {}
    impl RootableSealed for BuiltinPromiseResolvingFunction<'_> {}
    impl RootableSealed for BuiltinPromiseFinallyFunction<'_> {}
    #[cfg(feature = "date")]
    impl RootableSealed for Date<'_> {}
    impl RootableSealed for ECMAScriptFunction<'_> {}
    impl RootableSealed for EmbedderObject<'_> {}
    impl RootableSealed for Error<'_> {}
    impl RootableSealed for Executable<'_> {}
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
    impl RootableSealed for PromiseReaction<'_> {}
    impl RootableSealed for PromiseGroup<'_> {}
    impl RootableSealed for PropertyKey<'_> {}
    impl RootableSealed for Proxy<'_> {}
    impl RootableSealed for Realm<'_> {}
    #[cfg(feature = "regexp")]
    impl RootableSealed for RegExp<'_> {}
    #[cfg(feature = "regexp")]
    impl RootableSealed for RegExpStringIterator<'_> {}
    impl RootableSealed for Script<'_> {}
    #[cfg(feature = "set")]
    impl RootableSealed for Set<'_> {}
    #[cfg(feature = "set")]
    impl RootableSealed for SetIterator<'_> {}
    impl RootableSealed for SourceCode<'_> {}
    impl RootableSealed for SourceTextModule<'_> {}
    impl RootableSealed for String<'_> {}
    impl RootableSealed for Symbol<'_> {}

    #[cfg(feature = "array-buffer")]
    impl RootableSealed for ArrayBuffer<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for AnyArrayBuffer<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for AnyDataView<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for DataView<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for AnyTypedArray<'_> {}
    #[cfg(feature = "array-buffer")]
    impl RootableSealed for TypedArray<'_> {}
    #[cfg(feature = "array-buffer")]
    impl<T: Viewable> RootableSealed for GenericTypedArray<'_, T> {}

    #[cfg(feature = "shared-array-buffer")]
    impl RootableSealed for SharedArrayBuffer<'_> {}
    #[cfg(feature = "shared-array-buffer")]
    impl RootableSealed for SharedDataView<'_> {}
    #[cfg(feature = "shared-array-buffer")]
    impl RootableSealed for SharedTypedArray<'_> {}
    #[cfg(feature = "shared-array-buffer")]
    impl<T: Viewable> RootableSealed for GenericSharedTypedArray<'_, T> {}

    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakKey<'_> {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakMap<'_> {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakRef<'_> {}
    #[cfg(feature = "weak-refs")]
    impl RootableSealed for WeakSet<'_> {}

    impl RootableSealed for Value<'_> {}

    // Environments are also rootable
    impl RootableSealed for DeclarativeEnvironment<'_> {}
    impl RootableSealed for FunctionEnvironment<'_> {}
    impl RootableSealed for GlobalEnvironment<'_> {}
    impl RootableSealed for ModuleEnvironment<'_> {}
    impl RootableSealed for ObjectEnvironment<'_> {}
    impl RootableSealed for PrivateEnvironment<'_> {}
    impl RootableSealed for Environment<'_> {}
    // Errors are rootable as they're just a wrapper around Value.
    impl RootableSealed for JsError<'_> {}
    // Module Record references are also rootable
    impl RootableSealed for AbstractModule<'_> {}
    impl RootableSealed for InnerAbstractModule<'_> {}
    impl RootableSealed for CyclicModule<'_> {}
    impl RootableSealed for InnerCyclicModule<'_> {}
    impl RootableSealed for Referrer<'_> {}
    impl RootableSealed for InnerReferrer<'_> {}
    // Cache references.
    impl RootableSealed for PropertyLookupCache<'_> {}

    /// Marker trait to make RootableSealed not implementable outside of nova_vm.
    pub trait RootableCollectionSealed {
        /// Convert a rootable collection value to a heap data representation.
        fn to_heap_data(self) -> HeapRootCollectionData;

        /// Convert the rooted collection's heap data value to the type itself.
        ///
        /// ## Panics
        ///
        /// If the heap data does not match the type, the method should panic.
        fn from_heap_data(value: HeapRootCollectionData) -> Self;
    }

    #[derive(Debug)]
    #[repr(u8)]
    pub enum HeapRootCollectionData {
        /// Empty heap root collection data slot: The data was taken from heap.
        Empty,
        /// Not like the others: Arguments list cannot be given to the heap as
        /// owned, they can only be borrowed. Thus, they have no scoping API but
        /// instead have a `with_scoped` API that takes a callback, stores the list
        /// on the heap temporarily, performs the callback, removes the list from
        /// the heap and then returns control to the caller.
        ///
        /// As the arguments list is taken as an exclusive reference to the
        /// method, we're guaranteed that the list stored here
        ArgumentsList(NonNull<[Value<'static>]>),
        ValueVec(Vec<Value<'static>>),
        PropertyKeyVec(Vec<PropertyKey<'static>>),
        PropertyKeySet(PropertyKeySet<'static>),
        KeyedGroup(Box<KeyedGroup<'static>>),
    }

    impl HeapMarkAndSweep for HeapRootCollectionData {
        fn mark_values(&self, queues: &mut WorkQueues) {
            match self {
                Self::Empty => {}
                Self::ArgumentsList(slice) => {
                    // SAFETY: The slice is pushed to heap roots based on an
                    // exclusive reference, and gets taken out of the list when
                    // the pushing call stack is exited. This is not panic-safe
                    // though, so we may be indexing into deallocated memory if
                    // a panic has been caught.
                    unsafe { slice.as_ref().mark_values(queues) };
                }
                Self::ValueVec(values) => values.as_slice().mark_values(queues),
                Self::PropertyKeyVec(items) => items.mark_values(queues),
                Self::PropertyKeySet(items) => items.mark_values(queues),
                Self::KeyedGroup(group) => group.mark_values(queues),
            }
        }

        fn sweep_values(&mut self, compactions: &CompactionLists) {
            match self {
                Self::Empty => {}
                Self::ArgumentsList(slice) => {
                    // SAFETY: The slice is pushed to heap roots based on an
                    // exclusive reference, and gets taken out of the list when
                    // the pushing call stack is exited. This is not panic-safe
                    // though, so we may be indexing into deallocated memory if
                    // a panic has been caught.
                    unsafe { slice.as_mut().sweep_values(compactions) };
                }
                Self::ValueVec(values) => values.as_mut_slice().sweep_values(compactions),
                Self::PropertyKeyVec(items) => items.sweep_values(compactions),
                Self::PropertyKeySet(items) => items.sweep_values(compactions),
                Self::KeyedGroup(group) => group.sweep_values(compactions),
            }
        }
    }

    impl RootableCollectionSealed for ArgumentsList<'_, '_> {
        fn to_heap_data(mut self) -> HeapRootCollectionData {
            HeapRootCollectionData::ArgumentsList(self.as_mut_slice().into())
        }

        fn from_heap_data(_: HeapRootCollectionData) -> Self {
            unreachable!("ScopedCollection should never try to take ownership of ArgumentsList");
        }
    }
    impl RootableCollectionSealed for Vec<Value<'static>> {
        fn to_heap_data(self) -> HeapRootCollectionData {
            HeapRootCollectionData::ValueVec(self.unbind())
        }

        fn from_heap_data(value: HeapRootCollectionData) -> Self {
            let HeapRootCollectionData::ValueVec(value) = value else {
                unreachable!()
            };
            value
        }
    }
    impl RootableCollectionSealed for Vec<PropertyKey<'static>> {
        fn to_heap_data(self) -> HeapRootCollectionData {
            HeapRootCollectionData::PropertyKeyVec(self.unbind())
        }

        fn from_heap_data(value: HeapRootCollectionData) -> Self {
            let HeapRootCollectionData::PropertyKeyVec(value) = value else {
                unreachable!()
            };
            value
        }
    }
    impl RootableCollectionSealed for PropertyKeySet<'static> {
        fn to_heap_data(self) -> HeapRootCollectionData {
            HeapRootCollectionData::PropertyKeySet(self.unbind())
        }

        fn from_heap_data(value: HeapRootCollectionData) -> Self {
            let HeapRootCollectionData::PropertyKeySet(value) = value else {
                unreachable!()
            };
            value
        }
    }
    impl RootableCollectionSealed for Box<KeyedGroup<'static>> {
        fn to_heap_data(self) -> HeapRootCollectionData {
            HeapRootCollectionData::KeyedGroup(self.unbind())
        }

        fn from_heap_data(value: HeapRootCollectionData) -> Self {
            let HeapRootCollectionData::KeyedGroup(value) = value else {
                unreachable!()
            };
            value
        }
    }
}

pub use global::Global;
pub use scoped::{Scopable, ScopableCollection, Scoped, ScopedCollection};

use super::Executable;

pub trait Rootable: Copy + RootableSealed {
    type RootRepr: Sized + Clone;

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

// Blanket impl for heap references.
impl<T: Copy + RootableSealed + Into<HeapRootData> + TryFrom<HeapRootData>> Rootable for T {
    type RootRepr = HeapRootRef;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(value.into())
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
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum HeapRootData {
    /// Empty heap root data slot. This can be used to reserve a slot, or to
    /// remove a scoped value from the heap.
    Empty,
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
    BuiltinConstructorFunction(BuiltinConstructorFunction<'static>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'static>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseFinallyFunction(BuiltinPromiseFinallyFunction<'static>) =
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    PrimitiveObject(PrimitiveObject<'static>),
    Arguments(OrdinaryObject<'static>) = ARGUMENTS_DISCRIMINANT,
    Array(Array<'static>) = ARRAY_DISCRIMINANT,
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
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'static>) = WEAK_MAP_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'static>) = WEAK_REF_DISCRIMINANT,
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'static>) = WEAK_SET_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'static>) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'static>) = DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int8Array(Int8Array<'static>) = INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8Array(Uint8Array<'static>) = UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(Uint8ClampedArray<'static>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int16Array(Int16Array<'static>) = INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint16Array(Uint16Array<'static>) = UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Int32Array(Int32Array<'static>) = INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Uint32Array(Uint32Array<'static>) = UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigInt64Array(BigInt64Array<'static>) = BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    BigUint64Array(BigUint64Array<'static>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(Float16Array<'static>) = FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float32Array(Float32Array<'static>) = FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "array-buffer")]
    Float64Array(Float64Array<'static>) = FLOAT_64_ARRAY_DISCRIMINANT,

    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'static>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedDataView(SharedDataView<'static>) = SHARED_DATA_VIEW_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt8Array(SharedInt8Array<'static>) = SHARED_INT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8Array(SharedUint8Array<'static>) = SHARED_UINT_8_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint8ClampedArray(SharedUint8ClampedArray<'static>) =
        SHARED_UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt16Array(SharedInt16Array<'static>) = SHARED_INT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint16Array(SharedUint16Array<'static>) = SHARED_UINT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedInt32Array(SharedInt32Array<'static>) = SHARED_INT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedUint32Array(SharedUint32Array<'static>) = SHARED_UINT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedBigInt64Array(SharedBigInt64Array<'static>) = SHARED_BIGINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedBigUint64Array(SharedBigUint64Array<'static>) = SHARED_BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
    SharedFloat16Array(SharedFloat16Array<'static>) = SHARED_FLOAT_16_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat32Array(SharedFloat32Array<'static>) = SHARED_FLOAT_32_ARRAY_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedFloat64Array(SharedFloat64Array<'static>) = SHARED_FLOAT_64_ARRAY_DISCRIMINANT,

    AsyncGenerator(AsyncGenerator<'static>) = ASYNC_GENERATOR_DISCRIMINANT,
    ArrayIterator(ArrayIterator<'static>) = ARRAY_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'static>) = SET_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "set")]
    MapIterator(MapIterator<'static>) = MAP_ITERATOR_DISCRIMINANT,
    Generator(Generator<'static>) = GENERATOR_DISCRIMINANT,
    StringIterator(StringIterator<'static>) = STRING_ITERATOR_DISCRIMINANT,
    #[cfg(feature = "regexp")]
    RegExpStringIterator(RegExpStringIterator<'static>) = REGEXP_STRING_ITERATOR_DISCRIMINANT,
    Module(Module<'static>) = MODULE_DISCRIMINANT,
    EmbedderObject(EmbedderObject<'static>) = EMBEDDER_OBJECT_DISCRIMINANT,
    // Non-Value types go here. If the 128 variants here are not enough, we can
    // eventually take into use leftover "on-stack" discriminants but that has
    // to be done carefully, accepting that Value's TryFrom<InnerHeapRoot> must
    // not accept those "recirculated" values.
    //
    // The order here shouldn't be important at all, feel free to eg. keep
    // these in alphabetical order.
    Executable(Executable<'static>),
    AwaitReaction(AwaitReaction<'static>),
    PromiseReaction(PromiseReaction<'static>),
    PromiseGroup(PromiseGroup<'static>),
    Realm(Realm<'static>),
    Script(Script<'static>),
    SourceTextModule(SourceTextModule<'static>),
    SourceCode(SourceCode<'static>),
    DeclarativeEnvironment(DeclarativeEnvironment<'static>),
    FunctionEnvironment(FunctionEnvironment<'static>),
    GlobalEnvironment(GlobalEnvironment<'static>),
    ModuleEnvironment(ModuleEnvironment<'static>),
    ObjectEnvironment(ObjectEnvironment<'static>),
    PrivateEnvironment(PrivateEnvironment<'static>),
    PropertyLookupCache(PropertyLookupCache<'static>),
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
            Self::Empty => {}
            Self::String(heap_string) => heap_string.mark_values(queues),
            Self::Symbol(symbol) => symbol.mark_values(queues),
            Self::Number(heap_number) => heap_number.mark_values(queues),
            Self::BigInt(heap_big_int) => heap_big_int.mark_values(queues),
            Self::Object(ordinary_object) => ordinary_object.mark_values(queues),
            Self::BoundFunction(bound_function) => bound_function.mark_values(queues),
            Self::BuiltinFunction(builtin_function) => builtin_function.mark_values(queues),
            Self::ECMAScriptFunction(ecmascript_function) => {
                ecmascript_function.mark_values(queues)
            }

            Self::BuiltinConstructorFunction(builtin_constructor_function) => {
                builtin_constructor_function.mark_values(queues)
            }
            Self::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                builtin_promise_resolving_function.mark_values(queues)
            }
            Self::BuiltinPromiseFinallyFunction(builtin_promise_finally_function) => {
                builtin_promise_finally_function.mark_values(queues)
            }
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::PrimitiveObject(primitive_object) => primitive_object.mark_values(queues),
            Self::Arguments(ordinary_object) => ordinary_object.mark_values(queues),
            Self::Array(array) => array.mark_values(queues),
            #[cfg(feature = "date")]
            Self::Date(date) => date.mark_values(queues),
            Self::Error(error) => error.mark_values(queues),
            Self::FinalizationRegistry(finalization_registry) => {
                finalization_registry.mark_values(queues)
            }
            Self::Map(map) => map.mark_values(queues),
            Self::Promise(promise) => promise.mark_values(queues),
            Self::Proxy(proxy) => proxy.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExp(reg_exp) => reg_exp.mark_values(queues),
            #[cfg(feature = "set")]
            Self::Set(set) => set.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(weak_map) => weak_map.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(weak_ref) => weak_ref.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(weak_set) => weak_set.mark_values(queues),

            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(array_buffer) => array_buffer.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data_view) => data_view.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.mark_values(queues),

            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sab) => sab.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => sta.mark_values(queues),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => sta.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => sta.mark_values(queues),

            Self::AsyncGenerator(r#gen) => r#gen.mark_values(queues),
            Self::ArrayIterator(array_iterator) => array_iterator.mark_values(queues),
            #[cfg(feature = "set")]
            Self::SetIterator(set_iterator) => set_iterator.mark_values(queues),
            Self::MapIterator(map_iterator) => map_iterator.mark_values(queues),
            Self::StringIterator(generator) => generator.mark_values(queues),
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(generator) => generator.mark_values(queues),
            Self::Generator(generator) => generator.mark_values(queues),
            Self::Module(module) => module.mark_values(queues),
            Self::EmbedderObject(embedder_object) => embedder_object.mark_values(queues),
            Self::Executable(exe) => exe.mark_values(queues),
            Self::AwaitReaction(await_reaction) => await_reaction.mark_values(queues),
            Self::PromiseReaction(promise_reaction) => promise_reaction.mark_values(queues),
            Self::PromiseGroup(promise_group) => promise_group.mark_values(queues),
            Self::Realm(realm) => realm.mark_values(queues),
            Self::Script(script) => script.mark_values(queues),
            Self::SourceCode(source_code) => source_code.mark_values(queues),
            Self::SourceTextModule(m) => m.mark_values(queues),
            Self::DeclarativeEnvironment(declarative_environment_index) => {
                declarative_environment_index.mark_values(queues)
            }
            Self::FunctionEnvironment(function_environment_index) => {
                function_environment_index.mark_values(queues)
            }
            Self::GlobalEnvironment(global_environment_index) => {
                global_environment_index.mark_values(queues)
            }
            Self::ModuleEnvironment(module_environment_index) => {
                module_environment_index.mark_values(queues)
            }
            Self::ObjectEnvironment(object_environment_index) => {
                object_environment_index.mark_values(queues)
            }
            Self::PrivateEnvironment(private_environment_index) => {
                private_environment_index.mark_values(queues)
            }
            Self::PropertyLookupCache(property_lookup_cache) => {
                property_lookup_cache.mark_values(queues);
            }
        }
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        match self {
            Self::Empty => {}
            Self::String(heap_string) => heap_string.sweep_values(compactions),
            Self::Symbol(symbol) => symbol.sweep_values(compactions),
            Self::Number(heap_number) => heap_number.sweep_values(compactions),
            Self::BigInt(heap_big_int) => heap_big_int.sweep_values(compactions),
            Self::Object(ordinary_object) => ordinary_object.sweep_values(compactions),
            Self::BoundFunction(bound_function) => bound_function.sweep_values(compactions),
            Self::BuiltinFunction(builtin_function) => builtin_function.sweep_values(compactions),
            Self::ECMAScriptFunction(ecmascript_function) => {
                ecmascript_function.sweep_values(compactions)
            }

            Self::BuiltinConstructorFunction(builtin_constructor_function) => {
                builtin_constructor_function.sweep_values(compactions)
            }
            Self::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                builtin_promise_resolving_function.sweep_values(compactions)
            }
            Self::BuiltinPromiseFinallyFunction(builtin_promise_finally_function) => {
                builtin_promise_finally_function.sweep_values(compactions);
            }
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
            Self::PrimitiveObject(primitive_object) => primitive_object.sweep_values(compactions),
            Self::Arguments(ordinary_object) => ordinary_object.sweep_values(compactions),
            Self::Array(array) => array.sweep_values(compactions),
            #[cfg(feature = "date")]
            Self::Date(date) => date.sweep_values(compactions),
            Self::Error(error) => error.sweep_values(compactions),
            Self::FinalizationRegistry(finalization_registry) => {
                finalization_registry.sweep_values(compactions)
            }
            Self::Map(map) => map.sweep_values(compactions),
            Self::Promise(promise) => promise.sweep_values(compactions),
            Self::Proxy(proxy) => proxy.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExp(reg_exp) => reg_exp.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::Set(set) => set.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(weak_map) => weak_map.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(weak_ref) => weak_ref.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(weak_set) => weak_set.sweep_values(compactions),

            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(array_buffer) => array_buffer.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::DataView(data_view) => data_view.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(ta) => ta.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(ta) => ta.sweep_values(compactions),

            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(shared_array_buffer) => {
                shared_array_buffer.sweep_values(compactions)
            }
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedDataView(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt8Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint8ClampedArray(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedInt32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedUint32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigInt64Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedBigUint64Array(sta) => sta.sweep_values(compactions),
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Self::SharedFloat16Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat32Array(sta) => sta.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedFloat64Array(sta) => sta.sweep_values(compactions),

            Self::AsyncGenerator(r#gen) => r#gen.sweep_values(compactions),
            Self::ArrayIterator(array_iterator) => array_iterator.sweep_values(compactions),
            #[cfg(feature = "set")]
            Self::SetIterator(set_iterator) => set_iterator.sweep_values(compactions),
            Self::MapIterator(map_iterator) => map_iterator.sweep_values(compactions),
            Self::StringIterator(generator) => generator.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Self::RegExpStringIterator(generator) => generator.sweep_values(compactions),
            Self::Generator(generator) => generator.sweep_values(compactions),
            Self::Module(module) => module.sweep_values(compactions),
            Self::EmbedderObject(embedder_object) => embedder_object.sweep_values(compactions),
            Self::Executable(exe) => exe.sweep_values(compactions),
            Self::AwaitReaction(await_reaction) => await_reaction.sweep_values(compactions),
            Self::PromiseReaction(promise_reaction) => promise_reaction.sweep_values(compactions),
            Self::PromiseGroup(promise_group) => promise_group.sweep_values(compactions),
            Self::Realm(realm) => realm.sweep_values(compactions),
            Self::Script(script) => script.sweep_values(compactions),
            Self::SourceCode(source_code) => source_code.sweep_values(compactions),
            Self::SourceTextModule(m) => m.sweep_values(compactions),
            Self::DeclarativeEnvironment(declarative_environment_index) => {
                declarative_environment_index.sweep_values(compactions)
            }
            Self::FunctionEnvironment(function_environment_index) => {
                function_environment_index.sweep_values(compactions)
            }
            Self::GlobalEnvironment(global_environment_index) => {
                global_environment_index.sweep_values(compactions)
            }
            Self::ModuleEnvironment(module_environment_index) => {
                module_environment_index.sweep_values(compactions)
            }
            Self::ObjectEnvironment(object_environment_index) => {
                object_environment_index.sweep_values(compactions)
            }
            Self::PrivateEnvironment(private_environment_index) => {
                private_environment_index.sweep_values(compactions)
            }
            Self::PropertyLookupCache(property_lookup_cache) => {
                property_lookup_cache.sweep_values(compactions)
            }
        }
    }
}

pub trait RootableCollection: core::fmt::Debug + RootableCollectionSealed {}

impl RootableCollection for Vec<Value<'static>> {}
impl RootableCollection for Vec<PropertyKey<'static>> {}
impl RootableCollection for PropertyKeySet<'static> {}
impl RootableCollection for Box<KeyedGroup<'static>> {}
