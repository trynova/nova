// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod async_generator_abstract_operations;
mod async_generator_prototype;

use std::{
    collections::VecDeque,
    ops::{Index, IndexMut},
};

use async_generator_abstract_operations::resume_handle_result;
pub(crate) use async_generator_abstract_operations::{
    async_generator_await_return_on_fulfilled, async_generator_await_return_on_rejected,
    async_generator_start_result,
};
pub(crate) use async_generator_prototype::AsyncGeneratorPrototype;

use crate::{
    ecmascript::{
        builtins::control_abstraction_objects::{
            generator_objects::VmOrArguments,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{
            self, agent::JsError, Agent, ExecutionContext, ProtoIntrinsics, RealmIdentifier,
        },
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{
        context::{GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        Executable, Scoped, SuspendedVm,
    },
    heap::{
        indexes::{AsyncGeneratorIndex, BaseIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

use super::{
    generator_objects::SuspendedGeneratorState,
    promise_objects::promise_abstract_operations::promise_reaction_records::PromiseReactionType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncGenerator<'a>(pub(crate) AsyncGeneratorIndex<'a>);

impl AsyncGenerator<'_> {
    /// Unbind this AsyncGenerator from its current lifetime. This is necessary to use
    /// the AsyncGenerator as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> AsyncGenerator<'static> {
        unsafe { std::mem::transmute::<AsyncGenerator, AsyncGenerator<'static>>(self) }
    }

    // Bind this AsyncGenerator to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your AsyncGenerators cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let gen = gen.bind(&gc);
    // ```
    // to make sure that the unbound AsyncGenerator cannot be used after binding.
    pub const fn bind<'a>(self, _: NoGcScope<'a, '_>) -> AsyncGenerator<'a> {
        unsafe { std::mem::transmute::<AsyncGenerator, AsyncGenerator<'a>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, AsyncGenerator<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn get_realm(self, agent: &Agent) -> Option<RealmIdentifier> {
        println!("{:#?}", agent[self].async_generator_state);
        None
    }

    pub(crate) fn is_draining_queue(self, agent: &Agent) -> bool {
        matches!(
            agent[self].async_generator_state.as_ref().unwrap(),
            AsyncGeneratorState::DrainingQueue(_)
        )
    }

    pub(crate) fn queue_is_empty(self, agent: &Agent) -> bool {
        match agent[self].async_generator_state.as_ref().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::Suspended { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.is_empty(),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn peek_first(self, agent: &mut Agent) -> &AsyncGeneratorRequest {
        match agent[self].async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::Suspended { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.front().unwrap(),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn pop_first(self, agent: &mut Agent) -> AsyncGeneratorRequest {
        match agent[self].async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::Suspended { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.pop_front().unwrap(),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn append_to_queue(self, agent: &mut Agent, request: AsyncGeneratorRequest) {
        match agent[self].async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::Suspended { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.push_back(request),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn transition_to_draining_queue(self, agent: &mut Agent) {
        let async_generator_state = &mut agent[self].async_generator_state;
        let AsyncGeneratorState::Executing(queue) = async_generator_state.take().unwrap() else {
            unreachable!()
        };
        async_generator_state.replace(AsyncGeneratorState::DrainingQueue(queue));
    }

    pub(crate) fn transition_to_awaiting(
        self,
        agent: &mut Agent,
        vm: SuspendedVm,
        executable: Executable,
        execution_context: ExecutionContext,
    ) {
        let async_generator_state = &mut agent[self].async_generator_state;
        let AsyncGeneratorState::Executing(queue) = async_generator_state.take().unwrap() else {
            unreachable!()
        };
        async_generator_state.replace(AsyncGeneratorState::Awaiting {
            queue,
            state: SuspendedGeneratorState {
                vm_or_args: VmOrArguments::Vm(vm),
                executable,
                execution_context,
            },
        });
    }

    pub(crate) fn resume_await(
        self,
        agent: &mut Agent,
        reaction_type: PromiseReactionType,
        value: Value,
        mut gc: GcScope,
    ) {
        // TODO: Generator state should know if we're awaiting or yielding;
        // await will just continue work, yield will resolve an? entry from the queue.
        let AsyncGeneratorState::Awaiting { state, queue } =
            agent[self].async_generator_state.take().unwrap()
        else {
            unreachable!()
        };
        let SuspendedGeneratorState {
            vm_or_args,
            executable,
            execution_context,
        } = state;
        agent.execution_context_stack.push(execution_context);
        let VmOrArguments::Vm(vm) = vm_or_args else {
            unreachable!()
        };
        agent[self].async_generator_state = Some(AsyncGeneratorState::Executing(queue));
        let scoped_generator = self.scope(agent, gc.nogc());
        let execution_result = match reaction_type {
            PromiseReactionType::Fulfill => vm.resume(agent, executable, value, gc.reborrow()),
            PromiseReactionType::Reject => vm.resume_throw(agent, executable, value, gc.reborrow()),
        };

        resume_handle_result(agent, execution_result, executable, scoped_generator, gc);
    }
}

impl IntoValue for AsyncGenerator<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for AsyncGenerator<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl From<AsyncGenerator<'_>> for Value {
    fn from(val: AsyncGenerator) -> Self {
        Value::AsyncGenerator(val.unbind())
    }
}

impl<'a> From<AsyncGenerator<'a>> for Object<'a> {
    fn from(value: AsyncGenerator) -> Self {
        Object::AsyncGenerator(value.unbind())
    }
}

impl TryFrom<Value> for AsyncGenerator<'_> {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::AsyncGenerator(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Object<'a>> for AsyncGenerator<'a> {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        if let Object::AsyncGenerator(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> InternalSlots<'a> for AsyncGenerator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::AsyncGeneratorFunction;

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }
}

impl<'a> InternalMethods<'a> for AsyncGenerator<'a> {}

impl CreateHeapData<AsyncGeneratorHeapData, AsyncGenerator<'static>> for Heap {
    fn create(&mut self, data: AsyncGeneratorHeapData) -> AsyncGenerator<'static> {
        self.async_generators.push(Some(data));
        AsyncGenerator(AsyncGeneratorIndex::last(&self.async_generators))
    }
}

impl Index<AsyncGenerator<'_>> for Agent {
    type Output = AsyncGeneratorHeapData;

    fn index(&self, index: AsyncGenerator) -> &Self::Output {
        &self.heap.async_generators[index]
    }
}

impl IndexMut<AsyncGenerator<'_>> for Agent {
    fn index_mut(&mut self, index: AsyncGenerator) -> &mut Self::Output {
        &mut self.heap.async_generators[index]
    }
}

impl Index<AsyncGenerator<'_>> for Vec<Option<AsyncGeneratorHeapData>> {
    type Output = AsyncGeneratorHeapData;

    fn index(&self, index: AsyncGenerator) -> &Self::Output {
        self.get(index.get_index())
            .expect("AsyncGenerator out of bounds")
            .as_ref()
            .expect("AsyncGenerator slot empty")
    }
}

impl IndexMut<AsyncGenerator<'_>> for Vec<Option<AsyncGeneratorHeapData>> {
    fn index_mut(&mut self, index: AsyncGenerator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("AsyncGenerator out of bounds")
            .as_mut()
            .expect("AsyncGenerator slot empty")
    }
}

#[derive(Debug, Default)]
pub struct AsyncGeneratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject<'static>>,
    pub(crate) async_generator_state: Option<AsyncGeneratorState>,
}

#[derive(Debug)]
pub(crate) enum AsyncGeneratorState {
    // SUSPENDED-START has `vm_or_args` set to Arguments, SUSPENDED-YIELD has it set to Vm.
    Suspended {
        state: SuspendedGeneratorState,
        queue: VecDeque<AsyncGeneratorRequest>,
    },
    Executing(VecDeque<AsyncGeneratorRequest>),
    Awaiting {
        // TODO: Can never contain Arguments to the VmOrArgs is useless.
        // Just put the SuspendedVm in there.
        state: SuspendedGeneratorState,
        queue: VecDeque<AsyncGeneratorRequest>,
    },
    DrainingQueue(VecDeque<AsyncGeneratorRequest>),
    Completed,
}

impl AsyncGeneratorState {
    pub(crate) fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }

    pub(crate) fn is_suspended(&self) -> bool {
        matches!(self, Self::Suspended { .. })
    }

    pub(crate) fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Awaiting { .. } | Self::Executing { .. } | Self::DrainingQueue(_)
        )
    }

    pub(crate) fn is_draining(&self) -> bool {
        matches!(self, Self::DrainingQueue(_))
    }
}

