use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::ecmascript::{builtins::module::data::ModuleHeapData, execution::Agent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleIdentifier(u32, PhantomData<ModuleHeapData>);

impl ModuleIdentifier {
    /// Creates a module identififer from a usize.
    ///
    /// ## Panics
    /// If the given index is greater than `u32::MAX`.
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self(value as u32, PhantomData)
    }

    /// Creates a module identififer from a u32.
    pub(crate) const fn from_u32(value: u32) -> Self {
        Self(value, PhantomData)
    }

    pub(crate) fn last(modules: &[Option<ModuleHeapData>]) -> Self {
        let index = modules.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn into_u32(self) -> u32 {
        self.0
    }
}

impl Index<ModuleIdentifier> for Agent {
    type Output = ModuleHeapData;

    fn index(&self, index: ModuleIdentifier) -> &Self::Output {
        &self.heap.modules[index]
    }
}

impl IndexMut<ModuleIdentifier> for Agent {
    fn index_mut(&mut self, index: ModuleIdentifier) -> &mut Self::Output {
        &mut self.heap.modules[index]
    }
}

impl Index<ModuleIdentifier> for Vec<Option<ModuleHeapData>> {
    type Output = ModuleHeapData;

    fn index(&self, index: ModuleIdentifier) -> &Self::Output {
        self.get(index.into_index())
            .expect("ModuleIdentifier out of bounds")
            .as_ref()
            .expect("ModuleIdentifier slot empty")
    }
}

impl IndexMut<ModuleIdentifier> for Vec<Option<ModuleHeapData>> {
    fn index_mut(&mut self, index: ModuleIdentifier) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("ModuleIdentifier out of bounds")
            .as_mut()
            .expect("ModuleIdentifier slot empty")
    }
}
