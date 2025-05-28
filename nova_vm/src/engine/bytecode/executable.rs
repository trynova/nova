// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{
    num::NonZeroU32,
    ops::{Index, IndexMut},
};
use std::marker::PhantomData;

use crate::{
    ecmascript::{
        execution::Agent,
        scripts_and_modules::script::Script,
        syntax_directed_operations::function_definitions::CompileFunctionBodyData,
        types::{String, Value},
    },
    engine::{
        Scoped,
        bytecode::{
            CompileContext, CompileEvaluation, NamedEvaluationParameter, instructions::Instr,
        },
        context::{Bindable, GcToken, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};
use oxc_ast::ast::{self, Program, Statement};

#[derive(Debug)]
/// A `Send` and `Sync` wrapper over a `&'static T` where `T` might not itself
/// be `Sync`. This is safe because the reference can only be obtained from the
/// same thread in which the `SendableRef` was created.
pub(crate) struct SendableRef<T: ?Sized + 'static> {
    reference: &'static T,
    thread_id: std::thread::ThreadId,
}

impl<T: ?Sized + 'static> Clone for SendableRef<T> {
    fn clone(&self) -> Self {
        Self {
            reference: self.reference,
            thread_id: self.thread_id,
        }
    }
}

impl<T: ?Sized> SendableRef<T> {
    /// Creates a new [`SendableRef`] from a reference with a static lifetime.
    pub(crate) fn new(reference: &'static T) -> Self {
        Self {
            reference,
            thread_id: std::thread::current().id(),
        }
    }

    /// Unsafely creates a new [`SendableRef`] from a non-static reference.
    ///
    /// # Safety
    ///
    /// The safety conditions for this constructor are the same as for
    /// transmuting `reference` into a static lifetime.
    pub(crate) unsafe fn new_as_static(reference: &T) -> Self {
        Self::new(unsafe { core::mem::transmute::<&T, &'static T>(reference) })
    }

    pub(crate) fn get(&self) -> &'static T {
        assert_eq!(std::thread::current().id(), self.thread_id);
        self.reference
    }
}

// SAFETY: The reference will only be dereferenced in a thread in which the
// reference is valid, so it's fine to send or use this type from other threads.
unsafe impl<T: ?Sized> Send for SendableRef<T> {}
unsafe impl<T: ?Sized> Sync for SendableRef<T> {}

pub type IndexType = u16;

#[derive(Debug, Clone)]
pub(crate) struct FunctionExpression<'a> {
    pub(crate) expression: SendableRef<ast::Function<'static>>,
    pub(crate) identifier: Option<NamedEvaluationParameter>,
    /// Optionally eagerly compile the FunctionExpression into bytecode.
    pub(crate) compiled_bytecode: Option<Executable<'a>>,
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for FunctionExpression<'_> {
    type Of<'a> = FunctionExpression<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ArrowFunctionExpression {
    pub(crate) expression: SendableRef<ast::ArrowFunctionExpression<'static>>,
    pub(crate) identifier: Option<NamedEvaluationParameter>,
}

/// Reference to a heap-allocated executable VM bytecode.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Executable<'a>(
    NonZeroU32,
    PhantomData<ExecutableHeapData<'static>>,
    PhantomData<&'a GcToken>,
);

impl core::fmt::Debug for Executable<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Executable({:?})", self.get_index())
    }
}

const EXECUTABLE_OPTION_SIZE_IS_U32: () =
    assert!(size_of::<Executable<'_>>() == size_of::<Option<Executable<'_>>>());

