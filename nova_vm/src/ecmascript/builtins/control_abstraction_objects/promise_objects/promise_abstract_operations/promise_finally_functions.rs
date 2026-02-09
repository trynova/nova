// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Function, FunctionInternalProperties, JsError,
        JsResult, OrdinaryObject, Promise, String, Value, call_function, function_handle, invoke,
    },
    engine::{Bindable, GcScope, Scopable, bindable_handle},
    heap::{
        ArenaAccess, ArenaAccessMut, BaseIndex, CompactionLists, CreateHeapData, Heap,
        HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access,
    },
};

#[derive(Debug, Clone, Copy)]
enum PromiseFinallyFunctionType<'a> {
    ResolveFinally {
        on_finally: Function<'a>,
        c: Function<'a>,
    },
    RejectFinally {
        on_finally: Function<'a>,
        c: Function<'a>,
    },
    ReturnValue {
        value: Value<'a>,
    },
    ThrowReason {
        reason: JsError<'a>,
    },
}

/// ### [27.2.1.3.1 Promise Finally Functions](https://tc39.es/ecma262/#sec-promise.prototype.finally)
///
/// A promise finally function is an abstract closure
/// \[\[Promise\]\] and \[\[AlreadyResolved\]\] internal slots.
///
/// The "length" property of a promise reject function is 1𝔽.
#[derive(Debug, Clone)]
pub(crate) struct PromiseFinallyFunctionHeapData<'a> {
    backing_object: Option<OrdinaryObject<'a>>,
    resolve_type: PromiseFinallyFunctionType<'a>,
}
bindable_handle!(PromiseFinallyFunctionHeapData);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BuiltinPromiseFinallyFunction<'a>(
    BaseIndex<'a, PromiseFinallyFunctionHeapData<'static>>,
);
function_handle!(BuiltinPromiseFinallyFunction);
arena_vec_access!(BuiltinPromiseFinallyFunction, 'a, PromiseFinallyFunctionHeapData, promise_finally_functions);

impl<'f> BuiltinPromiseFinallyFunction<'f> {
    pub(crate) fn create_finally_functions(
        agent: &mut Agent,
        c: Function<'f>,
        on_finally: Function<'f>,
    ) -> (Self, Self) {
        let then_finally_closure = agent.heap.create(PromiseFinallyFunctionHeapData {
            backing_object: None,
            resolve_type: PromiseFinallyFunctionType::ResolveFinally { on_finally, c },
        });
        let catch_finally_closure = agent.heap.create(PromiseFinallyFunctionHeapData {
            backing_object: None,
            resolve_type: PromiseFinallyFunctionType::RejectFinally { on_finally, c },
        });
        (then_finally_closure, catch_finally_closure)
    }
}

impl<'a> FunctionInternalProperties<'a> for BuiltinPromiseFinallyFunction<'a> {
    fn get_name(self, _: &Agent) -> &String<'a> {
        &String::EMPTY_STRING
    }

    fn get_length(self, agent: &Agent) -> u8 {
        match self.get(agent).resolve_type {
            PromiseFinallyFunctionType::ResolveFinally { .. }
            | PromiseFinallyFunctionType::RejectFinally { .. } => 1,
            PromiseFinallyFunctionType::ReturnValue { .. }
            | PromiseFinallyFunctionType::ThrowReason { .. } => 0,
        }
    }

    #[inline(always)]
    fn get_function_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.unbind().get(agent).backing_object
    }

    fn set_function_backing_object(
        self,
        agent: &mut Agent,
        backing_object: OrdinaryObject<'static>,
    ) {
        assert!(
            self.get_mut(agent)
                .backing_object
                .replace(backing_object)
                .is_none()
        );
    }

    fn function_call<'gc>(
        self,
        agent: &mut Agent,
        _this_value: Value,
        arguments_list: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        agent.check_call_depth(gc.nogc()).unbind()?;
        let f = self.bind(gc.nogc());
        match f.get(agent).resolve_type {
            PromiseFinallyFunctionType::ResolveFinally { on_finally, c } => {
                let value = arguments_list.get(0).scope(agent, gc.nogc());
                let c = c.scope(agent, gc.nogc());
                // i. Let result be ? Call(onFinally, undefined).
                let result = call_function(
                    agent,
                    on_finally.unbind(),
                    Value::Undefined,
                    None,
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: not shared.
                let _c = unsafe { c.take(agent) }.bind(gc.nogc());
                // ii. Let p be ? PromiseResolve(C, result).
                let p = Promise::resolve(agent, result.unbind(), gc.reborrow())
                    .unbind()
                    .bind(gc.nogc());
                // SAFETY: not shared.
                let value = unsafe { value.take(agent) }.bind(gc.nogc());
                // iii. Let returnValue be a new Abstract Closure with no
                //      parameters that captures value and performs the
                //      following steps when called:
                // iv. Let valueThunk be CreateBuiltinFunction(returnValue, 0, "", « »).
                let value_thunk = agent.heap.create(PromiseFinallyFunctionHeapData {
                    backing_object: None,
                    // 1. Return NormalCompletion(value).
                    resolve_type: PromiseFinallyFunctionType::ReturnValue { value },
                });

                // v. Return ? Invoke(p, "then", « valueThunk »).
                invoke(
                    agent,
                    p.unbind().into(),
                    BUILTIN_STRING_MEMORY.then.to_property_key(),
                    Some(ArgumentsList::from_mut_value(
                        &mut value_thunk.unbind().into(),
                    )),
                    gc,
                )
            }
            PromiseFinallyFunctionType::RejectFinally { on_finally, c } => {
                let reason = arguments_list.get(0).scope(agent, gc.nogc());
                let c = c.scope(agent, gc.nogc());
                // i. Let result be ? Call(onFinally, undefined).
                let result = call_function(
                    agent,
                    on_finally.unbind(),
                    Value::Undefined,
                    None,
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: not shared.
                let _c = unsafe { c.take(agent) }.bind(gc.nogc());
                // ii. Let p be ? PromiseResolve(C, result).
                let p = Promise::resolve(agent, result.unbind(), gc.reborrow())
                    .unbind()
                    .bind(gc.nogc());
                let reason = unsafe { reason.take(agent) }.bind(gc.nogc());
                // iii. Let throwReason be a new Abstract Closure with no parameters that captures reason and performs the following steps when called:
                // iv. Let thrower be CreateBuiltinFunction(throwReason, 0, "", « »).
                let thrower = agent.heap.create(PromiseFinallyFunctionHeapData {
                    backing_object: None,
                    // 1. Return ThrowCompletion(reason).
                    resolve_type: PromiseFinallyFunctionType::ThrowReason {
                        reason: JsError::new(reason),
                    },
                });
                // v. Return ? Invoke(p, "then", « thrower »).
                invoke(
                    agent,
                    p.unbind().into(),
                    BUILTIN_STRING_MEMORY.then.to_property_key(),
                    Some(ArgumentsList::from_mut_value(&mut thrower.unbind().into())),
                    gc,
                )
            }
            PromiseFinallyFunctionType::ReturnValue { value } => {
                // 1. Return NormalCompletion(value).
                Ok(value.unbind().bind(gc.into_nogc()))
            }
            PromiseFinallyFunctionType::ThrowReason { reason } => {
                // 1. Return ThrowCompletion(reason).
                Err(reason.unbind().bind(gc.into_nogc()))
            }
        }
    }
}

