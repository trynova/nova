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
        gc: NoGcScope<'a, '_>,
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

        let SemanticBuilderReturn { errors, semantic } = SemanticBuilder::new()
            .with_check_syntax_error(true)
            .build(&program);

        if !errors.is_empty() {
            // Drop semantic & program before dropping allocator.
            #[allow(clippy::drop_non_drop)]
            drop(semantic);
            #[allow(clippy::drop_non_drop)]
            drop(program);
            // SAFETY: No references to allocator exist anymore. It is safe to
            // drop it.
            drop(unsafe { Box::from_raw(allocator.as_mut()) });
            // TODO: Include error messages in the exception.
            return Err(errors);
        }
        let semantic = unsafe { core::mem::transmute::<Semantic, Semantic<'static>>(semantic) };
        // SAFETY: Caller guarantees that they will drop the Program before
        // SourceCode can be garbage collected.
        let program = unsafe { core::mem::transmute::<Program, Program<'static>>(program) };
        let source_code = agent.heap.create(SourceCodeHeapData::new(
            allocator,
            source.unbind(),
            semantic,
        ));

        Ok((program, source_code))
    }

    pub(crate) fn get_source_text(self, agent: &Agent) -> &str {
        agent[agent[self].source].as_str()
    }

    pub(crate) fn take_semantic(
        self,
        agent: &mut Agent,
        _gc: NoGcScope<'a, '_>, // used as proof
    ) -> Box<Semantic<'a>> {
        let sc = &mut agent[self];
        let sema = sc.take_semantic();
        // SAFETY:
        // 1. taking Semantic out of the agent makes the SourceCode no longer responsible for it
        // 2. Semantic references data within the SourceCode, which lives at least as long as 'gc
        // 3. because of ^, Semantic must be dropped before the next garbage
        //    collection, less it reference now-invalid memory
        unsafe { std::mem::transmute::<Box<Semantic<'static>>, Box<Semantic<'a>>>(sema) }
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
    /// Semantic analysis results obtained when parsing a source file.
    /// 
    /// This only exists until this source code gets compiled. Since it's only
    /// needed then, the compiler takes this semantic out of the source file,
    /// obtaining ownership over it.
    semantic: Option<Box<Semantic<'a>>>,
    /// The arena that contains the parsed data of the eval source.
    allocator: NonNull<Allocator>,
}
impl<'a> SourceCodeHeapData<'a> {
    fn new(allocator: NonNull<Allocator>, source: HeapString<'a>, semantic: Semantic<'a>) -> Self {
        Self {
            source,
            allocator,
            semantic: Some(Box::new(semantic)),
        }
    }

    /// Take semantic analysis info from this source. This may only be called
    /// once.
    ///
    /// ## Panics
    /// If [`take_scoping`] has already been called.
    pub fn take_semantic(&mut self) -> Box<Semantic<'a>> {
        self.semantic.take().unwrap()
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
        drop(unsafe { Box::from_raw(self.allocator.as_mut()) });
        // drop(
        //     self.semantic
        //         .map(|mut sema| unsafe { Box::from_raw(sema.as_mut()) }),
        // )
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