/// ## Notes
///
/// - This is inspired by and/or copied from Kiesel engine:
///   Copyright (c) 2023-2024 Linus Groh
#[derive(Debug, Clone)]
pub struct ExecutableHeapData<'a> {
    pub(crate) instructions: Box<[u8]>,
    pub(crate) constants: Box<[Value<'a>]>,
    pub(crate) function_expressions: Box<[FunctionExpression<'a>]>,
    pub(crate) arrow_function_expressions: Box<[ArrowFunctionExpression]>,
    pub(crate) class_initializer_bytecodes: Box<[(Option<Executable<'a>>, bool)]>,
}

impl<'gc> Executable<'gc> {
    pub(crate) fn compile_script(
        agent: &mut Agent,
        script: Script,
        gc: NoGcScope<'gc, '_>,
    ) -> Self {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Script ===");
            eprintln!();
        }
        // SAFETY: Script uniquely owns the Program and the body buffer does
        // not move under any circumstances during heap operations.
        let body: &[Statement] =
            unsafe { core::mem::transmute(agent[script].ecmascript_code.body.as_slice()) };
        let mut ctx = CompileContext::new(agent, gc);

        ctx.compile_statements(body);
        ctx.do_implicit_return();
        ctx.finish()
    }

    pub(crate) fn compile_function_body(
        agent: &mut Agent,
        data: CompileFunctionBodyData<'_>,
        gc: NoGcScope<'gc, '_>,
    ) -> Self {
        let mut ctx = CompileContext::new(agent, gc);
        if data.is_async {
            ctx.set_async();
        }

        let is_concise = data.is_concise_body;

        ctx.compile_function_body(data);

        if is_concise {
            ctx.do_implicit_return();
        }

        ctx.finish()
    }

    pub(crate) fn compile_eval_body(
        agent: &mut Agent,
        program: &Program,
        gc: NoGcScope<'gc, '_>,
    ) -> Self {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Eval Body ===");
            eprintln!();
        }
        let mut ctx = CompileContext::new(agent, gc);

        // eval('"asd"') is parsed into an empty body with a single directive.
        // Multiple directives are also possible, but only the last one is
        // really relevant to us as storing constants cannot be observed.
        if program.body.is_empty() {
            if let Some(directive) = program.directives.last() {
                directive.expression.compile(&mut ctx);
            }
        } else {
            ctx.compile_statements(&program.body);
        }
        ctx.do_implicit_return();
        ctx.finish()
    }

    /// Drops the Executable's heap-allocated data if possible.
    ///
    /// ## Safety
    ///
    /// Any attempt to use the Executable after this call will lead to a crash
    /// if the drop was performed.
    pub(crate) unsafe fn try_drop(self, agent: &mut Agent) {
        debug_assert!(!agent.heap.executables.is_empty());
        let index = self.get_index();
        let last_index = agent.heap.executables.len() - 1;
        if last_index == index {
            // This bytecode was the last-allocated bytecode, and we can drop
            // it from the Heap without affecting any other indexes. The caller
            // guarantees that the Executable will not be used anymore.
            agent.heap.alloc_counter = agent
                .heap
                .alloc_counter
                .saturating_sub(core::mem::size_of::<ExecutableHeapData>());
            let _ = agent.heap.executables.pop().unwrap();
        }
    }

    pub(crate) fn get_index(self) -> usize {
        (self.0.get() - 1) as usize
    }

    /// SAFETY: The returned reference is valid until the Executable is garbage
    /// collected.
    #[inline]
    fn get_instructions(self, agent: &Agent) -> &'static [u8] {
        // SAFETY: As long as we're alive the instructions Box lives, and it is
        // never accessed mutably.
        unsafe { core::mem::transmute(&agent[self].instructions[..]) }
    }

    #[inline]
    fn get_instruction(self, agent: &Agent, ip: &mut usize) -> Option<Instr> {
        Instr::consume_instruction(&agent[self].instructions, ip)
    }

    #[inline]
    fn get_constants<'a>(self, agent: &'a Agent, _: NoGcScope<'gc, '_>) -> &'a [Value<'gc>] {
        &agent[self].constants[..]
    }

    #[inline]
    fn fetch_identifier(self, agent: &Agent, index: usize, gc: NoGcScope<'gc, '_>) -> String<'gc> {
        // SAFETY: As long as we're alive the constants Box lives. It is
        // accessed mutably only during GC, during which this function is never
        // called. As we do not hand out a reference here, the mutable
        // reference during GC and fetching references here never overlap.
        let value = agent[self].constants[index];
        let Ok(value) = String::try_from(value) else {
            handle_identifier_failure()
        };
        value.bind(gc)
    }

    #[inline]
    fn fetch_constant(self, agent: &Agent, index: usize, gc: NoGcScope<'gc, '_>) -> Value<'gc> {
        // SAFETY: As long as we're alive the constants Box lives. It is
        // accessed mutably only during GC, during which this function is never
        // called. As we do not hand out a reference here, the mutable
        // reference during GC and fetching references here never overlap.
        agent[self].constants[index].bind(gc)
    }

    fn fetch_function_expression<'a>(
        self,
        agent: &'a Agent,
        index: usize,
        _: NoGcScope<'gc, '_>,
    ) -> &'a FunctionExpression<'gc> {
        &agent[self].function_expressions[index]
    }

    fn fetch_arrow_function_expression(
        self,
        agent: &Agent,
        index: usize,
    ) -> &ArrowFunctionExpression {
        &agent[self].arrow_function_expressions[index]
    }

    fn fetch_class_initializer_bytecode(
        self,
        agent: &Agent,
        index: usize,
        _: NoGcScope<'gc, '_>,
    ) -> (Option<Executable<'gc>>, bool) {
        agent[self].class_initializer_bytecodes[index]
    }
}

