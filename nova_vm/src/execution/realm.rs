mod intrinsics;

use super::{
    environments::global_environment::GlobalEnvironmentIndex, Agent, ExecutionContext,
    GlobalEnvironment, JsResult,
};
use crate::{
    heap::indexes::ObjectIndex,
    types::{Object, PropertyDescriptor, PropertyKey, Value},
};
pub use intrinsics::Intrinsics;
use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

#[derive(Debug, Clone, Copy)]
pub struct RealmIdentifier<'ctx, 'host>(u32, PhantomData<Realm<'ctx, 'host>>);

impl<'ctx, 'host> RealmIdentifier<'ctx, 'host> {
    pub const fn from_u32_index(value: u32) -> Self {
        Self(value, PhantomData)
    }

    pub const fn from_usize_index(value: usize) -> Self {
        debug_assert!(value <= u32::MAX as usize);
        Self(value as u32, PhantomData)
    }

    pub const fn into_index(self) -> usize {
        self.0 as usize
    }
}

/// 9.3 Realms
/// https://tc39.es/ecma262/#sec-code-realms
#[derive(Debug)]
pub struct Realm<'ctx, 'host> {
    pub agent: Rc<RefCell<Agent<'ctx, 'host>>>,

    // NOTE: We will need an rng here at some point.

    // [[Intrinsics]]
    intrinsics: Intrinsics,

    /// [[GlobalObject]]
    pub global_object: Object,

    /// [[GlobalEnv]]
    pub global_env: GlobalEnvironmentIndex,

    /// [[HostDefined]]
    pub host_defined: Option<Rc<RefCell<dyn Any>>>,
    // TODO: [[TemplateMap]], [[LoadedModules]]
}

impl<'ctx, 'host> Realm<'ctx, 'host> {
    /// 9.3.1 CreateRealm ( ), https://tc39.es/ecma262/#sec-createrealm
    fn create(agent: Rc<RefCell<Agent<'ctx, 'host>>>) -> RealmIdentifier<'ctx, 'host> {
        // TODO: implement spec
        let realm = Self {
            agent: agent.clone(),
            global_env: GlobalEnvironmentIndex::from_u32_index(0),
            global_object: Object::Object(ObjectIndex::from_u32_index(0)),
            host_defined: None,
            intrinsics: Intrinsics::default(),
        };

        agent.borrow_mut().heap.add_realm(realm)
    }

    pub(crate) fn intrinsics(&self) -> &Intrinsics {
        &self.intrinsics
    }

    /// 9.3.3 SetRealmGlobalObject ( realmRec, globalObj, thisValue ), https://tc39.es/ecma262/#sec-setrealmglobalobject
    pub(crate) fn set_global_object(
        &mut self,
        global_object: Option<Object>,
        this_value: Option<Object>,
    ) {
        // 1. If globalObj is undefined, then
        let global_object = global_object.unwrap_or_else(|| {
            // a. Let intrinsics be realmRec.[[Intrinsics]].
            let intrinsics = &self.intrinsics;
            // b. Set globalObj to OrdinaryObjectCreate(intrinsics.[[%Object.prototype%]]).
            Object::Object(
                self.agent
                    .borrow_mut()
                    .heap
                    .create_object_with_prototype(intrinsics.object_prototype()),
            )
        });
        // 2. Assert: globalObj is an Object.
        // No-op

        // 3. If thisValue is undefined, set thisValue to globalObj.
        let this_value = this_value.unwrap_or(global_object);

        // 4. Set realmRec.[[GlobalObject]] to globalObj.
        self.global_object = global_object;

        // 5. Let newGlobalEnv be NewGlobalEnvironment(globalObj, thisValue).
        let new_global_env = GlobalEnvironment::new(global_object, this_value);
        // 6. Set realmRec.[[GlobalEnv]] to newGlobalEnv.
        self.global_env = self
            .agent
            .borrow_mut()
            .heap
            .environments
            .push_global_environment(new_global_env);
        // 7. Return UNUSED.
    }

