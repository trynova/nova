// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod async_generator_abstract_operations;
mod async_generator_prototype;

use std::collections::VecDeque;

use async_generator_abstract_operations::{
    async_generator_await_return_on_fulfilled, async_generator_await_return_on_rejected,
    async_generator_yield, resume_handle_result,
};
pub(crate) use async_generator_prototype::AsyncGeneratorPrototype;

use crate::{
    ecmascript::{
        builtins::control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        execution::{Agent, ExecutionContext, ProtoIntrinsics, agent::JsError},
        types::{InternalMethods, InternalSlots, OrdinaryObject, Value, object_handle},
    },
    engine::{
        Executable, SuspendedVm, context::{Bindable, GcScope, NoGcScope, bindable_handle}, rootable::Scopable
    },
    heap::{
        ArenaAccess, ArenaAccessMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access, indexes::BaseIndex
    },
};

use super::promise_objects::promise_abstract_operations::promise_reaction_records::PromiseReactionType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct AsyncGenerator<'a>(BaseIndex<'a, AsyncGeneratorHeapData<'static>>);
object_handle!(AsyncGenerator);
arena_vec_access!(AsyncGenerator, 'a, AsyncGeneratorHeapData, async_generators);

impl AsyncGenerator<'_> {
    pub(crate) fn get_executable<'gc>(
        self,
        agent: &Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> Executable<'gc> {
        self.get(agent).executable.unwrap().bind(gc)
    }

    /// Returns true if the state of the AsyncGenerator is DRAINING-QUEUE or
    /// EXECUTING.
    ///
    /// > NOTE: In our implementation, EXECUTING is split into an extra
    /// > EXECUTING-AWAIT state. This also checks for that.
    pub(crate) fn is_active(self, agent: &Agent) -> bool {
        self.get(agent)
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_active()
    }

    pub(crate) fn is_draining_queue(self, agent: &Agent) -> bool {
        self.get(agent)
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_draining_queue()
    }

    pub(crate) fn is_executing(self, agent: &Agent) -> bool {
        self.get(agent)
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_executing()
    }

    pub(crate) fn is_suspended_start(self, agent: &Agent) -> bool {
        self.get(agent)
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_suspended_start()
    }

    pub(crate) fn is_suspended_yield(self, agent: &Agent) -> bool {
        self.get(agent)
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_suspended_yield()
    }

    pub(crate) fn is_completed(self, agent: &Agent) -> bool {
        self.get(agent)
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_completed()
    }

    pub(crate) fn queue_is_empty(self, agent: &Agent) -> bool {
        match self.get(agent).async_generator_state.as_ref().unwrap() {
            AsyncGeneratorState::ExecutingAwait { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue)
            | AsyncGeneratorState::Completed(queue) => queue.is_empty(),
        }
    }

    pub(crate) fn peek_first<'a, 'gc>(
        self,
        agent: &'a mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> &'a AsyncGeneratorRequest<'gc> {
        match self
            .bind(gc)
            .get_mut(agent)
            .async_generator_state
            .as_mut()
            .unwrap()
        {
            AsyncGeneratorState::ExecutingAwait { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue)
            | AsyncGeneratorState::Completed(queue) => queue.front().unwrap(),
        }
    }

    pub(crate) fn pop_first<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> AsyncGeneratorRequest<'gc> {
        match self.get_mut(agent).async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::ExecutingAwait { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue)
            | AsyncGeneratorState::Completed(queue) => queue.pop_front().unwrap().bind(gc),
        }
    }

    pub(crate) fn append_to_queue(self, agent: &mut Agent, request: AsyncGeneratorRequest<'_>) {
        match self.get_mut(agent).async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::ExecutingAwait { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue)
            | AsyncGeneratorState::Completed(queue) => queue.push_back(request.unbind()),
        }
    }

    pub(crate) fn transition_to_draining_queue(self, agent: &mut Agent) {
        let async_generator_state = &mut self.get_mut(agent).async_generator_state;
        let state = async_generator_state.take().unwrap();
        let queue = match state {
            AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::Completed(queue) => queue,
            _ => unreachable!(),
        };
        async_generator_state.replace(AsyncGeneratorState::DrainingQueue(queue));
    }

    pub(crate) fn transition_to_complete(self, agent: &mut Agent) {
        let async_generator_state = &mut self.get_mut(agent).async_generator_state;
        let state = async_generator_state.take().unwrap();
        let queue = match state {
            AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::ExecutingAwait { queue, .. }
            | AsyncGeneratorState::DrainingQueue(queue)
            | AsyncGeneratorState::Completed(queue) => queue,
        };
        async_generator_state.replace(AsyncGeneratorState::Completed(queue));
    }

    pub(crate) fn transition_to_awaiting(
        self,
        agent: &mut Agent,
        vm: SuspendedVm,
        kind: AsyncGeneratorAwaitKind,
        execution_context: ExecutionContext,
    ) {
        let async_generator_state = &mut self.get_mut(agent).async_generator_state;
        let AsyncGeneratorState::Executing(queue) = async_generator_state.take().unwrap() else {
            unreachable!()
        };
        async_generator_state.replace(AsyncGeneratorState::ExecutingAwait {
            queue,
            vm,
            execution_context,
            kind,
        });
    }

    pub(crate) fn transition_to_executing<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> (SuspendedVm, ExecutionContext, Executable<'gc>) {
        let async_generator_state = &mut self.get_mut(agent).async_generator_state;
        let (vm, execution_context, queue) = match async_generator_state.take() {
            Some(AsyncGeneratorState::SuspendedStart {
                vm,
                execution_context,
                queue,
            }) => (vm, execution_context, queue),
            Some(AsyncGeneratorState::SuspendedYield {
                vm,
                execution_context,
                queue,
            }) => (vm, execution_context, queue),
            _ => unreachable!(),
        };
        async_generator_state.replace(AsyncGeneratorState::Executing(queue));
        (vm, execution_context, self.get_executable(agent, gc))
    }

    pub(crate) fn transition_to_suspended(
        self,
        agent: &mut Agent,
        vm: SuspendedVm,
        execution_context: ExecutionContext,
    ) {
        let async_generator_state = &mut self.get_mut(agent).async_generator_state;
        let AsyncGeneratorState::Executing(queue) = async_generator_state.take().unwrap() else {
            unreachable!()
        };
        async_generator_state.replace(AsyncGeneratorState::SuspendedYield {
            queue,
            vm,
            execution_context,
        });
    }

    pub(crate) fn resume_await(
        self,
        agent: &mut Agent,
        reaction_type: PromiseReactionType,
        value: Value,
        mut gc: GcScope,
    ) {
        let value = value.bind(gc.nogc());
        if self.is_draining_queue(agent) {
            // We're coming here because return was called.
            match reaction_type {
                PromiseReactionType::Fulfill => {
                    // AsyncGeneratorAwaitReturn onFulfilled
                    async_generator_await_return_on_fulfilled(agent, self, value.unbind(), gc);
                }
                PromiseReactionType::Reject => {
                    async_generator_await_return_on_rejected(agent, self, value.unbind(), gc);
                }
            }
            return;
        }
        // 1. Assert: generator.[[AsyncGeneratorState]] is either suspended-start or suspended-yield.
        let state = self.get_mut(agent).async_generator_state.take().unwrap();
        let (vm, execution_context, queue, kind) = match state {
            AsyncGeneratorState::SuspendedYield {
                vm,
                execution_context,
                queue,
            } => (vm, execution_context, queue, AsyncGeneratorAwaitKind::Yield),
            AsyncGeneratorState::ExecutingAwait {
                vm,
                execution_context,
                queue,
                kind,
            } => (vm, execution_context, queue, kind),
            _ => unreachable!(),
        };
        agent.push_execution_context(execution_context);
        self.get_mut(agent).async_generator_state = Some(AsyncGeneratorState::Executing(queue));
        let scoped_generator = self.scope(agent, gc.nogc());
        let execution_result = match kind {
            AsyncGeneratorAwaitKind::Await => {
                // Await only.
                let executable = self.get(agent).executable.unwrap().scope(agent, gc.nogc());
                match reaction_type {
                    PromiseReactionType::Fulfill => {
                        vm.resume(agent, executable, value.unbind(), gc.reborrow())
                    }
                    PromiseReactionType::Reject => {
                        vm.resume_throw(agent, executable, value.unbind(), gc.reborrow())
                    }
                }
            }
            AsyncGeneratorAwaitKind::Yield => {
                // Await yield
                if reaction_type == PromiseReactionType::Reject {
                    // ? Yield ( ? Await ( Value ) ), so Yield doesn't get
                    // performed at all and value is just thrown.
                    let executable = self.get(agent).executable.unwrap().scope(agent, gc.nogc());
                    vm.resume_throw(agent, executable, value.unbind(), gc.reborrow())
                } else {
                    async_generator_yield(
                        agent,
                        value.unbind(),
                        scoped_generator.clone(),
                        vm,
                        gc.reborrow(),
                    );
                    return;
                }
            }
        };
        resume_handle_result(agent, execution_result.unbind(), scoped_generator, gc);
    }
}

