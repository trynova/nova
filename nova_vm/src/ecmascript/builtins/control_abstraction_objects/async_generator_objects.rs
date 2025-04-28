// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod async_generator_abstract_operations;
mod async_generator_prototype;

use core::ops::{Index, IndexMut};
use std::collections::VecDeque;

use async_generator_abstract_operations::{
    async_generator_await_return_on_fulfilled, async_generator_await_return_on_rejected,
    async_generator_yield, resume_handle_result,
};
pub(crate) use async_generator_prototype::AsyncGeneratorPrototype;

use crate::{
    ecmascript::{
        builtins::control_abstraction_objects::{
            generator_objects::VmOrArguments,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{Agent, ExecutionContext, ProtoIntrinsics, agent::JsError},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{
        Executable, ExecutionResult, Scoped, SuspendedVm,
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable, Scopable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
        indexes::{AsyncGeneratorIndex, BaseIndex},
    },
};

use super::promise_objects::promise_abstract_operations::promise_reaction_records::PromiseReactionType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncGenerator<'a>(pub(crate) AsyncGeneratorIndex<'a>);

impl AsyncGenerator<'_> {
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

    pub(crate) fn get_executable<'gc>(
        self,
        agent: &Agent,
        _: NoGcScope<'gc, '_>,
    ) -> Executable<'gc> {
        agent[self].executable.unwrap()
    }

    pub(crate) fn is_draining_queue(self, agent: &Agent) -> bool {
        agent[self]
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_draining_queue()
    }

    pub(crate) fn is_executing(self, agent: &Agent) -> bool {
        agent[self]
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_executing()
    }

    pub(crate) fn is_suspended_start(self, agent: &Agent) -> bool {
        agent[self]
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_suspended_start()
    }

    pub(crate) fn is_suspended_yield(self, agent: &Agent) -> bool {
        agent[self]
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_suspended_yield()
    }

    pub(crate) fn is_completed(self, agent: &Agent) -> bool {
        agent[self]
            .async_generator_state
            .as_ref()
            .unwrap()
            .is_completed()
    }

    pub(crate) fn queue_is_empty(self, agent: &Agent) -> bool {
        match agent[self].async_generator_state.as_ref().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.is_empty(),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn peek_first<'a, 'gc>(
        self,
        agent: &'a mut Agent,
        _gc: NoGcScope<'gc, '_>,
    ) -> &'a AsyncGeneratorRequest<'gc> {
        match agent[self].async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.front().unwrap(),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn pop_first<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> AsyncGeneratorRequest<'gc> {
        match agent[self].async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.pop_front().unwrap().bind(gc),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn append_to_queue(self, agent: &mut Agent, request: AsyncGeneratorRequest<'_>) {
        match agent[self].async_generator_state.as_mut().unwrap() {
            AsyncGeneratorState::Awaiting { queue, .. }
            | AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue)
            | AsyncGeneratorState::DrainingQueue(queue) => queue.push_back(request.unbind()),
            AsyncGeneratorState::Completed => unreachable!(),
        }
    }

    pub(crate) fn transition_to_draining_queue(self, agent: &mut Agent) {
        let async_generator_state = &mut agent[self].async_generator_state;
        let state = async_generator_state.take().unwrap();
        let queue = match state {
            AsyncGeneratorState::SuspendedStart { queue, .. }
            | AsyncGeneratorState::SuspendedYield { queue, .. }
            | AsyncGeneratorState::Executing(queue) => queue,
            _ => unreachable!(),
        };
        async_generator_state.replace(AsyncGeneratorState::DrainingQueue(queue));
    }

    pub(crate) fn transition_to_awaiting(
        self,
        agent: &mut Agent,
        vm: SuspendedVm,
        kind: AsyncGeneratorAwaitKind,
        execution_context: ExecutionContext,
    ) {
        let async_generator_state = &mut agent[self].async_generator_state;
        let AsyncGeneratorState::Executing(queue) = async_generator_state.take().unwrap() else {
            unreachable!()
        };
        async_generator_state.replace(AsyncGeneratorState::Awaiting {
            queue,
            vm,
            execution_context,
            kind,
        });
    }

    pub(crate) fn transition_to_execution<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> (VmOrArguments, ExecutionContext, Executable<'gc>) {
        let async_generator_state = &mut agent[self].async_generator_state;
        let (vm_or_args, execution_context, queue) = match async_generator_state.take().unwrap() {
            AsyncGeneratorState::SuspendedStart {
                arguments,
                execution_context,
                queue,
            } => (
                VmOrArguments::Arguments(arguments),
                execution_context,
                queue,
            ),
            AsyncGeneratorState::SuspendedYield {
                vm,
                execution_context,
                queue,
            } => (VmOrArguments::Vm(vm), execution_context, queue),
            _ => unreachable!(),
        };
        async_generator_state.replace(AsyncGeneratorState::Executing(queue));
        (
            vm_or_args,
            execution_context,
            self.get_executable(agent, gc),
        )
    }

    pub(crate) fn transition_to_suspended(
        self,
        agent: &mut Agent,
        vm: SuspendedVm,
        execution_context: ExecutionContext,
    ) {
        let async_generator_state = &mut agent[self].async_generator_state;
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
        let AsyncGeneratorState::Awaiting {
            vm,
            execution_context,
            queue,
            kind,
        } = agent[self].async_generator_state.take().unwrap()
        else {
            unreachable!()
        };
        agent.execution_context_stack.push(execution_context);
        agent[self].async_generator_state = Some(AsyncGeneratorState::Executing(queue));
        let scoped_generator = self.scope(agent, gc.nogc());
        let execution_result = match kind {
            AsyncGeneratorAwaitKind::Await => {
                // Await only.
                let executable = agent[self].executable.unwrap().scope(agent, gc.nogc());
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
                    let executable = agent[self].executable.unwrap().scope(agent, gc.nogc());
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
            AsyncGeneratorAwaitKind::Return => {
                // 27.6.3.7 AsyncGeneratorUnwrapYieldResumption
                // 3. If awaited is a throw completion, return ? awaited.
                if reaction_type == PromiseReactionType::Reject {
                    let executable = agent[self].executable.unwrap().scope(agent, gc.nogc());
                    vm.resume_throw(agent, executable, value.unbind(), gc.reborrow())
                } else {
                    // TODO: vm.resume_return(agent, executable, value, gc.reborrow())
                    // 4. Assert: awaited is a normal completion.
                    // 5. Return ReturnCompletion(awaited.[[Value]]).
                    ExecutionResult::Return(value)
                }
            }
        };
        resume_handle_result(agent, execution_result.unbind(), scoped_generator, gc);
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for AsyncGenerator<'_> {
    type Of<'a> = AsyncGenerator<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> IntoValue<'a> for AsyncGenerator<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for AsyncGenerator<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<AsyncGenerator<'a>> for Value<'a> {
    fn from(value: AsyncGenerator<'a>) -> Self {
        Value::AsyncGenerator(value)
    }
}

impl<'a> From<AsyncGenerator<'a>> for Object<'a> {
    fn from(value: AsyncGenerator) -> Self {
        Object::AsyncGenerator(value.unbind())
    }
}

impl<'a> TryFrom<Value<'a>> for AsyncGenerator<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
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
            Ok(value.unbind())
        } else {
            Err(())
        }
    }
}

