use std::ops::{Index, IndexMut};

use crate::{ecmascript::execution::Agent, heap::{indexes::BaseIndex, Heap}};

use super::promise_capability_records::PromiseCapability;

/// \[\[Type\]\] 
/// 
/// fulfill or reject 
/// 
/// The \[\[Type\]\] is used when \[\[Handler\]\] is empty to allow for
/// behaviour specific to the settlement type.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PromiseReactionType {
    Fulfill,
    Reject,
}

/// \[\[Handler\]\] 
/// 
/// a JobCallback Record or empty 
/// 
/// The function that should be applied to the incoming value, and whose
/// return value will govern what happens to the derived promise. If
/// \[\[Handler\]\] is empty, a function that depends on the value of
/// \[\[Type\]\] will be used instead. 
#[derive(Debug, Clone, Copy)]
pub(crate) enum PromiseReactionHandler {
    Empty(PromiseReactionType),
    JobCallback(()),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PromiseReactionRecord {
    /// \[\[Capability\]\] 
    /// 
    /// a PromiseCapability Record or undefined 
    /// 
    /// The capabilities of the promise for which this record provides a
    /// reaction handler.
    pub(crate) capability: Option<PromiseCapability>,
    /// \[\[Handler\]\] 
    /// 
    /// a JobCallback Record or empty 
    /// 
    /// The function that should be applied to the incoming value, and whose
    /// return value will govern what happens to the derived promise. If
    /// \[\[Handler\]\] is empty, a function that depends on the value of
    /// \[\[Type\]\] will be used instead. 
    pub(crate) handler: PromiseReactionHandler,
}

pub type PromiseReaction = BaseIndex<PromiseReactionRecord>;

impl Index<PromiseReaction> for Agent {
    type Output = PromiseReactionRecord;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<PromiseReaction> for Agent {
    fn index_mut(&mut self, index: PromiseReaction) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<PromiseReaction> for Heap {
    type Output = PromiseReactionRecord;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        self.promise_reaction_records
            .get(index.into_index())
            .expect("PromiseReaction out of bounds")
            .as_ref()
            .expect("PromiseReaction slot empty")
    }
}

impl IndexMut<PromiseReaction> for Heap {
    fn index_mut(&mut self, index: PromiseReaction) -> &mut Self::Output {
        self.promise_reaction_records
            .get_mut(index.into_index())
            .expect("PromiseReaction out of bounds")
            .as_mut()
            .expect("PromiseReaction slot empty")
    }
}