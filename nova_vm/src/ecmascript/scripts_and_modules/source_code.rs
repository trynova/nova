// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! SourceCode is a Nova-engine specific concept to capture and keep any
//! `eval(source)` source strings alive after the eval call for the case where
//! that the eval call defines functions. Those functions will refer to the
//! SourceCode for their function source text.

use core::{fmt::Debug, ops::Index, ptr::NonNull};
use std::ops::IndexMut;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::{Semantic, SemanticBuilder, SemanticBuilderReturn};
use oxc_span::SourceType;

use crate::{
    ecmascript::{
        execution::Agent,
        types::{HeapString, String},
    },
    engine::{
        context::{Bindable, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues, indexes::BaseIndex,
    },
};

type SourceCodeIndex<'a> = BaseIndex<'a, SourceCodeHeapData<'static>>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceCode<'a>(SourceCodeIndex<'a>);

impl core::fmt::Debug for SourceCode<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SourceCode({:?})", self.0.into_u32_index())
    }
}

impl<'a> SourceCode<'a> {
    /// Parses the given source string as JavaScript code and returns a
    /// SourceCode heap reference.
    pub(crate) fn parse_source(
        agent: &mut Agent,
        source: String,
        source_type: SourceType,
        gc: NoGcScope<'a, '_>,
    ) -> Result<Self, Vec<OxcDiagnostic>> {
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
                    unsafe { core::mem::transmute::<&str, &'static str>(source.as_str(agent)) };
                (source.unbind(), source_text)
            }
            String::SmallString(source) => {
                // Add 10 whitespace bytes to the end of the eval string. This
                // should guarantee that the string gets heap-allocated.
                let original_length = source.len();
                let data = format!("{}          ", source.as_str());
                let source = String::from_string(agent, data, gc);
                let String::String(source) = source else {
                    unreachable!()
                };
                // SAFETY: Caller guarantees to keep SourceCode from being
                // garbage collected until the parsed Program is dropped.
                // Thus the source text is kept from garbage collection.
                let source_text =
                    unsafe { core::mem::transmute::<&str, &'static str>(source.as_str(agent)) };
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
            #[allow(clippy::drop_non_drop)]
            drop(program);
            // SAFETY: No references to allocator exist anymore. It is safe to
            // drop it.
            drop(unsafe { Box::from_raw(allocator.as_mut()) });
            // TODO: Include error messages in the exception.
            return Err(errors);
        }

        // SAFETY: We promise to drop the Allocator only after Program has been
        // dropped, so the Program can consider its internal references as
        // 'static.
        let mut program = unsafe {
            core::mem::transmute::<NonNull<Program>, NonNull<Program<'static>>>(NonNull::from(
                Box::leak(Box::new(program)),
            ))
        };

        let SemanticBuilderReturn { errors, semantic } = SemanticBuilder::new()
            .with_check_syntax_error(true)
            // SAFETY: program is alive and well right now, and we promise to
            // drop semantic before program.
            .build(unsafe { program.as_ref() });

        if !errors.is_empty() {
            // Drop semantic & program before dropping allocator.
            #[allow(clippy::drop_non_drop)]
            drop(semantic);
            #[allow(clippy::drop_non_drop)]
            // SAFETY: No references to program exist anymore. It is safe to
            // drop.
            drop(unsafe { Box::from_raw(program.as_mut()) });
            // SAFETY: No references to allocator exist anymore. It is safe to
            // drop.
            drop(unsafe { Box::from_raw(allocator.as_mut()) });
            // TODO: Include error messages in the exception.
            return Err(errors);
        }
        // SAFETY: We promise to drop the Semantic before Program and
        // Allocator, so the Semantic can consider its internal references as
        // 'static.
        let semantic = unsafe {
            core::mem::transmute::<NonNull<Semantic>, NonNull<Semantic<'static>>>(NonNull::from(
                Box::leak(Box::new(semantic)),
            ))
        };
        let source_code = agent.heap.create(SourceCodeHeapData::new(
            source, program, semantic, allocator,
        ));

        Ok(source_code)
    }

    pub(crate) fn get_source_text(self, agent: &Agent) -> &str {
        agent[agent[self].source].as_str()
    }

    /// Get a reference to the program AST of this SourceCode.
    ///
    /// ## Safety
    ///
    /// The program AST is valid until the SourceCode is garbage collected.
    pub(crate) fn get_program(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> &'a Program<'a> {
        agent[self].get_program(gc)
    }

    /// Get a non-null pointer to the program AST of this SourceCode.
    ///
    /// ## Safety
    ///
    /// The program AST pointer is valid until the SourceCode is garbage
    /// collected.
    pub(crate) fn get_program_pointer(self, agent: &Agent) -> NonNull<Program<'static>> {
        agent[self].program
    }

    /// Get a reference to the semantic analysis results of this SourceCode.
    ///
    /// ## Safety
    ///
    /// The semantic analysis results are valid until the SourceCode is
    /// garbage collected.
    pub(crate) fn get_semantic(
        self,
        agent: &Agent,
        gc: NoGcScope<'a, '_>,
    ) -> &'a Semantic<'static> {
        agent[self].get_semantic(gc)
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }
}