impl<'a> InternalSlots<'a> for AsyncGenerator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::AsyncGenerator;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for AsyncGenerator<'a> {}

impl<'a> CreateHeapData<AsyncGeneratorHeapData<'a>, AsyncGenerator<'a>> for Heap {
    fn create(&mut self, data: AsyncGeneratorHeapData<'a>) -> AsyncGenerator<'a> {
        self.async_generators.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<AsyncGeneratorHeapData<'static>>>();
        AsyncGenerator(AsyncGeneratorIndex::last(&self.async_generators))
    }
}

impl Index<AsyncGenerator<'_>> for Agent {
    type Output = AsyncGeneratorHeapData<'static>;

    fn index(&self, index: AsyncGenerator) -> &Self::Output {
        &self.heap.async_generators[index]
    }
}

impl IndexMut<AsyncGenerator<'_>> for Agent {
    fn index_mut(&mut self, index: AsyncGenerator) -> &mut Self::Output {
        &mut self.heap.async_generators[index]
    }
}

impl Index<AsyncGenerator<'_>> for Vec<Option<AsyncGeneratorHeapData<'static>>> {
    type Output = AsyncGeneratorHeapData<'static>;

    fn index(&self, index: AsyncGenerator) -> &Self::Output {
        self.get(index.get_index())
            .expect("AsyncGenerator out of bounds")
            .as_ref()
            .expect("AsyncGenerator slot empty")
    }
}

