#![feature(try_trait_v2)]

pub mod heap;
pub mod value;

use std::fmt::Debug;

use gc::Gc;
use heap::{Heap, StringHeapData};
use oxc_ast::{
    ast::{
        AssignmentOperator, AssignmentTarget, BinaryOperator, BindingPatternKind, Declaration,
        Expression, LogicalOperator, Program, SimpleAssignmentTarget, Statement,
        VariableDeclarator,
    },
    syntax_directed_operations::PropName,
};
use value::{JsResult, Value};
use wtf8::Wtf8Buf;

/// https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-ecmascript-language-types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type {
    Undefined,
    Null,
    Boolean,
    String,
    Symbol,
    Number,
    BigInt,
    Object,
}

#[repr(u32)]
pub enum Instruction {
    LoadInteger,
    LoadBoolean,
    LoadNull,
    LoadString,

    // [out] [in]
    CopyValue,

    // [out] [left] [right]
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    StrictEquality,
    Equality,
    StrictInequality,
    Inequality,

    /// `[jump_rel]`
    ///
    /// Jumps to the relative instruction.
    Jump,

    /// `[addr] [jump_rel]`
    ///
    /// If `addr` is a true when converted to a boolean, then the instruction
    /// will skip `jump_rel` instructions forward.
    Test,

    /// `[addr] [jump_rel]`
    /// If `addr` is false when converted to a boolean, then the instruction
    /// will skip `jump_rel` instructions forward.
    TestNot,
}

impl Into<u32> for Instruction {
    fn into(self) -> u32 {
        self as u32
    }
}

pub struct VM<'a> {
    pub source: &'a str,
    pub instructions: Vec<u32>,
    /// Program counter.
    pub pc: u32,
    pub heap: Heap,
}

#[derive(Debug, Default)]
pub struct Env<'a> {
    pub map: std::collections::HashMap<&'a str, u32>,
    pub parent: Option<Box<Env<'a>>>,
}

