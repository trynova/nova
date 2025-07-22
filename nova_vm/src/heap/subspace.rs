mod iso_subspace;

pub(crate) use iso_subspace::IsoSubspace;

use super::*;

pub trait Subspace<T: SubspaceResident> {
    fn alloc<'a>(&mut self, data: T::Bound<'a>) -> T::Key<'a>;
}
pub(crate) trait SubspaceResident: Bindable {
    type Key<'a>: SubspaceIndex<'a, Self>;
    type Bound<'a>: Bindable<Of<'static> = Self>;
}
pub trait WithSubspace<T: SubspaceResident> {
    type Space: Subspace<T>;
    // type Bound<'a> : Bindable<Of<'static> = Self>;
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

macro_rules! declare_subspace_resident {
    (iso = $space:ident; struct $Nominal:ident, $Data:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $Nominal<'a>(BaseIndex<'a, $Data<'static>>);

        impl<'a> From<BaseIndex<'a, $Data<'static>>> for $Nominal<'a> {
            fn from(value: BaseIndex<'a, $Data<'static>>) -> Self {
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
            const _DEF: Self = Self(BaseIndex::from_u32_index(0));
        }

        // SAFETY: Property implemented as a lifetime transmute.
        unsafe impl crate::engine::context::Bindable for $Nominal<'_> {
            type Of<'a> = $Nominal<'a>;

            #[inline(always)]
            fn unbind(self) -> Self::Of<'static> {
                unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
            }

            #[inline(always)]
            fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
                unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
            }
        }

        impl crate::heap::SubspaceResident for $Data<'static> {
            type Key<'a> = $Nominal<'a>;
            type Bound<'a> = $Data<'a>;
        }
        impl crate::heap::WithSubspace<$Data<'static>> for $Nominal<'_> {
            type Space = IsoSubspace<$Data<'static>>;
            fn subspace_for(heap: &Heap) -> &Self::Space {
                &heap.$space
            }
            fn subspace_for_mut(heap: &mut Heap) -> &mut Self::Space {
                &mut heap.$space
            }
        }
    };
}
pub(crate) use declare_subspace_resident;
