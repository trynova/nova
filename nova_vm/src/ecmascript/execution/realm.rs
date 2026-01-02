// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod intrinsics;

use super::{
    Agent, ExecutionContext, JsResult, environments::GlobalEnvironment, new_global_environment,
};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::define_property_or_throw,
        scripts_and_modules::{
            module::module_semantics::{
                LoadedModules, ModuleRequest, abstract_module_records::AbstractModule,
            },
            script::HostDefined,
        },
        types::{
            BUILTIN_STRING_MEMORY, IntoValue, Number, Object, OrdinaryObject, PropertyDescriptor,
            PropertyKey, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, GcToken, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable, Scopable},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
use core::{marker::PhantomData, num::NonZeroU32};
pub(crate) use intrinsics::Intrinsics;
pub(crate) use intrinsics::ProtoIntrinsics;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Realm<'a>(
    NonZeroU32,
    PhantomData<RealmRecord<'static>>,
    PhantomData<&'a GcToken>,
);

impl core::fmt::Debug for Realm<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Realm({:?})", self.into_u32_index())
    }
}

impl<'r> Realm<'r> {
    /// Creates a realm identififer from a usize.
    ///
    /// ## Panics
    /// If the given index is greater than `u32::MAX - 1`.
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value < u32::MAX as usize);
        Self::from_u32(value as u32)
    }

    /// Creates a module identififer from a u32.
    pub(crate) const fn from_u32(value: u32) -> Self {
        assert!(value < u32::MAX);
        // SAFETY: Not u32::MAX, so addition cannot overflow to 0.
        Self(
            unsafe { NonZeroU32::new_unchecked(value + 1) },
            PhantomData,
            PhantomData,
        )
    }

    pub(crate) fn last(realms: &[RealmRecord]) -> Self {
        let index = realms.len() - 1;
        Self::from_index(index)
    }

    pub(crate) const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }

    pub(crate) const fn into_u32_index(self) -> u32 {
        self.0.get() - 1
    }

    /// ### \[\[\HostDefined]]
    pub fn host_defined(self, agent: &Agent) -> Option<HostDefined> {
        agent[self].host_defined.clone()
    }

    /// Initialize the \[\[HostDefined]] field to a value.
    ///
    /// ## Panics
    ///
    /// Panics if the \[\[HostDefined]] field is non-empty.
    pub fn initialize_host_defined(self, agent: &mut Agent, host_defined: HostDefined) {
        assert!(
            agent[self].host_defined.is_none(),
            "Attempted to replace Realm's [[HostDefined]] slot data."
        );
        agent[self].host_defined.replace(host_defined);
    }

    /// ### \[\[GlobalObject]]
    pub fn global_object(self, agent: &mut Agent) -> Object<'r> {
        agent[self].global_object
    }

    /// ### \[\[GlobalEnv]]
    pub fn global_env<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<GlobalEnvironment<'gc>> {
        agent[self].global_env.bind(gc)
    }

    pub(crate) fn insert_loaded_module<'gc>(
        self,
        agent: &mut Agent,
        request: ModuleRequest<'gc>,
        module: AbstractModule<'gc>,
    ) {
        let requests = &agent.heap.module_request_records;
        agent.heap.realms[self]
            .loaded_modules
            .insert_loaded_module(requests, request, module);
    }
}

bindable_handle!(Realm);

impl Rootable for Realm<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Realm(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Realm(realm) => Some(realm),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for Realm<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.realms.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.realms.shift_non_zero_u32_index(&mut self.0);
    }
}

