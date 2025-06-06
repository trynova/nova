// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::engine::{
    context::{Bindable, GcScope, GcToken, NoGcScope},
    rootable::Scopable,
};
use crate::{
    ecmascript::{
        builtins::{
            ECMAScriptFunction,
            control_abstraction_objects::promise_objects::{
                promise_abstract_operations::{
                    promise_capability_records::PromiseCapability,
                    promise_reaction_records::{PromiseReactionHandler, PromiseReactionType},
                },
                promise_prototype::inner_promise_then,
            },
            promise::Promise,
        },
        execution::{Agent, ExecutionContext},
        types::Value,
    },
    engine::{ExecutionResult, SuspendedVm},
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct AwaitReactionIdentifier<'a>(
    u32,
    PhantomData<AwaitReaction<'static>>,
    PhantomData<&'a GcToken>,
);

impl AwaitReactionIdentifier<'_> {
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self::from_u32(value as u32)
    }

    pub(crate) const fn from_u32(value: u32) -> Self {
        Self(value, PhantomData, PhantomData)
    }

    pub(crate) fn last(scripts: &[Option<AwaitReaction>]) -> Self {
        let index = scripts.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn into_u32(self) -> u32 {
        self.0
    }

    pub(crate) fn resume(
        self,
        agent: &mut Agent,
        reaction_type: PromiseReactionType,
        value: Value,
        mut gc: GcScope,
    ) {
        let value = value.bind(gc.nogc());
        // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
        // 3. c. Push asyncContext onto the execution context stack; asyncContext is now the running execution context.
        let execution_context = agent[self].execution_context.take().unwrap();
        agent.push_execution_context(execution_context);

        // 3. d. Resume the suspended evaluation of asyncContext using NormalCompletion(v) as the result of the operation that suspended it.
        // 5. d. Resume the suspended evaluation of asyncContext using ThrowCompletion(reason) as the result of the operation that suspended it.
        let vm = agent[self].vm.take().unwrap();
        let async_function = agent[self].async_function.unwrap();
        let execution_result = match reaction_type {
            PromiseReactionType::Fulfill => {
                let executable = async_function
                    .get_executable(agent, gc.nogc())
                    .scope(agent, gc.nogc());
                vm.resume(agent, executable, value.unbind(), gc.reborrow())
            }
            PromiseReactionType::Reject => {
                let executable = async_function
                    .get_executable(agent, gc.nogc())
                    .scope(agent, gc.nogc());
                vm.resume_throw(agent, executable, value.unbind(), gc.reborrow())
            }
        };

        match execution_result {
            ExecutionResult::Return(result) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. d. Remove acAsyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent.pop_execution_context();
                // 2. e. If result is a normal completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « undefined »).
                //    f. Else if result is a return completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « result.[[Value]] »).
                agent[self]
                    .return_promise_capability
                    .clone()
                    .resolve(agent, result.unbind(), gc);
            }
            ExecutionResult::Throw(err) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. d. Remove acAsyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent.pop_execution_context();
                // 2. g. i. Assert: result is a throw completion.
                //       ii. Perform ! Call(promiseCapability.[[Reject]], undefined, « result.[[Value]] »).
                agent[self].return_promise_capability.clone().reject(
                    agent,
                    err.value().unbind(),
                    gc.nogc(),
                );
            }
            ExecutionResult::Await { vm, awaited_value } => {
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // 8. Remove asyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent[self].vm = Some(vm);
                agent[self].execution_context = Some(agent.pop_execution_context().unwrap());

                // `handler` corresponds to the `fulfilledClosure` and `rejectedClosure` functions,
                // which resume execution of the function.
                let handler = PromiseReactionHandler::Await(self);
                // 2. Let promise be ? PromiseResolve(%Promise%, value).
                let promise = Promise::resolve(agent, awaited_value.unbind(), gc.reborrow())
                    .unbind()
                    .bind(gc.nogc());
                // 7. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
                inner_promise_then(agent, promise, handler, handler, None, gc.nogc());
            }
            ExecutionResult::Yield { .. } => unreachable!(),
        }
    }
}

impl Index<AwaitReactionIdentifier<'_>> for Agent {
    type Output = AwaitReaction<'static>;

    fn index(&self, index: AwaitReactionIdentifier) -> &Self::Output {
        &self.heap.await_reactions[index]
    }
}

impl IndexMut<AwaitReactionIdentifier<'_>> for Agent {
    fn index_mut(&mut self, index: AwaitReactionIdentifier) -> &mut Self::Output {
        &mut self.heap.await_reactions[index]
    }
}

impl Index<AwaitReactionIdentifier<'_>> for Vec<Option<AwaitReaction<'static>>> {
    type Output = AwaitReaction<'static>;

    fn index(&self, index: AwaitReactionIdentifier) -> &Self::Output {
        self.get(index.into_index())
            .expect("AwaitReactionIdentifier out of bounds")
            .as_ref()
            .expect("AwaitReactionIdentifier slot empty")
    }
}

impl IndexMut<AwaitReactionIdentifier<'_>> for Vec<Option<AwaitReaction<'static>>> {
    fn index_mut(&mut self, index: AwaitReactionIdentifier) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("AwaitReactionIdentifier out of bounds")
            .as_mut()
            .expect("AwaitReactionIdentifier slot empty")
    }
}

impl HeapMarkAndSweep for AwaitReactionIdentifier<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.await_reactions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.await_reactions.shift_u32_index(&mut self.0);
    }
}

#[derive(Debug)]
pub(crate) struct AwaitReaction<'a> {
    pub(crate) vm: Option<SuspendedVm>,
    pub(crate) async_function: Option<ECMAScriptFunction<'a>>,
    pub(crate) execution_context: Option<ExecutionContext>,
    pub(crate) return_promise_capability: PromiseCapability<'a>,
}

impl<'a> CreateHeapData<AwaitReaction<'a>, AwaitReactionIdentifier<'a>> for Heap {
    fn create(&mut self, data: AwaitReaction<'a>) -> AwaitReactionIdentifier<'a> {
        self.await_reactions.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<AwaitReaction<'static>>>();
        AwaitReactionIdentifier::last(&self.await_reactions)
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for AwaitReaction<'_> {
    type Of<'a> = AwaitReaction<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for AwaitReaction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            vm,
            async_function,
            execution_context,
            return_promise_capability,
        } = self;
        vm.mark_values(queues);
        async_function.mark_values(queues);
        execution_context.mark_values(queues);
        return_promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            vm,
            async_function,
            execution_context,
            return_promise_capability,
        } = self;
        vm.sweep_values(compactions);
        async_function.sweep_values(compactions);
        execution_context.sweep_values(compactions);
        return_promise_capability.sweep_values(compactions);
    }
}
