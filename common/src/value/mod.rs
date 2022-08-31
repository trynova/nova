use gc::{Finalize, Gc, Trace};

pub use number::JsNumber;
pub use object::JsObject;
pub use string::JsString;

pub mod number;
pub mod object;
pub mod string;

#[derive(Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    String(Gc<JsString>),
    Symbol(Gc<JsSymbol>),
    Number(JsNumber),
    Object(Gc<dyn JsObject>),
}

#[derive(Trace, Finalize)]
pub struct JsSymbol {
    description: Option<Gc<JsString>>,
}
