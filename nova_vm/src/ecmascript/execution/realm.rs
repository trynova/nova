mod intrinsics;

use super::{
    environments::GlobalEnvironmentIndex, Agent, ExecutionContext, GlobalEnvironment, JsResult,
};
use crate::{
    ecmascript::types::{Object, PropertyDescriptor, PropertyKey, Value},
    heap::indexes::ObjectIndex,
};
pub use intrinsics::Intrinsics;
pub(crate) use intrinsics::ProtoIntrinsics;
use std::{any::Any, marker::PhantomData};

#[derive(Debug, Clone, Copy)]
pub struct RealmIdentifier(u32, PhantomData<Realm>);

impl RealmIdentifier {
    /// Creates a realm identififer from a usize.
    ///
    /// ## Panics
    /// If the given index is greater than `u32::MAX`.
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self(value as u32, PhantomData)
    }

    pub(crate) fn last(realms: &Vec<Option<Realm>>) -> Self {
        let index = realms.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0 as usize
    }
}

/// ### [9.3 Realms](https://tc39.es/ecma262/#sec-code-realms)
///
/// Before it is evaluated, all ECMAScript code must be associated with a realm.
/// Conceptually, a realm consists of a set of intrinsic objects, an ECMAScript
/// global environment, all of the ECMAScript code that is loaded within the
/// scope of that global environment, and other associated state and resources.
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

impl Realm {
    pub(crate) fn intrinsics(&self) -> &Intrinsics {
        &self.intrinsics
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
        intrinsics: create_intrinsics(),

        // 3. Set realmRec.[[AgentSignifier]] to AgentSignifier().
        agent_signifier: PhantomData,

        // 4. Set realmRec.[[GlobalObject]] to undefined.
        global_object: Object::Object(ObjectIndex::from_index(0)),

        // 5. Set realmRec.[[GlobalEnv]] to undefined.
        global_env: None,

        // 6. Set realmRec.[[TemplateMap]] to a new empty List.
        template_map: (),

        // NOTE: These fields are implicitly empty.
        host_defined: None,
        loaded_modules: (),
    };

    // 7. Return realmRec.
    agent.heap.add_realm(realm_rec)
}

/// ### [9.3.2 CreateIntrinsics ( realmRec )](https://tc39.es/ecma262/#sec-createintrinsics)
///
/// The abstract operation CreateIntrinsics takes argument realmRec (a Realm
/// Record) and returns UNUSED.
pub(crate) fn create_intrinsics() -> Intrinsics {
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

    Intrinsics::default()
}

/// 9.3.3 SetRealmGlobalObject ( realmRec, globalObj, thisValue ), https://tc39.es/ecma262/#sec-setrealmglobalobject
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
                .create_object_with_prototype(intrinsics.object_prototype()),
        )
    });

    // 2. Assert: globalObj is an Object.
    // No-op

    // 3. If thisValue is undefined, set thisValue to globalObj.
    let this_value = this_value.unwrap_or(global_object);

    // 4. Set realmRec.[[GlobalObject]] to globalObj.
    agent.heap.get_realm_mut(realm_id).global_object = global_object;

    // 5. Let newGlobalEnv be NewGlobalEnvironment(globalObj, thisValue).
    let new_global_env = GlobalEnvironment::new(agent, global_object, this_value);

    // 6. Set realmRec.[[GlobalEnv]] to newGlobalEnv.
    agent.heap.get_realm_mut(realm_id).global_env = Some(
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
/// Realm Record) and returns either a normal completion containing an Object or
/// a throw completion.
pub(crate) fn set_default_global_bindings(
    agent: &mut Agent,
    realm_id: RealmIdentifier,
) -> JsResult<Object> {
    // 1. Let global be realmRec.[[GlobalObject]].
    let global = agent.heap.get_realm(realm_id).global_object;

    // 2. For each property of the Global Object specified in clause 19, do
    // TODO: Actually do other properties aside from globalThis.
    {
        // a. Let name be the String value of the property name.
        let name = PropertyKey::try_from(Value::from_str(&mut agent.heap, "globalThis")).unwrap();

        // b. Let desc be the fully populated data Property Descriptor for the property, containing the specified attributes for the property. For properties listed in 19.2, 19.3, or 19.4 the value of the [[Value]] attribute is the corresponding intrinsic object from realmRec.
        let global_env = agent.heap.get_realm(realm_id).global_env;
        let desc = PropertyDescriptor {
            value: Some(
                agent
                    .heap
                    .environments
                    .get_global_environment(global_env.unwrap())
                    .global_this_value
                    .into_value(),
            ),
            ..Default::default()
        };

        // c. Perform ? DefinePropertyOrThrow(global, name, desc).
        global.define_property_or_throw(agent, name, desc)?;
    }

    // 3. Return global.
    Ok(global)
}

/// 9.6 InitializeHostDefinedRealm ( ), https://tc39.es/ecma262/#sec-initializehostdefinedrealm
pub fn initialize_host_defined_realm(
    agent: &mut Agent,
    realm_id: RealmIdentifier,
    create_global_object: Option<impl FnOnce(&mut Realm) -> Object>,
    create_global_this_value: Option<impl FnOnce(&mut Realm) -> Object>,
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
    let global = create_global_this_value
        .map(|create_global_this_value| create_global_this_value(agent.current_realm_mut()));

    // 8. If the host requires that the this binding in realm's global scope return an object other than the global object,
    // let thisValue be such an object created in a host-defined manner.
    // Otherwise, let thisValue be undefined, indicating that realm's global this binding should be the global object.
    let this_value = create_global_object
        .map(|create_global_object| create_global_object(agent.current_realm_mut()));

    // 9. Perform SetRealmGlobalObject(realm, global, thisValue).
    set_realm_global_object(agent, realm_id, global, this_value);

    // 10. Let globalObj be ? SetDefaultGlobalBindings(realm).
    let global_object = set_default_global_bindings(agent, realm_id).unwrap();

    // 11. Create any host-defined global object properties on globalObj.
    if let Some(initialize_global_object) = initialize_global_object {
        initialize_global_object(agent, global_object);
    };

    // 12. Return UNUSED.
}