impl<'a> InternalSlots<'a> for AsyncGenerator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::AsyncGenerator;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for AsyncGenerator<'a> {}

impl<'a> CreateHeapData<AsyncGeneratorHeapData<'a>, AsyncGenerator<'a>> for Heap {
    fn create(&mut self, data: AsyncGeneratorHeapData<'a>) -> AsyncGenerator<'a> {
        self.async_generators.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<AsyncGeneratorHeapData<'static>>();
        AsyncGenerator(BaseIndex::last(&self.async_generators))
    }
}

#[derive(Debug, Default)]
pub(crate) struct AsyncGeneratorHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) async_generator_state: Option<AsyncGeneratorState<'a>>,
    pub(crate) executable: Option<Executable<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsyncGeneratorAwaitKind {
    /// AsyncGenerator is currently executing an explicit await.
    Await,
    /// AsyncGenerator is currently executing a next(value)'s implicit await.
    Yield,
}

#[derive(Debug)]
pub(crate) enum AsyncGeneratorState<'a> {
    SuspendedStart {
        vm: SuspendedVm,
        execution_context: ExecutionContext,
        queue: VecDeque<AsyncGeneratorRequest<'a>>,
    },
    SuspendedYield {
        vm: SuspendedVm,
        execution_context: ExecutionContext,
        queue: VecDeque<AsyncGeneratorRequest<'a>>,
    },
    Executing(VecDeque<AsyncGeneratorRequest<'a>>),
    /// Custom addition to \[\[AsyncGeneratorState]]: this corresponds to an
    /// Executing generator performing an Await; from the specification
    /// perspective the generator is still executing but its execution context
    /// is suspended.
    ExecutingAwait {
        vm: SuspendedVm,
        execution_context: ExecutionContext,
        queue: VecDeque<AsyncGeneratorRequest<'a>>,
        kind: AsyncGeneratorAwaitKind,
    },
    DrainingQueue(VecDeque<AsyncGeneratorRequest<'a>>),
    Completed(VecDeque<AsyncGeneratorRequest<'a>>),
}

impl AsyncGeneratorState<'_> {
    pub(crate) fn is_active(&self) -> bool {
        matches!(
            self,
            AsyncGeneratorState::DrainingQueue(_)
                | AsyncGeneratorState::Executing(_)
                | AsyncGeneratorState::ExecutingAwait { .. }
        )
    }

    pub(crate) fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(_))
    }

    pub(crate) fn is_draining_queue(&self) -> bool {
        matches!(self, AsyncGeneratorState::DrainingQueue(_))
    }

    pub(crate) fn is_executing(&self) -> bool {
        matches!(self, AsyncGeneratorState::Executing(_))
    }

    pub(crate) fn is_suspended(&self) -> bool {
        matches!(
            self,
            Self::SuspendedStart { .. } | Self::SuspendedYield { .. }
        )
    }

    pub(crate) fn is_suspended_start(&self) -> bool {
        matches!(self, AsyncGeneratorState::SuspendedStart { .. })
    }

    pub(crate) fn is_suspended_yield(&self) -> bool {
        matches!(self, AsyncGeneratorState::SuspendedYield { .. })
    }
}