impl<'a> VM<'a> {
    pub fn interpret(&mut self) -> JsResult<()> {
        let mut memory = Vec::<Value>::with_capacity(self.pc as usize);

        for _ in 0..self.pc {
            memory.push(Value::Undefined);
        }

        let instructions = self.instructions.clone();
        let mut iter = instructions.iter();
        while let Some(leading) = iter.next() {
            match unsafe { std::mem::transmute::<u32, Instruction>(*leading) } {
                Instruction::LoadInteger => {
                    let addr = *iter.next().unwrap() as usize;
                    memory[addr] = Value::from_i32(*iter.next().unwrap() as i32);
                }
                Instruction::LoadBoolean => {
                    let addr = *iter.next().unwrap() as usize;
                    memory[addr] = Value::Boolean(*iter.next().unwrap() == 1);
                }
                Instruction::LoadNull => {
                    let addr = *iter.next().unwrap() as usize;
                    memory[addr] = Value::Null;
                }
                Instruction::LoadString => {
                    let addr = *iter.next().unwrap() as usize;
                    memory[addr] = Value::String(*iter.next().unwrap() as u32);
                }
                Instruction::CopyValue => {
                    let addr = *iter.next().unwrap() as usize;
                    memory[addr] = memory[*iter.next().unwrap() as usize].clone();
                }
                Instruction::Add => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr].try_into_f64(self)?;
                    let right = &memory[*iter.next().unwrap() as usize].try_into_f64(self)?;
                    memory[addr] = Value::from_f64(self, left + right);
                }
                Instruction::Sub => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr].try_into_f64(self)?;
                    let right = &memory[*iter.next().unwrap() as usize].try_into_f64(self)?;
                    memory[addr] = Value::from_f64(self, left - right);
                }
                Instruction::Mul => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr].try_into_f64(self)?;
                    let right = &memory[*iter.next().unwrap() as usize].try_into_f64(self)?;
                    memory[addr] = Value::from_f64(self, left * right);
                }
                Instruction::Mod => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr].try_into_f64(self)?;
                    let right = &memory[*iter.next().unwrap() as usize].try_into_f64(self)?;
                    memory[addr] = Value::from_f64(self, left % right);
                }
                Instruction::Div => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr].try_into_f64(self)?;
                    let right = &memory[*iter.next().unwrap() as usize].try_into_f64(self)?;
                    memory[addr] = Value::from_f64(self, left / right);
                }
                Instruction::StrictEquality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(left.is_strictly_equal(self, right)?);
                }
                Instruction::Equality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(left.is_loosely_equal(self, right)?);
                }
                Instruction::StrictInequality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(!left.is_strictly_equal(self, right)?);
                }
                Instruction::Inequality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(!left.is_loosely_equal(self, right)?);
                }
                Instruction::Test => {
                    let addr = *iter.next().unwrap() as usize;
                    let jump_rel = *iter.next().unwrap() as usize;

                    if memory[addr].into_bool() {
                        _ = iter.nth(jump_rel);
                    };
                }
                Instruction::TestNot => {
                    let addr = *iter.next().unwrap() as usize;
                    let jump_rel = *iter.next().unwrap() as usize;

                    if !memory[addr].into_bool() {
                        _ = iter.nth(jump_rel);
                    };
                }
                Instruction::Jump => {
                    let jump_rel = *iter.next().unwrap() as usize;
                    _ = iter.nth(jump_rel);
                }
            }
        }

        println!("{:?}", memory.as_slice());

        Ok(())
    }

    /// Builds the bytecode to run an expression.
    ///
    /// Assumes the memory location is valid to use throughout the evaluation as
    /// a scratch.
    fn build_expr(&mut self, addr: u32, expr: &Expression, env: &mut Env) {
        match expr {
            Expression::NullLiteral(_) => {
                self.instructions.push(Instruction::LoadNull.into());
                self.instructions.push(addr);
            }
            Expression::BooleanLiteral(l) => {
                self.instructions.push(Instruction::LoadBoolean.into());
                self.instructions.push(addr);
                self.instructions.push(l.value.into());
            }
            Expression::Identifier(ident) => {
                // TODO: figure out how to return the ident's memory addr as
                //       an optimization
                self.instructions.push(Instruction::CopyValue.into());
                self.instructions.push(addr);
                // TODO: support recursive ident lookups
                self.instructions
                    .push(*env.map.get(ident.name.as_str()).unwrap());
            }
            Expression::NumberLiteral(num) => {
                self.instructions.push(Instruction::LoadInteger.into());
                self.instructions.push(addr);
                self.instructions
                    .push(unsafe { std::mem::transmute(*num.value.as_f32()) });
            }
            Expression::LogicalExpression(expr) => match expr.operator {
                LogicalOperator::And => {
                    self.build_expr(addr, &expr.left, env);
                    self.instructions.push(Instruction::Test.into());
                    self.instructions.push(addr);
                    let jump_addr = self.instructions.len();
                    self.instructions.push(0);
                    self.build_expr(addr, &expr.right, env);
                    self.instructions[jump_addr] = (self.instructions.len() - jump_addr) as u32;
                }
                LogicalOperator::Or => {
                    self.build_expr(addr, &expr.left, env);
                    self.instructions.push(Instruction::TestNot.into());
                    self.instructions.push(addr);
                    let jump_addr = self.instructions.len();
                    self.instructions.push(0);
                    self.build_expr(addr, &expr.right, env);
                    self.instructions[jump_addr] = (self.instructions.len() - jump_addr) as u32;
                }
                _ => panic!(),
            },
            Expression::ConditionalExpression(cond) => {
                self.build_expr(addr, &cond.test, env);

                self.instructions.push(Instruction::Test.into());
                self.instructions.push(addr);
                let finish_idx = self.instructions.len();
                self.instructions.push(0);

                self.build_expr(addr, &cond.alternate, env);

                self.instructions.push(Instruction::Jump.into());
                let alternate_idx = self.instructions.len();
                self.instructions.push(0);

                self.instructions[finish_idx] = (self.instructions.len() - finish_idx - 2) as u32;
                self.build_expr(addr, &cond.consequent, env);

                self.instructions[alternate_idx] =
                    (self.instructions.len() - alternate_idx - 2) as u32;
            }
            Expression::ObjectExpression(obj) => {
                for prop in obj.properties.iter() {
                    prop.prop_name();
                }
            }
            Expression::BinaryExpression(binary_op) => {
                macro_rules! binary_op {
                    ($name: ident) => {{
                        let right = self.pc;
                        self.pc += 1;

                        self.build_expr(addr, &binary_op.left, env);
                        self.build_expr(right, &binary_op.right, env);

                        self.instructions.push(Instruction::$name.into());
                        self.instructions.push(addr);
                        self.instructions.push(right);
                    }};
                }

                match binary_op.operator {
                    BinaryOperator::Addition => binary_op!(Add),
                    BinaryOperator::Subtraction => binary_op!(Sub),
                    BinaryOperator::Multiplication => binary_op!(Mul),
                    BinaryOperator::Remainder => binary_op!(Mod),
                    BinaryOperator::Division => binary_op!(Div),
                    BinaryOperator::StrictEquality => binary_op!(StrictEquality),
                    BinaryOperator::Equality => binary_op!(Equality),
                    BinaryOperator::StrictInequality => binary_op!(StrictInequality),
                    BinaryOperator::Inequality => binary_op!(Inequality),
                    _ => todo!(),
                }
            }
            Expression::StringLiteral(s) => {
                let js_string = StringHeapData {
                    data: Wtf8Buf::from_str(&*s.value.as_str()),
                };
                let string_idx = self.heap.strings.len();
                self.heap.strings.push(Gc::new(js_string));

                self.instructions.push(Instruction::LoadString.into());
                self.instructions.push(addr);
                self.instructions.push(string_idx as u32);
            }
            Expression::ParenthesizedExpression(data) => {
                return self.build_expr(addr, &data.expression, env);
            }
            Expression::SequenceExpression(data) => {
                for expr in data.expressions.iter() {
                    self.build_expr(addr, expr, env);
                }
            }
            Expression::AssignmentExpression(s) => match s.operator {
                AssignmentOperator::Assign => match &s.left {
                    AssignmentTarget::SimpleAssignmentTarget(target) => match target {
                        SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
                            let Some(addr) = env.map.get(ident.name.as_str()) else {
								panic!("Unknown ident.");
							};
                            self.build_expr(*addr, &s.right, env);
                        }
                        _ => todo!(),
                    },
                    _ => todo!(),
                },
                _ => todo!(),
            },
            Expression::ArrayExpression(_) => todo!(),
            Expression::BigintLiteral(_) => todo!(),
            Expression::RegExpLiteral(_) => todo!(),
            Expression::TemplateLiteral(_) => todo!(),
            Expression::MetaProperty(_) => todo!(),
            Expression::Super(_) => todo!(),
            Expression::ArrowFunctionExpression(_) => todo!(),
            Expression::AwaitExpression(_) => todo!(),
            Expression::CallExpression(_) => todo!(),
            Expression::ChainExpression(_) => todo!(),
            Expression::ClassExpression(_) => todo!(),
            Expression::FunctionExpression(_) => todo!(),
            Expression::ImportExpression(_) => todo!(),
            Expression::MemberExpression(_) => todo!(),
            Expression::NewExpression(_) => todo!(),
            Expression::TaggedTemplateExpression(_) => todo!(),
            Expression::ThisExpression(_) => todo!(),
            Expression::UnaryExpression(_) => todo!(),
            Expression::UpdateExpression(_) => todo!(),
            Expression::YieldExpression(_) => todo!(),
            Expression::PrivateInExpression(_) => todo!(),
            // TypeScript and JSX not supported
            Expression::JSXElement(_)
            | Expression::JSXFragment(_)
            | Expression::TSAsExpression(_)
            | Expression::TSSatisfiesExpression(_)
            | Expression::TSTypeAssertion(_)
            | Expression::TSNonNullExpression(_)
            | Expression::TSInstantiationExpression(_) => unreachable!(),
        }
    }

    pub fn build_stmt<'b>(&mut self, stmt: &'b Statement, env: &mut Env<'b>) {
        match stmt {
            Statement::Declaration(Declaration::VariableDeclaration(decl)) => {
                for member in decl.declarations.as_slice() {
                    let member: &VariableDeclarator = member;
                    env.map.insert(
                        match &member.id.kind {
                            BindingPatternKind::BindingIdentifier(ident) => ident.name.as_str(),
                            _ => panic!(),
                        },
                        self.pc,
                    );
                    let addr = self.pc;
                    self.pc += 1;

                    if let Some(expr) = &member.init {
                        self.build_expr(addr, expr, env);
                    } else {
                        todo!("Load undefined.");
                    }
                }
            }
            Statement::Declaration(Declaration::FunctionDeclaration(_)) => todo!(),
            Statement::Declaration(Declaration::ClassDeclaration(_)) => todo!(),
            Statement::ExpressionStatement(expr) => {
                self.build_expr(self.pc, &expr.expression, env);
            }
            Statement::BlockStatement(block) => {
                for stmt in block.body.iter() {
                    self.build_stmt(&stmt, env);
                }
            }
            Statement::IfStatement(s) => {
                let addr = self.pc;
                self.pc += 1;

                self.build_expr(addr, &s.test, env);
                self.instructions.push(Instruction::Test.into());
                self.instructions.push(addr);
                let consequent_idx = self.instructions.len();
                self.instructions.push(0);

                if let Some(alternate) = &s.alternate {
                    self.build_stmt(alternate, env);
                }

                self.instructions.push(Instruction::Jump.into());
                let finish_idx = self.instructions.len();
                self.instructions.push(0);

                self.instructions[consequent_idx] =
                    (self.instructions.len() - consequent_idx - 2) as u32;
                self.build_stmt(&s.consequent, env);

                self.instructions[finish_idx] = (self.instructions.len() - finish_idx - 2) as u32;
            }
            Statement::BreakStatement(_) => todo!(),
            Statement::ContinueStatement(_) => todo!(),
            Statement::DebuggerStatement(_) => todo!(),
            Statement::DoWhileStatement(_) => todo!(),
            Statement::EmptyStatement(_) => todo!(),
            Statement::ForInStatement(_) => todo!(),
            Statement::ForOfStatement(_) => todo!(),
            Statement::ForStatement(_) => todo!(),
            Statement::LabeledStatement(_) => todo!(),
            Statement::ReturnStatement(_) => todo!(),
            Statement::SwitchStatement(_) => todo!(),
            Statement::ThrowStatement(_) => todo!(),
            Statement::TryStatement(_) => todo!(),
            Statement::WhileStatement(_) => todo!(),
            Statement::WithStatement(_) => todo!(),
            Statement::ModuleDeclaration(_) => todo!(),
            // TypeScript not supported
            Statement::Declaration(Declaration::TSTypeAliasDeclaration(_))
            | Statement::Declaration(Declaration::TSInterfaceDeclaration(_))
            | Statement::Declaration(Declaration::TSEnumDeclaration(_))
            | Statement::Declaration(Declaration::TSModuleDeclaration(_))
            | Statement::Declaration(Declaration::TSImportEqualsDeclaration(_)) => unreachable!(),
        }
    }

    pub fn load_program(&mut self, program: Program) {
        let mut env = Env::default();
        for stmt in program.body.iter() {
            self.build_stmt(stmt, &mut env);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn foo() {}
}
