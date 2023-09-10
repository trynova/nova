use crate::{
    heap::{
        indexes::{
            ArrayIndex, BigIntIndex, DateIndex, ErrorIndex, FunctionIndex, NumberIndex,
            ObjectIndex, RegExpIndex, StringIndex, SymbolIndex,
        },
        Heap,
    },
    stack_string::StackString,
    Type, VM,
};
use std::{fmt::Debug, mem::size_of};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Value {
    Array(ArrayIndex),
    BigIntObject(u32),  // TODO: Implement primitive value objects :(
    BooleanObject(u32), // TODO: Implement primitive value objects :(
    Boolean(bool),
    Date(DateIndex),
    EmptyString,
    Error(ErrorIndex),
    Function(FunctionIndex),
    HeapBigInt(BigIntIndex),
    HeapNumber(NumberIndex),
    HeapString(StringIndex),
    Infinity,
    NaN,
    NegativeInfinity,
    NegativeZero,
    Null,
    NumberObject(u32), // TODO: Implement primitive value objects :(
    Object(ObjectIndex),
    RegExp(RegExpIndex),
    StackString(StackString),
    // TOO: Extend these to i56.
    SmallBigInt(i32),
    SmallBigIntU(u32),
    // TODO: Extend these to i48 or even i56.
    // i56 would provide safe integer operations
    // superior to f64 but is no longer spec-compliant.
    Smi(i32),
    SmiU(u32),
    StringObject(u32), // TODO: Implement primitive value objects :(
    Symbol(SymbolIndex),
    SymbolObject(u32), // TODO: Implement primitive value objects :(
    Undefined,
}

/// We want to guarantee that all handles to JS values are register sized. This assert must never be removed or broken.
const _VALUE_SIZE_IS_WORD: () = assert!(size_of::<Value>() == size_of::<usize>());
// We may also want to keep Option<Value> register sized to allow returning it in some cases.
// This may not be possible in the long run and it may not be necessary as we might want to use Undefined instead.
const _OPTIONAL_VALUE_SIZE_IS_WORD: () = assert!(size_of::<Option<Value>>() == size_of::<usize>());

impl Value {
    pub fn new_string(heap: &mut Heap, message: &str) -> Value {
        if let Some(ascii_string) = StackString::try_from_str(message) {
            Value::StackString(ascii_string)
        } else {
            Value::HeapString(heap.alloc_string(message))
        }
    }

    pub fn create_exception(heap: &mut Heap, message: &str) -> Value {
        Value::HeapString(heap.alloc_string(message))
    }

    pub fn get_type(&self) -> Type {
        match self {
            Value::Boolean(_) => Type::Boolean,
            Value::EmptyString | Value::HeapString(_) | Value::StackString(_) => Type::String,
            Value::Function(_) => Type::Function,
            Value::HeapNumber(_)
            | Value::Infinity
            | Value::NaN
            | Value::NegativeInfinity
            | Value::NegativeZero
            | Value::Smi(_)
            | Value::SmiU(_) => Type::Number,
            Value::Null => Type::Null,
            Value::Object(_)
            | Value::Array(_)
            | Value::BigIntObject(_)
            | Value::BooleanObject(_)
            | Value::Date(_)
            | Value::Error(_)
            | Value::NumberObject(_)
            | Value::RegExp(_)
            | Value::StringObject(_)
            | Value::SymbolObject(_) => Type::Object,
            Value::HeapBigInt(_) | Value::SmallBigInt(_) | Value::SmallBigIntU(_) => Type::BigInt,
            Value::Symbol(_) => Type::Symbol,
            Value::Undefined => Type::Undefined,
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
            (&Value::HeapBigInt(x), &Value::HeapNumber(y)) => {
                let big_int = &vm.heap.bigints[x.into_index()];
                let number = &vm.heap.numbers[y.into_index()];
                big_int.as_ref().unwrap().try_into_f64() == Some(number.as_ref().unwrap().value())
            }
            (&Value::HeapNumber(x), &Value::HeapBigInt(y)) => {
                let big_int = &vm.heap.bigints[y.into_index()];
                let number = &vm.heap.numbers[x.into_index()];
                big_int.as_ref().unwrap().try_into_f64() == Some(number.as_ref().unwrap().value())
            }
            (Value::HeapNumber(_), Value::HeapString(_)) => todo!("use ToNumber() intrinsics"),
            (Value::HeapString(_), Value::HeapNumber(_)) => todo!("use ToNumber() intrinsics"),
            (Value::HeapBigInt(_), Value::HeapString(_)) => {
                todo!("use StringToBigInt() intrinsics")
            }
            (Value::HeapString(_), Value::HeapBigInt(_)) => other.is_loosely_equal(vm, self)?,
            (Value::Boolean(_), _) => {
                let self_as_f64 = self.try_into_f64(vm)?;
                Value::from_f64(&mut vm.heap, self_as_f64).is_loosely_equal(vm, other)?
            }
            (_, Value::Boolean(_)) => {
                let other_as_f64 = other.try_into_f64(vm)?;
                Value::from_f64(&mut vm.heap, other_as_f64).is_loosely_equal(vm, self)?
            }
            (
                Value::HeapString(_)
                | Value::HeapNumber(_)
                | Value::HeapBigInt(_)
                | Value::Symbol(_),
                _,
            ) => other.is_loosely_equal(vm, &self.to_primitive()?)?,
            (
                Value::Object(_),
                Value::HeapString(_)
                | Value::HeapNumber(_)
                | Value::HeapBigInt(_)
                | Value::Symbol(_),
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

            (Value::HeapNumber(n1), Value::HeapNumber(n2)) => {
                n1 == n2
                    || vm.heap.numbers[n1.into_index()].as_ref().unwrap().value()
                        == vm.heap.numbers[n2.into_index()].as_ref().unwrap().value()
            }

            // https://tc39.es/ecma262/multipage/abstract-operations.html#sec-samevaluenonnumber
            (Value::Null | Value::Undefined, _) => true,
            (Value::HeapBigInt(n1), Value::HeapBigInt(n2)) => n1 == n2,
            (Value::HeapString(s1), Value::HeapString(s2)) => {
                s1 == s2
                    || vm.heap.strings[s1.into_index()].as_ref().unwrap().data
                        == vm.heap.strings[s2.into_index()].as_ref().unwrap().data
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
            Value::HeapNumber(_)
            | Value::Smi(_)
            | Value::SmiU(_)
            | Value::Infinity
            | Value::NegativeInfinity
            | Value::NegativeZero => self.clone(),
            Value::Function(_)
            | Value::Symbol(_)
            | Value::HeapBigInt(_)
            | Value::SmallBigInt(_)
            | Value::SmallBigIntU(_) => todo!("type error"),
            Value::Undefined | Value::NaN => Value::NaN,
            Value::Null | Value::Boolean(false) | Value::EmptyString => Value::SmiU(0),
            Value::Boolean(true) => Value::SmiU(1),
            Value::StackString(_) | Value::HeapString(_) => todo!("parse number from string"),
            Value::Object(_) | Value::Error(_) => todo!("call valueOf"),
            _ => todo!("implement primitive value objects :("),
        })
    }

    pub fn from_f64(heap: &mut Heap, value: f64) -> Value {
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
            Value::HeapNumber(heap.alloc_number(value))
        } else if value.is_sign_positive() {
            Value::SmiU(value as u32)
        } else {
            Value::Smi(value as i32)
        }
    }