/// ### [27.6.3.1 AsyncGeneratorRequest Records](https://tc39.es/ecma262/#sec-asyncgeneratorrequest-records)
///
/// An AsyncGeneratorRequest is a Record value used to store information about
/// how an async generator should be resumed and contains capabilities for
/// fulfilling or rejecting the corresponding promise.
#[derive(Debug)]
pub(crate) struct AsyncGeneratorRequest<'a> {
    /// \[\[Completion]]
    pub(crate) completion: AsyncGeneratorRequestCompletion<'a>,
    /// \[\[Capability]]
    pub(crate) capability: PromiseCapability<'a>,
}

bindable_handle!(AsyncGeneratorRequest);

#[derive(Debug, Clone, Copy)]
pub(crate) enum AsyncGeneratorRequestCompletion<'a> {
    Ok(Value<'a>),
    Err(JsError<'a>),
    Return(Value<'a>),
}

bindable_handle!(AsyncGeneratorRequestCompletion);

impl HeapMarkAndSweep for AsyncGenerator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.async_generators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.async_generators.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for AsyncGenerator<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .async_generators
            .shift_weak_index(self.0)
            .map(Self)
    }
}

impl HeapMarkAndSweep for AsyncGeneratorRequest<'static> {
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

bindable_handle!(AsyncGeneratorHeapData);

