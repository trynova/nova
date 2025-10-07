use crate::{
    ecmascript::{
        builtins::{
            Array,
            promise_objects::promise_abstract_operations::promise_reaction_records::PromiseReactionHandler,
        },
        execution::Agent,
        types::Value,
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::Rootable,
    },
};

pub(crate) trait PromiseAllReactionHandler<'a>: Rootable + Bindable {
    fn get_result_array(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Array<'a>;
    fn increase_remaining_elements_count(self, agent: &mut Agent, gc: NoGcScope<'a, '_>);
    fn decrease_remaining_elements_count(self, agent: &mut Agent, gc: NoGcScope<'a, '_>);
    fn get_remaining_elements_count(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> u32;
    fn on_promise_fulfilled(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    );
    fn on_promise_rejected(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    );
    fn to_reaction_handler(self, index: u32, gc: NoGcScope<'a, '_>) -> PromiseReactionHandler<'a>;
}
