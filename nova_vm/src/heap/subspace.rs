//! Regions of semi-isolated heap-managed memory.
//!
//! ## Notes
//!
//! Subspaces are designed around two primary types: a _resident_ that allocated
//! on the heap (and in a subspace) and a pointer-like _key_ newtype that looks it
//! up.  both types are generic over the lifetime of the heap. They may be
//! bound/unbound to a garbage collection scope. This means there are actually 4 types:
//! `Key<'a>`, `Key<'static>`, `Resident<'a>`, `Resident<'static>`.
//! 
//! Since trait impls may not pass lifetimes to the types they're being implemented on,
//! we're forced to use associated types. [`SubspaceResident`], which should be
//! implemented on `Resident<'static>` holds those types via
//! `SubspaceResident::Key<'a>` and `SubspaceResident::Bound<'a>`.
//! 
//! The `Key<'a>` is effectively a pointer to a `Resident<'a>`, but uses a [`BaseIndex`]
//! to make it smaller. Note that this API does not use [`BaseIndex`] directly, preferring
//! a newtype wrapper around it to prevent indexing into other subspaces.
//! 
//! > note: I originally designed this with the goal of having `&'a Resident<'static>`
//! > as a valid implementation of `Key<'a>`. The `From<BaseIndex<'a, Resident<'static>>>`
//! > type constraint currently prevents this. This would allow things like storing
//! > strings in a subspace, and using a straight-up pointer as a key.
//! 
//! The `Bound<'a>` type must be
//! - the same exact type as `Resident<'a>`, but accepting a lifetime parameter.
//! - the type bound/unbound to a garbage collection scope via [`bind`] /[`unbind`]
//! Note that this is actually the same restriction since [`Bindable`] requires those
//! methods are semantically equivalent to a lifetime transmute.
//! 
//! [`bind`]: crate::engine::context::Bindable::bind
//! [`unbind`]: crate::engine::context::Bindable::unbind
mod iso_subspace;
mod name;

pub(crate) use iso_subspace::IsoSubspace;

use super::*;

// NOTE: please be very selective when expanding this API.
// when possible, prefer expanding APIs for concrete subspaces.
//
/// An isolated region of heap-managed memory.
///
/// 1. Subspaces choose how to allocate their residents, as well as the
///    best way to store those allocations. It may be a [`Vec`]. It may be a
///    map. It may be whatever.
/// 2. Subspaces should, but are not required to, store homogenous data.
///    Subspaces _may_ choose to upgrade that suggestion to a requirement.
pub trait Subspace<T: SubspaceResident> {
    /// Display name for debugging purposes.
    ///
    /// Names are not guaranteed to be unique.  Do not rely on subspaces
    /// returning the same name in all cases; for example, a subspace may
    /// provide a `name` in debug builds but not in release builds.
    fn name(&self) -> Option<&str> {
        None
    }
    /// Store `data` into this subspace, returning a handle to the allocation.
    /// `data` may contain references to, or ownership of, other heap-allocated
    /// values.
    ///
    /// ## Safety
    /// - The lifetime parameter `'a` must be of the currently active [`NoGcScope`].
    /// - The subspace have the exact same lifetime as the [`Heap`] it belongs to,
    ///   which obviously must live longer than `'a`.
    ///
    /// The latter point is easy to enforce for subspaces put directly onto the
    /// [`Heap`], but if we decide to allow external subspaces, this could
    /// become more challenging
    fn alloc<'a>(&mut self, data: T::Bound<'a>) -> T::Key<'a>;
    // TODO: drop? len?
}

/// A thing that can live within a [`Subspace`].
pub trait SubspaceResident: Bindable {
    type Key<'a>: SubspaceIndex<'a, Self>;
    type Bound<'a>: Bindable<Of<'static> = Self>;
}

