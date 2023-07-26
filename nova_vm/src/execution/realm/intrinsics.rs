use crate::{
    heap::BuiltinObjectIndexes,
    types::{Object, Value},
};

// TODO: We should probably consider lazily loading intrinsics. This would
//       contain a mutable reference to [`Realm`] and be created via a
//       `Realm::intrinsic()` method to guarantee safety.

pub struct Intrinsics;

impl Intrinsics {
    /// %Array%
    pub fn array() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::ArrayConstructorIndex.into(),
        ))
    }

    /// %Array.prototype%
    pub fn array_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::ArrayPrototypeIndex.into(),
        ))
    }

    /// %BigInt%
    pub fn big_int() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::BigintConstructorIndex.into(),
        ))
    }

    /// %BigInt.prototype%
    pub fn big_int_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::BigintPrototypeIndex.into(),
        ))
    }

    /// %Boolean%
    pub fn boolean() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::BooleanConstructorIndex.into(),
        ))
    }

    /// %Boolean.prototype%
    pub fn boolean_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::BooleanPrototypeIndex.into(),
        ))
    }

    /// %Error%
    pub fn error() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::ErrorConstructorIndex.into(),
        ))
    }

    /// %Error.prototype%
    pub fn error_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::ErrorPrototypeIndex.into(),
        ))
    }

    /// %eval%
    pub fn eval() -> Object {
        todo!()
    }

    /// %EvalError%
    pub fn eval_error() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::ArrayConstructorIndex.into(),
        ))
    }

    /// %EvalError.prototype%
    pub fn eval_error_prototype() -> Object {
        todo!()
    }

    /// %Function%
    pub fn function() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::FunctionConstructorIndex.into(),
        ))
    }

    /// %Function.prototype%
    pub fn function_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        ))
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
        Object::new(Value::Object(BuiltinObjectIndexes::MathObjectIndex.into()))
    }

    /// %Number%
    pub fn number() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::NumberConstructorIndex.into(),
        ))
    }

    /// %Number.prototype%
    pub fn number_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::NumberPrototypeIndex.into(),
        ))
    }

    /// %Object%
    pub fn object() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::ObjectConstructorIndex.into(),
        ))
    }

    /// %Object.prototype%
    pub fn object_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        ))
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
        Object::new(Value::Object(
            BuiltinObjectIndexes::StringConstructorIndex.into(),
        ))
    }

    /// %String.prototype%
    pub fn string_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::StringPrototypeIndex.into(),
        ))
    }

    /// %Symbol%
    pub fn symbol() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::SymbolConstructorIndex.into(),
        ))
    }

    /// %Symbol.prototype%
    pub fn symbol_prototype() -> Object {
        Object::new(Value::Object(
            BuiltinObjectIndexes::SymbolPrototypeIndex.into(),
        ))
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
