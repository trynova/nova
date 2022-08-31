pub mod value;

pub use value::Value;

/// This enum is equivalent to a completion record in cases where only normal
/// and throw completions are allowed – except that it also allows
/// [`Self::Killed`] as a variant to allow termination of a running script.
///
/// See <https://tc39.es/ecma262/#sec-completion-record-specification-type>.
pub enum JsResult<T, E = Value> {
    Return(T),
    Exception(E),
    Killed,
}
