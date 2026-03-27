#![allow(dead_code, unused_variables, clippy::disallowed_names)]

type Agent = nova_vm::ecmascript::Agent;

fn test_no_params() {
    unimplemented!()
}

fn test_one_param(_foo: ()) {
    unimplemented!()
}

fn test_owned_qualified_agent_only(agent: nova_vm::ecmascript::Agent) {
    unimplemented!()
}

fn test_owned_agent_only(agent: Agent) {
    unimplemented!()
}

fn test_borrowed_agent_only(agent: &Agent) {
    unimplemented!()
}

fn test_mut_agent_only(agent: &mut Agent) {
    unimplemented!()
}

fn test_multiple_agents(agent1: Agent, agent2: Agent) {
    unimplemented!()
}

fn test_something_else_before_agent(foo: (), agent: Agent) {
    unimplemented!()
}

fn test_multiple_agents_with_something_in_between(agent1: Agent, foo: (), agent2: Agent) {
    unimplemented!()
}

fn test_impl_can_come_before_agent(foo: impl std::fmt::Debug, agent: Agent) {
    unimplemented!()
}

fn test_generic_can_come_before_agent<T>(foo: T, agent: Agent) {
    unimplemented!()
}

struct Test;

impl Test {
    fn test_no_params(&self) {
        unimplemented!()
    }

    fn test_one_param(&self, _foo: ()) {
        unimplemented!()
    }

    fn test_self_and_owned_agent_only(&self, agent: Agent) {
        unimplemented!()
    }

    fn test_self_and_something_before_agent(&self, foo: (), agent: &Agent) {
        unimplemented!()
    }

    fn test_something_before_agent(foo: (), agent: &Agent) {
        unimplemented!()
    }

    fn test_self_and_impl_can_come_before_agent(&self, foo: impl std::fmt::Debug, agent: Agent) {
        unimplemented!()
    }

    fn test_self_and_generic_can_come_before_agent<T>(&self, foo: T, agent: Agent) {
        unimplemented!()
    }

    // TODO: Support `Self` as a leading parameter before `Agent`
    // fn test_uppercase_self(me: Self, agent: Agent) {
    //   unimplemented!()
    // }
    // fn test_optional_uppercase_self(me: Option<Self>, agent: Agent) {
    //   unimplemented!()
    // }
    // fn test_optional_borrowed_uppercase_self(me: Option<&Self>, agent: Agent) {
    //   unimplemented!()
    // }
}

fn main() {
    unimplemented!()
}
