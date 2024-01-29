use super::Value;

pub trait IntoValue
where
    Self: Sized + Copy,
{
    fn into_value(self) -> Value;
}
