use super::Realm;
use crate::{execution::JsResult, types::Object};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct Intrinsics<'ctx, 'host> {
    pub realm: Rc<RefCell<Realm<'ctx, 'host>>>,

    // Not stored as top-level properties so we can have methods of the same names
    pub lazy_intrinsics: LazyIntrinsics,
}

macro_rules! lazy_intrinsic {
    ($name: ident $ptr: ty) => {
        pub fn $name(&mut self) -> Object {
            let intrinsic = &mut self.lazy_intrinsics.$name;

            if let Some(intrinsic) = intrinsic {
                intrinsic
            } else {
            }
        }
    };
}

impl Intrinsics<'_, '_> {
    pub fn function_prototype(&mut self) -> Object {
        todo!()
    }
}

#[derive(Debug)]
pub struct LazyIntrinsics {
    /// %Array%
    pub array: Option<Object>,

    /// %Array.prototype%
    pub array_prototype_prototype: Option<Object>,

    /// %BigInt%
    pub big_int: Option<Object>,

    /// %BigInt.prototype%
    pub big_int_prototype: Option<Object>,

    /// %Boolean%
    pub boolean: Option<Object>,

    /// %Boolean.prototype%
    pub boolean_prototype: Option<Object>,

    /// %Error%
    pub error: Option<Object>,

    /// %Error.prototype%
    pub error_prototype: Option<Object>,

    /// %eval%
    pub eval: Option<Object>,

    /// %EvalError%
    pub eval_error: Option<Object>,

    /// %EvalError.prototype%
    pub eval_error_prototype: Option<Object>,

    /// %Function%
    pub function: Option<Object>,

    /// %Function.prototype%
    pub function_prototype: Option<Object>,

    /// %isFinite%
    pub is_finite: Option<Object>,

    /// %isNaN%
    pub is_nan: Option<Object>,

    /// %Math%
    pub math: Option<Object>,

    /// %Number%
    pub number: Option<Object>,

    /// %Number.prototype%
    pub number_prototype: Option<Object>,

    /// %Object%
    pub object: Option<Object>,

    /// %Object.prototype%
    pub object_prototype: Option<Object>,

    /// %Object.prototype.toString%
    pub object_prototype_to_string: Option<Object>,

    /// %RangeError%
    pub range_error: Option<Object>,

    /// %RangeError.prototype%
    pub range_error_prototype: Option<Object>,

    /// %ReferenceError%
    pub reference_error: Option<Object>,

    /// %ReferenceError.prototype%
    pub reference_error_prototype: Option<Object>,

    /// %Reflect%
    pub reflect: Option<Object>,

    /// %String%
    pub string: Option<Object>,

    /// %String.prototype%
    pub string_prototype: Option<Object>,

    /// %Symbol%
    pub symbol: Option<Object>,

    /// %Symbol.prototype%
    pub symbol_prototype: Option<Object>,

    /// %SyntaxError%
    pub syntax_error: Option<Object>,

    /// %SyntaxError.prototype%
    pub syntax_error_prototype: Option<Object>,

    /// %ThrowTypeError%
    pub throw_type_error: Option<Object>,

    /// %TypeError%
    pub type_error: Option<Object>,

    /// %TypeError.prototype%
    pub type_error_prototype: Option<Object>,

    /// %URIError%
    pub uri_error: Option<Object>,

    /// %URIError.prototype%
    pub uri_error_prototype: Option<Object>,
}
