mod iso_subspace;

pub(crate) use iso_subspace::{IsoSubspace, IsoSubspaceResident};

use super::*;
// type Ptr<'a, T: ?Sized> = BaseIndex<'a, T>;

pub trait Subspace<'a, T: ?Sized, Ptr: ?Sized> {
    fn alloc(&'a mut self, data: T) -> Ptr;
}

pub trait SubspaceResident<'a, HeapRepr: ?Sized> {
    type Space: Subspace<'a, HeapRepr, Self>;
    fn subspace_for(heap: &Heap) -> &Self::Space;
    fn subspace_for_mut(heap: &mut Heap) -> &mut Self::Space;
}
pub(crate) trait HeapIndexable {
    fn get_index(self) -> usize;
}


pub(crate) trait SubspaceIndex<'a, T: Bindable>:
    From<BaseIndex<'a, T::Of<'static>>> + HeapIndexable
{
    /// # Do not use this
    /// This is only for Value discriminant creation.
    const _DEF: Self;
    // const fn _def() -> Self {
    //     Self(BaseIndex::from_u32_index(0))
    // }
    // fn get_index(self) -> usize;
    //  {
    //     self.0.into_index()
    // }
    // fn id(self) -> BaseIndex<'a, T>;
    // fn get_index(self) -> usize {

    // }
}

// pub trait IsoSubspaceResident {
//     type Data<'a>: Bindable<Of<'a> = Self::Data<'a>>;
// }

macro_rules! declare_subspace_resident {
    (iso; struct $Nominal:ident, $Data:ident) => {
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

        impl<'a> crate::heap::SubspaceIndex<'a, $Data<'a>> for $Nominal<'a> {
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

        impl crate::heap::IsoSubspaceResident for $Data<'_> {
            type Key<'a> = $Nominal<'a>;
        }
    };
}
pub(crate) use declare_subspace_resident;
