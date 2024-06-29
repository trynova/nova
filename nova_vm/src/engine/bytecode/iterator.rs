use std::collections::VecDeque;

use crate::ecmascript::{
    execution::{Agent, JsResult},
    types::{InternalMethods, Object, PropertyKey},
};

#[derive(Debug)]
pub(super) enum VmIterator {
    ObjectProperties(ObjectPropertiesIterator),
}

#[derive(Debug)]
pub(super) struct ObjectPropertiesIterator {
    object: Object,
    object_was_visited: bool,
    visited_keys: Vec<PropertyKey>,
    remaining_keys: VecDeque<PropertyKey>,
}

impl ObjectPropertiesIterator {
    pub(super) fn new(object: Object) -> Self {
        Self {
            object,
            object_was_visited: false,
            visited_keys: Default::default(),
            remaining_keys: Default::default(),
        }
    }

    pub(super) fn next(&mut self, agent: &mut Agent) -> JsResult<Option<PropertyKey>> {
        loop {
            let object = self.object;
            if !self.object_was_visited {
                let keys = object.internal_own_property_keys(agent)?;
                for key in keys {
                    if let PropertyKey::Symbol(_) = key {
                        continue;
                    } else {
                        self.remaining_keys.push_back(key);
                    }
                }
                self.object_was_visited = true;
            }
            while let Some(r) = self.remaining_keys.pop_front() {
                if self.visited_keys.contains(&r) {
                    continue;
                }
                let desc = object.internal_get_own_property(agent, r)?;
                if let Some(desc) = desc {
                    self.visited_keys.push(r);
                    if desc.enumerable == Some(true) {
                        return Ok(Some(r));
                    }
                }
            }
            let prototype = object.internal_get_prototype_of(agent)?;
            if let Some(prototype) = prototype {
                self.object_was_visited = false;
                self.object = prototype;
            } else {
                return Ok(None);
            }
        }
    }
}
