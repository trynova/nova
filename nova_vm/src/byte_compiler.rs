use crate::context::{Context, Sym};
use hashbrown::HashMap;
use oxc_ast::ast::{
    BindingPattern, BindingPatternKind, Declaration, Expression, Program, Statement,
    VariableDeclaration, VariableDeclarationKind,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Pop,
    PushUndefined,
    PushNull,
    PushTrue,
    PushFalse,
    PushNum32,
    PushNum64,
    PushEnvironment,
    PopEnvironment,
}

#[derive(Debug)]
pub struct ByteCompiler<'ctx> {
    pub(crate) program: Vec<u32>,
    environment_stack: Vec<Rc<RefCell<Environment>>>,
    current_environment: Rc<RefCell<Environment>>,
    context: &'ctx mut Context,
}

impl<'ctx> ByteCompiler<'ctx> {
    pub fn new(global_environment: Rc<RefCell<Environment>>, context: &'ctx mut Context) -> Self {
        Self {
            program: Vec::new(),
            environment_stack: vec![global_environment.clone()],
            current_environment: global_environment,
            context,
        }
    }
}

/// A compile-time binding for how to resolve a binding at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Binding {
    index: u32,
    is_lexical: bool,
    is_mutable: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BindingRef {
    name: Sym,
    environment_index: u32,
    binding_index: u32,
}

/// Environments define the variables that are available in a given scope.
/// Furthermore, they give information on what sort of rules should be applied
/// to the members in the scope.
#[derive(Debug, Default)]
pub struct Environment {
    bindings: HashMap<Sym, Binding>,
    parent: Option<Rc<RefCell<Self>>>,
}

impl Environment {
    pub fn new(parent: Option<Rc<RefCell<Environment>>>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent,
        }
    }
}

impl<'ctx> ByteCompiler<'ctx> {
    #[inline]
    pub(crate) fn emit_opcode(&mut self, opcode: OpCode) {
        self.program.push(opcode as u32);
    }

    #[inline]
    pub(crate) fn emit(&mut self, opcode: OpCode, args: &[u32]) {
        self.program.push(opcode as u32);
        self.program.extend(args);
    }

    /// Pushes an environment onto the environment stack and emits the bytecode
    /// instructions for the runtime.
    pub(crate) fn push_environment_stack(&mut self) {
        self.emit(
            OpCode::PushEnvironment,
            &[self.environment_stack.len() as u32],
        );

        self.environment_stack
            .push(self.current_environment.clone());
        self.current_environment = Rc::new(RefCell::new(Environment::new(Some(
            self.current_environment.clone(),
        ))));
    }

    /// Pops an environment from the environment stack and emits the bytecode
    /// instructions for the runtime.
    #[inline]
    pub(crate) fn pop_environment_stack(&mut self) {
        self.emit_opcode(OpCode::PopEnvironment);

        let parent = self.current_environment.borrow().parent.clone();
        self.current_environment = parent.expect("We should never pop the global environment.");
    }

