use crate::ecmascript::{
    execution::EnvironmentIndex,
    types::{Symbol, Value},
};
use oxc_span::Atom;

/// ### [6.2.5 The Reference Record Specification Type](https://tc39.es/ecma262/#sec-reference-record-specification-type)
///
/// The Reference Record type is used to explain the behaviour of such operators
/// as delete, typeof, the assignment operators, the super keyword and other
/// language features. For example, the left-hand operand of an assignment is
/// expected to produce a Reference Record.
#[derive(Debug)]
pub struct Reference {
    /// ### \[\[Base]]
    ///
    /// The value or Environment Record which holds the binding. A \[\[Base]]
    /// of UNRESOLVABLE indicates that the binding could not be resolved.
    pub(crate) base: Base,

    /// ### \[\[ReferencedName]]
    ///
    /// The name of the binding. Always a String if \[\[Base]] value is an
    /// Environment Record.
    pub(crate) referenced_name: ReferencedName,

    /// ### \[\[Strict]]
    ///
    /// true if the Reference Record originated in strict mode code, false
    /// otherwise.
    pub(crate) strict: bool,

    /// ### \[\[ThisValue]]
    ///
    /// If not EMPTY, the Reference Record represents a property binding that
    /// was expressed using the super keyword; it is called a Super Reference
    /// Record and its \[\[Base]] value will never be an Environment Record. In
    /// that case, the \[\[ThisValue]] field holds the this value at the time
    /// the Reference Record was created.
    pub(crate) this_value: Option<Value>,
}

/// ### [6.2.5.1 IsPropertyReference ( V )](https://tc39.es/ecma262/#sec-ispropertyreference)
///
/// The abstract operation IsPropertyReference takes argument V (a Reference
/// Record) and returns a Boolean.
pub(crate) fn is_property_reference(reference: &Reference) -> bool {
    match reference.base {
        // 1. if V.[[Base]] is unresolvable, return false.
        Base::Unresolvable => false,

        // 2. If V.[[Base]] is an Environment Record, return false; otherwise return true.
        Base::Environment(_) => false,
        _ => true,
    }
}

/// ### [6.2.5.2 IsUnresolvableReference ( V )](https://tc39.es/ecma262/#sec-isunresolvablereference)
///
/// The abstract operation IsUnresolvableReference takes argument V (a
/// Reference Record) and returns a Boolean.
pub(crate) fn is_unresolvable_reference(reference: &Reference) -> bool {
    // 1. If V.[[Base]] is unresolvable, return true; otherwise return false.
    matches!(reference.base, Base::Unresolvable)
}

/// ### [6.2.5.3 IsSuperReference ( V )](https://tc39.es/ecma262/#sec-issuperreference)
///
/// The abstract operation IsSuperReference takes argument V (a Reference
/// Record) and returns a Boolean.
pub(crate) fn is_super_reference(reference: &Reference) -> bool {
    // 1. If V.[[ThisValue]] is not empty, return true; otherwise return false.
    reference.this_value.is_some()
}

/// ### [6.2.5.4 IsPrivateReference ( V )](https://tc39.es/ecma262/#sec-isprivatereference)
/// The abstract operation IsPrivateReference takes argument V (a Reference Record) and returns a Boolean. It performs the following steps when called:
pub(crate) fn is_private_reference(reference: &Reference) -> bool {
    // 1. If V.[[ReferencedName]] is a Private Name, return true; otherwise return false.
    matches!(reference.referenced_name, ReferencedName::PrivateName)
}

#[derive(Debug)]
pub(crate) enum Base {
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
