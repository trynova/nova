// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    num::NonZeroU32,
    ops::{Index, IndexMut},
};

use super::{instructions::Instr, CompileContext, Instruction, NamedEvaluationParameter};
use crate::{
    ecmascript::{
        execution::Agent,
        scripts_and_modules::script::ScriptIdentifier,
        syntax_directed_operations::function_definitions::CompileFunctionBodyData,
        types::{String, Value},
    },
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};
use oxc_ast::ast::{self, Statement};

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
        Self::new(unsafe { std::mem::transmute::<&T, &'static T>(reference) })
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
pub(crate) struct FunctionExpression {
    pub(crate) expression: SendableRef<ast::Function<'static>>,
    pub(crate) identifier: Option<NamedEvaluationParameter>,
    /// Optionally eagerly compile the FunctionExpression into bytecode.
    pub(crate) compiled_bytecode: Option<Executable>,
}

#[derive(Debug, Clone)]
pub(crate) struct ArrowFunctionExpression {
    pub(crate) expression: SendableRef<ast::ArrowFunctionExpression<'static>>,
    pub(crate) identifier: Option<NamedEvaluationParameter>,
}

/// Reference to a heap-allocated executable VM bytecode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct Executable(NonZeroU32);

const EXECUTABLE_OPTION_SIZE_IS_U32: () =
    assert!(size_of::<Executable>() == size_of::<Option<Executable>>());

/// ## Notes
///
/// - This is inspired by and/or copied from Kiesel engine:
///   Copyright (c) 2023-2024 Linus Groh
#[derive(Debug, Clone)]
pub(crate) struct ExecutableHeapData {
    pub instructions: Box<[u8]>,
    pub(crate) constants: Box<[Value]>,
    pub(crate) function_expressions: Box<[FunctionExpression]>,
    pub(crate) arrow_function_expressions: Box<[ArrowFunctionExpression]>,
    pub(crate) class_initializer_bytecodes: Box<[(Option<Executable>, bool)]>,
}

impl Executable {
    pub(crate) fn compile_script(agent: &mut Agent, script: ScriptIdentifier) -> Self {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Script ===");
            eprintln!();
        }
        // SAFETY: Script uniquely owns the Program and the body buffer does
        // not move under any circumstances during heap operations.
        let body: &[Statement] =
            unsafe { std::mem::transmute(agent[script].ecmascript_code.body.as_slice()) };
        let mut ctx = CompileContext::new(agent);

        ctx.compile_statements(body);
        ctx.do_implicit_return();
        ctx.finish()
    }

    pub(crate) fn compile_function_body(
        agent: &mut Agent,
        data: CompileFunctionBodyData<'_>,
    ) -> Self {
        let mut ctx = CompileContext::new(agent);

        let is_concise = data.is_concise_body;

        ctx.compile_function_body(data);

        if is_concise {
            ctx.do_implicit_return();
        }

        ctx.finish()
    }

    pub(crate) fn compile_eval_body(agent: &mut Agent, body: &[Statement]) -> Self {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Eval Body ===");
            eprintln!();
        }
        let mut ctx = CompileContext::new(agent);

        ctx.compile_statements(body);
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
            let _ = agent.heap.executables.pop().unwrap();
        }
    }

    pub(crate) fn get_index(self) -> usize {
        (self.0.get() - 1) as usize
    }

    /// SAFETY: The returned reference is valid until the Executable is garbage
    /// collected.
    #[inline]
    pub(super) fn get_instructions(self, agent: &Agent) -> &'static [u8] {
        // SAFETY: As long as we're alive the instructions Box lives, and it is
        // never accessed mutably.
        unsafe { std::mem::transmute(&agent[self].instructions[..]) }
    }

    #[inline]
    pub(super) fn get_instruction(self, agent: &Agent, ip: &mut usize) -> Option<Instr> {
        // SAFETY: As long as we're alive the instructions Box lives, and it is
        // never accessed mutably.
        get_instruction(&agent[self].instructions[..], ip)
    }

    #[inline]
    pub(super) fn get_constants(self, agent: &Agent) -> &[Value] {
        &agent[self].constants[..]
    }

    #[inline]
    pub(super) fn fetch_identifier(self, agent: &Agent, index: usize) -> String {
        // SAFETY: As long as we're alive the constants Box lives. It is
        // accessed mutably only during GC, during which this function is never
        // called. As we do not hand out a reference here, the mutable
        // reference during GC and fetching references here never overlap.
        let value = agent[self].constants[index];
        let Ok(value) = String::try_from(value) else {
            handle_identifier_failure()
        };
        value
    }

    #[inline]
    pub(super) fn fetch_constant(self, agent: &Agent, index: usize) -> Value {
        // SAFETY: As long as we're alive the constants Box lives. It is
        // accessed mutably only during GC, during which this function is never
        // called. As we do not hand out a reference here, the mutable
        // reference during GC and fetching references here never overlap.
        agent[self].constants[index]
    }

    pub(super) fn fetch_function_expression(
        self,
        agent: &Agent,
        index: usize,
    ) -> &FunctionExpression {
        &agent[self].function_expressions[index]
    }

    pub(super) fn fetch_arrow_function_expression(
        self,
        agent: &Agent,
        index: usize,
    ) -> &ArrowFunctionExpression {
        &agent[self].arrow_function_expressions[index]
    }

    pub(super) fn fetch_class_initializer_bytecode(
        self,
        agent: &Agent,
        index: usize,
    ) -> (Option<Executable>, bool) {
        agent[self].class_initializer_bytecodes[index]
    }
}

