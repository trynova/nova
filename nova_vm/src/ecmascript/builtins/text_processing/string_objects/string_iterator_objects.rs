// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::create_iter_result_object,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, IntoValue, Object,
            OrdinaryObject, String, Value,
        },
    },
    engine::context::{Bindable, GcScope, NoGcScope, bindable_handle},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WellKnownSymbolIndexes, WorkQueues, indexes::BaseIndex,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct StringIterator<'a>(BaseIndex<'a, StringIteratorHeapData<'static>>);

impl<'a> StringIterator<'a> {
    pub fn create(agent: &mut Agent, string: String, gc: NoGcScope<'a, '_>) -> StringIterator<'a> {
        agent
            .heap
            .create(StringIteratorHeapData::new(string))
            .bind(gc)
    }

    pub fn is_completed(self, agent: &Agent) -> bool {
        // a. Let len be the length of s.
        // b. Let position be 0.
        // c. Repeat, while position < len,
        // d. Return undefined.
        let StringIteratorHeapData { s, position, .. } = self.get_data(agent);
        let len = s.len(agent);
        *position >= len
    }

    /// # Do not use this
    /// This is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn get_data(self, agent: &Agent) -> &StringIteratorHeapData<'a> {
        agent
            .heap
            .string_iterators
            .get(self.get_index())
            .expect("StringIterator use-after-free")
            .as_ref()
            .expect("StringIterator deleted")
    }

    pub(crate) fn get_data_mut(self, agent: &mut Agent) -> &mut StringIteratorHeapData<'static> {
        agent
            .heap
            .string_iterators
            .get_mut(self.get_index())
            .expect("StringIterator use-after-free")
            .as_mut()
            .expect("StringIterator deleted")
    }
}

impl<'a> From<StringIterator<'a>> for Object<'a> {
    fn from(iter: StringIterator<'a>) -> Self {
        Object::StringIterator(iter)
    }
}

impl<'a> From<StringIterator<'a>> for Value<'a> {
    fn from(iter: StringIterator<'a>) -> Self {
        Value::StringIterator(iter)
    }
}

impl<'a> TryFrom<Value<'a>> for StringIterator<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::StringIterator(iter) => Ok(iter),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for StringIterator<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::StringIterator(iter) => Ok(iter),
            _ => Err(()),
        }
    }
}

bindable_handle!(StringIterator);

impl<'a> InternalSlots<'a> for StringIterator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::StringIterator;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get_data(agent).backing_object.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_data_mut(agent)
                .backing_object
                .replace(backing_object)
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for StringIterator<'a> {}

impl HeapMarkAndSweep for StringIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.string_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.string_iterators.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for StringIterator<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .string_iterators
            .shift_weak_index(self.0)
            .map(Self)
    }
}

pub(crate) struct StringIteratorPrototype;

struct StringIteratorPrototypeNext;
impl Builtin for StringIteratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringIteratorPrototype::next);
}

impl StringIteratorPrototype {
    /// ### [22.1.5.1.1 %StringIteratorPrototype%.next ( )](https://tc39.es/ecma262/#sec-%stringiteratorprototype%.next)
    fn next<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 1. Return ? GeneratorResume(this value, empty, "%StringIteratorPrototype%").
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let Value::StringIterator(generator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "not a string iterator",
                gc,
            ));
        };
        // 2. If state is completed, return CreateIteratorResultObject(undefined, true).
        if generator.is_completed(agent) {
            return Ok(create_iter_result_object(agent, Value::Undefined, true).into_value());
        }
        let StringIteratorHeapData { s, position, .. } = generator.get_data(agent);
        let position = *position;
        // 3. Assert: state is either suspended-start or suspended-yield.
        // i. Let cp be CodePointAt(s, position).
        let cp = s
            .as_wtf8(agent)
            .slice_from(position)
            .code_points()
            .next()
            .expect("Unexpected end of StringIterator data");
        // iii. Let resultString be the substring of s from position to nextIndex.
        let result_string = String::from_code_point(cp);
        // ii. Let nextIndex be position + cp.[[CodeUnitCount]].
        let next_index = position + result_string.len(agent);
        // iv. Set position to nextIndex.
        generator.get_data_mut(agent).position = next_index;
        // v. Perform ? GeneratorYield(CreateIteratorResultObject(resultString, false)).
        // 11. Return ? result.
        Ok(create_iter_result_object(agent, result_string.into_value(), false).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.string_iterator_prototype();
        let iterator_prototype = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(iterator_prototype)
            .with_builtin_function_property::<StringIteratorPrototypeNext>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.String_Iterator.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[derive(Debug)]
pub struct StringIteratorHeapData<'a> {
    backing_object: Option<OrdinaryObject<'a>>,
    s: String<'a>,
    /// UTF-8 index into s.
    position: usize,
}

impl<'a> StringIteratorHeapData<'a> {
    pub(crate) fn new(string: String<'a>) -> Self {
        Self {
            backing_object: None,
            s: string,
            position: 0,
        }
    }
}

bindable_handle!(StringIteratorHeapData);

impl<'a> CreateHeapData<StringIteratorHeapData<'a>, StringIterator<'a>> for Heap {
    fn create(&mut self, data: StringIteratorHeapData<'a>) -> StringIterator<'a> {
        self.string_iterators.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<StringIteratorHeapData<'static>>>();
        StringIterator(BaseIndex::last(&self.string_iterators))
    }
}

impl HeapMarkAndSweep for StringIteratorHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            backing_object,
            s,
            position: _,
        } = self;
        backing_object.mark_values(queues);
        s.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            backing_object,
            s,
            position: _,
        } = self;
        backing_object.sweep_values(compactions);
        s.sweep_values(compactions);
    }
}
