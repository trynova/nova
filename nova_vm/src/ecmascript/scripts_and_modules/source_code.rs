// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! SourceCode is a Nova-engine specific concept to capture and keep any
//! `eval(source)` source strings alive after the eval call for the case where
//! that the eval call defines functions. Those functions will refer to the
//! SourceCode for their function source text.

use core::{fmt::Debug, ops::Index, ptr::NonNull};

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceCodeType {
    Script,
    StrictScript,
    Module,
}

impl<'a> SourceCode<'a> {
    /// Parses the given source string as JavaScript code and returns the
    /// parsed result and a SourceCode heap reference.
    ///
    /// ### Program lifetime
    ///
    /// The Program is a structure containing references to the SourceCode's
    /// internal bump allocator memory, and to the source code String's heap
    /// allocated data (if the source code was not heap allocated, it is forced
    /// onto the heap). The SourceCode's heap data keeps a reference to the
    /// source code String, keeping it from being garbage collected while the
    /// SourceCode lives. The bump allocator lives as long as the SourceCode
    /// lives, meaning that the caller must ensure that the Program is not used
    /// after the SourceCode is garbage collected.
    ///
    /// In general, this means either not retaining the Program past a garbage
    /// collection safepoint, or keeping the SourceCode reference alive for as
    /// long as the Program is referenced.
    pub(crate) unsafe fn parse_source(
        agent: &mut Agent,
        source: String,
        source_type: SourceCodeType,
        #[cfg(feature = "typescript")] typescript: bool,
        gc: NoGcScope<'a, '_>,
    ) -> Result<(Program<'a>, Self), Vec<OxcDiagnostic>> {
        // If the source code is not a heap string, pad it with whitespace and
        // allocate it on the heap. This makes it safe (for some definition of
        // "safe") for the any functions created referring to this source code to
        // keep references to the string buffer.
        let (source, source_text) = match source {
            String::String(source) => {
                match source.to_string_lossy(agent) {
                    std::borrow::Cow::Borrowed(source_text) => {
                        // Source text is a valid heap-allocated UTF-8 string.
                        // SAFETY: Caller guarantees to keep SourceCode from being
                        // garbage collected until the parsed Program is dropped.
                        // Thus the source text is kept from garbage collection.
                        (source.unbind(), unsafe {
                            core::mem::transmute::<&str, &'static str>(source_text)
                        })
                    }
                    std::borrow::Cow::Owned(string) => {
                        // Source text is invalid UTF-8 and needed to be copied.
                        let String::String(source) = String::from_string(agent, string, gc) else {
                            unreachable!()
                        };
                        // SAFETY: Allocating a String into the heap cannot
                        // turn it into non-UTF-8.
                        let source_text = unsafe { source.as_str(agent).unwrap_unchecked() };
                        // SAFETY: Caller guarantees to keep SourceCode from being
                        // garbage collected until the parsed Program is dropped.
                        // Thus the source text is kept from garbage collection.
                        (source.unbind(), unsafe {
                            core::mem::transmute::<&str, &'static str>(source_text)
                        })
                    }
                }
            }
            String::SmallString(source) => {
                // Add 10 whitespace bytes to the end of the eval string. This
                // should guarantee that the string gets heap-allocated.
                let original_length = source.len();
                let data = format!("{}          ", source.to_string_lossy());
                let source = String::from_string(agent, data, gc);
                let String::String(source) = source else {
                    unreachable!()
                };
                // SAFETY: Allocating a String into the heap cannot turn it
                // into non-UTF-8.
                let source_text = unsafe { source.as_str(agent).unwrap_unchecked() };
                // SAFETY: Caller guarantees to keep SourceCode from being
                // garbage collected until the parsed Program is dropped.
                // Thus the source text is kept from garbage collection.
                let source_text =
                    unsafe { core::mem::transmute::<&str, &'static str>(source_text) };
                // Slice the source text back to the original length so that the
                // whitespace we added doesn't get fed to the parser: It shouldn't
                // need it.
                let source_text = &source_text[..original_length];
                (source, source_text)
            }
        };

        let mut allocator = NonNull::from(Box::leak(Box::default()));
        // SAFETY: Parser is dropped before allocator.
        let alloc = unsafe { allocator.as_mut() };
        let parser_result = match source_type {
            SourceCodeType::Script => {
                #[allow(unused_mut)]
                let mut source_type = SourceType::cjs();
                #[cfg(feature = "typescript")]
                if typescript {
                    source_type = source_type.with_typescript(true);
                }
                Parser::new(alloc, source_text, source_type).parse()
            }
            SourceCodeType::StrictScript => {
                #[allow(unused_mut)]
                let mut source_type = SourceType::mjs();
                #[cfg(feature = "typescript")]
                if typescript {
                    source_type = source_type.with_typescript(true);
                }

                // Strict script! We first parse this as a module, which makes
                // the script parsing strict but allows module declarations. If
                // that works, then we parse it as a normal script and check
                // that it works as well: this will catch module declarations
                // and TLA.
                let parser_result = Parser::new(alloc, source_text, source_type).parse();
                if parser_result.panicked {
                    let errors = parser_result.errors;
                    // SAFETY: No references to allocator exist anymore. It is safe to
                    // drop it.
                    drop(unsafe { Box::from_raw(allocator.as_mut()) });
                    // TODO: Include error messages in the exception.
                    return Err(errors);
                }

                #[allow(unused_mut)]
                let mut source_type = SourceType::cjs();
                #[cfg(feature = "typescript")]
                if typescript {
                    source_type = source_type.with_typescript(true);
                }
                let sloppy_parser = Parser::new(alloc, source_text, source_type);
                let ParserReturn {
                    errors: sloppy_errors,
                    program: sloppy_program,
                    ..
                } = sloppy_parser.parse();
                if !sloppy_errors.is_empty() {
                    // SAFETY: No references to allocator exist anymore. It is safe to
                    // drop it.
                    drop(unsafe { Box::from_raw(allocator.as_mut()) });
                    // TODO: Include error messages in the exception.
                    return Err(sloppy_errors);
                }
                let SemanticBuilderReturn {
                    errors: sloppy_errors,
                    ..
                } = SemanticBuilder::new()
                    .with_check_syntax_error(true)
                    .build(&sloppy_program);

                if !sloppy_errors.is_empty() {
                    // Drop program before dropping allocator.
                    // SAFETY: No references to allocator exist anymore. It is safe to
                    // drop it.
                    drop(unsafe { Box::from_raw(allocator.as_mut()) });
                    // TODO: Include error messages in the exception.
                    return Err(sloppy_errors);
                }
                parser_result
            }
            SourceCodeType::Module => {
                #[allow(unused_mut)]
                let mut source_type = SourceType::mjs();
                #[cfg(feature = "typescript")]
                if typescript {
                    source_type = source_type.with_typescript(true);
                }
                Parser::new(alloc, source_text, source_type).parse()
            }
        };

        let ParserReturn {
            errors, program, ..
        } = parser_result;

        if !errors.is_empty() {
            // SAFETY: No references to allocator exist anymore. It is safe to
            // drop it.
            drop(unsafe { Box::from_raw(allocator.as_mut()) });
            // TODO: Include error messages in the exception.
            return Err(errors);
        }

        let SemanticBuilderReturn { errors, .. } = SemanticBuilder::new()
            .with_check_syntax_error(true)
            .build(&program);

        if !errors.is_empty() {
            // SAFETY: No references to allocator exist anymore. It is safe to
            // drop it.
            drop(unsafe { Box::from_raw(allocator.as_mut()) });
            // TODO: Include error messages in the exception.
            return Err(errors);
        }
        // SAFETY: Caller guarantees that they will drop the Program before
        // SourceCode can be garbage collected.
        let program = unsafe { core::mem::transmute::<Program, Program<'static>>(program) };
        let source_code = agent.heap.create(SourceCodeHeapData {
            source: source.unbind(),
            allocator,
        });

        Ok((program, source_code))
    }

    pub(crate) fn get_source_text(self, agent: &Agent) -> &str {
        // SAFETY: parse_source will always copy non-UTF-8 source texts into
        // well-formed UTF-8.
        unsafe { agent[agent[self].source].as_str().unwrap_unchecked() }
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
    /// The arena that contains the parsed data of the eval source.
    allocator: NonNull<Allocator>,
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

impl HeapMarkAndSweep for SourceCode<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.source_codes.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.source_codes.shift_index(&mut self.0);
    }
}

#[cfg(test)]
mod test {
    use crate::{
        ecmascript::{
            execution::{Agent, DefaultHostHooks, agent::Options, initialize_default_realm},
            scripts_and_modules::source_code::{SourceCode, SourceCodeType},
            types::String,
        },
        engine::context::GcScope,
    };

    #[test]
    fn script_with_imports() {
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let mut gc = GcScope::new(&mut gc, &mut scope);
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent, gc.reborrow());

        let source_text = String::from_static_str(&mut agent, "import 'foo';", gc.nogc());
        // SAFETY: tests.
        let errors = unsafe {
            SourceCode::parse_source(
                &mut agent,
                source_text,
                SourceCodeType::Script,
                #[cfg(feature = "typescript")]
                false,
                gc.nogc(),
            )
        }
        .unwrap_err();

        assert!(!errors.is_empty());
    }

    #[test]
    fn strict_script_with_imports() {
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let mut gc = GcScope::new(&mut gc, &mut scope);
        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent, gc.reborrow());

        let source_text = String::from_static_str(&mut agent, "import 'foo';", gc.nogc());
        // SAFETY: tests.
        let errors = unsafe {
            SourceCode::parse_source(
                &mut agent,
                source_text,
                SourceCodeType::StrictScript,
                #[cfg(feature = "typescript")]
                false,
                gc.nogc(),
            )
        }
        .unwrap_err();

        assert!(!errors.is_empty());
    }
}