/// ## [9.3 Realms](https://tc39.es/ecma262/#sec-code-realms)
///
/// Before it is evaluated, all ECMAScript code must be associated with a
/// realm. Conceptually, a realm consists of a set of intrinsic objects, an
/// ECMAScript global environment, all of the ECMAScript code that is loaded
/// within the scope of that global environment, and other associated state and
/// resources.
#[derive(Debug)]
pub struct RealmRecord<'a> {
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
    pub(crate) global_object: Object<'a>,

    /// ### \[\[GlobalEnv]]
    ///
    /// The global environment for this realm.
    pub(crate) global_env: Option<GlobalEnvironment<'a>>,

    /// ### \[\[TemplateMap]]
    ///
    /// Template objects are canonicalized separately for each realm using its
    /// Realm Record's \[\[TemplateMap]]. Each \[\[Site]] value is a Parse Node
    /// that is a TemplateLiteral. The associated \[\[Array]] value is the
    /// corresponding template object that is passed to a tag function.
    /// NOTE: The template data is included in the AST.
    #[expect(dead_code)]
    template_map: (),

    /// ### \[\[LoadedModules]]
    ///
    /// A map from the specifier strings imported by this realm to the resolved
    /// Module Record. The list does not contain two different Records with the
    /// same \[\[Specifier]].
    loaded_modules: LoadedModules<'a>,

    /// ### \[\[HostDefined]]
    ///
    /// Field reserved for use by hosts that need to associate additional
    /// information with a Realm Record.
    pub(crate) host_defined: Option<HostDefined>,
}

unsafe impl Send for RealmRecord<'_> {}

impl RealmRecord<'_> {
    pub(crate) fn intrinsics(&self) -> &Intrinsics {
        &self.intrinsics
    }
}

bindable_handle!(RealmRecord);

impl HeapMarkAndSweep for RealmRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            agent_signifier: _,
            intrinsics,
            global_object,
            global_env,
            template_map: _,
            loaded_modules,
            host_defined: _,
        } = self;
        intrinsics.mark_values(queues);
        global_env.mark_values(queues);
        global_object.mark_values(queues);
        loaded_modules.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            agent_signifier: _,
            intrinsics,
            global_object,
            global_env,
            template_map: _,
            loaded_modules,
            host_defined: _,
        } = self;
        intrinsics.sweep_values(compactions);
        global_env.sweep_values(compactions);
        global_object.sweep_values(compactions);
        loaded_modules.sweep_values(compactions);
    }
}

/// ### [9.3.1 CreateRealm ( )](https://tc39.es/ecma262/#sec-createrealm)
///
/// The abstract operation CreateRealm takes no arguments and returns a Realm
/// Record.
pub(crate) fn create_realm<'gc>(agent: &mut Agent, gc: NoGcScope<'gc, '_>) -> Realm<'gc> {
    // 1. Let realmRec be a new Realm Record.
    let realm_rec = RealmRecord {
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
        loaded_modules: Default::default(),
    };

    // 7. Return realmRec.
    let realm = agent.heap.add_realm(realm_rec, gc);
    Intrinsics::create_intrinsics(agent, realm.unbind(), gc);
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
    realm_id: Realm,
    global_object: Option<Object>,
    this_value: Option<Object>,
    gc: NoGcScope,
) {
    // 1. If globalObj is undefined, then
    let global_object = global_object.unwrap_or_else(|| {
        // a. Let intrinsics be realmRec.[[Intrinsics]].
        let intrinsics = &agent.get_realm_record_by_id(realm_id).intrinsics;
        // b. Set globalObj to OrdinaryObjectCreate(intrinsics.[[%Object.prototype%]]).
        Object::Object(
            OrdinaryObject::create_intrinsic_object(
                agent,
                Some(intrinsics.object_prototype().into()),
                &[],
            )
            .expect("Should perform GC here"),
        )
    });

    // 2. Assert: globalObj is an Object.
    // No-op

    // 3. If thisValue is undefined, set thisValue to globalObj.
    let this_value = this_value.unwrap_or(global_object);

    // 4. Set realmRec.[[GlobalObject]] to globalObj.
    agent[realm_id].global_object = global_object.unbind();

    // 5. Let newGlobalEnv be NewGlobalEnvironment(globalObj, thisValue).
    let new_global_env = new_global_environment(agent, global_object, this_value, gc);

    // 6. Set realmRec.[[GlobalEnv]] to newGlobalEnv.
    agent[realm_id].global_env = Some(new_global_env.unbind());

    // 7. Return UNUSED.
}

