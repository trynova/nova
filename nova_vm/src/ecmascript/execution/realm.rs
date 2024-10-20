// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod intrinsics;

use super::{
    environments::GlobalEnvironmentIndex, Agent, ExecutionContext, GlobalEnvironment, JsResult,
};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::define_property_or_throw,
        types::{
            IntoValue, Number, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
pub(crate) use intrinsics::Intrinsics;
pub(crate) use intrinsics::ProtoIntrinsics;
use std::{
    any::Any,
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RealmIdentifier(NonZeroU32, PhantomData<Realm>);

impl RealmIdentifier {
    /// Creates a realm identififer from a usize.
    ///
    /// ## Panics
    /// If the given index is greater than `u32::MAX - 1`.
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value < u32::MAX as usize);
        // SAFETY: Not u32::MAX, so addition cannot overflow to 0.
        Self(
            unsafe { NonZeroU32::new_unchecked(value as u32 + 1) },
            PhantomData,
        )
    }

    /// Creates a module identififer from a u32.
    pub(crate) const fn from_u32(value: u32) -> Self {
        // SAFETY: Not u32::MAX, so addition cannot overflow to 0.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub(crate) fn last(realms: &[Option<Realm>]) -> Self {
        let index = realms.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }

    pub(crate) const fn into_u32_index(self) -> u32 {
        self.0.get() - 1
    }
}

impl Index<RealmIdentifier> for Agent {
    type Output = Realm;

    fn index(&self, index: RealmIdentifier) -> &Self::Output {
        &self.heap.realms[index]
    }
}

impl IndexMut<RealmIdentifier> for Agent {
    fn index_mut(&mut self, index: RealmIdentifier) -> &mut Self::Output {
        &mut self.heap.realms[index]
    }
}

impl Index<RealmIdentifier> for Vec<Option<Realm>> {
    type Output = Realm;

    fn index(&self, index: RealmIdentifier) -> &Self::Output {
        self.get(index.into_index())
            .expect("RealmIdentifier out of bounds")
            .as_ref()
            .expect("RealmIdentifier slot empty")
    }
}

impl IndexMut<RealmIdentifier> for Vec<Option<Realm>> {
    fn index_mut(&mut self, index: RealmIdentifier) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("RealmIdentifier out of bounds")
            .as_mut()
            .expect("RealmIdentifier slot empty")
    }
}

impl HeapMarkAndSweep for RealmIdentifier {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.realms.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = Self::from_u32(self_index - compactions.realms.get_shift_for_index(self_index));
    }
}

/// ### [9.3 Realms](https://tc39.es/ecma262/#sec-code-realms)
///
/// Before it is evaluated, all ECMAScript code must be associated with a
/// realm. Conceptually, a realm consists of a set of intrinsic objects, an
/// ECMAScript global environment, all of the ECMAScript code that is loaded
/// within the scope of that global environment, and other associated state and
/// resources.
#[derive(Debug)]
pub struct Realm {
    /// ### \[\[AgentSignifier]]
    ///
    /// The agent that owns this realm
    agent_signifier: PhantomData<Agent>,

    /// ### \[\[Intrinsics]]
    ///
    /// The intrinsic values used by code associated with this realm.
    intrinsics: Intrinsics,

    /// ### \[\[GlobalObject]]
    ///
    /// The global object for this realm.
    pub(crate) global_object: Object,

    /// ### \[\[GlobalEnv]]
    ///
    /// The global environment for this realm.
    pub(crate) global_env: Option<GlobalEnvironmentIndex>,

    /// ### \[\[TemplateMap]]
    ///
    /// Template objects are canonicalized separately for each realm using its
    /// Realm Record's \[\[TemplateMap]]. Each \[\[Site]] value is a Parse Node
    /// that is a TemplateLiteral. The associated \[\[Array]] value is the
    /// corresponding template object that is passed to a tag function.
    /// NOTE: The template data is included in the AST.
    template_map: (),

    /// ### \[\[LoadedModules]]
    ///
    /// A map from the specifier strings imported by this realm to the resolved
    /// Module Record. The list does not contain two different Records with the
    /// same \[\[Specifier]].
    // TODO: Include this once we support modules.
    loaded_modules: (),

    /// ### \[\[HostDefined]]
    ///
    /// Field reserved for use by hosts that need to associate additional
    /// information with a Realm Record.
    pub(crate) host_defined: Option<&'static dyn Any>,
}

unsafe impl Send for Realm {}

