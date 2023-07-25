use super::Realm;
use crate::types::Object;

#[derive(Debug)]
pub struct Intrinsics<'a, 'ctx, 'host> {
    pub realm: &'a Realm<'ctx, 'host>,
}

impl Intrinsics<'_, '_, '_> {
    /// %Array%
    pub fn array(&self) -> Object {
        todo!()
    }

    /// %Array.prototype%
    pub fn array_prototype_prototype(&self) -> Object {
        todo!()
    }

    /// %BigInt%
    pub fn big_int(&self) -> Object {
        todo!()
    }

    /// %BigInt.prototype%
    pub fn big_int_prototype(&self) -> Object {
        todo!()
    }

    /// %Boolean%
    pub fn boolean(&self) -> Object {
        todo!()
    }

    /// %Boolean.prototype%
    pub fn boolean_prototype(&self) -> Object {
        todo!()
    }

    /// %Error%
    pub fn error(&self) -> Object {
        todo!()
    }

    /// %Error.prototype%
    pub fn error_prototype(&self) -> Object {
        todo!()
    }

    /// %eval%
    pub fn eval(&self) -> Object {
        todo!()
    }

    /// %EvalError%
    pub fn eval_error(&self) -> Object {
        todo!()
    }

    /// %EvalError.prototype%
    pub fn eval_error_prototype(&self) -> Object {
        todo!()
    }

    /// %Function%
    pub fn function(&self) -> Object {
        todo!()
    }

    /// %Function.prototype%
    pub fn function_prototype(&self) -> Object {
        todo!()
    }

    /// %isFinite%
    pub fn is_finite(&self) -> Object {
        todo!()
    }

    /// %isNaN%
    pub fn is_nan(&self) -> Object {
        todo!()
    }

    /// %Math%
    pub fn math(&self) -> Object {
        todo!()
    }

    /// %Number%
    pub fn number(&self) -> Object {
        todo!()
    }

    /// %Number.prototype%
    pub fn number_prototype(&self) -> Object {
        todo!()
    }

    /// %Object%
    pub fn object(&self) -> Object {
        todo!()
    }

    /// %Object.prototype%
    pub fn object_prototype(&self) -> Object {
        todo!()
    }

    /// %Object.prototype.toString%
    pub fn object_prototype_to_string(&self) -> Object {
        todo!()
    }

    /// %RangeError%
    pub fn range_error(&self) -> Object {
        todo!()
    }

    /// %RangeError.prototype%
    pub fn range_error_prototype(&self) -> Object {
        todo!()
    }

    /// %ReferenceError%
    pub fn reference_error(&self) -> Object {
        todo!()
    }

    /// %ReferenceError.prototype%
    pub fn reference_error_prototype(&self) -> Object {
        todo!()
    }

    /// %Reflect%
    pub fn reflect(&self) -> Object {
        todo!()
    }

    /// %String%
    pub fn string(&self) -> Object {
        todo!()
    }

    /// %String.prototype%
    pub fn string_prototype(&self) -> Object {
        todo!()
    }

    /// %Symbol%
    pub fn symbol(&self) -> Object {
        todo!()
    }

    /// %Symbol.prototype%
    pub fn symbol_prototype(&self) -> Object {
        todo!()
    }

    /// %SyntaxError%
    pub fn syntax_error(&self) -> Object {
        todo!()
    }

    /// %SyntaxError.prototype%
    pub fn syntax_error_prototype(&self) -> Object {
        todo!()
    }

    /// %ThrowTypeError%
    pub fn throw_type_error(&self) -> Object {
        todo!()
    }

    /// %TypeError%
    pub fn type_error(&self) -> Object {
        todo!()
    }

    /// %TypeError.prototype%
    pub fn type_error_prototype(&self) -> Object {
        todo!()
    }

    /// %URIError%
    pub fn uri_error(&self) -> Object {
        todo!()
    }

    /// %URIError.prototype%
    pub fn uri_error_prototype(&self) -> Object {
        todo!()
    }
}
