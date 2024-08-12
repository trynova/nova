// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::{
    ecmascript::{
        builtins::{
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
    engine::{Executable, ExecutionResult, Vm},
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AwaitReactionIdentifier<'gen>(u32, PhantomData<AwaitReaction<'gen>>);

impl<'gen> AwaitReactionIdentifier<'gen> {
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self(value as u32, PhantomData)
    }

    pub(crate) const fn from_u32(value: u32) -> Self {
        Self(value, PhantomData)
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
    ) {
        // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
        // 3. c. Push asyncContext onto the execution context stack; asyncContext is now the running execution context.
        let execution_context = agent[self].execution_context.take().unwrap();
        agent.execution_context_stack.push(execution_context);

        // 3. d. Resume the suspended evaluation of asyncContext using NormalCompletion(v) as the result of the operation that suspended it.
        // 5. d. Resume the suspended evaluation of asyncContext using ThrowCompletion(reason) as the result of the operation that suspended it.
        let vm = agent[self].vm.take().unwrap();
        let executable = agent[self].executable.take().unwrap();
        let execution_result = match reaction_type {
            PromiseReactionType::Fulfill => vm.resume(agent, &executable, value),
            PromiseReactionType::Reject => vm.resume_throw(agent, &executable, value),
        };

        match execution_result {
            ExecutionResult::Return(result) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. d. Remove acAsyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent.execution_context_stack.pop();
                // 2. e. If result is a normal completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « undefined »).
                //    f. Else if result is a return completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « result.[[Value]] »).
                agent[self].return_promise_capability.resolve(agent, result);
            }
            ExecutionResult::Throw(err) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. d. Remove acAsyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent.execution_context_stack.pop();
                // 2. g. i. Assert: result is a throw completion.
                //       ii. Perform ! Call(promiseCapability.[[Reject]], undefined, « result.[[Value]] »).
                agent[self]
                    .return_promise_capability
                    .reject(agent, err.value());
            }
            ExecutionResult::Await { vm, awaited_value } => {
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // 8. Remove asyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent[self].vm = Some(vm);
                agent[self].executable = Some(executable);
                agent[self].execution_context = Some(agent.execution_context_stack.pop().unwrap());

                // `handler` corresponds to the `fulfilledClosure` and `rejectedClosure` functions,
                // which resume execution of the function.
                let handler = PromiseReactionHandler::Await(self);
                // 2. Let promise be ? PromiseResolve(%Promise%, value).
                let promise = Promise::resolve(agent, awaited_value);
                // 7. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
                inner_promise_then(agent, promise, handler, handler, None);
            }
            ExecutionResult::Yield { .. } => unreachable!(),
        }
    }
}

impl<'gen> Index<AwaitReactionIdentifier<'gen>> for Agent<'gen> {
    type Output = AwaitReaction<'gen>;

    fn index(&self, index: AwaitReactionIdentifier<'gen>) -> &Self::Output {
        &self.heap.await_reactions[index]
    }
}

impl<'gen> IndexMut<AwaitReactionIdentifier<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: AwaitReactionIdentifier<'gen>) -> &mut Self::Output {
        &mut self.heap.await_reactions[index]
    }
}

impl<'gen> Index<AwaitReactionIdentifier<'gen>> for Vec<Option<AwaitReaction<'gen>>> {
    type Output = AwaitReaction<'gen>;

    fn index(&self, index: AwaitReactionIdentifier<'gen>) -> &Self::Output {
        self.get(index.into_index())
            .expect("AwaitReactionIdentifier out of bounds")
            .as_ref()
            .expect("AwaitReactionIdentifier slot empty")
    }
}

impl<'gen> IndexMut<AwaitReactionIdentifier<'gen>> for Vec<Option<AwaitReaction<'gen>>> {
    fn index_mut(&mut self, index: AwaitReactionIdentifier<'gen>) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("AwaitReactionIdentifier out of bounds")
            .as_mut()
            .expect("AwaitReactionIdentifier slot empty")
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for AwaitReactionIdentifier<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.await_reactions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index - compactions.await_reactions.get_shift_for_index(self_index),
        );
    }
}

#[derive(Debug)]
pub(crate) struct AwaitReaction<'gen> {
    pub(crate) vm: Option<Vm<'gen>>,
    pub(crate) executable: Option<Executable<'gen>>,
    pub(crate) execution_context: Option<ExecutionContext<'gen>>,
    pub(crate) return_promise_capability: PromiseCapability<'gen>,
}

impl<'gen> CreateHeapData<AwaitReaction<'gen>, AwaitReactionIdentifier<'gen>> for Heap<'gen> {
    fn create(&mut self, data: AwaitReaction<'gen>) -> AwaitReactionIdentifier<'gen> {
        self.await_reactions.push(Some(data));
        AwaitReactionIdentifier::last(&self.await_reactions)
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for AwaitReaction<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.vm.mark_values(queues);
        self.executable.mark_values(queues);
        self.return_promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.vm.sweep_values(compactions);
        self.executable.sweep_values(compactions);
        self.return_promise_capability.sweep_values(compactions);
    }
}