pub struct SourceCodeHeapData<'a> {
    /// The source JavaScript string data the eval was called with. The string
    /// is known and required to be a HeapString because functions created
    /// in the eval call may keep references to the string data. If the eval
    /// string was small-string optimised and on the stack, then those
    /// references would necessarily and definitely be invalid.
    source: HeapString<'a>,
    /// The semantic analysis results of the source code.
    ///
    /// ## Safety
    ///
    /// The semantic analysis results contains self-referential pointers into
    /// the program and allocator fields. It must be dropped before the others.
    semantic: NonNull<Semantic<'static>>,
    /// The parsed AST of the source code.
    ///
    /// ## Safety
    ///
    /// The program contains self-referential pointers into the allocator
    /// field. It must be dropped before the allocator.
    program: NonNull<Program<'static>>,
    /// The arena allocator that contains the parsed data of the eval source.
    allocator: NonNull<Allocator>,
}
impl<'a> SourceCodeHeapData<'a> {
    fn new(
        source: HeapString<'a>,
        program: NonNull<Program<'static>>,
        semantic: NonNull<Semantic<'static>>,
        allocator: NonNull<Allocator>,
    ) -> Self {
        Self {
            source,
            semantic,
            program,
            allocator,
        }
    }

    /// Get a reference to the program AST of this SourceCode.
    ///
    /// ## Safety
    ///
    /// The program AST is valid until the SourceCode is garbage collected.
    pub(crate) fn get_program(
        &self,
        // NoGcScope used only as proof.
        _gc: NoGcScope<'a, '_>,
    ) -> &'a Program<'a> {
        unsafe { core::mem::transmute::<&Program, &'a Program<'a>>(self.program.as_ref()) }
    }

    /// Get a reference to the semantic analysis results of this SourceCode.
    ///
    /// ## Safety
    ///
    /// The semantic analysis results are valid until the SourceCode is
    /// garbage collected.
    fn get_semantic(
        &self,
        // NoGcScope used only as proof.
        _gc: NoGcScope<'a, '_>,
    ) -> &'a Semantic<'static> {
        // SAFETY: SourceCodeHeapData only drops Semantic, Program, and
        // Allocator when it is dropped, ie. GC'd.
        unsafe { core::mem::transmute::<&Semantic, &'a Semantic<'static>>(self.semantic.as_ref()) }
    }
}

unsafe impl Send for SourceCodeHeapData<'_> {}

impl Debug for SourceCodeHeapData<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SourceCodeHeapData")
            .field("source", &self.source)
            .field("allocator", &"[binary data]")
            .finish()
    }
}

impl Drop for SourceCodeHeapData<'_> {
    fn drop(&mut self) {
        // SAFETY: All references to this SourceCode should have been dropped
        // before we drop this.
        unsafe {
            // Note: The drop order here is important. Semantic refers to
            // Program and Allocator, Program refers to Allocator.
            drop(Box::from_raw(self.semantic.as_mut()));
            drop(Box::from_raw(self.program.as_mut()));
            drop(Box::from_raw(self.allocator.as_mut()));
        }
    }
}

impl Index<SourceCode<'_>> for Agent {
    type Output = SourceCodeHeapData<'static>;

    fn index(&self, index: SourceCode) -> &Self::Output {
        self.heap
            .source_codes
            .get(index.get_index())
            .expect("SourceCode out of bounds")
            .as_ref()
            .expect("SourceCode slot empty")
    }
}
impl IndexMut<SourceCode<'_>> for Agent {
    fn index_mut(&mut self, index: SourceCode<'_>) -> &mut Self::Output {
        self.heap
            .source_codes
            .get_mut(index.get_index())
            .expect("SourceCode out of bounds")
            .as_mut()
            .expect("SourceCode slot empty")
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SourceCode<'_> {
    type Of<'a> = SourceCode<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl Rootable for SourceCode<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::SourceCode(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::SourceCode(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<SourceCodeHeapData<'a>, SourceCode<'a>> for Heap {
    fn create(&mut self, data: SourceCodeHeapData<'a>) -> SourceCode<'a> {
        self.source_codes.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<SourceCodeHeapData<'static>>>();
        SourceCode(SourceCodeIndex::last(&self.source_codes))
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SourceCodeHeapData<'_> {
    type Of<'a> = SourceCodeHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for SourceCodeHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { source, .. } = self;
        source.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { source, .. } = self;
        source.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for SourceCode<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.source_codes.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.source_codes.shift_index(&mut self.0);
    }
}
