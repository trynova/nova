// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ptr::NonNull;

use crate::{
    ecmascript::types::{String, Value},
    heap::HeapMarkAndSweep,
};

use super::{
    executable::{get_instruction, ArrowFunctionExpression},
    instructions::Instr,
    Executable, FunctionExpression,
};

/// Abstracts over a heap-allocated bytecode structure that can be safely
/// run interleaved with garbage collection
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct HeapAllocatedBytecode {
    pointer: NonNull<Executable>,
}

impl HeapAllocatedBytecode {
    pub(crate) fn new(bytecode: Executable) -> Self {
        Self {
            // SAFETY: Box::leak(Box::new()) never returns a null pointer for
            // non-ZST.
            pointer: unsafe { NonNull::new_unchecked(Box::leak(Box::new(bytecode))) },
        }
    }

    pub(crate) fn clone(&self) -> Self {
        Self::new(unsafe { self.pointer.as_ref() }.clone())
    }

    /// SAFETY: The returned reference is valid until the Script, Module, or
    /// Function it was compiled from is garbage collected.
    #[inline]
    pub(super) fn get_instructions(self) -> &'static [u8] {
        // SAFETY: As long as we're alive the instructions Box lives, and it is
        // never accessed mutably.
        unsafe { &(*self.pointer.as_ptr()).instructions }
    }

    /// SAFETY: The returned reference is valid until the next garbage
    /// collection happens.
    #[inline]
    pub(super) fn get_constants(self) -> &'static [Value] {
        // SAFETY: As long as we're alive the instructions Box lives, and it is
        // never accessed mutably.
        unsafe { &(*self.pointer.as_ptr()).constants }
    }

    #[inline]
    pub(super) fn get_instruction(self, ip: &mut usize) -> Option<Instr> {
        // SAFETY: As long as we're alive the instructions Box lives, and it is
        // never accessed mutably.
        get_instruction(unsafe { &(*self.pointer.as_ptr()).instructions }, ip)
    }

    #[inline]
    pub(super) fn fetch_identifier(self, index: usize) -> String {
        // SAFETY: As long as we're alive the constants Box lives. It is
        // accessed mutably only during GC, during which this function is never
        // called. As we do not hand out a reference here, the mutable
        // reference during GC and fetching references here never overlap.
        let value = unsafe { (*self.pointer.as_ptr()).constants[index] };
        let Ok(value) = String::try_from(value) else {
            handle_identifier_failure()
        };
        value
    }

    #[inline]
    pub(super) fn fetch_constant(self, index: usize) -> Value {
        // SAFETY: As long as we're alive the constants Box lives. It is
        // accessed mutably only during GC, during which this function is never
        // called. As we do not hand out a reference here, the mutable
        // reference during GC and fetching references here never overlap.
        unsafe { (*self.pointer.as_ptr()).constants[index] }
    }

    /// SAFETY: The returned reference is valid the next garbage collection
    /// happens.
    pub(super) fn fetch_function_expression(self, index: usize) -> &'static FunctionExpression {
        unsafe { &(*self.pointer.as_ptr()).function_expressions[index] }
    }

    /// SAFETY: The returned reference is valid the next garbage collection
    /// happens.
    pub(super) fn fetch_arrow_function_expression(
        self,
        index: usize,
    ) -> &'static ArrowFunctionExpression {
        unsafe { &(*self.pointer.as_ptr()).arrow_function_expressions[index] }
    }

    /// SAFETY: The returned reference is valid the next garbage collection
    /// happens.
    pub(super) fn fetch_class_initializer_bytecode(
        self,
        index: usize,
    ) -> &'static (Option<Executable>, bool) {
        unsafe { &(*self.pointer.as_ptr()).class_initializer_bytecodes[index] }
    }

    /// SAFETY: Normal drop safety rules apply.
    pub(crate) unsafe fn drop(self) {
        drop(unsafe { Box::from_raw(self.pointer.as_ptr()) });
    }
}

#[cold]
fn handle_identifier_failure() -> ! {
    panic!("Invalid identifier index: Value was not a String")
}

impl HeapMarkAndSweep for HeapAllocatedBytecode {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        // SAFETY: This is a valid, non-null pointer to an owned Executable
        // that cannot have any live mutable references to it.
        unsafe { self.pointer.as_ref() }.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        // SAFETY: This is a valid, non-null pointer to an owned Executable
        // that cannot have any live references to it.
        // References to this Executable are only created above for marking
        // and for running the bytecode. Both of the
        // references only live for the duration of a synchronous call and
        // no longer. Sweeping cannot run concurrently with marking or with
        // ECMAScript code execution. Hence we can be sure that this is not
        // an aliasing violation.
        unsafe { self.pointer.as_mut() }.sweep_values(compactions);
    }
}

unsafe impl Send for HeapAllocatedBytecode {}