    pub(crate) fn emit_expr(&mut self, expr: &Expression, use_expr: bool) {
        match expr {
            Expression::BooleanLiteral(lit) => self.emit_opcode(if lit.value {
                OpCode::PushTrue
            } else {
                OpCode::PushFalse
            }),
            Expression::NullLiteral(_) => self.emit_opcode(OpCode::PushNull),
            Expression::NumberLiteral(lit) => {
                assert!(
                    lit.value.into_inner() == (lit.value.into_inner() as f32 as f64),
                    "TODO: figure out how to serialize float with 64 bits"
                );

                self.emit(
                    OpCode::PushNum32,
                    &[unsafe { std::mem::transmute(lit.value.into_inner() as f32) }],
                );
            }
            Expression::BigintLiteral(_) => todo!(),
            Expression::RegExpLiteral(_) => todo!(),
            Expression::StringLiteral(_) => todo!(),
            Expression::TemplateLiteral(_) => todo!(),
            Expression::Identifier(ident) if ident.name == "undefined" => {
                self.emit_opcode(OpCode::PushUndefined)
            }
            Expression::Identifier(_) => todo!(),
            Expression::MetaProperty(_) => todo!(),
            Expression::Super(_) => todo!(),
            Expression::ArrayExpression(_) => todo!(),
            Expression::ArrowFunctionExpression(_) => todo!(),
            Expression::AssignmentExpression(_) => todo!(),
            Expression::AwaitExpression(_) => todo!(),
            Expression::BinaryExpression(_) => todo!(),
            Expression::CallExpression(_) => todo!(),
            Expression::ChainExpression(_) => todo!(),
            Expression::ClassExpression(_) => todo!(),
            Expression::ConditionalExpression(_) => todo!(),
            Expression::FunctionExpression(_) => todo!(),
            Expression::ImportExpression(_) => todo!(),
            Expression::LogicalExpression(_) => todo!(),
            Expression::MemberExpression(_) => todo!(),
            Expression::NewExpression(_) => todo!(),
            Expression::ObjectExpression(_) => todo!(),
            Expression::ParenthesizedExpression(_) => todo!(),
            Expression::SequenceExpression(_) => todo!(),
            Expression::TaggedTemplateExpression(_) => todo!(),
            Expression::ThisExpression(_) => todo!(),
            Expression::UnaryExpression(_) => todo!(),
            Expression::UpdateExpression(_) => todo!(),
            Expression::YieldExpression(_) => todo!(),
            Expression::PrivateInExpression(_) => todo!(),
            Expression::JSXElement(_) => todo!(),
            Expression::JSXFragment(_) => todo!(),
            Expression::TSAsExpression(_) => todo!(),
            Expression::TSSatisfiesExpression(_) => todo!(),
            Expression::TSTypeAssertion(_) => todo!(),
            Expression::TSNonNullExpression(_) => todo!(),
            Expression::TSInstantiationExpression(_) => todo!(),
        }

        if !use_expr {
            self.emit_opcode(OpCode::Pop);
        }
    }

    #[inline]
    pub(crate) fn emit_var_decl(&mut self, decls: &VariableDeclaration) {
        match &decls.kind {
            VariableDeclarationKind::Var => todo!(),
            VariableDeclarationKind::Const => todo!(),
            VariableDeclarationKind::Let => {
                for decl in decls.declarations.iter() {
                    if let Some(expr) = &decl.init {
                        self.emit_expr(expr, true);
                    }

                    let ident = match &decl.id.kind {
                        BindingPatternKind::BindingIdentifier(ident) => {
                            self.context.interner.get_or_intern(&ident.name)
                        }
                        _ => todo!(),
                    };
                }
            }
        }
    }

    #[inline]
    pub(crate) fn emit_decl(&mut self, decl: &Declaration) {
        match decl {
            Declaration::VariableDeclaration(var_decl) => self.emit_var_decl(var_decl),
            Declaration::FunctionDeclaration(_) => todo!(),
            Declaration::ClassDeclaration(_) => todo!(),
            Declaration::TSTypeAliasDeclaration(_) => todo!(),
            Declaration::TSInterfaceDeclaration(_) => todo!(),
            Declaration::TSEnumDeclaration(_) => todo!(),
            Declaration::TSModuleDeclaration(_) => todo!(),
            Declaration::TSImportEqualsDeclaration(_) => todo!(),
        }
    }

    #[inline]
    pub(crate) fn emit_stmt_block(&mut self, block: &[Statement]) {
        self.push_environment_stack();
        for stmt in block {
            self.emit_stmt(stmt);
        }
        self.pop_environment_stack();
    }

    pub(crate) fn emit_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::BlockStatement(stmt) => {
                self.emit_stmt_block(&stmt.0.body);
            }
            Statement::BreakStatement(_) => todo!(),
            Statement::ContinueStatement(_) => todo!(),
            Statement::DebuggerStatement(_) => todo!(),
            Statement::Declaration(decl) => self.emit_decl(decl),
            Statement::DoWhileStatement(_) => todo!(),
            Statement::EmptyStatement(_) => todo!(),
            Statement::ExpressionStatement(data) => self.emit_expr(&data.expression, false),
            Statement::ForInStatement(_) => todo!(),
            Statement::ForOfStatement(_) => todo!(),
            Statement::ForStatement(_) => todo!(),
            Statement::IfStatement(_) => todo!(),
            Statement::LabeledStatement(_) => todo!(),
            Statement::ReturnStatement(_) => todo!(),
            Statement::SwitchStatement(_) => todo!(),
            Statement::ThrowStatement(_) => todo!(),
            Statement::TryStatement(_) => todo!(),
            Statement::WhileStatement(_) => todo!(),
            Statement::WithStatement(_) => todo!(),
            Statement::ModuleDeclaration(_) => todo!(),
        }
    }
}
