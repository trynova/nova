use crate::ecmascript::types::language::into_value::IntoValue;

use super::Object;

pub trait IntoObject
where
    Self: Sized + Copy + IntoValue,
{
    fn into_object(self) -> Object;
}
