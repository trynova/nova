use crate::{
    heap::{BuiltinObjectIndexes, Handle},
    types::{Object, Value},
};

// TODO: We should probably consider lazily loading intrinsics. This would
//       contain a mutable reference to [`Realm`] and be created via a
//       `Realm::intrinsic()` method to guarantee safety.

pub struct Intrinsics;

impl Intrinsics {
    /// %Array%
    pub fn array() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::ArrayConstructorIndex as u32,
        )))
    }

    /// %Array.prototype%
    pub fn array_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::ArrayPrototypeIndex as u32,
        )))
    }

    /// %BigInt%
    pub fn big_int() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::BigintConstructorIndex as u32,
        )))
    }

    /// %BigInt.prototype%
    pub fn big_int_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::BigintPrototypeIndex as u32,
        )))
    }

    /// %Boolean%
    pub fn boolean() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::BooleanConstructorIndex as u32,
        )))
    }

    /// %Boolean.prototype%
    pub fn boolean_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::BooleanPrototypeIndex as u32,
        )))
    }

    /// %Error%
    pub fn error() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::ErrorConstructorIndex as u32,
        )))
    }

    /// %Error.prototype%
    pub fn error_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::ErrorPrototypeIndex as u32,
        )))
    }

    /// %eval%
    pub fn eval() -> Object {
        todo!()
    }

    /// %EvalError%
    pub fn eval_error() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::ArrayConstructorIndex as u32,
        )))
    }

    /// %EvalError.prototype%
    pub fn eval_error_prototype() -> Object {
        todo!()
    }

    /// %Function%
    pub fn function() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::FunctionConstructorIndex as u32,
        )))
    }

    /// %Function.prototype%
    pub fn function_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::FunctionPrototypeIndex as u32,
        )))
    }

    /// %isFinite%
    pub fn is_finite() -> Object {
        todo!()
    }

    /// %isNaN%
    pub fn is_nan() -> Object {
        todo!()
    }

    /// %Math%
    pub fn math() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::MathObjectIndex as u32,
        )))
    }

    /// %Number%
    pub fn number() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::NumberConstructorIndex as u32,
        )))
    }

    /// %Number.prototype%
    pub fn number_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::NumberPrototypeIndex as u32,
        )))
    }

    /// %Object%
    pub fn object() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::ObjectConstructorIndex as u32,
        )))
    }

    /// %Object.prototype%
    pub fn object_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::ObjectPrototypeIndex as u32,
        )))
    }

    /// %Object.prototype.toString%
    pub fn object_prototype_to_string() -> Object {
        todo!()
    }

    /// %RangeError%
    pub fn range_error() -> Object {
        todo!()
    }

    /// %RangeError.prototype%
    pub fn range_error_prototype() -> Object {
        todo!()
    }

    /// %ReferenceError%
    pub fn reference_error() -> Object {
        todo!()
    }

    /// %ReferenceError.prototype%
    pub fn reference_error_prototype() -> Object {
        todo!()
    }

    /// %Reflect%
    pub fn reflect() -> Object {
        todo!()
    }

    /// %String%
    pub fn string() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::StringConstructorIndex as u32,
        )))
    }

    /// %String.prototype%
    pub fn string_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::StringPrototypeIndex as u32,
        )))
    }

    /// %Symbol%
    pub fn symbol() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::SymbolConstructorIndex as u32,
        )))
    }

    /// %Symbol.prototype%
    pub fn symbol_prototype() -> Object {
        Object::new(Value::Object(Handle::new(
            BuiltinObjectIndexes::SymbolPrototypeIndex as u32,
        )))
    }

    /// %SyntaxError%
    pub fn syntax_error() -> Object {
        todo!()
    }

    /// %SyntaxError.prototype%
    pub fn syntax_error_prototype() -> Object {
        todo!()
    }

    /// %ThrowTypeError%
    pub fn throw_type_error() -> Object {
        todo!()
    }

    /// %TypeError%
    pub fn type_error() -> Object {
        todo!()
    }

    /// %TypeError.prototype%
    pub fn type_error_prototype() -> Object {
        todo!()
    }

    /// %URIError%
    pub fn uri_error() -> Object {
        todo!()
    }

    /// %URIError.prototype%
    pub fn uri_error_prototype() -> Object {
        todo!()
    }
}