    /// 9.3.4 SetDefaultGlobalBindings ( realmRec )
    /// https://tc39.es/ecma262/#sec-setdefaultglobalbindings
    pub(crate) fn set_default_global_bindings(
        &mut self,
        agent: Rc<RefCell<Agent<'ctx, 'host>>>,
    ) -> JsResult<Object> {
        // 1. Let global be realmRec.[[GlobalObject]].
        let global = self.global_object;

        // 2. For each property of the Global Object specified in clause 19, do
        // a. Let name be the String value of the property name.
        let name =
            PropertyKey::try_from(Value::from_str(&mut agent.borrow_mut().heap, "globalThis"))
                .unwrap();
        // b. Let desc be the fully populated data Property Descriptor for the property, containing the specified attributes for the property. For properties listed in 19.2, 19.3, or 19.4 the value of the [[Value]] attribute is the corresponding intrinsic object from realmRec.
        let desc = PropertyDescriptor {
            value: Some(
                agent
                    .borrow()
                    .heap
                    .environments
                    .get_global_environment(self.global_env)
                    .global_this_value
                    .into_value(),
            ),
            ..Default::default()
        };
        // c. Perform ? DefinePropertyOrThrow(global, name, desc).
        global.define_property_or_throw(&mut agent.borrow_mut(), name, desc)?;

        // TODO: Actually do other properties aside from globalThis.

        // 3. Return global.
        Ok(global)
    }

    /// 9.6 InitializeHostDefinedRealm ( ), https://tc39.es/ecma262/#sec-initializehostdefinedrealm
    pub fn initialize_host_defined_realm<F, Init>(
        agent: Rc<RefCell<Agent<'ctx, 'host>>>,
        create_global_object: Option<F>,
        create_global_this_value: Option<F>,
        initialize_global_object: Option<Init>,
    ) where
        F: FnOnce(&mut Realm<'ctx, 'host>) -> Object,
        Init: FnOnce(Rc<RefCell<Agent<'ctx, 'host>>>, Object) -> (),
    {
        // 1. Let realm be CreateRealm().
        let realm = Self::create(agent.clone());

        // 2. Let newContext be a new execution context.
        let mut new_context = ExecutionContext::new();

        // 3. Set the Function of newContext to null.
        // No-op

        // 4. Set the Realm of newContext to realm.
        new_context.set_realm(realm);

        // 5. Set the ScriptOrModule of newContext to null.
        // No-op

        // 6. Push newContext onto the execution context stack; newContext is now the running execution context.
        agent.borrow_mut().execution_context_stack.push(new_context);

        // 7. If the host requires use of an exotic object to serve as realm's global object,
        let global = if let Some(create_global_this_value) = create_global_this_value {
            // let global be such an object created in a host-defined manner.
            Some(create_global_this_value(
                agent.borrow_mut().current_realm_mut(),
            ))
        } else {
            // Otherwise, let global be undefined, indicating that an ordinary object should be created as the global object.
            None
        };

        // 8. If the host requires that the this binding in realm's global scope return an object other than the global object,
        let this_value = if let Some(create_global_object) = create_global_object {
            // let thisValue be such an object created in a host-defined manner.
            Some(create_global_object(agent.borrow_mut().current_realm_mut()))
        } else {
            // Otherwise, let thisValue be undefined, indicating that realm's global this binding should be the global object.
            None
        };

        // 9. Perform SetRealmGlobalObject(realm, global, thisValue).
        agent
            .borrow_mut()
            .get_realm_mut(realm)
            .set_global_object(global, this_value);

        // 10. Let globalObj be ? SetDefaultGlobalBindings(realm).
        let global_obj = agent
            .borrow_mut()
            .get_realm_mut(realm)
            .set_default_global_bindings(agent.clone())
            .unwrap();

        // 11. Create any host-defined global object properties on globalObj.
        if let Some(initialize_global_object) = initialize_global_object {
            initialize_global_object(agent, global_obj);
        };

        // 12. Return UNUSED.
    }
}
