use crate::{
    execution::{
        Agent, ECMAScriptCode, EnvironmentIndex, ExecutionContext, RealmIdentifier, ScriptOrModule,
    },
    types::Value,
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
    pub ecmascript_code: Rc<Program<'host>>,

    // TODO: [[LoadedModules]]
    /// [[HostDefined]]
    pub host_defined: Option<HostDefined<'host>>,
}

#[derive(Debug)]
pub enum ScriptOrErrors<'ctx, 'host> {
    Script(Script<'ctx, 'host>),
    Errors(Vec<oxc_diagnostics::Error>),
}

impl<'ctx, 'host: 'ctx> Script<'ctx, 'host> {
    /// 16.1.5 ParseScript ( sourceText, realm, hostDefined )
    /// https://tc39.es/ecma262/#sec-parse-script
    pub fn parse(
        allocator: &'host Allocator,
        source_text: &'host str,
        realm: RealmIdentifier<'ctx, 'host>,
        host_defined: Option<HostDefined<'host>>,
    ) -> ScriptOrErrors<'ctx, 'host> {
        // 1. Let script be ParseText(sourceText, Script).
        // 2. If script is a List of errors, return script.
        let parser = Parser::new(&allocator, source_text, SourceType::default());
        let script = parser.parse();

        if script.errors.len() != 0 {
            return ScriptOrErrors::Errors(script.errors);
        }

        // 3. Return Script Record {
        //      [[Realm]]: realm, [[ECMAScriptCode]]: script, [[LoadedModules]]: « », [[HostDefined]]: hostDefined
        //    }.
        ScriptOrErrors::Script(Self {
            realm,
            ecmascript_code: Rc::new(script.program),
            host_defined,
        })
    }

    /// 16.1.6 ScriptEvaluation ( scriptRecord )
    /// https://tc39.es/ecma262/#sec-runtime-semantics-scriptevaluation
    pub fn evaluate(self, agent: &mut Agent<'ctx, 'host>) -> Value {
        let ecmascript_code = self.ecmascript_code.clone();
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
            script_or_module: Some(ScriptOrModule::Script(Rc::new(RefCell::new(self)))),

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
        let script = ecmascript_code.as_ref();

        // TODO: 12. Let result be Completion(GlobalDeclarationInstantiation(script, globalEnv)).
        // NOTE: This is totally ad-hoc for now.
        let mut seen = HashMap::new();
        for variable_declaration in script.body.iter() {
            match &variable_declaration {
                Statement::Declaration(decl) => match decl {
                    Declaration::VariableDeclaration(decl) => {
                        if decl.kind.is_var() {
                            for decl in decl.declarations.iter() {
                                let var_name = match &decl.id.kind {
                                    BindingPatternKind::BindingIdentifier(name) => {
                                        name.name.as_str()
                                    }
                                    _ => continue,
                                };

                                if !seen.contains_key(var_name) {
                                    // global_env.create_global_var_binding(agent, var_name, false);
                                    _ = seen.insert(var_name, ());
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // 13. If result.[[Type]] is normal, then
        //     a. Set result to Completion(Evaluation of script).
        //     b. If result.[[Type]] is normal and result.[[Value]] is empty, then
        //         i. Set result to NormalCompletion(undefined).

        // 14. Suspend scriptContext and remove it from the execution context stack.
        _ = agent.execution_context_stack.pop();

        // 15. Assert: The execution context stack is not empty.
        debug_assert!(agent.execution_context_stack.len() > 0);

        // TODO: 16. Resume the context that is now on the top of the execution context stack as the
        //     running execution context.

        // 17. Return ? result.
        todo!()
    }
}