impl Scoped<'_, Executable<'static>> {
    #[inline]
    pub(super) fn get_instructions(&self, agent: &Agent) -> &[u8] {
        // SAFETY: Executable is scoped, the instructions reference is bound to
        // the Scoped.
        self.get(agent).get_instructions(agent)
    }

    #[inline]
    pub(super) fn get_instruction(&self, agent: &Agent, ip: &mut usize) -> Option<Instr> {
        self.get(agent).get_instruction(agent, ip)
    }

    #[inline]
    pub(super) fn get_constants<'a, 'gc>(
        &self,
        agent: &'a Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> &'a [Value<'gc>] {
        self.get(agent).get_constants(agent, gc)
    }

    #[inline]
    pub(super) fn fetch_identifier<'gc>(
        &self,
        agent: &Agent,
        index: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> String<'gc> {
        self.get(agent).fetch_identifier(agent, index, gc)
    }

    #[inline]
    pub(super) fn fetch_constant<'gc>(
        &self,
        agent: &Agent,
        index: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> Value<'gc> {
        self.get(agent).fetch_constant(agent, index, gc)
    }

    #[inline]
    pub(super) fn fetch_function_expression<'a, 'gc>(
        &self,
        agent: &'a Agent,
        index: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> &'a FunctionExpression<'gc> {
        self.get(agent).fetch_function_expression(agent, index, gc)
    }

    #[inline]
    pub(super) fn fetch_arrow_function_expression<'a>(
        &self,
        agent: &'a Agent,
        index: usize,
    ) -> &'a ArrowFunctionExpression {
        self.get(agent)
            .fetch_arrow_function_expression(agent, index)
    }

    #[inline]
    pub(super) fn fetch_class_initializer_bytecode<'gc>(
        &self,
        agent: &Agent,
        index: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> (Option<Executable<'gc>>, bool) {
        self.get(agent)
            .fetch_class_initializer_bytecode(agent, index, gc)
    }
}

impl Index<Executable<'_>> for Agent {
    type Output = ExecutableHeapData<'static>;

    fn index(&self, index: Executable) -> &Self::Output {
        self.heap
            .executables
            .get(index.get_index())
            .expect("Executable out of bounds")
    }
}

impl IndexMut<Executable<'_>> for Agent {
    fn index_mut(&mut self, index: Executable) -> &mut Self::Output {
        self.heap
            .executables
            .get_mut(index.get_index())
            .expect("Executable out of bounds")
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Executable<'_> {
    type Of<'a> = Executable<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl Rootable for Executable<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Executable(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Executable(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<ExecutableHeapData<'a>, Executable<'a>> for Heap {
    fn create(&mut self, data: ExecutableHeapData<'a>) -> Executable<'a> {
        self.executables.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<Option<ExecutableHeapData<'static>>>();
        let index = u32::try_from(self.executables.len()).expect("Executables overflowed");
        // SAFETY: After pushing to executables, the vector cannot be empty.
        Executable(
            unsafe { NonZeroU32::new_unchecked(index) },
            PhantomData,
            PhantomData,
        )
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ExecutableHeapData<'_> {
    type Of<'a> = ExecutableHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for Executable<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.executables.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .executables
            .shift_non_zero_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for ExecutableHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            instructions: _,
            constants,
            function_expressions: _,
            arrow_function_expressions: _,
            class_initializer_bytecodes,
        } = self;
        constants.mark_values(queues);
        for ele in class_initializer_bytecodes {
            ele.0.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            instructions: _,
            constants,
            function_expressions: _,
            arrow_function_expressions: _,
            class_initializer_bytecodes,
        } = self;
        constants.sweep_values(compactions);
        for ele in class_initializer_bytecodes {
            ele.0.sweep_values(compactions);
        }
    }
}

#[cold]
fn handle_identifier_failure() -> ! {
    panic!("Invalid identifier index: Value was not a String")
}