impl Realm {
    pub(crate) fn intrinsics(&self) -> &Intrinsics {
        &self.intrinsics
    }

    pub(crate) fn intrinsics_mut(&mut self) -> &mut Intrinsics {
        &mut self.intrinsics
    }
}

impl HeapMarkAndSweep for Realm {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            agent_signifier: _,
            intrinsics,
            global_object,
            global_env,
            template_map: _,
            loaded_modules: _,
            host_defined: _,
        } = self;
        intrinsics.mark_values(queues);
        global_env.mark_values(queues);
        global_object.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            agent_signifier: _,
            intrinsics,
            global_object,
            global_env,
            template_map: _,
            loaded_modules: _,
            host_defined: _,
        } = self;
        intrinsics.sweep_values(compactions);
        global_env.sweep_values(compactions);
        global_object.sweep_values(compactions);
    }
}

/// ### [9.3.1 CreateRealm ( )](https://tc39.es/ecma262/#sec-createrealm)
///
/// The abstract operation CreateRealm takes no arguments and returns a Realm
/// Record.
pub fn create_realm(agent: &mut Agent) -> RealmIdentifier {
    // 1. Let realmRec be a new Realm Record.
    let realm_rec = Realm {
        // 2. Perform CreateIntrinsics(realmRec).
        intrinsics: create_intrinsics(agent),

        // 3. Set realmRec.[[AgentSignifier]] to AgentSignifier().
        agent_signifier: PhantomData,

        // 4. Set realmRec.[[GlobalObject]] to undefined.
        global_object: Object::Object(OrdinaryObject::_def()),

        // 5. Set realmRec.[[GlobalEnv]] to undefined.
        global_env: None,

        // 6. Set realmRec.[[TemplateMap]] to a new empty List.
        template_map: (),

        // NOTE: These fields are implicitly empty.
        host_defined: None,
        loaded_modules: (),
    };

    // 7. Return realmRec.
    let realm = agent.heap.add_realm(realm_rec);
    Intrinsics::create_intrinsics(agent, realm);
    realm
}

/// ### [9.3.2 CreateIntrinsics ( realmRec )](https://tc39.es/ecma262/#sec-createintrinsics)
///
/// The abstract operation CreateIntrinsics takes argument realmRec (a Realm
/// Record) and returns UNUSED.
pub(crate) fn create_intrinsics(agent: &mut Agent) -> Intrinsics {
    // TODO: Follow the specification.
    // 1. Set realmRec.[[Intrinsics]] to a new Record.
    // 2. Set fields of realmRec.[[Intrinsics]] with the values listed in
    //    Table 6. The field names are the names listed in column one of the
    //    table. The value of each field is a new object value fully and
    //    recursively populated with property values as defined by the
    //    specification of each object in clauses 19 through 28. All object
    //    property values are newly created object values. All values that are
    //    built-in function objects are created by performing
    //    CreateBuiltinFunction(steps, length, name, slots, realmRec, prototype)
    //    where steps is the definition of that function provided by this
    //    specification, name is the initial value of the function's "name"
    //    property, length is the initial value of the function's "length"
    //    property, slots is a list of the names, if any, of the function's
    //    specified internal slots, and prototype is the specified value of the
    //    function's [[Prototype]] internal slot. The creation of the intrinsics
    //    and their properties must be ordered to avoid any dependencies upon
    //    objects that have not yet been created.
    // 3. Perform AddRestrictedFunctionProperties(realmRec.[[Intrinsics]].[[%Function.prototype%]], realmRec).

    // 4. Return UNUSED.
    // NOTE: We divert from the specification to allow us to call
    //       CreateIntrinsics when we create the Realm.

    Intrinsics::new(agent)
}

/// ### [9.3.3 SetRealmGlobalObject ( realmRec, globalObj, thisValue )](https://tc39.es/ecma262/#sec-setrealmglobalobject)
pub(crate) fn set_realm_global_object(
    agent: &mut Agent,
    realm_id: RealmIdentifier,
    global_object: Option<Object>,
    this_value: Option<Object>,
) {
    // 1. If globalObj is undefined, then
    let global_object = global_object.unwrap_or_else(|| {
        // a. Let intrinsics be realmRec.[[Intrinsics]].
        let intrinsics = &agent.get_realm(realm_id).intrinsics;
        // b. Set globalObj to OrdinaryObjectCreate(intrinsics.[[%Object.prototype%]]).
        Object::Object(
            agent
                .heap
                .create_object_with_prototype(intrinsics.object_prototype().into(), &[]),
        )
    });

    // 2. Assert: globalObj is an Object.
    // No-op

    // 3. If thisValue is undefined, set thisValue to globalObj.
    let this_value = this_value.unwrap_or(global_object);

    // 4. Set realmRec.[[GlobalObject]] to globalObj.
    agent[realm_id].global_object = global_object;

    // 5. Let newGlobalEnv be NewGlobalEnvironment(globalObj, thisValue).
    let new_global_env = GlobalEnvironment::new(agent, global_object, this_value);

    // 6. Set realmRec.[[GlobalEnv]] to newGlobalEnv.
    agent[realm_id].global_env = Some(
        agent
            .heap
            .environments
            .push_global_environment(new_global_env),
    );

    // 7. Return UNUSED.
}