pub(super) fn get_instruction(instructions: &[u8], ip: &mut usize) -> Option<Instr> {
    if *ip >= instructions.len() {
        return None;
    }

    let kind: Instruction = unsafe { std::mem::transmute::<u8, Instruction>(instructions[*ip]) };
    *ip += 1;

    let mut args: [Option<IndexType>; 2] = [None, None];

    for item in args.iter_mut().take(kind.argument_count() as usize) {
        let length = instructions[*ip..].len();
        if length >= 2 {
            let bytes = IndexType::from_ne_bytes(unsafe {
                *std::mem::transmute::<*const u8, *const [u8; 2]>(instructions[*ip..].as_ptr())
            });
            *ip += 2;
            *item = Some(bytes);
        } else {
            *ip += 1;
            *item = None;
        }
    }

    Some(Instr { kind, args })
}

impl ExecutableHeapData {
    #[inline]
    pub(super) fn get_instruction(&self, ip: &mut usize) -> Option<Instr> {
        get_instruction(&self.instructions, ip)
    }

    pub(crate) fn compile_script(agent: &mut Agent, script: ScriptIdentifier) -> Executable {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Script ===");
            eprintln!();
        }
        // SAFETY: Script uniquely owns the Program and the body buffer does
        // not move under any circumstances during heap operations.
        let body: &[Statement] =
            unsafe { std::mem::transmute(agent[script].ecmascript_code.body.as_slice()) };
        let mut ctx = CompileContext::new(agent);

        ctx.compile_statements(body);
        ctx.do_implicit_return();
        ctx.finish()
    }

    pub(crate) fn compile_function_body(
        agent: &mut Agent,
        data: CompileFunctionBodyData<'_>,
    ) -> Executable {
        let mut ctx = CompileContext::new(agent);

        let is_concise = data.is_concise_body;

        ctx.compile_function_body(data);

        if is_concise {
            ctx.do_implicit_return();
        }

        ctx.finish()
    }

    pub(crate) fn compile_eval_body(agent: &mut Agent, body: &[Statement]) -> Executable {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Eval Body ===");
            eprintln!();
        }
        let mut ctx = CompileContext::new(agent);

        ctx.compile_statements(body);
        ctx.do_implicit_return();
        ctx.finish()
    }
}

impl Index<Executable> for Agent {
    type Output = ExecutableHeapData;

    fn index(&self, index: Executable) -> &Self::Output {
        self.heap
            .executables
            .get(index.get_index())
            .expect("Executable out of bounds")
    }
}

impl IndexMut<Executable> for Agent {
    fn index_mut(&mut self, index: Executable) -> &mut Self::Output {
        self.heap
            .executables
            .get_mut(index.get_index())
            .expect("Executable out of bounds")
    }
}

impl CreateHeapData<ExecutableHeapData, Executable> for Heap {
    fn create(&mut self, data: ExecutableHeapData) -> Executable {
        self.executables.push(data);
        let index = u32::try_from(self.executables.len()).expect("Executables overflowed");
        // SAFETY: After pushing to executables, the vector cannot be empty.
        Executable(unsafe { NonZeroU32::new_unchecked(index) })
    }
}

impl HeapMarkAndSweep for Executable {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.executables.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .executables
            .shift_non_zero_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for ExecutableHeapData {
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
