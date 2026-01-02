// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{
    marker::PhantomData,
};

use crate::{
    ecmascript::scripts_and_modules::module::module_semantics::source_text_module_records::SourceTextModule,
    engine::{
        Executable,
        context::{Bindable, GcScope, GcToken, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable, Scopable},
    },
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
pub struct AwaitReaction<'a>(
    u32,
    PhantomData<AwaitReactionRecord<'static>>,
    PhantomData<&'a GcToken>,
);

impl AwaitReaction<'_> {
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self::from_u32(value as u32)
    }

    pub(crate) const fn from_u32(value: u32) -> Self {
        Self(value, PhantomData, PhantomData)
    }

    pub(crate) fn last(scripts: &[AwaitReactionRecord]) -> Self {
        let index = scripts.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn resume(
        self,
        agent: &mut Agent,
        reaction_type: PromiseReactionType,
        value: Value,
        mut gc: GcScope,
    ) {
        let reaction = self.bind(gc.nogc());
        let value = value.bind(gc.nogc());
        // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
        // 3. c. Push asyncContext onto the execution context stack; asyncContext is now the running execution context.
        let record = &mut agent[reaction];
        let execution_context = record.execution_context.take().unwrap();
        let vm = record.vm.take().unwrap();
        let async_function = record.async_executable.unwrap().bind(gc.nogc());
        agent.push_execution_context(execution_context);

        let reaction = reaction.scope(agent, gc.nogc());
        // 3. d. Resume the suspended evaluation of asyncContext using NormalCompletion(v) as the result of the operation that suspended it.
        // 5. d. Resume the suspended evaluation of asyncContext using ThrowCompletion(reason) as the result of the operation that suspended it.
        let execution_result = match reaction_type {
            PromiseReactionType::Fulfill => {
                let executable = async_function.get_executable(agent).scope(agent, gc.nogc());
                vm.resume(agent, executable, value.unbind(), gc.reborrow())
                    .unbind()
                    .bind(gc.nogc())
            }
            PromiseReactionType::Reject => {
                let executable = async_function.get_executable(agent).scope(agent, gc.nogc());
                vm.resume_throw(agent, executable, value.unbind(), gc.reborrow())
                    .unbind()
                    .bind(gc.nogc())
            }
        };

        // SAFETY: reaction is not shared.
        let reaction = unsafe { reaction.take(agent) }.bind(gc.nogc());
        match execution_result {
            ExecutionResult::Return(result) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. d. Remove acAsyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent.pop_execution_context();
                // 2. e. If result is a normal completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « undefined »).
                //    f. Else if result is a return completion, then
                //       i. Perform ! Call(promiseCapability.[[Resolve]], undefined, « result.[[Value]] »).
                agent[reaction].return_promise_capability.clone().resolve(
                    agent,
                    result.unbind(),
                    gc,
                );
            }
            ExecutionResult::Throw(err) => {
                // [27.7.5.2 AsyncBlockStart ( promiseCapability, asyncBody, asyncContext )](https://tc39.es/ecma262/#sec-asyncblockstart)
                // 2. d. Remove acAsyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent.pop_execution_context();
                // 2. g. i. Assert: result is a throw completion.
                //       ii. Perform ! Call(promiseCapability.[[Reject]], undefined, « result.[[Value]] »).
                agent[reaction].return_promise_capability.clone().reject(
                    agent,
                    err.value().unbind(),
                    gc.nogc(),
                );
            }
            ExecutionResult::Await { vm, awaited_value } => {
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // 8. Remove asyncContext from the execution context stack and restore the execution context that is at the top of the execution context stack as the running execution context.
                agent[reaction].vm = Some(vm);
                agent[reaction].execution_context = Some(agent.pop_execution_context().unwrap());

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

impl Rootable for AwaitReaction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::AwaitReaction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::AwaitReaction(object) => Some(object),
            _ => None,
        }
    }
}

bindable_handle!(AwaitReaction);

impl HeapMarkAndSweep for AwaitReaction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.await_reactions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.await_reactions.shift_u32_index(&mut self.0);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AsyncExecutable<'a> {
    AsyncFunction(ECMAScriptFunction<'a>),
    AsyncModule(SourceTextModule<'a>),
}

impl<'a> AsyncExecutable<'a> {
    fn get_executable(self, agent: &Agent) -> Executable<'a> {
        match self {
            AsyncExecutable::AsyncFunction(f) => f.get_executable(agent),
            AsyncExecutable::AsyncModule(m) => m.get_executable(agent),
        }
    }
}

impl<'a> From<ECMAScriptFunction<'a>> for AsyncExecutable<'a> {
    fn from(value: ECMAScriptFunction<'a>) -> Self {
        Self::AsyncFunction(value)
    }
}

impl<'a> From<SourceTextModule<'a>> for AsyncExecutable<'a> {
    fn from(value: SourceTextModule<'a>) -> Self {
        Self::AsyncModule(value)
    }
}

bindable_handle!(AsyncExecutable);

#[derive(Debug)]
pub struct AwaitReactionRecord<'a> {
    pub(crate) vm: Option<SuspendedVm>,
    pub(crate) async_executable: Option<AsyncExecutable<'a>>,
    pub(crate) execution_context: Option<ExecutionContext>,
    pub(crate) return_promise_capability: PromiseCapability<'a>,
}

impl<'a> CreateHeapData<AwaitReactionRecord<'a>, AwaitReaction<'a>> for Heap {
    fn create(&mut self, data: AwaitReactionRecord<'a>) -> AwaitReaction<'a> {
        self.await_reactions.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<AwaitReactionRecord<'static>>();
        AwaitReaction::last(&self.await_reactions)
    }
}

bindable_handle!(AwaitReactionRecord);

impl HeapMarkAndSweep for AsyncExecutable<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            AsyncExecutable::AsyncFunction(f) => f.mark_values(queues),
            AsyncExecutable::AsyncModule(m) => m.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            AsyncExecutable::AsyncFunction(f) => f.sweep_values(compactions),
            AsyncExecutable::AsyncModule(m) => m.sweep_values(compactions),
        }
    }
}

impl HeapMarkAndSweep for AwaitReactionRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            vm,
            async_executable: async_function,
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
            async_executable: async_function,
            execution_context,
            return_promise_capability,
        } = self;
        vm.sweep_values(compactions);
        async_function.sweep_values(compactions);
        execution_context.sweep_values(compactions);
        return_promise_capability.sweep_values(compactions);
    }
}
