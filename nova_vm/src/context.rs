use std::{cell::RefCell, rc::Rc};

use crate::{byte_compiler::Environment, vm::VM, ByteCompiler};
use oxc_ast::ast::Program;
use string_interner::{symbol::SymbolU32, StringInterner};

#[derive(Debug)]
pub struct Context {
    pub(crate) interner: Interner,

    pub(crate) vm: VM,
}

impl Context {
    pub fn new() -> Self {
        Self {
            interner: Interner {
                internal: StringInterner::new(),
            },
            vm: VM {},
        }
    }

    pub fn eval(&mut self, program: &Program) {
        let global_env = Rc::new(RefCell::new(Environment::new(None)));
        let mut compiler: ByteCompiler<'_> = ByteCompiler::new(global_env, self);

        compiler.emit_stmt_block(&program.body);

        println!("{:?}", compiler.program);
    }
}

#[derive(Debug)]
pub struct Interner {
    internal: StringInterner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sym {
    data: SymbolU32,
}

impl From<SymbolU32> for Sym {
    fn from(data: SymbolU32) -> Self {
        Sym { data }
    }
}

impl Interner {
    pub fn get(&mut self, string: impl AsRef<str>) -> Option<Sym> {
        self.internal.get(string.as_ref()).map(|data| data.into())
    }

    pub fn get_or_intern(&mut self, string: impl AsRef<str>) -> Sym {
        self.internal.get_or_intern(string.as_ref()).into()
    }

    pub fn resolve(&mut self, sym: Sym) -> Option<&str> {
        self.internal.resolve(sym.data)
    }
}