/// ### [9.3.4 SetDefaultGlobalBindings ( realmRec )](https://tc39.es/ecma262/#sec-setdefaultglobalbindings)
///
/// The abstract operation SetDefaultGlobalBindings takes argument realmRec (a
/// Realm Record) and returns either a normal completion containing an Object
/// or a throw completion.
pub(crate) fn set_default_global_bindings<'a>(
    agent: &mut Agent,
    realm_id: Realm,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    // 1. Let global be realmRec.[[GlobalObject]].
    let global = agent[realm_id].global_object.scope(agent, gc.nogc());

    // 2. For each property of the Global Object specified in clause 19, do
    macro_rules! define_property {
        (intrinsic $name:ident, $value:ident) => {
            // most of the properties have this configuration
            let value = agent
                .get_realm_record_by_id(realm_id)
                .intrinsics()
                .$value()
                .into_value();
            define_property!($name, value, Some(true), Some(false), Some(true));
        };
        ($name:ident, $value:ident, $writable:expr, $enumerable:expr, $configurable:expr) => {
            // a. Let name be the String value of the property name.
            let name = PropertyKey::from(BUILTIN_STRING_MEMORY.$name);
            let value = $value;

            // b. Let desc be the fully populated data Property Descriptor for the
            //    property, containing the specified attributes for the property. For
            //    properties listed in 19.2, 19.3, or 19.4 the value of the [[Value]]
            //    attribute is the corresponding intrinsic object from realmRec.
            let desc = PropertyDescriptor {
                value: Some(value),
                writable: $writable,
                enumerable: $enumerable,
                configurable: $configurable,
                ..Default::default()
            };

            // c. Perform ? DefinePropertyOrThrow(global, name, desc).
            define_property_or_throw(agent, global.get(agent), name, desc, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
        };
    }

    // 19.1 Value Properties of the Global Object
    {
        // 19.1.1 globalThis
        let global_env = agent[realm_id].global_env.bind(gc.nogc());
        let value = global_env
            .unwrap()
            .get_this_binding(agent)
            .into_value()
            .unbind();
        define_property!(globalThis, value, Some(true), Some(false), Some(true));

        // 19.1.2 Infinity
        let value = Number::from_f64(agent, f64::INFINITY, gc.nogc())
            .into_value()
            .unbind();
        define_property!(Infinity, value, Some(false), Some(false), Some(false));

        // 19.1.3 NaN
        let value = Number::from_f64(agent, f64::NAN, gc.nogc())
            .into_value()
            .unbind();
        define_property!(NaN, value, Some(false), Some(false), Some(false));

        // 19.1.4 undefined
        let value = Value::Undefined;
        define_property!(undefined, value, Some(false), Some(false), Some(false));
    }

    // 19.2 Function Properties of the Global Object
    {
        // 19.2.1 eval ( x )
        define_property!(intrinsic eval, eval);

        // 19.2.2 isFinite ( number )
        define_property!(intrinsic isFinite, is_finite);

        // 19.2.3 isNaN ( number )
        define_property!(intrinsic isNaN, is_nan);

        // 19.2.4 parseFloat ( string )
        define_property!(intrinsic parseFloat, parse_float);

        // 19.2.5 parseInt ( string, radix )
        define_property!(intrinsic parseInt, parse_int);

        // 19.2.6.1 decodeURI ( . . . )
        define_property!(intrinsic decodeURI, decode_uri);

        // 19.2.6.2 decodeURIComponent ( . . . )
        define_property!(intrinsic decodeURIComponent, decode_uri_component);

        // 19.2.6.3 encodeURI ( . . . )
        define_property!(intrinsic encodeURI, encode_uri);

        // 19.2.6.4 encodeURIComponent ( . . . )
        define_property!(intrinsic encodeURIComponent, encode_uri_component);

        #[cfg(feature = "annex-b-global")]
        {
            // B.2.1.1 escape ( string )
            define_property!(intrinsic escape, escape);
            // B.2.1.2 unescape ( string )
            define_property!(intrinsic unescape, unescape);
        }
    }

    // 19.3 Constructor Properties of the Global Object
    {
        // 19.3.1 AggregateError ( . . . )
        define_property!(intrinsic AggregateError, aggregate_error);

        // 19.3.2 Array ( . . . )
        define_property!(intrinsic Array, array);

        // 19.3.3 ArrayBuffer ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic ArrayBuffer, array_buffer);

        // 19.3.4 BigInt ( . . . )
        define_property!(intrinsic BigInt, big_int);

        // 19.3.5 BigInt64Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic BigInt64Array, big_int64_array);

        // 19.3.6 BigUint64Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic BigUint64Array, big_uint64_array);

        // 19.3.7 Boolean ( . . . )
        define_property!(intrinsic Boolean, boolean);

        // 19.3.8 DataView ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic DataView, data_view);

        // 19.3.9 Date ( . . . )
        #[cfg(feature = "date")]
        define_property!(intrinsic Date, date);

        // 19.3.10 Error ( . . . )
        define_property!(intrinsic Error, error);

        // 19.3.11 EvalError ( . . . )
        define_property!(intrinsic EvalError, eval_error);

        // 19.3.12 FinalizationRegistry ( . . . )
        define_property!(intrinsic FinalizationRegistry, finalization_registry);

        // 19.3.13 Float16Array ( . . . )
        #[cfg(feature = "proposal-float16array")]
        define_property!(intrinsic Float16Array, float16_array);

        // 19.3.14 Float32Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Float32Array, float32_array);

        // 19.3.15 Float64Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Float64Array, float64_array);

        // 19.3.16 Function ( . . . )
        define_property!(intrinsic Function, function);

        // 19.3.17 Int8Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Int8Array, int8_array);

        // 19.3.18 Int16Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Int16Array, int16_array);

        // 19.3.19 Int32Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Int32Array, int32_array);

        // 19.3.20 Iterator ( . . . )
        define_property!(intrinsic Iterator, iterator);

        // 19.3.21 Map ( . . . )
        define_property!(intrinsic Map, map);

        // 19.3.22 Number ( . . . )
        define_property!(intrinsic Number, number);

        // 19.3.23 Object ( . . . )
        define_property!(intrinsic Object, object);

        // 19.3.24 Promise ( . . . )
        define_property!(intrinsic Promise, promise);

        // 19.3.25 Proxy ( . . . )
        define_property!(intrinsic Proxy, proxy);

        // 19.3.26 RangeError ( . . . )
        define_property!(intrinsic RangeError, range_error);

        // 19.3.27 ReferenceError ( . . . )
        define_property!(intrinsic ReferenceError, reference_error);

        // 19.3.28 RegExp ( . . . )
        #[cfg(feature = "regexp")]
        define_property!(intrinsic RegExp, reg_exp);

        // 19.3.29 Set ( . . . )
        #[cfg(feature = "set")]
        define_property!(intrinsic Set, set);

        // 19.3.30 SharedArrayBuffer ( . . . )
        #[cfg(feature = "shared-array-buffer")]
        define_property!(intrinsic SharedArrayBuffer, shared_array_buffer);

        // 19.3.31 String ( . . . )
        define_property!(intrinsic String, string);

        // 19.3.32 Symbol ( . . . )
        define_property!(intrinsic Symbol, symbol);

        // 19.3.33 SyntaxError ( . . . )
        define_property!(intrinsic SyntaxError, syntax_error);

        // 19.3.34 TypeError ( . . . )
        define_property!(intrinsic TypeError, type_error);

        // 19.3.35 Uint8Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Uint8Array, uint8_array);

        // 19.3.36 Uint8ClampedArray ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Uint8ClampedArray, uint8_clamped_array);

        // 19.3.37 Uint16Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Uint16Array, uint16_array);

        // 19.3.38 Uint32Array ( . . . )
        #[cfg(feature = "array-buffer")]
        define_property!(intrinsic Uint32Array, uint32_array);

        // 19.3.39 URIError ( . . . )
        define_property!(intrinsic URIError, uri_error);

        // 19.3.40 WeakMap ( . . . )
        #[cfg(feature = "weak-refs")]
        define_property!(intrinsic WeakMap, weak_map);

        // 19.3.41 WeakRef ( . . . )
        #[cfg(feature = "weak-refs")]
        define_property!(intrinsic WeakRef, weak_ref);

        // 19.3.42 WeakSet ( . . . )
        #[cfg(feature = "weak-refs")]
        define_property!(intrinsic WeakSet, weak_set);
    }

    // 19.4 Other Properties of the Global Object
    {
        // 19.4.1 Atomics
        #[cfg(feature = "atomics")]
        define_property!(intrinsic Atomics, atomics);

        // 19.4.2 JSON
        #[cfg(feature = "json")]
        define_property!(intrinsic JSON, json);

        // 19.4.3 Math
        #[cfg(feature = "math")]
        define_property!(intrinsic Math, math);

        // 19.4.4 Reflect
        define_property!(intrinsic Reflect, reflect);
    }

    // 3. Return global.
    Ok(global.get(agent).bind(gc.into_nogc()))
}

