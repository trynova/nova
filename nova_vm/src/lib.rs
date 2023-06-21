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
        Expression, LogicalOperator, Program, SimpleAssignmentTarget, Statement,
        VariableDeclarator,
    },
    syntax_directed_operations::PropName,
};
use wtf8::Wtf8Buf;

// Completely unoptimized...look away.
#[derive(Clone)]
#[repr(u8)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    EmptyString,
    String(u32),
    Symbol(u32),
    Smi(i32),
    SmiU(u32),
    NaN,
    Infinity,
    NegativeInfinity,
    NegativeZero,
    Number(u32),
    SmallBigInt(i32),
    SmallBigIntU(u32),
    BigInt(u32),
    Object(u32),
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
    pub fn create_exception(vm: &mut VM, message: &str) -> Value {
        let data = Gc::new(JsString {
            data: Wtf8Buf::from_str(message),
        });
        vm.strings.push(data);
        Value::String(vm.strings.len() as u32)
    }

    pub fn get_type(&self) -> Type {
        match self {
            Value::Undefined => Type::Undefined,
            Value::Null => Type::Null,
            Value::Boolean(_) => Type::Boolean,
            Value::EmptyString | Value::String(_) => Type::String,
            Value::Symbol(_) => Type::Symbol,
            Value::NaN
            | Value::NegativeInfinity
            | Value::NegativeZero
            | Value::Infinity
            | Value::Smi(_)
            | Value::SmiU(_)
            | Value::Number(_) => Type::Number,
            Value::SmallBigInt(_) | Value::SmallBigIntU(_) | Value::BigInt(_) => Type::BigInt,
            Value::Object(_) => Type::Object,
        }
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islooselyequal
    pub fn is_loosely_equal(&self, vm: &mut VM, other: &Value) -> JsResult<bool> {
        if self.get_type() == other.get_type() {
            return self.is_strictly_equal(vm, other);
        }

        Ok(match (self, other) {
            (Value::Null | Value::Undefined, Value::Null | Value::Undefined) => true,
            (
                Value::SmallBigInt(this) | Value::Smi(this),
                Value::SmallBigInt(that) | Value::Smi(that),
            ) => this == that,
            (
                Value::SmallBigIntU(this) | Value::SmiU(this),
                Value::SmallBigIntU(that) | Value::SmiU(that),
            ) => this == that,
            (
                Value::SmallBigInt(this) | Value::Smi(this),
                Value::SmallBigIntU(that) | Value::SmiU(that),
            ) => *this as u32 == *that,
            (
                Value::SmallBigIntU(this) | Value::SmiU(this),
                Value::SmallBigInt(that) | Value::Smi(that),
            ) => *this == *that as u32,
            (&Value::BigInt(x), &Value::Number(y)) => {
                let big_int = &vm.heap_bigints[x as usize];
                let number = &vm.heap_numbers[y as usize];
                big_int.len == 1 && big_int.parts[0] as f64 == number.data
            }
            (&Value::Number(x), &Value::BigInt(y)) => {
                let big_int = &vm.heap_bigints[y as usize];
                let number = &vm.heap_numbers[x as usize];
                big_int.len == 1 && big_int.parts[0] as f64 == number.data
            }
            (Value::Number(_), Value::String(_)) => todo!("use ToNumber() intrinsics"),
            (Value::String(_), Value::Number(_)) => todo!("use ToNumber() intrinsics"),
            (Value::BigInt(_), Value::String(_)) => todo!("use StringToBigInt() intrinsics"),
            (Value::String(_), Value::BigInt(_)) => other.is_loosely_equal(vm, self)?,
            (Value::Boolean(_), _) => {
                let self_as_f64 = self.try_into_f64(vm)?;
                Value::from_f64(vm, self_as_f64).is_loosely_equal(vm, other)?
            }
            (_, Value::Boolean(_)) => {
                let other_as_f64 = other.try_into_f64(vm)?;
                Value::from_f64(vm, other_as_f64).is_loosely_equal(vm, self)?
            }
            (Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Symbol(_), _) => {
                other.is_loosely_equal(vm, &self.to_primitive()?)?
            }
            (
                Value::Object(_),
                Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Symbol(_),
            ) => self.to_primitive()?.is_loosely_equal(vm, other)?,
            _ => false,
        })
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-isstrictlyequal
    pub fn is_strictly_equal(&self, vm: &VM, other: &Value) -> JsResult<bool> {
        if self.get_type() != other.get_type() {
            return Ok(false);
        }

        Ok(match (self, other) {
            (Value::SmiU(n1), Value::NegativeZero) | (Value::NegativeZero, Value::SmiU(n1)) => {
                *n1 == 0
            }
            (Value::Smi(n1) | Value::SmallBigInt(n1), Value::Smi(n2) | Value::SmallBigInt(n2)) => {
                n1 == n2
            }
            (
                Value::SmiU(n1) | Value::SmallBigIntU(n1),
                Value::SmiU(n2) | Value::SmallBigIntU(n2),
            ) => n1 == n2,

            (Value::Number(n1), Value::Number(n2)) => {
                n1 == n2 || vm.heap_numbers[*n1 as usize].data == vm.heap_numbers[*n2 as usize].data
            }

            // https://tc39.es/ecma262/multipage/abstract-operations.html#sec-samevaluenonnumber
            (Value::Null | Value::Undefined, _) => true,
            (Value::BigInt(n1), Value::BigInt(n2)) => n1 == n2,
            (Value::String(s1), Value::String(s2)) => {
                s1 == s2 || vm.strings[*s1 as usize].data == vm.strings[*s2 as usize].data
            }
            (Value::Boolean(b1), Value::Boolean(b2)) => b1 == b2,
            // TODO: implement x is y procedures
            (Value::Object(obj1), Value::Object(obj2)) => obj1 == obj2,
            _ => false,
        })
    }

    pub fn to_primitive(&self) -> JsResult<Value> {
        Ok(Value::Null)
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toboolean
    pub fn to_boolean(&self) -> Value {
        match self {
            &Value::Boolean(b) => Value::Boolean(b),
            &Value::SmiU(n) => Value::Boolean(n == 0),
            Value::Null | Value::EmptyString | Value::NaN | Value::NegativeZero => {
                Value::Boolean(false)
            }
            _ => Value::Boolean(true),
        }
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
    pub fn to_number(&self, _vm: &mut VM) -> JsResult<Value> {
        Ok(match self {
            Value::Number(_)
            | Value::Smi(_)
            | Value::SmiU(_)
            | Value::Infinity
            | Value::NegativeInfinity
            | Value::NegativeZero => self.clone(),
            Value::Symbol(_)
            | Value::BigInt(_)
            | Value::SmallBigInt(_)
            | Value::SmallBigIntU(_) => todo!("type error"),
            Value::Undefined | Value::NaN => Value::NaN,
            Value::Null | Value::Boolean(false) | Value::EmptyString => Value::SmiU(0),
            Value::Boolean(true) => Value::SmiU(1),
            Value::String(_) => todo!("parse number from string"),
            Value::Object(_) => todo!("call valueOf"),
        })
    }

    pub fn from_f64(vm: &mut VM, value: f64) -> Value {
        let is_int = value.fract() == 0.0;
        if value.is_nan() {
            Value::NaN
        } else if value.is_infinite() {
            if value.is_sign_positive() {
                Value::Infinity
            } else {
                Value::NegativeInfinity
            }
        } else if !is_int || value > u32::MAX as f64 || value < i32::MIN as f64 {
            vm.heap_numbers.push(Gc::new(HeapNumber::new(value)));
            Value::Number(vm.heap_numbers.len() as u32)
        } else if value.is_sign_positive() {
            Value::SmiU(value as u32)
        } else {
            Value::Smi(value as i32)
        }
    }

    pub fn try_into_f64(&self, vm: &mut VM) -> JsResult<f64> {
        match self {
            &Value::Number(n) => Ok(vm.heap_numbers[n as usize].data),
            &Value::Smi(n) => Ok(n as f64),
            &Value::SmiU(n) => Ok(n as f64),
            Value::Infinity => Ok(f64::INFINITY),
            Value::NegativeInfinity => Ok(f64::NEG_INFINITY),
            Value::NegativeZero => Ok(0.),
            Value::Undefined | Value::NaN => Ok(f64::NAN),
            Value::Symbol(_)
            | Value::BigInt(_)
            | Value::SmallBigInt(_)
            | Value::SmallBigIntU(_) => todo!("type error"),
            Value::Null | Value::Boolean(false) | Value::EmptyString => Ok(0.),
            Value::Boolean(true) => Ok(1.),
            Value::String(_) => todo!("parse number from string"),
            Value::Object(_) => todo!("call valueOf"),
        }
    }

    pub fn into_bool(&self) -> bool {
        match self {
            &Value::Boolean(b) => b,
            &Value::SmiU(n) => n == 0,
            Value::Null | Value::EmptyString | Value::NaN | Value::NegativeZero => false,
            _ => true,
        }
    }

    pub fn from_u32(value: u32) -> Value {
        Value::SmiU(value)
    }

    pub fn from_i32(value: i32) -> Value {
        if value >= 0 {
            Value::from_u32(value as u32)
        } else {
            Value::Smi(value)
        }
    }
}

type JsResult<T> = std::result::Result<T, Value>;

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Undefined => write!(f, "Undefined"),
            Value::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Value::Number(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Value::Smi(arg0) => f.debug_tuple("Smi").field(arg0).finish(),
            Value::SmiU(arg0) => f.debug_tuple("SmiU").field(arg0).finish(),
            Value::BigInt(arg0) => f.debug_tuple("BigInt").field(arg0).finish(),
            Value::SmallBigInt(arg0) => f.debug_tuple("SmallBigInt").field(arg0).finish(),
            Value::SmallBigIntU(arg0) => f.debug_tuple("SmallBigIntU").field(arg0).finish(),
            Value::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Value::Object(arg0) => f.debug_tuple("JsObject").field(arg0).finish(),
            Value::Symbol(arg0) => f.debug_tuple("Symbol").field(arg0).finish(),
            Value::EmptyString => write!(f, "EmptyString"),
            Value::NaN => write!(f, "NaN"),
            Value::Infinity => write!(f, "Infinity"),
            Value::NegativeInfinity => write!(f, "-Infinity"),
            Value::NegativeZero => write!(f, "-0"),
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

#[derive(Clone)]
pub struct HeapNumber {
    data: f64,
}
impl Finalize for HeapNumber {
    fn finalize(&self) {}
}
unsafe impl Trace for HeapNumber {
    unsafe_empty_trace!();
}

impl HeapNumber {
    pub fn new(data: f64) -> HeapNumber {
        HeapNumber { data }
    }
}

#[derive(Clone)]
pub struct HeapBigInt {
    sign: bool,
    len: u32,
    parts: Box<[u64]>,
}
impl Finalize for HeapBigInt {
    fn finalize(&self) {}
}
unsafe impl Trace for HeapBigInt {
    unsafe_empty_trace!();
}

#[derive(Trace, Finalize)]
pub struct JsSymbol {
    descriptor: Option<usize>,
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
    pub objects: Vec<Gc<dyn JsObject>>,
    pub heap_numbers: Vec<Gc<HeapNumber>>,
    pub heap_bigints: Vec<Gc<HeapBigInt>>,
    pub symbols: Vec<Gc<JsSymbol>>,
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
