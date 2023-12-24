use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct ModuleIdentifier<'ctx, 'host>(u32, PhantomData<Module<'ctx, 'host>>);

impl<'ctx, 'host> ModuleIdentifier<'ctx, 'host> {
    /// Creates a module identififer from a usize.
    ///
    /// ## Panics
    /// If the given index is greater than `u32::MAX`.
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self(value as u32, PhantomData)
    }

    pub(crate) fn last(modules: &Vec<Option<Module>>) -> Self {
        let index = modules.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug)]
pub struct Module<'ctx, 'host> {
    ctx: PhantomData<&'ctx ()>,
    host: PhantomData<&'host ()>,
}