    pub fn try_into_f64(&self, vm: &mut VM) -> JsResult<f64> {
        match self {
            &Value::HeapNumber(n) => Ok(vm.heap.numbers[n.into_index()].as_ref().unwrap().value()),
            &Value::Smi(n) => Ok(n as f64),
            &Value::SmiU(n) => Ok(n as f64),
            Value::Infinity => Ok(f64::INFINITY),
            Value::NegativeInfinity => Ok(f64::NEG_INFINITY),
            Value::NegativeZero => Ok(0.),
            Value::Undefined | Value::NaN => Ok(f64::NAN),
            Value::Function(_)
            | Value::Symbol(_)
            | Value::HeapBigInt(_)
            | Value::SmallBigInt(_)
            | Value::SmallBigIntU(_) => todo!("type error"),
            Value::Null | Value::Boolean(false) | Value::EmptyString => Ok(0.),
            Value::Boolean(true) => Ok(1.),
            Value::StackString(_) | Value::HeapString(_) => todo!("parse number from string"),
            Value::Object(_) => todo!("call valueOf"),
            _ => todo!("sigh"),
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

    pub fn is_undefined(&self) -> bool {
        match self {
            Value::Undefined => true,
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Value::Null => true,
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Value::Smi(_) => true,
            Value::SmiU(_) => true,
            Value::NaN => true,
            Value::Infinity => true,
            Value::NegativeInfinity => true,
            Value::NegativeZero => true,
            Value::HeapNumber(_) => true,
            _ => false,
        }
    }
}

pub type JsResult<T> = std::result::Result<T, Value>;

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Value::BigIntObject(arg0) => f.debug_tuple("BigIntObject").field(arg0).finish(),
            Value::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Value::BooleanObject(arg0) => f.debug_tuple("BooleanObject").field(arg0).finish(),
            Value::EmptyString => write!(f, "EmptyString"),
            Value::Date(arg0) => f.debug_tuple("Date").field(arg0).finish(),
            Value::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
            Value::Function(arg0) => f.debug_tuple("Function").field(arg0).finish(),
            Value::HeapBigInt(arg0) => f.debug_tuple("BigInt").field(arg0).finish(),
            Value::HeapNumber(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Value::HeapString(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Value::Infinity => write!(f, "Infinity"),
            Value::NaN => write!(f, "NaN"),
            Value::NegativeInfinity => write!(f, "-Infinity"),
            Value::NegativeZero => write!(f, "-0"),
            Value::Null => write!(f, "Null"),
            Value::NumberObject(arg0) => f.debug_tuple("NumberObject").field(arg0).finish(),
            Value::Object(arg0) => f.debug_tuple("Object").field(arg0).finish(),
            Value::RegExp(arg0) => f.debug_tuple("RegExp").field(arg0).finish(),
            Value::StackString(arg0) => f.debug_tuple("SmallAsciiString").field(arg0).finish(),
            Value::SmallBigInt(arg0) => f.debug_tuple("SmallBigInt").field(arg0).finish(),
            Value::SmallBigIntU(arg0) => f.debug_tuple("SmallBigIntU").field(arg0).finish(),
            Value::Smi(arg0) => f.debug_tuple("Smi").field(arg0).finish(),
            Value::SmiU(arg0) => f.debug_tuple("SmiU").field(arg0).finish(),
            Value::StringObject(arg0) => f.debug_tuple("StringObject").field(arg0).finish(),
            Value::Symbol(arg0) => f.debug_tuple("Symbol").field(arg0).finish(),
            Value::SymbolObject(arg0) => f.debug_tuple("SymbolObject").field(arg0).finish(),
            Value::Undefined => write!(f, "Undefined"),
        }
    }
}