/// ## [27.6.3.1 AsyncGeneratorRequest Records](https://tc39.es/ecma262/#sec-asyncgeneratorrequest-records)
///
/// An AsyncGeneratorRequest is a Record value used to store information about
/// how an async generator should be resumed and contains capabilities for
/// fulfilling or rejecting the corresponding promise.
#[derive(Debug)]
pub(crate) struct AsyncGeneratorRequest {
    /// \[\[Completion]]
    pub(crate) completion: AsyncGeneratorRequestCompletion,
    /// \[\[Capability]]
    pub(crate) capability: PromiseCapability,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AsyncGeneratorRequestCompletion {
    Ok(Value),
    Err(JsError),
    Return(Value),
}

impl Rootable for AsyncGenerator<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::AsyncGenerator(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::AsyncGenerator(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for AsyncGenerator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.async_generators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.async_generators.shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for AsyncGeneratorRequest {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            completion,
            capability,
        } = self;
        match completion {
            AsyncGeneratorRequestCompletion::Ok(value)
            | AsyncGeneratorRequestCompletion::Return(value) => value.mark_values(queues),
            AsyncGeneratorRequestCompletion::Err(err) => err.mark_values(queues),
        }
        capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            completion,
            capability,
        } = self;
        match completion {
            AsyncGeneratorRequestCompletion::Ok(value)
            | AsyncGeneratorRequestCompletion::Return(value) => value.sweep_values(compactions),
            AsyncGeneratorRequestCompletion::Err(err) => err.sweep_values(compactions),
        }
        capability.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for AsyncGeneratorHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            async_generator_state: generator_state,
        } = self;
        object_index.mark_values(queues);
        let Some(generator_state) = generator_state else {
            return;
        };
        match generator_state {
            AsyncGeneratorState::Awaiting { state, queue }
            | AsyncGeneratorState::Suspended { state, queue } => {
                state.mark_values(queues);
                for req in queue {
                    req.mark_values(queues);
                }
            }
            AsyncGeneratorState::Executing(queue) | AsyncGeneratorState::DrainingQueue(queue) => {
                for req in queue {
                    req.mark_values(queues);
                }
            }
            AsyncGeneratorState::Completed => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            async_generator_state: generator_state,
        } = self;
        object_index.sweep_values(compactions);
        let Some(generator_state) = generator_state else {
            return;
        };
        match generator_state {
            AsyncGeneratorState::Awaiting { state, queue }
            | AsyncGeneratorState::Suspended { state, queue } => {
                state.sweep_values(compactions);
                for req in queue {
                    req.sweep_values(compactions);
                }
            }
            AsyncGeneratorState::Executing(queue) | AsyncGeneratorState::DrainingQueue(queue) => {
                for req in queue {
                    req.sweep_values(compactions);
                }
            }
            AsyncGeneratorState::Completed => {}
        }
    }
}