/// ### [9.3.4 SetDefaultGlobalBindings ( realmRec )](https://tc39.es/ecma262/#sec-setdefaultglobalbindings)
///
/// The abstract operation SetDefaultGlobalBindings takes argument realmRec (a
/// Realm Record) and returns either a normal completion containing an Object
/// or a throw completion.
pub(crate) fn set_default_global_bindings(
    agent: &mut Agent,
    realm_id: RealmIdentifier,
) -> JsResult<Object> {
    // 1. Let global be realmRec.[[GlobalObject]].
    let global = agent[realm_id].global_object;

    // 2. For each property of the Global Object specified in clause 19, do
    // a. Let name be the String value of the property name.
    // b. Let desc be the fully populated data Property Descriptor for the
    //    property, containing the specified attributes for the property. For
    //    properties listed in 19.2, 19.3, or 19.4 the value of the [[Value]]
    //    attribute is the corresponding intrinsic object from realmRec.
    // c. Perform ? DefinePropertyOrThrow(global, name, desc).

    // 19.1 Value Properties of the Global Object
    {
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.globalThis);

        let global_env = agent[realm_id].global_env;
        let desc = PropertyDescriptor {
            value: Some(global_env.unwrap().get_this_binding(agent).into_value()),
            ..Default::default()
        };

        define_property_or_throw(agent, global, name, desc)?;

        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Infinity);
        let value = Number::from_f64(agent, f64::INFINITY);
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(false),
            enumerable: Some(false),
            configurable: Some(false),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.NaN);
        let value = Number::from_f64(agent, f64::NAN);
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(false),
            enumerable: Some(false),
            configurable: Some(false),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.undefined);
        let desc = PropertyDescriptor {
            value: Some(Value::Undefined),
            writable: Some(false),
            enumerable: Some(false),
            configurable: Some(false),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;
    }

    // 19.2 Function Properties of the Global Object
    {
        // 19.2.1 eval ( x )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.eval);
        let value = agent.get_realm(realm_id).intrinsics().eval();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.2 isFinite ( number )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.isFinite);
        let value = agent.get_realm(realm_id).intrinsics().is_finite();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.3 isNaN ( number )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.isNaN);
        let value = agent.get_realm(realm_id).intrinsics().is_nan();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.4 parseFloat ( string )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.parseFloat);
        let value = agent.get_realm(realm_id).intrinsics().parse_float();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.5 parseInt ( string, radix )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.parseInt);
        let value = agent.get_realm(realm_id).intrinsics().parse_int();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.6.1 decodeURI ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.decodeURI);
        let value = agent.get_realm(realm_id).intrinsics().decode_uri();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.6.2 decodeURIComponent ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.decodeURIComponent);
        let value = agent
            .get_realm(realm_id)
            .intrinsics()
            .decode_uri_component();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.6.3 encodeURI ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.encodeURI);
        let value = agent.get_realm(realm_id).intrinsics().encode_uri();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.2.6.4 encodeURIComponent ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.encodeURIComponent);
        let value = agent
            .get_realm(realm_id)
            .intrinsics()
            .encode_uri_component();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;
    }

    // 19.3 Constructor Properties of the Global Object
    {
        // 19.3.1 AggregateError ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.AggregateError);
        let value = agent.get_realm(realm_id).intrinsics().aggregate_error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.2 Array ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Array);
        let value = agent.get_realm(realm_id).intrinsics().array();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.3 ArrayBuffer ( . . . )
        #[cfg(feature = "array-buffer")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.ArrayBuffer);
            let value = agent.get_realm(realm_id).intrinsics().array_buffer();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.3.4 BigInt ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.BigInt);
        let value = agent.get_realm(realm_id).intrinsics().big_int();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.5 BigInt64Array ( . . . )
        #[cfg(feature = "array-buffer")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.BigInt64Array);
            let value = agent.get_realm(realm_id).intrinsics().big_int64_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.6 BigUint64Array ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.BigUint64Array);
            let value = agent.get_realm(realm_id).intrinsics().big_uint64_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.3.7 Boolean ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Boolean);
        let value = agent.get_realm(realm_id).intrinsics().boolean();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.8 DataView ( . . . )
        #[cfg(feature = "array-buffer")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.DataView);
            let value = agent.get_realm(realm_id).intrinsics().data_view();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        #[cfg(feature = "date")]
        {
            // 19.3.9 Date ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Date);
            let value = agent.get_realm(realm_id).intrinsics().date();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.3.10 Error ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Error);
        let value = agent.get_realm(realm_id).intrinsics().error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.11 EvalError ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.EvalError);
        let value = agent.get_realm(realm_id).intrinsics().eval_error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.12 FinalizationRegistry ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.FinalizationRegistry);
        let value = agent
            .get_realm(realm_id)
            .intrinsics()
            .finalization_registry();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.13 Float32Array ( . . . )
        #[cfg(feature = "array-buffer")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Float32Array);
            let value = agent.get_realm(realm_id).intrinsics().float32_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.14 Float64Array ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Float64Array);
            let value = agent.get_realm(realm_id).intrinsics().float64_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.3.15 Function ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Function);
        let value = agent.get_realm(realm_id).intrinsics().function();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;
        // 19.3.16 Int8Array ( . . . )
        #[cfg(feature = "array-buffer")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Int8Array);
            let value = agent.get_realm(realm_id).intrinsics().int8_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.17 Int16Array ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Int16Array);
            let value = agent.get_realm(realm_id).intrinsics().int16_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.18 Int32Array ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Int32Array);
            let value = agent.get_realm(realm_id).intrinsics().int32_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.3.19 Map ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Map);
        let value = agent.get_realm(realm_id).intrinsics().map();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.20 Number ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Number);
        let value = agent.get_realm(realm_id).intrinsics().number();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.21 Object ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Object);
        let value = agent.get_realm(realm_id).intrinsics().object();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.22 Promise ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Promise);
        let value = agent.get_realm(realm_id).intrinsics().promise();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.23 Proxy ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Proxy);
        let value = agent.get_realm(realm_id).intrinsics().proxy();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.24 RangeError ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.RangeError);
        let value = agent.get_realm(realm_id).intrinsics().range_error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.25 ReferenceError ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.ReferenceError);
        let value = agent.get_realm(realm_id).intrinsics().reference_error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.26 RegExp ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.RegExp);
        let value = agent.get_realm(realm_id).intrinsics().reg_exp();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.27 Set ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Set);
        let value = agent.get_realm(realm_id).intrinsics().set();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.28 SharedArrayBuffer ( . . . )
        #[cfg(feature = "shared-array-buffer")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.SharedArrayBuffer);
            let value = agent.get_realm(realm_id).intrinsics().shared_array_buffer();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.3.29 String ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.String);
        let value = agent.get_realm(realm_id).intrinsics().string();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.30 Symbol ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Symbol);
        let value = agent.get_realm(realm_id).intrinsics().symbol();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.31 SyntaxError ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.SyntaxError);
        let value = agent.get_realm(realm_id).intrinsics().syntax_error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.32 TypeError ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.TypeError);
        let value = agent.get_realm(realm_id).intrinsics().type_error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.33 Uint8Array ( . . . )
        #[cfg(feature = "array-buffer")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Uint8Array);
            let value = agent.get_realm(realm_id).intrinsics().uint8_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.34 Uint8ClampedArray ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Uint8ClampedArray);
            let value = agent.get_realm(realm_id).intrinsics().uint8_clamped_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.35 Uint16Array ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Uint16Array);
            let value = agent.get_realm(realm_id).intrinsics().uint16_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.36 Uint32Array ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Uint32Array);
            let value = agent.get_realm(realm_id).intrinsics().uint32_array();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.3.37 URIError ( . . . )
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.URIError);
        let value = agent.get_realm(realm_id).intrinsics().uri_error();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;

        // 19.3.38 WeakMap ( . . . )
        #[cfg(feature = "weak-refs")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.WeakMap);
            let value = agent.get_realm(realm_id).intrinsics().weak_map();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
            // 19.3.39 WeakRef ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.WeakRef);
            let value = agent.get_realm(realm_id).intrinsics().weak_ref();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;

            // 19.3.40 WeakSet ( . . . )
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.WeakSet);
            let value = agent.get_realm(realm_id).intrinsics().weak_set();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
    }

    // 19.4 Other Properties of the Global Object
    {
        // 19.4.1 Atomics
        #[cfg(feature = "atomics")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Atomics);
            let value = agent.get_realm(realm_id).intrinsics().atomics();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.4.2 JSON
        #[cfg(feature = "json")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.JSON);
            let value = agent.get_realm(realm_id).intrinsics().json();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }

        // 19.4.3 Math
        #[cfg(feature = "math")]
        {
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Math);
            let value = agent.get_realm(realm_id).intrinsics().math();
            let desc = PropertyDescriptor {
                value: Some(value.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            };
            define_property_or_throw(agent, global, name, desc)?;
        }
        // 19.4.4 Reflect
        let name = PropertyKey::from(BUILTIN_STRING_MEMORY.Reflect);
        let value = agent.get_realm(realm_id).intrinsics().reflect();
        let desc = PropertyDescriptor {
            value: Some(value.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        };
        define_property_or_throw(agent, global, name, desc)?;
    }

    // 3. Return global.
    Ok(global)
}

