#![feature(try_trait_v2)]

use std::{
    borrow::Borrow,
    fmt::Debug,
    ops::{ControlFlow, FromResidual, Try},
};

use gc::{unsafe_empty_trace, Finalize, Gc, GcCell, Trace};
use oxc_ast::{
    ast::{
        AssignmentOperator, AssignmentTarget, BinaryOperator, BindingPatternKind, Declaration,
        Expression, LogicalExpression, LogicalOperator, Program, SimpleAssignmentTarget, Statement,
        VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
    },
    syntax_directed_operations::PropName,
};
use oxc_parser::*;
use wtf8::{Wtf8, Wtf8Buf};

// Completely unoptimized...look away.
#[derive(Clone)]
#[repr(u16)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    String(Gc<JsString>),
    Symbol(Gc<JsSymbol>),
    Number(f64),
    BigInt(i64),
    Object(Gc<dyn JsObject>),
}

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

impl Value {
    pub fn create_exception(message: &str) -> Value {
        Value::String(Gc::new(JsString {
            data: Wtf8Buf::from_str(message),
        }))
    }

    pub fn get_type(&self) -> Type {
        match self {
            Self::Undefined => Type::Undefined,
            Self::Null => Type::Null,
            Self::Boolean(_) => Type::Boolean,
            Self::String(_) => Type::String,
            Self::Symbol(_) => Type::Symbol,
            Self::Number(_) => Type::Number,
            Self::BigInt(_) => Type::BigInt,
            Self::Object(_) => Type::Object,
        }
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islooselyequal
    pub fn is_loosely_equal(&self, other: &Value) -> JsResult<bool> {
        if self.get_type() == other.get_type() {
            return self.is_strictly_equal(other);
        }

        Ok(match (self, other) {
            (Value::Null, Value::Undefined) => true,
            (Value::Undefined, Value::Null) => true,
            (Value::Number(_), Value::String(_)) => todo!("use ToNumber() intrinsics"),
            (Value::String(_), Value::Number(_)) => todo!("use ToNumber() intrinsics"),
            (Value::BigInt(_), Value::String(_)) => todo!("use StringToBigInt() intrinsics"),
            (Value::String(_), Value::BigInt(_)) => other.is_loosely_equal(self)?,
            (Value::Boolean(_), _) => Value::Number(self.to_number()?).is_loosely_equal(other)?,
            (_, Value::Boolean(_)) => Value::Number(other.to_number()?).is_loosely_equal(self)?,
            (Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Symbol(_), _) => {
                other.is_loosely_equal(&self.to_primitive()?)?
            }
            (
                Value::Object(_),
                Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Symbol(_),
            ) => self.to_primitive()?.is_loosely_equal(other)?,
            (&Value::BigInt(x), &Value::Number(y)) => (x as f64) == y,
            (&Value::Number(x), &Value::BigInt(y)) => x == (y as f64),
            _ => false,
        })
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-isstrictlyequal
    pub fn is_strictly_equal(&self, other: &Value) -> JsResult<bool> {
        if self.get_type() != other.get_type() {
            return Ok(false);
        }

        Ok(match (self, other) {
            (Value::Number(n1), Value::Number(n2)) => n1 == n2,

            // https://tc39.es/ecma262/multipage/abstract-operations.html#sec-samevaluenonnumber
            (Value::Null | Value::Undefined, _) => true,
            (Value::BigInt(n1), Value::BigInt(n2)) => n1 == n2,
            (Value::String(s1), Value::String(s2)) => s1.data == s2.data,
            (Value::Boolean(b1), Value::Boolean(b2)) => b1 == b2,
            // TODO: implement x is y procedures
            (Value::Object(obj1), Value::Object(obj2)) => {
                Gc::<(dyn JsObject + 'static)>::ptr_eq(&obj1, obj2)
            }
            _ => false,
        })
    }

    pub fn to_primitive(&self) -> JsResult<Value> {
        Ok(Value::Null)
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toboolean
    pub fn to_boolean(&self) -> bool {
        match self {
            &Value::Boolean(b) => b,
            Value::Null => false,
            &Value::Number(n) if n == 0. || n == f64::NAN => false,
            Value::String(s) if s.data.len() == 0 => false,
            _ => true,
        }
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
    pub fn to_number(&self) -> JsResult<f64> {
        Ok(match self {
            Value::Number(n) => *n,
            Value::Symbol(_) | Value::BigInt(_) => todo!("type error"),
            Value::Undefined => f64::NAN,
            Value::Null | Value::Boolean(false) => 0.,
            Value::Boolean(true) => 1.,
            Value::String(_) => todo!("parse number from string"),
            _ => todo!("should assert as object and do other steps"),
        })
    }
}

type JsResult<T> = std::result::Result<T, Value>;

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "Null"),
            Self::Undefined => write!(f, "Undefined"),
            Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Self::Number(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Self::BigInt(arg0) => f.debug_tuple("BigInt").field(arg0).finish(),
            Self::String(arg0) => f.debug_tuple("String").field(&arg0.data.as_str()).finish(),
            Self::Object(arg0) => f.debug_tuple("JsObject").field(&arg0).finish(),
            Self::Symbol(arg0) => f
                .debug_tuple("Symbol")
                .field(&arg0.descriptor.as_ref().map(|wtf8| wtf8.data.as_str()))
                .finish(),
        }
    }
}

pub trait JsObject: Trace + Debug {}

#[derive(Clone)]
pub struct JsString {
    data: Wtf8Buf,
}

impl Finalize for JsString {
    fn finalize(&self) {}
}
unsafe impl Trace for JsString {
    unsafe_empty_trace!();
}

#[derive(Trace, Finalize)]
pub struct JsSymbol {
    descriptor: Option<Gc<JsString>>,
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
    pub strings: Vec<Gc<JsString>>,
}

#[derive(Debug, Default)]
pub struct Env<'a> {
    pub map: std::collections::HashMap<&'a str, u32>,
    pub parent: Option<Box<Env<'a>>>,
}

impl<'a> VM<'a> {
    pub fn interpret(&self) -> JsResult<()> {
        let mut memory = Vec::<Value>::with_capacity(self.pc as usize);

        for _ in 0..self.pc {
            memory.push(Value::Undefined);
        }

        let mut iter = self.instructions.iter();
        while let Some(leading) = iter.next() {
            match unsafe { std::mem::transmute(*leading) } {
                Instruction::LoadInteger => {
                    let addr = *iter.next().unwrap() as usize;
                    memory[addr] = Value::Number(unsafe {
                        std::mem::transmute::<u32, f32>(*iter.next().unwrap())
                    } as f64);
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
                    memory[addr] =
                        Value::String(self.strings[*iter.next().unwrap() as usize].clone());
                }
                Instruction::CopyValue => {
                    let addr = *iter.next().unwrap() as usize;
                    memory[addr] = memory[*iter.next().unwrap() as usize].clone();
                }
                Instruction::Add => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Number(left.to_number()? + right.to_number()?);
                }
                Instruction::Sub => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Number(left.to_number()? - right.to_number()?);
                }
                Instruction::Mul => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Number(left.to_number()? * right.to_number()?);
                }
                Instruction::Mod => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Number(left.to_number()? % right.to_number()?);
                }
                Instruction::Div => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Number(left.to_number()? / right.to_number()?);
                }
                Instruction::StrictEquality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(left.is_strictly_equal(right)?);
                }
                Instruction::Equality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(left.is_loosely_equal(right)?);
                }
                Instruction::StrictInequality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(!left.is_strictly_equal(right)?);
                }
                Instruction::Inequality => {
                    let addr = *iter.next().unwrap() as usize;
                    let left = &memory[addr];
                    let right = &memory[*iter.next().unwrap() as usize];
                    memory[addr] = Value::Boolean(!left.is_loosely_equal(right)?);
                }
                Instruction::Test => {
                    let addr = *iter.next().unwrap() as usize;
                    let jump_rel = *iter.next().unwrap() as usize;

                    if memory[addr].to_boolean() {
                        _ = iter.nth(jump_rel);
                    };
                }
                Instruction::TestNot => {
                    let addr = *iter.next().unwrap() as usize;
                    let jump_rel = *iter.next().unwrap() as usize;

                    if !memory[addr].to_boolean() {
                        _ = iter.nth(jump_rel);
                    };
                }
                Instruction::Jump => {
                    let jump_rel = *iter.next().unwrap() as usize;
                    _ = iter.nth(jump_rel);
                }
                _ => panic!(),
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
                let js_string = JsString {
                    data: Wtf8Buf::from_str(&*s.value.as_str()),
                };
                let string_idx = self.strings.len();
                self.strings.push(Gc::new(js_string));

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
            other => todo!("{:?}", other),
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
            // Statement::IfStatement(s) => {}
            other => todo!("{other:?}"),
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
