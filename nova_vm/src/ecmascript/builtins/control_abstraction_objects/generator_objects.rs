// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ExecutionContext, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, Value,
        },
    },
    engine::Executable,
    heap::{
        indexes::{BaseIndex, GeneratorIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generator(pub(crate) GeneratorIndex);

impl Generator {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<Generator> for GeneratorIndex {
    fn from(val: Generator) -> Self {
        val.0
    }
}

impl From<GeneratorIndex> for Generator {
    fn from(value: GeneratorIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Generator {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Generator {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Generator> for Value {
    fn from(val: Generator) -> Self {
        Value::Generator(val)
    }
}

impl From<Generator> for Object {
    fn from(value: Generator) -> Self {
        Object::Generator(value)
    }
}

impl InternalSlots for Generator {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Generator;

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl InternalMethods for Generator {}

impl CreateHeapData<GeneratorHeapData, Generator> for Heap {
    fn create(&mut self, data: GeneratorHeapData) -> Generator {
        self.generators.push(Some(data));
        Generator(GeneratorIndex::last(&self.generators))
    }
}

impl Index<Generator> for Agent {
    type Output = GeneratorHeapData;

    fn index(&self, index: Generator) -> &Self::Output {
        &self.heap.generators[index]
    }
}

impl IndexMut<Generator> for Agent {
    fn index_mut(&mut self, index: Generator) -> &mut Self::Output {
        &mut self.heap.generators[index]
    }
}

impl Index<Generator> for Vec<Option<GeneratorHeapData>> {
    type Output = GeneratorHeapData;

    fn index(&self, index: Generator) -> &Self::Output {
        self.get(index.get_index())
            .expect("Generator out of bounds")
            .as_ref()
            .expect("Generator slot empty")
    }
}

impl IndexMut<Generator> for Vec<Option<GeneratorHeapData>> {
    fn index_mut(&mut self, index: Generator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Generator out of bounds")
            .as_mut()
            .expect("Generator slot empty")
    }
}

impl HeapMarkAndSweep for Generator {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.generators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.generators.shift_index(&mut self.0)
    }
}

#[derive(Debug, Default)]
pub struct GeneratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) generator_state: Option<GeneratorState>,
}

#[derive(Debug)]
pub(crate) enum GeneratorState {
    SuspendedStart {
        executable: Executable,
        execution_context: ExecutionContext,
    },
    Executing,
    Completed,
}

impl HeapMarkAndSweep for GeneratorHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        if let Some(GeneratorState::SuspendedStart {
            executable,
            execution_context,
        }) = &self.generator_state
        {
            executable.mark_values(queues);
            execution_context.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        if let Some(GeneratorState::SuspendedStart {
            executable,
            execution_context,
        }) = &mut self.generator_state
        {
            executable.sweep_values(compactions);
            execution_context.sweep_values(compactions);
        }
    }
}
