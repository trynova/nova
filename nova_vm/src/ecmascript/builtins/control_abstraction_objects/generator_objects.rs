// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::create_iter_result_object,
        execution::{
            Agent, ExecutionContext, JsResult, ProtoIntrinsics,
            agent::{ExceptionType, JsError},
        },
        types::{InternalMethods, InternalSlots, IntoValue, Object, OrdinaryObject, Value},
    },
    engine::{
        Executable, ExecutionResult, SuspendedVm,
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, Scopable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues,
        indexes::{BaseIndex, GeneratorIndex},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generator<'a>(pub(crate) GeneratorIndex<'a>);

impl Generator<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// [27.5.3.3 GeneratorResume ( generator, value, generatorBrand )](https://tc39.es/ecma262/#sec-generatorresume)
    pub(crate) fn resume<'a>(
        self,
        agent: &mut Agent,
        value: Value,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        let value = value.bind(gc.nogc());
        let generator = self.bind(gc.nogc());
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        match agent[generator].generator_state.as_ref().unwrap() {
            GeneratorState::Executing => {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "The generator is currently running",
                    gc.into_nogc(),
                ));
            }
            GeneratorState::Completed => {
                // 2. If state is completed, return CreateIterResultObject(undefined, true).
                return Ok(create_iter_result_object(agent, Value::Undefined, true).into_value());
            }
            GeneratorState::SuspendedStart(_) | GeneratorState::SuspendedYield(_) => {
                // 3. Assert: state is either suspended-start or suspended-yield.
            }
        };

        // 7. Set generator.[[GeneratorState]] to executing.
        let SuspendedGeneratorState {
            vm,
            executable,
            execution_context,
        } = match agent[generator]
            .generator_state
            .replace(GeneratorState::Executing)
        {
            Some(GeneratorState::SuspendedYield(state))
            | Some(GeneratorState::SuspendedStart(state)) => state,
            _ => unreachable!(),
        };
        let executable = executable.scope(agent, gc.nogc());

        // 4. Let genContext be generator.[[GeneratorContext]].
        // 5. Let methodContext be the running execution context.
        // 6. Suspend methodContext.
        // 8. Push genContext onto the execution context stack; genContext is now the running
        // execution context.
        agent.push_execution_context(execution_context);

        let saved = generator.scope(agent, gc.nogc());

        // 9. Resume the suspended evaluation of genContext using NormalCompletion(value) as the
        // result of the operation that suspended it. Let result be the value returned by the
        // resumed computation.
        let execution_result = vm.resume(agent, executable.clone(), value.unbind(), gc.reborrow());

        let execution_result = execution_result.unbind();
        let gc = gc.into_nogc();
        let generator = saved.get(agent).bind(gc);
        let execution_result = execution_result.bind(gc);

        // GeneratorStart: 4.f. Remove acGenContext from the execution context stack and restore the
        // execution context that is at the top of the execution context stack as the running
        // execution context.
        // GeneratorYield 6 is the same.
        let execution_context = agent.pop_execution_context().unwrap();

        // 10. Assert: When we return here, genContext has already been removed
        // from the execution context stack and methodContext is the currently
        // running execution context.
        // 11. Return ? result.
        match execution_result {
            ExecutionResult::Return(result_value) => {
                // GeneratorStart step 4:
                // g. Set acGenerator.[[GeneratorState]] to completed.
                // h. NOTE: Once a generator enters the completed state it never leaves it and its
                // associated execution context is never resumed. Any execution state associated
                // with acGenerator can be discarded at this point.
                agent[generator].generator_state = Some(GeneratorState::Completed);
                // i. If result is a normal completion, then
                //    i. Let resultValue be undefined.
                // j. Else if result is a return completion, then
                //    i. Let resultValue be result.[[Value]].
                // l. Return CreateIterResultObject(resultValue, true).
                Ok(create_iter_result_object(agent, result_value, true).into_value())
            }
            ExecutionResult::Throw(err) => {
                // GeneratorStart step 4:
                // g. Set acGenerator.[[GeneratorState]] to completed.
                // h. NOTE: Once a generator enters the completed state it never leaves it and its
                // associated execution context is never resumed. Any execution state associated
                // with acGenerator can be discarded at this point.
                agent[generator].generator_state = Some(GeneratorState::Completed);
                // k. i. Assert: result is a throw completion.
                //    ii. Return ? result.
                Err(err.unbind())
            }
            ExecutionResult::Yield { vm, yielded_value } => {
                // Yield:
                // 3. Otherwise, return ? GeneratorYield(CreateIterResultObject(value, false)).
                // GeneratorYield:
                // 3. Let generator be the value of the Generator component of genContext.
                // 5. Set generator.[[GeneratorState]] to suspended-yield.
                agent[generator].generator_state =
                    Some(GeneratorState::SuspendedYield(SuspendedGeneratorState {
                        vm,
                        executable: executable.get(agent),
                        execution_context,
                    }));
                // 8. Resume callerContext passing NormalCompletion(iterNextObj). ...
                // NOTE: `callerContext` here is the `GeneratorResume` execution context.
                Ok(yielded_value)
            }
            ExecutionResult::Await { .. } => unreachable!(),
        }
    }

    /// [27.5.3.4 GeneratorResumeAbrupt ( generator, abruptCompletion, generatorBrand )](https://tc39.es/ecma262/#sec-generatorresumeabrupt)
    /// NOTE: This method only accepts throw completions.
    pub(crate) fn resume_throw<'a>(
        self,
        agent: &mut Agent,
        value: Value,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        let value = value.bind(gc.nogc());
        let generator = self.bind(gc.nogc());
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        match agent[generator].generator_state.as_ref().unwrap() {
            GeneratorState::SuspendedStart(_) => {
                // 2. If state is suspended-start, then
                // a. Set generator.[[GeneratorState]] to completed.
                // b. NOTE: Once a generator enters the completed state it never leaves it and its
                // associated execution context is never resumed. Any execution state associated
                // with generator can be discarded at this point.
                agent[generator].generator_state = Some(GeneratorState::Completed);
                // c. Set state to completed.

                // 3. If state is completed, then
                // b. Return ? abruptCompletion.
                return Err(JsError::new(value.unbind()));
            }
            GeneratorState::SuspendedYield(_) => {
                // 4. Assert: state is suspended-yield.
            }
            GeneratorState::Executing => {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "The generator is currently running",
                    gc.into_nogc(),
                ));
            }
            GeneratorState::Completed => {
                // 3. If state is completed, then
                //    b. Return ? abruptCompletion.
                return Err(JsError::new(value.unbind()));
            }
        };

        // 8. Set generator.[[GeneratorState]] to executing.
        let Some(GeneratorState::SuspendedYield(SuspendedGeneratorState {
            vm,
            executable,
            execution_context,
        })) = agent[generator]
            .generator_state
            .replace(GeneratorState::Executing)
        else {
            unreachable!()
        };
        let generator = generator.scope(agent, gc.nogc());
        let executable = executable.scope(agent, gc.nogc());

        // 5. Let genContext be generator.[[GeneratorContext]].
        // 6. Let methodContext be the running execution context.
        // 7. Suspend methodContext.
        // 9. Push genContext onto the execution context stack; genContext is now the running
        // execution context.
        agent.push_execution_context(execution_context);

        // 10. Resume the suspended evaluation of genContext using NormalCompletion(value) as the
        // result of the operation that suspended it. Let result be the value returned by the
        // resumed computation.
        let execution_result = vm
            .resume_throw(agent, executable.clone(), value.unbind(), gc.reborrow())
            .unbind();
        let gc = gc.into_nogc();
        let execution_result = execution_result.bind(gc);
        // SAFETY: shared but not stored by resume.
        let executable = unsafe { executable.take(agent).bind(gc) };
        // SAFETY: not shared.
        let generator = unsafe { generator.take(agent).bind(gc) };

        // GeneratorStart: 4.f. Remove acGenContext from the execution context stack and restore the
        // execution context that is at the top of the execution context stack as the running
        // execution context.
        // GeneratorYield 6 is the same.
        let execution_context = agent.pop_execution_context().unwrap();

        // 11. Assert: When we return here, genContext has already been removed
        // from the execution context stack and methodContext is the currently
        // running execution context.
        // 12. Return ? result.
        match execution_result {
            ExecutionResult::Return(result) => {
                agent[generator].generator_state = Some(GeneratorState::Completed);
                Ok(create_iter_result_object(agent, result.unbind(), true).into_value())
            }
            ExecutionResult::Throw(err) => {
                agent[generator].generator_state = Some(GeneratorState::Completed);
                Err(err)
            }
            ExecutionResult::Yield { vm, yielded_value } => {
                agent[generator].generator_state =
                    Some(GeneratorState::SuspendedYield(SuspendedGeneratorState {
                        vm,
                        executable: executable.unbind(),
                        execution_context,
                    }));
                Ok(yielded_value.unbind())
            }
            ExecutionResult::Await { .. } => unreachable!(),
        }
    }

    /// [27.5.3.4 GeneratorResumeAbrupt ( generator, abruptCompletion, generatorBrand )](https://tc39.es/ecma262/#sec-generatorresumeabrupt)
    /// NOTE: This method only accepts return completions.
    pub(crate) fn resume_return<'a>(
        self,
        agent: &mut Agent,
        abrupt_completion: Value,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        let abrupt_completion = abrupt_completion.bind(gc.nogc());
        let generator = self.bind(gc.nogc());
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        match agent[generator].generator_state.as_ref().unwrap() {
            GeneratorState::SuspendedStart(_) => {
                // 2. If state is suspended-start, then
                // a. Set generator.[[GeneratorState]] to completed.
                // b. NOTE: Once a generator enters the completed state it never leaves it and its
                // associated execution context is never resumed. Any execution state associated
                // with generator can be discarded at this point.
                agent[generator].generator_state = Some(GeneratorState::Completed);
                // c. Set state to completed.

                // 3. If abruptCompletion is a return completion, then
                // i. Return CreateIteratorResultObject(abruptCompletion.[[Value]], true).
                return Ok(
                    create_iter_result_object(agent, abrupt_completion.unbind(), true).into_value(),
                );
            }
            GeneratorState::SuspendedYield(_) => {
                // 4. Assert: state is suspended-yield.
            }
            GeneratorState::Executing => {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "The generator is currently running",
                    gc.into_nogc(),
                ));
            }
            GeneratorState::Completed => {
                // 3. If abruptCompletion is a return completion, then
                // i. Return CreateIteratorResultObject(abruptCompletion.[[Value]], true).
                return Ok(
                    create_iter_result_object(agent, abrupt_completion.unbind(), true).into_value(),
                );
            }
        };

        // 4. Assert: state is suspended-yield.
        // 8. Set generator.[[GeneratorState]] to executing.
        let Some(GeneratorState::SuspendedYield(SuspendedGeneratorState {
            vm,
            executable,
            execution_context,
        })) = agent[generator]
            .generator_state
            .replace(GeneratorState::Executing)
        else {
            unreachable!()
        };
        let generator = generator.scope(agent, gc.nogc());
        let executable = executable.scope(agent, gc.nogc());

        // 5. Let genContext be generator.[[GeneratorContext]].
        // 6. Let methodContext be the running execution context.
        // 7. Suspend methodContext.
        // 9. Push genContext onto the execution context stack; genContext is now the running
        // execution context.
        agent.push_execution_context(execution_context);

        // 10. Resume the suspended evaluation of genContext using
        //     abruptCompletion as the result of the operation that suspended
        //     it. Let result be the Completion Record returned by the resumed
        //     computation.
        let execution_result = vm
            .resume_return(
                agent,
                executable.clone(),
                abrupt_completion.unbind(),
                gc.reborrow(),
            )
            .unbind();
        let gc = gc.into_nogc();
        let execution_result = execution_result.bind(gc);
        // SAFETY: shared but not stored by resume.
        let executable = unsafe { executable.take(agent).bind(gc) };
        // SAFETY: not shared.
        let generator = unsafe { generator.take(agent).bind(gc) };

        // GeneratorStart: 4.f. Remove acGenContext from the execution context stack and restore the
        // execution context that is at the top of the execution context stack as the running
        // execution context.
        // GeneratorYield 6 is the same.
        let execution_context = agent.pop_execution_context().unwrap();

        // 11. Assert: When we return here, genContext has already been removed
        // from the execution context stack and methodContext is the currently
        // running execution context.
        // 12. Return ? result.
        match execution_result {
            ExecutionResult::Return(result) => {
                agent[generator].generator_state = Some(GeneratorState::Completed);
                Ok(create_iter_result_object(agent, result, true).into_value())
            }
            ExecutionResult::Throw(err) => {
                agent[generator].generator_state = Some(GeneratorState::Completed);
                Err(err)
            }
            ExecutionResult::Yield { vm, yielded_value } => {
                agent[generator].generator_state =
                    Some(GeneratorState::SuspendedYield(SuspendedGeneratorState {
                        vm,
                        executable: executable.unbind(),
                        execution_context,
                    }));
                Ok(yielded_value)
            }
            ExecutionResult::Await { .. } => unreachable!(),
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Generator<'_> {
    type Of<'a> = Generator<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<Generator<'a>> for Value<'a> {
    fn from(value: Generator<'a>) -> Self {
        Value::Generator(value)
    }
}

