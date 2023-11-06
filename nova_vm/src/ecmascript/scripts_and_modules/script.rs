use crate::{
    ecmascript::{
        execution::{
            Agent, ECMAScriptCode, EnvironmentIndex, ExecutionContext, JsResult, RealmIdentifier,
        },
        scripts_and_modules::ScriptOrModule,
        types::Value,
    },
    engine::{Executable, Vm},
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPatternKind, Declaration, Program, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

pub type HostDefined<'ctx> = &'ctx mut dyn Any;

/// 16.1.4 Script Records
/// https://tc39.es/ecma262/#sec-script-records
#[derive(Debug)]
pub struct Script<'ctx, 'host> {
    /// [[Realm]]
    pub realm: RealmIdentifier<'ctx, 'host>,

    /// [[ECMAScriptCode]]
    pub ecmascript_code: Program<'ctx>,

    // TODO: [[LoadedModules]]
    /// [[HostDefined]]
    pub host_defined: Option<HostDefined<'host>>,
}

pub type ScriptOrErrors<'ctx, 'host> = Result<Script<'ctx, 'host>, Vec<oxc_diagnostics::Error>>;

impl<'ctx, 'host> Script<'ctx, 'host> {
    /// 16.1.5 ParseScript ( sourceText, realm, hostDefined )
    /// https://tc39.es/ecma262/#sec-parse-script
    pub fn parse(
        allocator: &'ctx Allocator,
        source_text: &'ctx str,
        realm: RealmIdentifier<'ctx, 'host>,
        host_defined: Option<HostDefined<'host>>,
    ) -> ScriptOrErrors<'ctx, 'host> {
        // 1. Let script be ParseText(sourceText, Script).
        // 2. If script is a List of errors, return script.
        let parser = Parser::new(allocator, source_text, SourceType::default());
        let script = parser.parse();

        if !script.errors.is_empty() {
            return Err(script.errors);
        }

        // 3. Return Script Record {
        //      [[Realm]]: realm, [[ECMAScriptCode]]: script, [[LoadedModules]]: « », [[HostDefined]]: hostDefined
        //    }.
        Ok(Script {
            realm,
            ecmascript_code: script.program,
            host_defined,
        })
    }

    /// 16.1.6 ScriptEvaluation ( scriptRecord )
    /// https://tc39.es/ecma262/#sec-runtime-semantics-scriptevaluation
    pub fn evaluate(self, agent: &mut Agent<'ctx, 'host>) -> JsResult<Value> {
        let realm_id = self.realm;
        let realm = agent.get_realm_mut(realm_id);

        // 1. Let globalEnv be scriptRecord.[[Realm]].[[GlobalEnv]].
        let global_env = realm.global_env;

        // 2. Let scriptContext be a new ECMAScript code execution context.
        let script_context = ExecutionContext {
            // 3. Set the Function of scriptContext to null.
            function: None,

            // 4. Set the Realm of scriptContext to scriptRecord.[[Realm]].
            realm: realm_id,

            // 5. Set the ScriptOrModule of scriptContext to scriptRecord.
            script_or_module: Some(ScriptOrModule::Script(self)),

            ecmascript_code: Some(ECMAScriptCode {
                // 6. Set the VariableEnvironment of scriptContext to globalEnv.
                variable_environment: EnvironmentIndex::GlobalEnvironment(global_env),

                // 7. Set the LexicalEnvironment of scriptContext to globalEnv.
                lexical_environment: EnvironmentIndex::GlobalEnvironment(global_env),

                // 8. Set the PrivateEnvironment of scriptContext to null.
                private_environment: None,
            }),
        };

        // TODO: 9. Suspend the running execution context.

        // 10. Push scriptContext onto the execution context stack; scriptContext is now the running execution context.
        agent.execution_context_stack.push(script_context);

        // 11. Let script be scriptRecord.[[ECMAScriptCode]].
        let ScriptOrModule::Script(Script {
            ecmascript_code: script,
            ..
        }) = &agent
            .execution_context_stack
            .last()
            .as_ref()
            .unwrap()
            .script_or_module
            .as_ref()
            .unwrap()
        else {
            unreachable!();
        };

        // TODO: 12. Let result be Completion(GlobalDeclarationInstantiation(script, globalEnv)).
        // NOTE: This is totally ad-hoc for now.
        let mut seen = HashMap::new();
        for variable_declaration in script.body.iter() {
            if let Statement::Declaration(Declaration::VariableDeclaration(decl)) =
                variable_declaration
            {
                if decl.kind.is_var() {
                    for decl in decl.declarations.iter() {
                        let var_name =
                            if let BindingPatternKind::BindingIdentifier(name) = &decl.id.kind {
                                name.name.as_str()
                            } else {
                                continue;
                            };

                        if !seen.contains_key(var_name) {
                            // global_env.create_global_var_binding(agent, var_name, false);
                            _ = seen.insert(var_name, ());
                        }
                    }
                }
            }
        }

        // 13. If result.[[Type]] is normal, then
        //     a. Set result to Completion(Evaluation of script).
        //     b. If result.[[Type]] is normal and result.[[Value]] is empty, then
        //         i. Set result to NormalCompletion(undefined).
        // TODO: Follow these steps exactly.

        let exe = Executable::compile(&mut agent.heap, &script.body);
        let result = Vm::execute(agent, &exe)?;

        // 14. Suspend scriptContext and remove it from the execution context stack.
        _ = agent.execution_context_stack.pop();

        // TODO: 15. Assert: The execution context stack is not empty.
        // debug_assert!(!agent.execution_context_stack.is_empty());

        // TODO: 16. Resume the context that is now on the top of the execution context stack as the
        //     running execution context.

        // 17. Return ? result.
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::Script;
    use crate::ecmascript::{
        execution::{agent::Options, Agent, DefaultHostHooks, Realm},
        types::Value,
    };
    use oxc_allocator::Allocator;

    #[test]
    fn empty_script() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = Realm::create(&mut agent);

        let script = Script::parse(&allocator, "", realm, None).unwrap();

        let result = script.evaluate(&mut agent).unwrap();

        assert_eq!(result, Value::Undefined);
    }

    #[test]
    fn basic_constants() {
        let allocator = Allocator::default();

        let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
        let realm = Realm::create(&mut agent);

        let script = Script::parse(&allocator, "true", realm, None).unwrap();

        let result = script.evaluate(&mut agent).unwrap();

        assert_eq!(result, true.into());
    }
}
