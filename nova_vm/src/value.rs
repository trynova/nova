use crate::{heap::NumberHeapData, Type, VM};
use std::{fmt::Debug, mem::size_of};

// TODO(@aapoalas): Use transparent struct (u32)'s to ensure proper indexing.
pub type StringIndex = u32;
pub type SymbolIndex = u32;
pub type NumberIndex = u32;
pub type BigIntIndex = u32;
pub type ObjectIndex = u32;
pub type FunctionIndex = u32;

// Completely unoptimized...look away.
#[derive(Clone)]
#[repr(u8)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    EmptyString,
    SmallAsciiString([i8; 7]),
    String(StringIndex),
    Symbol(SymbolIndex),
    Smi(i32),
    SmiU(u32),
    NaN,
    Infinity,
    NegativeInfinity,
    NegativeZero,
    Number(NumberIndex),
    SmallBigInt(i32),
    SmallBigIntU(u32),
    BigInt(BigIntIndex),
    Object(ObjectIndex),
    Function(FunctionIndex),
}

const VALUE_SIZE_IS_WORD: () = assert!(size_of::<Value>() == size_of::<usize>());

impl Value {
    pub fn new_string(vm: &mut VM, message: &str) -> Value {
        let _ = VALUE_SIZE_IS_WORD;
        Value::String(vm.heap.alloc_string(message))
    }

    pub fn create_exception(vm: &mut VM, message: &str) -> Value {
        let _ = VALUE_SIZE_IS_WORD;
        Value::String(vm.heap.alloc_string(message))
    }

    pub fn get_type(&self) -> Type {
        let _ = VALUE_SIZE_IS_WORD;
        match self {
            Value::Undefined => Type::Undefined,
            Value::Null => Type::Null,
            Value::Boolean(_) => Type::Boolean,
            Value::EmptyString | Value::SmallAsciiString(_) | Value::String(_) => Type::String,
            Value::Symbol(_) => Type::Symbol,
            Value::NaN
            | Value::NegativeInfinity
            | Value::NegativeZero
            | Value::Infinity
            | Value::Smi(_)
            | Value::SmiU(_)
            | Value::Number(_) => Type::Number,
            Value::SmallBigInt(_) | Value::SmallBigIntU(_) | Value::BigInt(_) => Type::BigInt,
            Value::Function(_) => Type::Function,
            Value::Object(_) => Type::Object,
        }
    }

    /// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islooselyequal
    pub fn is_loosely_equal(&self, vm: &mut VM, other: &Value) -> JsResult<bool> {
        let _ = VALUE_SIZE_IS_WORD;
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
                let big_int = &vm.heap.bigints[x as usize];
                let number = &vm.heap.numbers[y as usize];
                big_int.as_ref().unwrap().len == 1
                    && big_int.as_ref().unwrap().parts[0] as f64 == number.as_ref().unwrap().data
            }
            (&Value::Number(x), &Value::BigInt(y)) => {
                let big_int = &vm.heap.bigints[y as usize];
                let number = &vm.heap.numbers[x as usize];
                big_int.as_ref().unwrap().len == 1
                    && big_int.as_ref().unwrap().parts[0] as f64 == number.as_ref().unwrap().data
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
        let _ = VALUE_SIZE_IS_WORD;
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
                n1 == n2
                    || vm.heap.numbers[*n1 as usize].as_ref().unwrap().data
                        == vm.heap.numbers[*n2 as usize].as_ref().unwrap().data
            }

            // https://tc39.es/ecma262/multipage/abstract-operations.html#sec-samevaluenonnumber
            (Value::Null | Value::Undefined, _) => true,
            (Value::BigInt(n1), Value::BigInt(n2)) => n1 == n2,
            (Value::String(s1), Value::String(s2)) => {
                s1 == s2
                    || vm.heap.strings[*s1 as usize].as_ref().unwrap().data
                        == vm.heap.strings[*s2 as usize].as_ref().unwrap().data
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
            Value::Function(_)
            | Value::Symbol(_)
            | Value::BigInt(_)
            | Value::SmallBigInt(_)
            | Value::SmallBigIntU(_) => todo!("type error"),
            Value::Undefined | Value::NaN => Value::NaN,
            Value::Null | Value::Boolean(false) | Value::EmptyString => Value::SmiU(0),
            Value::Boolean(true) => Value::SmiU(1),
            Value::SmallAsciiString(_) | Value::String(_) => todo!("parse number from string"),
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
            vm.heap.numbers.push(Some(NumberHeapData::new(value)));
            Value::Number(vm.heap.numbers.len() as u32)
        } else if value.is_sign_positive() {
            Value::SmiU(value as u32)
        } else {
            Value::Smi(value as i32)
        }
    }

    pub fn try_into_f64(&self, vm: &mut VM) -> JsResult<f64> {
        match self {
            &Value::Number(n) => Ok(vm.heap.numbers[n as usize].as_ref().unwrap().data),
            &Value::Smi(n) => Ok(n as f64),
            &Value::SmiU(n) => Ok(n as f64),
            Value::Infinity => Ok(f64::INFINITY),
            Value::NegativeInfinity => Ok(f64::NEG_INFINITY),
            Value::NegativeZero => Ok(0.),
            Value::Undefined | Value::NaN => Ok(f64::NAN),
            Value::Function(_)
            | Value::Symbol(_)
            | Value::BigInt(_)
            | Value::SmallBigInt(_)
            | Value::SmallBigIntU(_) => todo!("type error"),
            Value::Null | Value::Boolean(false) | Value::EmptyString => Ok(0.),
            Value::Boolean(true) => Ok(1.),
            Value::SmallAsciiString(_) | Value::String(_) => todo!("parse number from string"),
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

pub type JsResult<T> = std::result::Result<T, Value>;

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
            Value::SmallAsciiString(arg0) => f.debug_tuple("SmallAsciiString").field(arg0).finish(),
            Value::Object(arg0) => f.debug_tuple("JsObject").field(arg0).finish(),
            Value::Symbol(arg0) => f.debug_tuple("Symbol").field(arg0).finish(),
            Value::Function(arg0) => f.debug_tuple("Function").field(arg0).finish(),
            Value::EmptyString => write!(f, "EmptyString"),
            Value::NaN => write!(f, "NaN"),
            Value::Infinity => write!(f, "Infinity"),
            Value::NegativeInfinity => write!(f, "-Infinity"),
            Value::NegativeZero => write!(f, "-0"),
        }
    }
}
