// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::marker::PhantomData;

use small_string::SmallString;

use crate::{
    ecmascript::{
        execution::Agent,
        types::{
            bigint::SmallBigInt, BigInt, IntoObject, IntoValue, Number, String, Symbol, Value,
            BOOLEAN_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT, NULL_DISCRIMINANT,
            SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT, UNDEFINED_DISCRIMINANT,
        },
    },
    SmallInteger,
};

use super::small_f64::SmallF64;

/// # Scoped JavaScript Value.
///
/// This holds either an on-stack primitive JavaScript Value, or an index to a
/// scoped heap-allocated Value. This type is intended for cheap rooting of
/// JavaScript Values that need to be used after calling into functions that
/// may trigger garbage collection.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Local<T: Sized + IntoValue + TryFrom<Value>> {
    inner: LocalInner,
    _marker: PhantomData<T>,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum LocalInner {
    /// ### [6.1.1 The Undefined Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type)
    Undefined = UNDEFINED_DISCRIMINANT,

    /// ### [6.1.2 The Null Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-null-type)
    Null = NULL_DISCRIMINANT,

    /// ### [6.1.3 The Boolean Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-boolean-type)
    Boolean(bool) = BOOLEAN_DISCRIMINANT,

    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// 7-byte UTF-8 string on the stack. End of the string is determined by
    /// the first 0xFF byte in the data. UTF-16 indexing is calculated on
    /// demand from the data.
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,

    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 53-bit signed integer on the stack.
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 56-bit f64 on the stack. The missing byte is a zero least significant
    /// byte.
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,

    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// 56-bit signed integer on the stack.
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,

    /// ### Scoped Value
    ///
    /// Value stored in the current scope stack.
    ScopedValue(u32) = 0x80,
}

impl Local<Value> {
    pub fn get(self, agent: &Agent) -> Value {
        match self.inner {
            LocalInner::Undefined => Value::Undefined,
            LocalInner::Null => Value::Null,
            LocalInner::Boolean(bool) => Value::Boolean(bool),
            LocalInner::SmallString(small_string) => Value::SmallString(small_string),
            LocalInner::Integer(small_integer) => Value::Integer(small_integer),
            LocalInner::SmallF64(small_f64) => Value::SmallF64(small_f64),
            LocalInner::SmallBigInt(small_big_int) => Value::SmallBigInt(small_big_int),
            LocalInner::ScopedValue(index) => {
                let Some(&value) = agent.stack_values.borrow().get(index as usize) else {
                    handle_bound_check_failure()
                };
                value
            }
        }
    }
}

impl Local<String> {
    pub fn get(self, agent: &Agent) -> String {
        match self.inner {
            LocalInner::SmallString(small_string) => String::SmallString(small_string),
            LocalInner::ScopedValue(index) => {
                let Some(&value) = agent.stack_values.borrow().get(index as usize) else {
                    handle_bound_check_failure()
                };
                let Value::String(string) = value else {
                    unreachable!()
                };
                String::String(string)
            }
            _ => unreachable!(),
        }
    }
}

impl Local<Number> {
    pub fn get(self, agent: &Agent) -> Number {
        match self.inner {
            LocalInner::Integer(integer) => Number::Integer(integer),
            LocalInner::SmallF64(float) => Number::SmallF64(float),
            LocalInner::ScopedValue(index) => {
                let Some(&value) = agent.stack_values.borrow().get(index as usize) else {
                    handle_bound_check_failure()
                };
                let Value::Number(number) = value else {
                    unreachable!()
                };
                Number::Number(number)
            }
            _ => unreachable!(),
        }
    }
}

impl Local<BigInt> {
    pub fn get(self, agent: &Agent) -> BigInt {
        match self.inner {
            LocalInner::SmallBigInt(small_bigint) => BigInt::SmallBigInt(small_bigint),
            LocalInner::ScopedValue(index) => {
                let Some(&value) = agent.stack_values.borrow().get(index as usize) else {
                    handle_bound_check_failure()
                };
                let Value::BigInt(bigint) = value else {
                    unreachable!()
                };
                BigInt::BigInt(bigint)
            }
            _ => unreachable!(),
        }
    }
}

impl Local<Symbol> {
    pub fn get(self, agent: &Agent) -> Symbol {
        match self.inner {
            LocalInner::ScopedValue(index) => {
                let Some(&value) = agent.stack_values.borrow().get(index as usize) else {
                    handle_bound_check_failure()
                };
                let Value::Symbol(value) = value else {
                    unreachable!()
                };
                value
            }
            _ => unreachable!(),
        }
    }
}

