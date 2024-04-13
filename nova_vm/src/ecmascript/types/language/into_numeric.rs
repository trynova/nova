use super::Numeric;

pub trait IntoNumeric
where
    Self: Sized + Copy,
{
    fn into_numeric(self) -> Numeric;
}