impl<'a> From<Generator<'a>> for Object<'a> {
    fn from(value: Generator) -> Self {
        Object::Generator(value.unbind())
    }
}

impl<'a> TryFrom<Value<'a>> for Generator<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        if let Value::Generator(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> InternalSlots<'a> for Generator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Generator;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for Generator<'a> {}

impl<'a> CreateHeapData<GeneratorHeapData<'a>, Generator<'a>> for Heap {
    fn create(&mut self, data: GeneratorHeapData<'a>) -> Generator<'a> {
        self.generators.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<GeneratorHeapData<'static>>>();
        Generator(GeneratorIndex::last(&self.generators))
    }
}

impl Index<Generator<'_>> for Agent {
    type Output = GeneratorHeapData<'static>;

    fn index(&self, index: Generator) -> &Self::Output {
        &self.heap.generators[index]
    }
}

impl IndexMut<Generator<'_>> for Agent {
    fn index_mut(&mut self, index: Generator) -> &mut Self::Output {
        &mut self.heap.generators[index]
    }
}

impl Index<Generator<'_>> for Vec<Option<GeneratorHeapData<'static>>> {
    type Output = GeneratorHeapData<'static>;

    fn index(&self, index: Generator) -> &Self::Output {
        self.get(index.get_index())
            .expect("Generator out of bounds")
            .as_ref()
            .expect("Generator slot empty")
    }
}

