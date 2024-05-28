use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use crate::{
    ecmascript::{
        builtins::ArgumentsList,
        execution::{Agent, JsResult, RealmIdentifier},
        types::{
            Function, InternalMethods, IntoFunction, IntoObject, IntoValue, Object,
            OrdinaryObjectInternalSlots, String, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, ObjectIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

pub(crate) trait AbstractClosureBehaviour: HeapMarkAndSweep {
    fn call(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments: Option<ArgumentsList>,
    ) -> JsResult<Value>;
}

pub struct AbstractClosureHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: RealmIdentifier,
    /// #### \[\[InitialName]]
    /// A String that is the initial name of the function. It is used by
    /// 20.2.3.5 (`Function.prototype.toString()`).
    pub(crate) initial_name: Option<String>,
    pub(crate) behaviour: Box<dyn AbstractClosureBehaviour>,
}

unsafe impl Send for AbstractClosureHeapData {}

pub type AbstractClosureIndex = BaseIndex<AbstractClosureHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct AbstractClosure(AbstractClosureIndex);

impl AbstractClosure {
    /// ## Never use this; this is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        AbstractClosure(AbstractClosureIndex::from_u32_index(0))
    }
}

impl From<AbstractClosure> for Value {
    fn from(value: AbstractClosure) -> Self {
        Value::BuiltinAbstractClosure(value)
    }
}

impl From<AbstractClosure> for Object {
    fn from(value: AbstractClosure) -> Self {
        Object::BuiltinAbstractClosure(value)
    }
}

impl From<AbstractClosure> for Function {
    fn from(value: AbstractClosure) -> Self {
        Function::BuiltinAbstractClosure(value)
    }
}

impl IntoValue for AbstractClosure {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for AbstractClosure {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoFunction for AbstractClosure {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl Debug for AbstractClosureHeapData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbstractClosureHeapData")
            .field("object_index", &self.object_index)
            .field("length", &self.length)
            .field("realm", &self.realm)
            .field("initial_name", &self.initial_name)
            .field("behaviour", &"some closure")
            .finish()
    }
}

impl OrdinaryObjectInternalSlots for AbstractClosure {
    fn internal_extensible(self, agent: &Agent) -> bool {
        todo!()
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        todo!()
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        todo!()
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        todo!()
    }
}

impl InternalMethods for AbstractClosure {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        todo!()
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: crate::ecmascript::types::PropertyKey,
    ) -> JsResult<Option<super::PropertyDescriptor>> {
        todo!()
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: crate::ecmascript::types::PropertyKey,
        property_descriptor: super::PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: crate::ecmascript::types::PropertyKey,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: crate::ecmascript::types::PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        todo!()
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: crate::ecmascript::types::PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_delete(
        self,
        agent: &mut Agent,
        property_key: crate::ecmascript::types::PropertyKey,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_own_property_keys(
        self,
        agent: &mut Agent,
    ) -> JsResult<Vec<crate::ecmascript::types::PropertyKey>> {
        todo!()
    }

    fn internal_call(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn internal_construct(
        self,
        _agent: &mut Agent,
        _arguments_list: ArgumentsList,
        _new_target: Function,
    ) -> JsResult<Object> {
        todo!()
    }
}

impl Index<AbstractClosure> for Agent {
    type Output = AbstractClosureHeapData;

    fn index(&self, index: AbstractClosure) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<AbstractClosure> for Agent {
    fn index_mut(&mut self, index: AbstractClosure) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<AbstractClosure> for Heap {
    type Output = AbstractClosureHeapData;

    fn index(&self, index: AbstractClosure) -> &Self::Output {
        self.abstract_closures
            .get(index.0.into_index())
            .expect("AbstractClosure out of bounds")
            .as_ref()
            .expect("AbstractClosure slot empty")
    }
}

impl IndexMut<AbstractClosure> for Heap {
    fn index_mut(&mut self, index: AbstractClosure) -> &mut Self::Output {
        self.abstract_closures
            .get_mut(index.0.into_index())
            .expect("AbstractClosure out of bounds")
            .as_mut()
            .expect("AbstractClosure slot empty")
    }
}

impl HeapMarkAndSweep for AbstractClosureHeapData {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.object_index.mark_values(queues);
        self.realm.mark_values(queues);
        self.initial_name.mark_values(queues);
        self.behaviour.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.realm.sweep_values(compactions);
        self.initial_name.sweep_values(compactions);
        self.behaviour.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for AbstractClosure {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.abstract_closures.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        let idx = &mut self.0;
        let value = idx.into_u32();
        *idx = AbstractClosureIndex::from_u32(
            value - compactions.abstract_closures.get_shift_for_index(value),
        );
    }
}

impl CreateHeapData<AbstractClosureHeapData, AbstractClosure> for Heap {
    fn create(&mut self, data: AbstractClosureHeapData) -> AbstractClosure {
        self.abstract_closures.push(Some(data));
        AbstractClosure(AbstractClosureIndex::last(&self.abstract_closures))
    }
}