/// ## [9.6 InitializeHostDefinedRealm ( )](https://tc39.es/ecma262/#sec-initializehostdefinedrealm)
pub(crate) fn initialize_host_defined_realm(
    agent: &mut Agent,
    create_global_object: Option<impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>>,
    create_global_this_value: Option<
        impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
    >,
    initialize_global_object: Option<impl FnOnce(&mut Agent, Object, GcScope)>,
    mut gc: GcScope,
) {
    // 1. Let realm be CreateRealm().
    let realm = create_realm(agent, gc.nogc());

    // 2. Let newContext be a new execution context.
    let new_context = ExecutionContext {
        // NOTE: This property is assumed to be null until the specification
        //       assigns it.
        ecmascript_code: None,

        // 3. Set the Function of newContext to null.
        function: None,

        // 4. Set the Realm of newContext to realm.
        realm: realm.unbind(),

        // 5. Set the ScriptOrModule of newContext to null.
        script_or_module: None,
    };

    // 6. Push newContext onto the execution context stack; newContext is now the running execution context.
    agent.push_execution_context(new_context);

    // 7. If the host requires use of an exotic object to serve as realm's global object,
    // let global be such an object created in a host-defined manner.
    // Otherwise, let global be undefined, indicating that an ordinary object should be created as the global object.
    let global = create_global_this_value.map(|create_global_this_value| {
        create_global_this_value(agent, gc.reborrow())
            .unbind()
            .scope(agent, gc.nogc())
    });

    // 8. If the host requires that the this binding in realm's global scope return an object other than the global object,
    // let thisValue be such an object created in a host-defined manner.
    // Otherwise, let thisValue be undefined, indicating that realm's global this binding should be the global object.
    let this_value =
        create_global_object.map(|create_global_object| create_global_object(agent, gc.reborrow()));

    // 9. Perform SetRealmGlobalObject(realm, global, thisValue).
    set_realm_global_object(
        agent,
        agent.current_realm_id_internal(),
        global.map(|g| g.get(agent)),
        this_value.unbind(),
        gc.nogc(),
    );

    // 10. Let globalObj be ? SetDefaultGlobalBindings(realm).
    let global_object =
        set_default_global_bindings(agent, agent.current_realm_id_internal(), gc.reborrow())
            .unbind()
            .map_err(|err| {
                err.value()
                    .string_repr(agent, gc.reborrow())
                    .to_string_lossy(agent)
                    .to_string()
            })
            .unwrap()
            .unbind()
            .bind(gc.nogc());

    // 11. Create any host-defined global object properties on globalObj.
    if let Some(initialize_global_object) = initialize_global_object {
        initialize_global_object(agent, global_object.unbind(), gc.reborrow());
    };

    // 12. Return UNUSED.
}

