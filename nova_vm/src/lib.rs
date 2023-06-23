#![feature(coerce_unsized)]

mod byte_compiler;
mod context;
mod vm;

pub use byte_compiler::ByteCompiler;
pub use context::Context;
use gc::{unsafe_empty_trace, Finalize, Gc, GcCell, Trace};
use hashbrown::HashMap;
use oxc_ast::{
    ast::{
        AssignmentOperator, AssignmentTarget, BinaryOperator, BindingPatternKind, Declaration,
        Expression, LogicalExpression, LogicalOperator, ObjectProperty, Program, Property,
        PropertyValue, SimpleAssignmentTarget, Statement, VariableDeclaration,
        VariableDeclarationKind, VariableDeclarator,
    },
    syntax_directed_operations::PropName,
};
use oxc_parser::*;
use std::{
    borrow::Borrow,
    fmt::Debug,
    hash::Hash,
    ops::{CoerceUnsized, ControlFlow},
};
pub use vm::VM;
use wtf8::{Wtf8, Wtf8Buf};

// Completely unoptimized...look away.
#[derive(Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    String(Gc<JsString>),
    Symbol(Gc<JsSymbol>),
    Number(f64),
    BigInt(i64),
    Object(Gc<GcCell<dyn JsObject>>),
}

impl Finalize for Value {}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Value::Boolean(x) => x.hash(state),
            Value::String(x) => x.hash(state),
            Value::Symbol(x) => x.hash(state),
            Value::Number(x) => (unsafe { std::mem::transmute::<f64, u64>(*x) }).hash(state),
            Value::BigInt(x) => x.hash(state),
            Value::Object(x) => {
                // TODO: validate this - very hacky
                let obj = &**x;
                let obj = obj.borrow();
                ((&*obj) as *const dyn JsObject).hash(state)
            }
            _ => {}
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.is_strictly_equal(other).unwrap_or(false)
    }
}

unsafe impl Trace for Value {
    unsafe fn trace(&self) {
        match self {
            Value::String(x) => x.trace(),
            Value::Symbol(x) => x.trace(),
            Value::Object(x) => x.trace(),
            _ => {}
        }
    }

    unsafe fn root(&self) {
        match self {
            Value::String(x) => x.root(),
            Value::Symbol(x) => x.root(),
            Value::Object(x) => x.root(),
            _ => {}
        }
    }

    unsafe fn unroot(&self) {
        match self {
            Value::String(x) => x.unroot(),
            Value::Symbol(x) => x.unroot(),
            Value::Object(x) => x.unroot(),
            _ => {}
        }
    }

    fn finalize_glue(&self) {
        match self {
            Value::String(x) => x.finalize_glue(),
            Value::Symbol(x) => x.finalize_glue(),
            Value::Object(x) => x.finalize_glue(),
            _ => {}
        }
    }
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
                Gc::<GcCell<(dyn JsObject + 'static)>>::ptr_eq(&obj1, obj2)
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

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tostring
    pub fn to_string(&self) -> JsResult<Gc<JsString>> {
        Ok(match self {
            Value::String(s) => s.clone(),
            Value::Symbol(_) => todo!("type error"),
            Value::Undefined => Gc::new(JsString {
                data: Wtf8Buf::from_str("undefined"),
            }),
            Value::Null => Gc::new(JsString {
                data: Wtf8Buf::from_str("null"),
            }),
            &Value::Boolean(b) => Gc::new(JsString {
                data: Wtf8Buf::from_str(if b { "true" } else { "false" }),
            }),
            Value::Number(n) => Gc::new(JsString {
                data: Wtf8Buf::from_string(n.to_string()),
            }),
            Value::BigInt(_) => todo!(),
            other => todo!("{other:?}"),
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

pub trait JsObject: Trace + Debug {
    fn get_prototype_of(&mut self) -> JsResult<Option<Gc<dyn JsObject>>>;
    // fn set_prototype_of(&mut self) -> JsResult<Option<Gc<dyn JsObject>>>;
    // fn is_extensible(&mut self) -> JsResult<bool>;
    // fn prevent_extensions(&mut self) -> JsResult<bool>;
    fn get_own_property(&mut self, key: Value) -> JsResult<Option<Value>>;
    fn set_own_property(&mut self, key: Value, value2: Value) -> JsResult<()>;
}

#[derive(Debug, Default)]
pub struct BasicObject {
    pub entries: HashMap<Value, Value>,
    pub prototype: Option<Gc<dyn JsObject>>,
}

impl Finalize for BasicObject {}

impl JsObject for BasicObject {
    fn get_prototype_of(&mut self) -> JsResult<Option<Gc<dyn JsObject>>> {
        Ok(self.prototype.clone())
    }

    fn get_own_property(&mut self, key: Value) -> JsResult<Option<Value>> {
        Ok(self.entries.get(&key).map(|x| x.clone()))
    }

    fn set_own_property(&mut self, key: Value, value: Value) -> JsResult<()> {
        self.entries.insert(key, value);
        Ok(())
    }
}

unsafe impl Trace for BasicObject {
    unsafe fn trace(&self) {
        for (key, value) in self.entries.iter() {
            key.trace();
            value.trace();
        }
    }

    unsafe fn root(&self) {
        for (key, value) in self.entries.iter() {
            key.root();
            value.root();
        }
    }

    unsafe fn unroot(&self) {
        for (key, value) in self.entries.iter() {
            key.unroot();
            value.unroot();
        }
    }

    fn finalize_glue(&self) {
        for (key, value) in self.entries.iter() {
            key.finalize_glue();
            value.finalize_glue();
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct JsString {
    data: Wtf8Buf,
}

impl Finalize for JsString {
    fn finalize(&self) {}
}
unsafe impl Trace for JsString {
    unsafe_empty_trace!();
}

#[derive(Hash, PartialEq, Eq)]
pub struct JsSymbol {
    descriptor: Option<Gc<JsString>>,
}

impl Finalize for JsSymbol {}

unsafe impl Trace for JsSymbol {
    unsafe fn trace(&self) {
        if let Some(x) = &self.descriptor {
            x.trace();
        }
    }

    unsafe fn root(&self) {
        if let Some(x) = &self.descriptor {
            x.root();
        }
    }

    unsafe fn unroot(&self) {
        if let Some(x) = &self.descriptor {
            x.unroot();
        }
    }

    fn finalize_glue(&self) {
        if let Some(x) = &self.descriptor {
            x.finalize_glue();
        }
    }
}