/// Ties a type to a specific [`Subspace`]. Implementing this trait
/// allows for `T`s to be created using [`Heap::alloc`].
///
/// ## Notes
/// - Eventually this will be used to support [`EmbedderObject`]s
/// - Ideally (and hopefully in the future) this will take the [`Agent`] instead of the [`Heap`] as an argument.
///   This would allow external consumers (e.g. runtimes) to put custom subspaces
///   into [`HostHooks`].
/// - Another possible alternative to the above option is storing a dynamic list
///   of subspaces, possibly an [`IndexVec`]. This introduces the challenge of
///   statically storing/knowing which index a data structure is stored in.
///
/// [`EmbedderObject`]: crate::ecmascript::builtins::embedder_object::EmbedderObject
/// [`HostHooks`]: crate::ecmascript::execution::agent::HostHooks
/// [`Agent`]: crate::ecmascript::execution::Agent
/// [`IndexVec`]: https://docs.rs/indexvec/latest/indexvec/struct.IndexVec.html
pub trait WithSubspace<T: SubspaceResident> {
    type Space: Subspace<T>;
    fn subspace_for(heap: &Heap) -> &Self::Space;
    fn subspace_for_mut(heap: &mut Heap) -> &mut Self::Space;
}

pub(crate) trait HeapIndexable {
    fn get_index(self) -> usize;
}

pub(crate) trait SubspaceIndex<'a, T: Bindable>:
    From<BaseIndex<'a, T>> + HeapIndexable
{
    /// # Do not use this
    /// This is only for Value discriminant creation.
    const _DEF: Self;
}

/// Declare a newtype backed by a heap allocation.
///
/// There is currently a single variant of this macro, that takes the form
/// ```rust,nocompile
/// declare_subspace_resident(iso = foospace; struct Foo, FooHeapData);
/// ```
/// where
/// - `foospace` is a property on [`Heap`] that is an [`IsoSubspace`] storing `FooHeapData<'static>`s.
/// - `Foo` is a newtime that wraps a [heap index](crate::heap::indexes::BaseIndex)
/// - `FooHeapData` is a struct that stores data in the heap.
///
/// This form is intended for declaring intrinsics within Nova. It should not be
/// used externally.
///
/// This macro creates `Foo<'a>` and attaches traits to it. It also implements
/// [`SubspaceResident`] for `FooHeapData<'static>`.
macro_rules! declare_subspace_resident {
    (iso = $space:ident; struct $Nominal:ident, $Data:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $Nominal<'a>(crate::heap::indexes::BaseIndex<'a, $Data<'static>>);

        impl<'a> From<BaseIndex<'a, $Data<'static>>> for $Nominal<'a> {
            fn from(value: crate::heap::indexes::BaseIndex<'a, $Data<'static>>) -> Self {
                $Nominal(value)
            }
        }

        impl crate::heap::HeapIndexable for $Nominal<'_> {
            #[inline]
            fn get_index(self) -> usize {
                self.0.into_index()
            }
        }

        impl<'a> crate::heap::SubspaceIndex<'a, $Data<'static>> for $Nominal<'a> {
            const _DEF: Self = Self(crate::heap::indexes::BaseIndex::from_u32_index(0));
        }

        // SAFETY: Property implemented as a lifetime transmute.
        unsafe impl crate::engine::context::Bindable for $Nominal<'_> {
            type Of<'a> = $Nominal<'a>;

            #[inline(always)]
            fn unbind(self) -> Self::Of<'static> {
                unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
            }

            #[inline(always)]
            fn bind<'a>(self, _gc: crate::engine::context::NoGcScope<'a, '_>) -> Self::Of<'a> {
                unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
            }
        }

        impl crate::heap::SubspaceResident for $Data<'static> {
            type Key<'a> = $Nominal<'a>;
            type Bound<'a> = $Data<'a>;
        }
        impl crate::heap::WithSubspace<$Data<'static>> for $Nominal<'_> {
            type Space = crate::heap::IsoSubspace<$Data<'static>>;
            fn subspace_for(heap: &Heap) -> &Self::Space {
                &heap.$space
            }
            fn subspace_for_mut(heap: &mut Heap) -> &mut Self::Space {
                &mut heap.$space
            }
        }

        impl<'a> crate::heap::CreateHeapData<$Data<'a>, $Nominal<'a>> for Heap {
            fn create(&mut self, data: ArrayHeapData<'a>) -> Array<'a> {
                self.alloc::<$Data<'static>>(data)
            }
        }
    };
}
pub(crate) use declare_subspace_resident;
