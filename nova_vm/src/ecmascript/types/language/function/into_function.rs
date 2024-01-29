use super::Function;
use crate::ecmascript::types::language::IntoObject;

pub trait IntoFunction
where
    Self: Sized + Copy + IntoObject,
{
    fn into_function(self) -> Function;
}