impl HeapMarkAndSweep for AsyncGeneratorHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            async_generator_state: generator_state,
            executable,
        } = self;
        object_index.mark_values(queues);
        executable.mark_values(queues);
        let Some(generator_state) = generator_state else {
            return;
        };
        match generator_state {
            AsyncGeneratorState::SuspendedStart {
                vm,
                execution_context,
                queue,
            }
            | AsyncGeneratorState::ExecutingAwait {
                vm,
                execution_context,
                queue,
                ..
            }
            | AsyncGeneratorState::SuspendedYield {
                vm,
                execution_context,
                queue,
            } => {
                vm.mark_values(queues);
                execution_context.mark_values(queues);
                for req in queue {
                    req.mark_values(queues);
                }
            }
            AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue)
            | AsyncGeneratorState::Completed(queue) => {
                for req in queue {
                    req.mark_values(queues);
                }
            }
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            async_generator_state: generator_state,
            executable,
        } = self;
        object_index.sweep_values(compactions);
        executable.sweep_values(compactions);
        let Some(generator_state) = generator_state else {
            return;
        };
        match generator_state {
            AsyncGeneratorState::SuspendedStart {
                vm,
                execution_context,
                queue,
            }
            | AsyncGeneratorState::ExecutingAwait {
                vm,
                queue,
                execution_context,
                ..
            }
            | AsyncGeneratorState::SuspendedYield {
                vm,
                execution_context,
                queue,
            } => {
                vm.sweep_values(compactions);
                execution_context.sweep_values(compactions);
                for req in queue {
                    req.sweep_values(compactions);
                }
            }
            AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue)
            | AsyncGeneratorState::Completed(queue) => {
                for req in queue {
                    req.sweep_values(compactions);
                }
            }
        }
    }
}
