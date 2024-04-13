use super::Primitive;

pub trait IntoPrimitive
where
    Self: Sized + Copy,
{
    fn into_primitive(self) -> Primitive;
}