/// ### [9.6 InitializeHostDefinedRealm ( )](https://tc39.es/ecma262/#sec-initializehostdefinedrealm)
pub(crate) fn initialize_host_defined_realm(
    agent: &mut Agent,
    create_global_object: Option<impl FnOnce(&mut Agent) -> Object>,
    create_global_this_value: Option<impl FnOnce(&mut Agent) -> Object>,
    initialize_global_object: Option<impl FnOnce(&mut Agent, Object)>,
) {
    // 1. Let realm be CreateRealm().
    let realm = create_realm(agent);

    // 2. Let newContext be a new execution context.
    let new_context = ExecutionContext {
        // NOTE: This property is assumed to be null until the specification
        //       assigns it.
        ecmascript_code: None,

        // 3. Set the Function of newContext to null.
        function: None,

        // 4. Set the Realm of newContext to realm.
        realm,

        // 5. Set the ScriptOrModule of newContext to null.
        script_or_module: None,
    };

    // 6. Push newContext onto the execution context stack; newContext is now the running execution context.
    agent.execution_context_stack.push(new_context);

    // 7. If the host requires use of an exotic object to serve as realm's global object,
    // let global be such an object created in a host-defined manner.
    // Otherwise, let global be undefined, indicating that an ordinary object should be created as the global object.
    let global =
        create_global_this_value.map(|create_global_this_value| create_global_this_value(agent));

    // 8. If the host requires that the this binding in realm's global scope return an object other than the global object,
    // let thisValue be such an object created in a host-defined manner.
    // Otherwise, let thisValue be undefined, indicating that realm's global this binding should be the global object.
    let this_value = create_global_object.map(|create_global_object| create_global_object(agent));

    // 9. Perform SetRealmGlobalObject(realm, global, thisValue).
    set_realm_global_object(agent, realm, global, this_value);

    // 10. Let globalObj be ? SetDefaultGlobalBindings(realm).
    let global_object = set_default_global_bindings(agent, realm).unwrap();

    // 11. Create any host-defined global object properties on globalObj.
    if let Some(initialize_global_object) = initialize_global_object {
        initialize_global_object(agent, global_object);
    };

    // 12. Return UNUSED.
}

