use super::Object;

#[derive(Debug)]
pub struct ObjectData {
    /// [[Prototype]]
    pub prototype: Option<Object>,

    /// [[Extensible]]
    pub extensible: bool,
}