impl IndexMut<Generator<'_>> for Vec<Option<GeneratorHeapData<'static>>> {
    fn index_mut(&mut self, index: Generator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Generator out of bounds")
            .as_mut()
            .expect("Generator slot empty")
    }
}

impl TryFrom<HeapRootData> for Generator<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::Generator(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl HeapMarkAndSweep for Generator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.generators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.generators.shift_index(&mut self.0)
    }
}

impl HeapSweepWeakReference for Generator<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.generators.shift_weak_index(self.0).map(Self)
    }
}

#[derive(Debug, Default)]
pub struct GeneratorHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) generator_state: Option<GeneratorState>,
}

#[derive(Debug)]
pub(crate) struct SuspendedGeneratorState {
    pub(crate) vm: SuspendedVm,
    pub(crate) executable: Executable<'static>,
    pub(crate) execution_context: ExecutionContext,
}

#[derive(Debug)]
pub(crate) enum GeneratorState {
    SuspendedStart(SuspendedGeneratorState),
    SuspendedYield(SuspendedGeneratorState),
    Executing,
    Completed,
}

impl HeapMarkAndSweep for SuspendedGeneratorState {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            vm,
            executable,
            execution_context,
        } = self;
        vm.mark_values(queues);
        executable.mark_values(queues);
        execution_context.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            vm,
            executable,
            execution_context,
        } = self;
        vm.sweep_values(compactions);
        executable.sweep_values(compactions);
        execution_context.sweep_values(compactions);
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for GeneratorHeapData<'_> {
    type Of<'a> = GeneratorHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for GeneratorHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            generator_state,
        } = self;
        object_index.mark_values(queues);
        generator_state.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            generator_state,
        } = self;
        object_index.sweep_values(compactions);
        generator_state.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for GeneratorState {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            GeneratorState::SuspendedStart(s) | GeneratorState::SuspendedYield(s) => {
                s.mark_values(queues)
            }
            GeneratorState::Executing | GeneratorState::Completed => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            GeneratorState::SuspendedStart(s) | GeneratorState::SuspendedYield(s) => {
                s.sweep_values(compactions)
            }
            GeneratorState::Executing | GeneratorState::Completed => {}
        }
    }
}
