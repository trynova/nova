use crate::{ecmascript::{builtins::control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability, types::Value}, heap::indexes::ObjectIndex};

#[derive(Debug, Clone, Default)]
pub struct PromiseHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) promise_state: PromiseState,
    pub(crate) promise_fulfill_reactions: Option<PromiseReactions>,
    pub(crate) promise_reject_reactions: Option<PromiseReactions>,
    pub(crate) promise_is_handled: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum PromiseState {
    #[default]
    Pending,
    Fulfilled {
        promise_result: Value,
    },
    Rejected {
        promise_result: Value,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum PromiseReactions {
    One(PromiseReactionRecord),
    Many(Vec<PromiseReactionRecord>),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PromiseReactionRecord {
    capability: PromiseCapability,
    r#type: PromiseReactionType,
    handler: (),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PromiseReactionType {
    Fulfill,
    Reject,
}