pub(crate) fn initialize_default_realm(agent: &mut Agent, gc: GcScope) {
    let create_global_object: Option<for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>> = None;
    let create_global_this_value: Option<for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>> =
        None;
    let initialize_global_object: Option<fn(&mut Agent, Object, GcScope)> = None;
    initialize_host_defined_realm(
        agent,
        create_global_object,
        create_global_this_value,
        initialize_global_object,
        gc,
    );
}

#[cfg(test)]
mod test {
    use crate::ecmascript::types::BuiltinFunctionHeapData;
    #[allow(unused_imports)]
    use crate::{
        engine::context::{Bindable, GcScope},
        heap::{
            IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, IntrinsicObjectIndexes,
            LAST_INTRINSIC_CONSTRUCTOR_INDEX, LAST_INTRINSIC_FUNCTION_INDEX,
            LAST_INTRINSIC_OBJECT_INDEX, LAST_WELL_KNOWN_SYMBOL_INDEX,
        },
    };
    fn panic_builtin_function_missing(index: usize) {
        let index = index as u32;
        let mut changed_index = index;
        if changed_index <= LAST_INTRINSIC_CONSTRUCTOR_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!(
                "Found a missing BuiltinFunction at constructor index {:?}",
                unsafe { core::mem::transmute::<u32, IntrinsicConstructorIndexes>(changed_index) }
            );
        }
        changed_index -= LAST_INTRINSIC_CONSTRUCTOR_INDEX as u32 + 1;
        if changed_index <= LAST_INTRINSIC_FUNCTION_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!(
                "Found a missing BuiltinFunction at function index {:?}",
                unsafe { core::mem::transmute::<u32, IntrinsicFunctionIndexes>(changed_index) }
            );
        }
        panic!("Found a missing BuiltinFunction at index {index:?}");
    }

    fn panic_blank_object(index: usize) {
        let index = index as u32;
        let mut changed_index = index;
        if changed_index <= LAST_INTRINSIC_OBJECT_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!("Found a blank Object at object index {:?}", unsafe {
                core::mem::transmute::<u32, IntrinsicObjectIndexes>(changed_index)
            });
        }
        changed_index -= LAST_INTRINSIC_OBJECT_INDEX as u32 + 1;
        if changed_index <= LAST_INTRINSIC_CONSTRUCTOR_INDEX as u32 {
            // Safety: Tested to be within limits.
            panic!(
                "Found a blank BuiltinFunction at constructor index {:?}",
                unsafe { core::mem::transmute::<u32, IntrinsicConstructorIndexes>(changed_index) }
            );
        }
        panic!("Found a blank object at index {index:?}");
    }

    #[test]
    fn test_default_realm_sanity() {
        use super::initialize_default_realm;
        use crate::ecmascript::{
            execution::{Agent, DefaultHostHooks, agent::Options},
            types::ObjectRecord,
        };

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut gc, &mut scope);
        initialize_default_realm(&mut agent, gc);
        assert_eq!(
            agent
                .current_realm_record()
                .intrinsics()
                .object_prototype()
                .get_index(),
            0
        );
        let object_constructor = agent.current_realm_record().intrinsics().object();
        assert_eq!(object_constructor.get_index(), 0);
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
            .find(|(_, item)| *item == &BuiltinFunctionHeapData::BLANK);
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
        let blank_object = agent
            .heap
            .objects
            .iter()
            .enumerate()
            .find(|(_, item)| *item == &ObjectRecord::BLANK);
        if let Some((blank_object_index, _)) = blank_object {
            panic_blank_object(blank_object_index);
        }
        assert_eq!(agent.heap.realms.len(), 1);
        assert!(agent.heap.scripts.is_empty());
        assert_eq!(
            agent.heap.symbols.len() - 1,
            LAST_WELL_KNOWN_SYMBOL_INDEX as usize
        );
        #[cfg(feature = "regexp")]
        assert!(agent.heap.regexps.is_empty());
    }
}
