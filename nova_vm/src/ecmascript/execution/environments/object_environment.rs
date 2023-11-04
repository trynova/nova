use super::OuterEnv;
use crate::ecmascript::types::Object;

/// ### [9.1.1.2 Object Environment Records](https://tc39.es/ecma262/#sec-object-environment-records)
///
/// Each Object Environment Record is associated with an object called its
/// binding object. An Object Environment Record binds the set of string
/// identifier names that directly correspond to the property names of its
/// binding object. Property keys that are not strings in the form of an
/// IdentifierName are not included in the set of bound identifiers. Both own
/// and inherited properties are included in the set regardless of the setting
/// of their \[\[Enumerable\]\] attribute. Because properties can be dynamically
/// added and deleted from objects, the set of identifiers bound by an Object
/// Environment Record may potentially change as a side-effect of any operation
/// that adds or deletes properties. Any bindings that are created as a result
/// of such a side-effect are considered to be a mutable binding even if the
/// Writable attribute of the corresponding property is false. Immutable
/// bindings do not exist for Object Environment Records.
#[derive(Debug)]
pub struct ObjectEnvironment {
    /// ### \[\[BindingObject\]\]
    ///
    /// The binding object of this Environment Record.
    binding_object: Object,

    /// ### \[\[IsWithEnvironment\]\]
    ///
    /// Indicates whether this Environment Record is created for a with
    /// statement.
    is_with_environment: bool,

    /// ### \[\[OuterEnv\]\]
    ///
    /// See [OuterEnv].
    outer_env: OuterEnv,
}

impl ObjectEnvironment {
    /// ### [9.1.2.3 NewObjectEnvironment ( O, W, E )](https://tc39.es/ecma262/#sec-newobjectenvironmenthttps://tc39.es/ecma262/#sec-newobjectenvironment)
    ///
    /// The abstract operation NewObjectEnvironment takes arguments O (an Object), W (a Boolean), and E (an Environment Record or null) and returns an Object Environment Record. It performs the following steps when called:
    pub(crate) fn new(
        binding_object: Object,
        is_with_environment: bool,
        outer_env: OuterEnv,
    ) -> ObjectEnvironment {
        // 1. Let env be a new Object Environment Record.
        ObjectEnvironment {
            // 2. Set env.[[BindingObject]] to O.
            binding_object,
            // 3. Set env.[[IsWithEnvironment]] to W.
            is_with_environment,
            // 4. Set env.[[OuterEnv]] to E.
            outer_env,
        }
        // 5. Return env.
    }
}
