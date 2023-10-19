use super::{ExecutionContext, Realm, RealmIdentifier};
use crate::{
    types::{Object, Symbol, Value},
    Heap,
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Options {
    pub disable_gc: bool,
    pub print_ast: bool,
    pub print_bytecode: bool,
}

pub type JsResult<T> = std::result::Result<T, ()>;

// #[derive(Debug)]
// pub struct PreAllocated;

#[derive(Debug)]
pub struct HostHooks {
    pub host_ensure_can_compile_strings: fn(callee_realm: &mut Realm) -> JsResult<()>,
    pub host_has_source_text_available: fn(func: Object) -> bool,
}

/// 9.7 Agents
/// https://tc39.es/ecma262/#sec-agents
#[derive(Debug)]
pub struct Agent<'ctx, 'host> {
    pub heap: Heap<'ctx, 'host>,
    pub options: Options,
    // pre_allocated: PreAllocated,
    pub exception: Option<Value>,
    pub symbol_id: usize,
    pub global_symbol_registry: HashMap<&'static str, Symbol>,
    pub host_hooks: HostHooks,
    pub execution_context_stack: Vec<ExecutionContext<'ctx, 'host>>,
}

impl<'ctx, 'host> Agent<'ctx, 'host> {
    pub fn current_realm_id(&self) -> RealmIdentifier<'ctx, 'host> {
        self.execution_context_stack.last().unwrap().realm
    }

    pub fn current_realm(&self) -> &Realm<'ctx, 'host> {
        self.get_realm(self.current_realm_id())
    }

    pub fn current_realm_mut(&mut self) -> &mut Realm<'ctx, 'host> {
        self.get_realm_mut(self.current_realm_id())
    }

    pub fn get_realm(&self, id: RealmIdentifier<'ctx, 'host>) -> &Realm<'ctx, 'host> {
        self.heap.get_realm(id)
    }

    pub fn get_realm_mut(&mut self, id: RealmIdentifier<'ctx, 'host>) -> &mut Realm<'ctx, 'host> {
        self.heap.get_realm_mut(id)
    }

    /// 5.2.3.2 Throw an Exception
    /// https://tc39.es/ecma262/#sec-throw-an-exception
    pub fn throw_exception(&mut self, kind: ExceptionType, message: &'static str) -> () {
        todo!()
    }
}

#[derive(Debug)]
pub enum ExceptionType {
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
}
