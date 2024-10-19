// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! SourceCode is a Nova-engine specific concept to capture and keep any
//! `eval(source)` source strings alive after the eval call for the case where
//! that the eval call defines functions. Those functions will refer to the
//! SourceCode for their function source text.

use std::{fmt::Debug, ops::Index, ptr::NonNull};

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
use oxc_span::SourceType;

use crate::{
    ecmascript::{
        execution::Agent,
        types::{HeapString, String},
    },
    heap::{
        indexes::BaseIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

type SourceCodeIndex = BaseIndex<SourceCodeHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceCode(SourceCodeIndex);

impl SourceCode {
    /// Parses the given source string as JavaScript code and returns the
    /// parsed result and a SourceCode heap reference.
    ///
    /// ### Safety
    ///
    /// The caller must keep the SourceCode from being garbage collected until
    /// they drop the parsed code.
    pub(crate) unsafe fn parse_source(
        agent: &mut Agent,
        source: String,
        source_type: SourceType,
    ) -> Result<(Program<'static>, Self), Vec<OxcDiagnostic>> {
        // If the source code is not a heap string, pad it with whitespace and
        // allocate it on the heap. This makes it safe (for some definition of
        // "safe") for the any functions created referring to this source code to
        // keep references to the string buffer.
        let (source, source_text) = match source {
            String::String(source) => {
                // SAFETY: Caller guarantees to keep SourceCode from being
                // garbage collected until the parsed Program is dropped.
                // Thus the source text is kept from garbage collection.
                let source_text =
                    unsafe { std::mem::transmute::<&str, &'static str>(source.as_str(agent)) };
                (source, source_text)
            }
            String::SmallString(source) => {
                // Add 10 whitespace bytes to the end of the eval string. This
                // should guarantee that the string gets heap-allocated.
                let original_length = source.len();
                let data = format!("{}          ", source.as_str());
                let source = String::from_string(agent, data);
                let String::String(source) = source else {
                    unreachable!()
                };
                // SAFETY: Caller guarantees to keep SourceCode from being
                // garbage collected until the parsed Program is dropped.
                // Thus the source text is kept from garbage collection.
                let source_text =
                    unsafe { std::mem::transmute::<&str, &'static str>(source.as_str(agent)) };
                // Slice the source text back to the original length so that the
                // whitespace we added doesn't get fed to the parser: It shouldn't
                // need it.
                let source_text = &source_text[..original_length];
                (source, source_text)
            }
        };

        let mut allocator = NonNull::from(Box::leak(Box::default()));
        // SAFETY: Parser is dropped before allocator.
        let parser = Parser::new(unsafe { allocator.as_mut() }, source_text, source_type);

        let ParserReturn {
            errors, program, ..
        } = parser.parse();

        if !errors.is_empty() {
            // Drop program before dropping allocator.
            drop(program);
            // SAFETY: No references to allocator exist anymore. It is safe to
            // drop it.
            drop(unsafe { Box::from_raw(allocator.as_mut()) });
            // TODO: Include error messages in the exception.
            return Err(errors);
        }

        let SemanticBuilderReturn { errors, .. } = SemanticBuilder::new(source_text)
            .with_check_syntax_error(true)
            .build(&program);

        if !errors.is_empty() {
            // Drop program before dropping allocator.
            drop(program);
            // SAFETY: No references to allocator exist anymore. It is safe to
            // drop it.
            drop(unsafe { Box::from_raw(allocator.as_mut()) });
            // TODO: Include error messages in the exception.
            return Err(errors);
        }
        // SAFETY: Caller guarantees that they will drop the Program before
        // SourceCode can be garbage collected.
        let program = unsafe { std::mem::transmute::<Program<'_>, Program<'static>>(program) };
        let source_code = agent.heap.create(SourceCodeHeapData { source, allocator });

        Ok((program, source_code))
    }

    pub(crate) fn get_source_text(self, agent: &Agent) -> &str {
        agent[agent[self].source].as_str()
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }
}

pub(crate) struct SourceCodeHeapData {
    /// The source JavaScript string data the eval was called with. The string
    /// is known and required to be a HeapString because functions created
    /// in the eval call may keep references to the string data. If the eval
    /// string was small-string optimised and on the stack, then those
    /// references would necessarily and definitely be invalid.
    source: HeapString,
    /// The arena that contains the parsed data of the eval source.
    allocator: NonNull<Allocator>,
}

unsafe impl Send for SourceCodeHeapData {}

impl Debug for SourceCodeHeapData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceCodeHeapData")
            .field("source", &self.source)
            .field("allocator", &"[binary data]")
            .finish()
    }
}

impl Drop for SourceCodeHeapData {
    fn drop(&mut self) {
        // SAFETY: All references to this SourceCode should have been dropped
        // before we drop this.
        drop(unsafe { Box::from_raw(self.allocator.as_mut()) });
    }
}

impl Index<SourceCode> for Agent {
    type Output = SourceCodeHeapData;

    fn index(&self, index: SourceCode) -> &Self::Output {
        self.heap
            .source_codes
            .get(index.get_index())
            .expect("SourceCode out of bounds")
            .as_ref()
            .expect("SourceCode slot empty")
    }
}

impl CreateHeapData<SourceCodeHeapData, SourceCode> for Heap {
    fn create(&mut self, data: SourceCodeHeapData) -> SourceCode {
        self.source_codes.push(Some(data));
        SourceCode(SourceCodeIndex::last(&self.source_codes))
    }
}

impl HeapMarkAndSweep for SourceCodeHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            source,
            allocator: _,
        } = self;
        source.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            source,
            allocator: _,
        } = self;
        source.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for SourceCode {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.source_codes.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.source_codes.shift_index(&mut self.0);
    }
}
