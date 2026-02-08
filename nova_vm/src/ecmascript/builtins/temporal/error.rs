use crate::{
    ecmascript::{Agent, ExceptionType, JsError},
    engine::NoGcScope,
};
use temporal_rs::{TemporalError, error::ErrorKind};

pub fn temporal_err_to_js_err<'gc>(
    agent: &mut Agent,
    error: TemporalError,
    gc: NoGcScope<'gc, '_>,
) -> JsError<'gc> {
    let message = error.into_message();
    match error.kind() {
        ErrorKind::Generic => {
            agent.throw_exception_with_static_message(ExceptionType::Error, message, gc)
        }
        ErrorKind::Type => {
            agent.throw_exception_with_static_message(ExceptionType::TypeError, message, gc)
        }
        ErrorKind::Range => {
            agent.throw_exception_with_static_message(ExceptionType::RangeError, message, gc)
        }
        ErrorKind::Syntax => {
            agent.throw_exception_with_static_message(ExceptionType::SyntaxError, message, gc)
        }
        ErrorKind::Assert => {
            agent.throw_exception_with_static_message(ExceptionType::Error, message, gc)
        }
    }
}
