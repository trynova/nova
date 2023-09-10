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
    pub const fn array() -> Object {
        Object::Function(BuiltinObjectIndexes::ArrayConstructorIndex.into())
    }

    /// %Array.prototype%
    pub const fn array_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::ArrayPrototypeIndex.into())
    }

    /// %BigInt%
    pub const fn big_int() -> Object {
        Object::Function(BuiltinObjectIndexes::BigintConstructorIndex.into())
    }

    /// %BigInt.prototype%
    pub const fn big_int_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::BigintPrototypeIndex.into())
    }

    /// %Boolean%
    pub const fn boolean() -> Object {
        Object::Function(BuiltinObjectIndexes::BooleanConstructorIndex.into())
    }

    /// %Boolean.prototype%
    pub const fn boolean_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::BooleanPrototypeIndex.into())
    }

    /// %Error%
    pub const fn error() -> Object {
        Object::Function(BuiltinObjectIndexes::ErrorConstructorIndex.into())
    }

    /// %Error.prototype%
    pub const fn error_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::ErrorPrototypeIndex.into())
    }

    /// %eval%
    pub const fn eval() -> Object {
        todo!()
    }

    /// %EvalError%
    pub const fn eval_error() -> Object {
        Object::Function(BuiltinObjectIndexes::ArrayConstructorIndex.into())
    }

    /// %EvalError.prototype%
    pub const fn eval_error_prototype() -> Object {
        todo!()
    }

    /// %Function%
    pub const fn function() -> Object {
        Object::Function(BuiltinObjectIndexes::FunctionConstructorIndex.into())
    }

    /// %Function.prototype%
    pub const fn function_prototype() -> Object {
        // Note: This is not spec-compliant. Function prototype should
        // be a function that always returns undefined no matter how
        // it is called. That's stupid so we do not have that.
        Object::Object(BuiltinObjectIndexes::FunctionPrototypeIndex.into())
    }

    /// %isFinite%
    pub const fn is_finite() -> Object {
        todo!()
    }

    /// %isNaN%
    pub const fn is_nan() -> Object {
        todo!()
    }

    /// %Math%
    pub const fn math() -> Object {
        Object::Object(BuiltinObjectIndexes::MathObjectIndex.into())
    }

    /// %Number%
    pub const fn number() -> Object {
        Object::Function(BuiltinObjectIndexes::NumberConstructorIndex.into())
    }

    /// %Number.prototype%
    pub const fn number_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::NumberPrototypeIndex.into())
    }

    /// %Object%
    pub const fn object() -> Object {
        Object::Function(BuiltinObjectIndexes::ObjectConstructorIndex.into())
    }

    /// %Object.prototype%
    pub const fn object_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::ObjectPrototypeIndex.into())
    }

    /// %Object.prototype.toString%
    pub const fn object_prototype_to_string() -> Object {
        todo!()
    }

    /// %RangeError%
    pub const fn range_error() -> Object {
        todo!()
    }

    /// %RangeError.prototype%
    pub const fn range_error_prototype() -> Object {
        todo!()
    }

    /// %ReferenceError%
    pub const fn reference_error() -> Object {
        todo!()
    }

    /// %ReferenceError.prototype%
    pub const fn reference_error_prototype() -> Object {
        todo!()
    }

    /// %Reflect%
    pub const fn reflect() -> Object {
        todo!()
    }

    /// %String%
    pub const fn string() -> Object {
        Object::Function(BuiltinObjectIndexes::StringConstructorIndex.into())
    }

    /// %String.prototype%
    pub const fn string_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::StringPrototypeIndex.into())
    }

    /// %Symbol%
    pub const fn symbol() -> Object {
        Object::Function(BuiltinObjectIndexes::SymbolConstructorIndex.into())
    }

    /// %Symbol.prototype%
    pub const fn symbol_prototype() -> Object {
        Object::Object(BuiltinObjectIndexes::SymbolPrototypeIndex.into())
    }

    /// %SyntaxError%
    pub const fn syntax_error() -> Object {
        todo!()
    }

    /// %SyntaxError.prototype%
    pub const fn syntax_error_prototype() -> Object {
        todo!()
    }

    /// %ThrowTypeError%
    pub const fn throw_type_error() -> Object {
        todo!()
    }

    /// %TypeError%
    pub const fn type_error() -> Object {
        todo!()
    }

    /// %TypeError.prototype%
    pub const fn type_error_prototype() -> Object {
        todo!()
    }

    /// %URIError%
    pub const fn uri_error() -> Object {
        todo!()
    }

    /// %URIError.prototype%
    pub const fn uri_error_prototype() -> Object {
        todo!()
    }
}