impl<T: Sized + IntoObject + TryFrom<Value>> Local<T> {
    pub fn get(self, agent: &Agent) -> T {
        match self.inner {
            LocalInner::ScopedValue(index) => {
                let Some(&value) = agent.stack_values.borrow().get(index as usize) else {
                    handle_bound_check_failure();
                };
                let Ok(value) = T::try_from(value) else {
                    unreachable!()
                };
                value
            }
            _ => unreachable!(),
        }
    }
}

impl Value {
    pub fn root(self, agent: &Agent) -> Local<Value> {
        let inner = match self {
            Value::Undefined => LocalInner::Undefined,
            Value::Null => LocalInner::Null,
            Value::Boolean(bool) => LocalInner::Boolean(bool),
            Value::SmallString(small_string) => LocalInner::SmallString(small_string),
            Value::Integer(small_integer) => LocalInner::Integer(small_integer),
            Value::SmallF64(small_f64) => LocalInner::SmallF64(small_f64),
            Value::SmallBigInt(small_big_int) => LocalInner::SmallBigInt(small_big_int),
            _ => {
                let stack_values = agent.stack_values.borrow_mut();
                let Ok(index) = u32::try_from(stack_values.len()) else {
                    handle_index_overflow();
                };
                agent.stack_values.borrow_mut().push(self);
                LocalInner::ScopedValue(index)
            }
        };

        Local {
            inner,
            _marker: PhantomData,
        }
    }
}

impl String {
    pub fn root(self, agent: &Agent) -> Local<String> {
        let inner = match self {
            String::SmallString(small_string) => LocalInner::SmallString(small_string),
            String::String(string) => {
                let stack_values = agent.stack_values.borrow_mut();
                let Ok(index) = u32::try_from(stack_values.len()) else {
                    handle_index_overflow();
                };
                agent.stack_values.borrow_mut().push(Value::String(string));
                LocalInner::ScopedValue(index)
            }
        };

        Local {
            inner,
            _marker: PhantomData,
        }
    }
}

impl Number {
    pub fn root(self, agent: &Agent) -> Local<Number> {
        let inner = match self {
            Number::Integer(number) => LocalInner::Integer(number),
            Number::SmallF64(float) => LocalInner::SmallF64(float),
            Number::Number(number) => {
                let stack_values = agent.stack_values.borrow_mut();
                let Ok(index) = u32::try_from(stack_values.len()) else {
                    handle_index_overflow();
                };
                agent.stack_values.borrow_mut().push(Value::Number(number));
                LocalInner::ScopedValue(index)
            }
        };

        Local {
            inner,
            _marker: PhantomData,
        }
    }
}

impl BigInt {
    pub fn root(self, agent: &Agent) -> Local<BigInt> {
        let inner = match self {
            BigInt::SmallBigInt(small_bigint) => LocalInner::SmallBigInt(small_bigint),
            BigInt::BigInt(bigint) => {
                let stack_values = agent.stack_values.borrow_mut();
                let Ok(index) = u32::try_from(stack_values.len()) else {
                    handle_index_overflow();
                };
                agent.stack_values.borrow_mut().push(Value::BigInt(bigint));
                LocalInner::ScopedValue(index)
            }
        };

        Local {
            inner,
            _marker: PhantomData,
        }
    }
}

impl Symbol {
    pub fn root(self, agent: &Agent) -> Local<Symbol> {
        let stack_values = agent.stack_values.borrow_mut();
        let Ok(index) = u32::try_from(stack_values.len()) else {
            handle_index_overflow();
        };
        agent.stack_values.borrow_mut().push(Value::Symbol(self));
        let inner = LocalInner::ScopedValue(index);

        Local {
            inner,
            _marker: PhantomData,
        }
    }
}

pub trait ObjectScopeRoot
where
    Self: Sized + IntoObject + TryFrom<Value>,
{
    fn root(self, agent: &Agent) -> Local<Self> {
        let value = self.into_value();
        let mut stack_values = agent.stack_values.borrow_mut();
        let Ok(index) = u32::try_from(stack_values.len()) else {
            handle_index_overflow();
        };
        stack_values.push(value);
        let inner = LocalInner::ScopedValue(index);

        Local {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<T> ObjectScopeRoot for T where T: Sized + IntoObject + TryFrom<Value> {}

#[cold]
#[inline(never)]
fn handle_index_overflow() -> ! {
    panic!("Local Values stack overflowed");
}

#[cold]
#[inline(never)]
fn handle_bound_check_failure() -> ! {
    panic!("Attempted to access dropped Local Value")
}