pub(crate) fn initialize_default_realm(agent: &mut Agent) {
    let create_global_object: Option<fn(&mut Agent) -> Object> = None;
    let create_global_this_value: Option<fn(&mut Agent) -> Object> = None;
    let initialize_global_object: Option<fn(&mut Agent, Object)> = None;
    initialize_host_defined_realm(
        agent,
        create_global_object,
        create_global_this_value,
        initialize_global_object,
    );
}

#[cfg(test)]
mod test {

    use crate::heap::{
        IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, IntrinsicObjectIndexes,
        LAST_INTRINSIC_CONSTRUCTOR_INDEX, LAST_INTRINSIC_FUNCTION_INDEX,
        LAST_INTRINSIC_OBJECT_INDEX, LAST_WELL_KNOWN_SYMBOL_INDEX,
    };
    fn panic_builtin_function_missing(index: usize) {
        let index = index as u32;
        let mut changed_index = index;
        if changed_index <= LAST_INTRINSIC_CONSTRUCTOR_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!(
                "Found a missing BuiltinFunction at constructor index {:?}",
                unsafe { std::mem::transmute::<u32, IntrinsicConstructorIndexes>(changed_index) }
            );
        }
        changed_index -= LAST_INTRINSIC_CONSTRUCTOR_INDEX as u32 + 1;
        if changed_index <= LAST_INTRINSIC_FUNCTION_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!(
                "Found a missing BuiltinFunction at function index {:?}",
                unsafe { std::mem::transmute::<u32, IntrinsicFunctionIndexes>(changed_index) }
            );
        }
        panic!("Found a missing BuiltinFunction at index {:?}", index);
    }

    fn panic_object_missing(index: usize) {
        let index = index as u32;
        let mut changed_index = index;
        if changed_index <= LAST_INTRINSIC_OBJECT_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!("Found a missing Object at object index {:?}", unsafe {
                std::mem::transmute::<u32, IntrinsicObjectIndexes>(changed_index)
            });
        }
        changed_index -= LAST_INTRINSIC_OBJECT_INDEX as u32 + 1;
        if changed_index <= LAST_INTRINSIC_CONSTRUCTOR_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!(
                "Found a missing BuiltinFunction at constructor index {:?}",
                unsafe { std::mem::transmute::<u32, IntrinsicConstructorIndexes>(changed_index) }
            );
        }
        panic!("Found a missing object at index {:?}", index);
    }

    #[test]
    fn test_default_realm_sanity() {
        use super::initialize_default_realm;
        use crate::ecmascript::execution::{agent::Options, Agent, DefaultHostHooks};
        use crate::heap::indexes::BuiltinFunctionIndex;
        use crate::heap::indexes::ObjectIndex;

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        initialize_default_realm(&mut agent);
        assert_eq!(
            agent.current_realm().intrinsics().object_index_base,
            ObjectIndex::from_index(0)
        );
        assert_eq!(
            agent
                .current_realm()
                .intrinsics()
                .builtin_function_index_base,
            BuiltinFunctionIndex::from_index(0)
        );
        #[cfg(feature = "array-buffer")]
        assert!(agent.heap.array_buffers.is_empty());
        // Array prototype is itself an Array :/
        assert_eq!(agent.heap.arrays.len(), 1);
        assert!(agent.heap.bigints.is_empty());
        assert!(agent.heap.bound_functions.is_empty());
        let missing_builtin = agent
            .heap
            .builtin_functions
            .iter()
            .enumerate()
            .find(|(_, item)| item.is_none());
        if let Some((missing_builtin_index, _)) = missing_builtin {
            panic_builtin_function_missing(missing_builtin_index);
        }
        #[cfg(feature = "date")]
        assert!(agent.heap.dates.is_empty());
        assert!(agent.heap.ecmascript_functions.is_empty());
        assert_eq!(agent.heap.environments.declarative.len(), 1);
        assert!(agent.heap.environments.function.is_empty());
        assert_eq!(agent.heap.environments.global.len(), 1);
        assert_eq!(agent.heap.environments.object.len(), 1);
        assert!(agent.heap.errors.is_empty());
        assert!(agent.heap.globals.borrow().is_empty());
        assert!(agent.heap.modules.is_empty());
        let missing_number = agent
            .heap
            .numbers
            .iter()
            .enumerate()
            .find(|(_, item)| item.is_none());
        if let Some((missing_number_index, _)) = missing_number {
            panic!("Found a missing Number at index {}", missing_number_index);
        }
        let missing_object = agent
            .heap
            .objects
            .iter()
            .enumerate()
            .find(|(_, item)| item.is_none());
        if let Some((missing_object_index, _)) = missing_object {
            panic_object_missing(missing_object_index);
        }
        assert_eq!(agent.heap.realms.len(), 1);
        assert!(agent.heap.scripts.is_empty());
        assert_eq!(
            agent.heap.symbols.len() - 1,
            LAST_WELL_KNOWN_SYMBOL_INDEX as usize
        );
        let missing_symbol = agent
            .heap
            .symbols
            .iter()
            .enumerate()
            .find(|(_, item)| item.is_none());
        if let Some((missing_symbol_index, _)) = missing_symbol {
            panic!("Found a missing Symbol at index {}", missing_symbol_index);
        }
        let missing_string = agent
            .heap
            .strings
            .iter()
            .enumerate()
            .find(|(_, item)| item.is_none());
        if let Some((missing_string_index, _)) = missing_string {
            panic!("Found a missing String at index {}", missing_string_index);
        }
        assert!(agent.heap.regexps.is_empty());
    }
}