impl<'a> CreateHeapData<PromiseFinallyFunctionHeapData<'a>, BuiltinPromiseFinallyFunction<'a>>
    for Heap
{
    fn create(
        &mut self,
        data: PromiseFinallyFunctionHeapData<'a>,
    ) -> BuiltinPromiseFinallyFunction<'a> {
        self.promise_finally_functions.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PromiseFinallyFunctionHeapData<'static>>();

        BuiltinPromiseFinallyFunction(BaseIndex::last(&self.promise_finally_functions))
    }
}

impl HeapMarkAndSweep for BuiltinPromiseFinallyFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_finally_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .promise_finally_functions
            .shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for BuiltinPromiseFinallyFunction<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .promise_finally_functions
            .shift_weak_index(self.0)
            .map(Self)
    }
}

impl HeapMarkAndSweep for PromiseFinallyFunctionHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            backing_object,
            resolve_type,
        } = self;
        backing_object.mark_values(queues);
        resolve_type.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            backing_object,
            resolve_type,
        } = self;
        backing_object.sweep_values(compactions);
        resolve_type.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for PromiseFinallyFunctionType<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            PromiseFinallyFunctionType::ResolveFinally { on_finally, c }
            | PromiseFinallyFunctionType::RejectFinally { on_finally, c } => {
                on_finally.mark_values(queues);
                c.mark_values(queues);
            }
            PromiseFinallyFunctionType::ReturnValue { value } => value.mark_values(queues),
            PromiseFinallyFunctionType::ThrowReason { reason } => reason.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            PromiseFinallyFunctionType::ResolveFinally { on_finally, c }
            | PromiseFinallyFunctionType::RejectFinally { on_finally, c } => {
                on_finally.sweep_values(compactions);
                c.sweep_values(compactions);
            }
            PromiseFinallyFunctionType::ReturnValue { value } => value.sweep_values(compactions),
            PromiseFinallyFunctionType::ThrowReason { reason } => reason.sweep_values(compactions),
        }
    }
}
