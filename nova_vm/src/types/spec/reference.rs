use crate::{
    execution::EnvironmentIndex,
    types::{Symbol, Value},
};
use oxc_span::Atom;

/// 6.2.5 The Reference Record Specification Type
/// https://tc39.es/ecma262/#sec-reference-record-specification-type
#[derive(Debug)]
pub struct Reference {
    /// [[Base]]
    pub base: Base,

    /// [[ReferencedName]]
    pub referenced_name: ReferencedName,

    /// [[Strict]]
    pub strict: bool,

    /// [[ThisValue]]
    pub this_value: Option<Value>,
}

impl Reference {
    /// 6.2.5.1 IsPropertyReference ( V )
    /// https://tc39.es/ecma262/#sec-ispropertyreference
    pub fn is_property_reference(self) -> bool {
        match self.base {
            // 1. if V.[[Base]] is unresolvable, return false.
            Base::Unresolvable => false,

            // 2. If V.[[Base]] is an Environment Record, return false; otherwise return true.
            Base::Environment(_) => false,
            _ => true,
        }
    }

    /// 6.2.5.2 IsUnresolvableReference ( V )
    /// https://tc39.es/ecma262/#sec-isunresolvablereference
    pub fn is_unresolvable_reference(self) -> bool {
        // 1. If V.[[Base]] is unresolvable, return true; otherwise return false.
        return matches!(self.base, Base::Unresolvable);
    }

    /// 6.2.5.3 IsSuperReference ( V )
    /// https://tc39.es/ecma262/#sec-issuperreference
    pub fn is_super_reference(self) -> bool {
        // 1. If V.[[ThisValue]] is not empty, return true; otherwise return false.
        return !matches!(self.this_value, None);
    }
}

#[derive(Debug)]
pub enum Base {
    Value(Value),
    Environment(EnvironmentIndex),
    Unresolvable,
}

#[derive(Debug)]
pub enum ReferencedName {
    String(Atom),
    Symbol(Symbol),
    // TODO: implement private names
    PrivateName,
}