impl IndexMut<AsyncGenerator<'_>> for Vec<Option<AsyncGeneratorHeapData<'static>>> {
    fn index_mut(&mut self, index: AsyncGenerator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("AsyncGenerator out of bounds")
            .as_mut()
            .expect("AsyncGenerator slot empty")
    }
}

#[derive(Debug, Default)]
pub struct AsyncGeneratorHeapData<'a> {
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
    /// AsyncGenerator is currently executing a return(value)'s implicit await.
    Return,
}

#[derive(Debug)]
pub(crate) enum AsyncGeneratorState<'a> {
    SuspendedStart {
        arguments: Box<[Value<'static>]>,
        execution_context: ExecutionContext,
        queue: VecDeque<AsyncGeneratorRequest<'a>>,
    },
    SuspendedYield {
        vm: SuspendedVm,
        execution_context: ExecutionContext,
        queue: VecDeque<AsyncGeneratorRequest<'a>>,
    },
    Executing(VecDeque<AsyncGeneratorRequest<'a>>),
    Awaiting {
        vm: SuspendedVm,
        execution_context: ExecutionContext,
        queue: VecDeque<AsyncGeneratorRequest<'a>>,
        kind: AsyncGeneratorAwaitKind,
    },
    DrainingQueue(VecDeque<AsyncGeneratorRequest<'a>>),
    Completed,
}

impl AsyncGeneratorState<'_> {
    pub(crate) fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
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

    pub(crate) fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Awaiting { .. } | Self::Executing { .. } | Self::DrainingQueue(_)
        )
    }
}

/// ## [27.6.3.1 AsyncGeneratorRequest Records](https://tc39.es/ecma262/#sec-asyncgeneratorrequest-records)
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for AsyncGeneratorRequest<'_> {
    type Of<'a> = AsyncGeneratorRequest<'a>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AsyncGeneratorRequestCompletion<'a> {
    Ok(Value<'a>),
    Err(JsError<'a>),
    Return(Value<'a>),
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for AsyncGeneratorRequestCompletion<'_> {
    type Of<'a> = AsyncGeneratorRequestCompletion<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for AsyncGeneratorHeapData<'_> {
    type Of<'a> = AsyncGeneratorHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

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
                arguments,
                execution_context,
                queue,
            } => {
                arguments.mark_values(queues);
                execution_context.mark_values(queues);
                for req in queue {
                    req.mark_values(queues);
                }
            }
            AsyncGeneratorState::Awaiting {
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
            executable,
        } = self;
        object_index.sweep_values(compactions);
        executable.sweep_values(compactions);
        let Some(generator_state) = generator_state else {
            return;
        };
        match generator_state {
            AsyncGeneratorState::SuspendedStart {
                arguments,
                execution_context,
                queue,
            } => {
                arguments.sweep_values(compactions);
                execution_context.sweep_values(compactions);
                for req in queue {
                    req.sweep_values(compactions);
                }
            }
            AsyncGeneratorState::Awaiting {
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
            AsyncGeneratorState::Executing(queue) | AsyncGeneratorState::DrainingQueue(queue) => {
                for req in queue {
                    req.sweep_values(compactions);
                }
            }
            AsyncGeneratorState::Completed => {}
        }
    }
}
