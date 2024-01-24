use oxc_ast::{
    ast::{
        BindingIdentifier, Declaration, ExportDefaultDeclarationKind, ForStatementInit,
        ForStatementLeft, Function, FunctionBody, LabeledStatement, ModuleDeclaration, Program,
        Statement, StaticBlock, VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
    },
    syntax_directed_operations::BoundNames,
};
use oxc_span::Atom;

/// ### [8.2.4 Static Semantics: LexicallyDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-lexicallydeclarednames)
/// The syntax-directed operation LexicallyDeclaredNames takes no arguments and returns a List of Strings.
pub(crate) trait LexicallyDeclaredNames<'a> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F);
}

pub(crate) fn script_lexically_declared_names(script: &Program<'_>) -> Vec<Atom> {
    let mut lexically_declared_names = Vec::new();
    // Script : [empty]
    // 1. Return a new empty List.
    // ScriptBody : StatementList
    // 1. Return TopLevelLexicallyDeclaredNames of StatementList.
    script
        .body
        .top_level_lexically_declared_names(&mut |identifier| {
            lexically_declared_names.push(identifier.name.clone());
        });
    // NOTE 1
    // At the top level of a Script, function declarations are treated like var declarations rather than like lexical declarations.
    lexically_declared_names
}

pub(crate) fn module_lexically_declared_names(script: &Program<'_>) -> Vec<Atom> {
    let mut lexically_declared_names = Vec::new();
    // NOTE 2
    // The LexicallyDeclaredNames of a Module includes the names of all of its imported bindings.

    // ModuleItemList : ModuleItemList ModuleItem
    // 1. Let names1 be LexicallyDeclaredNames of ModuleItemList.
    // 2. Let names2 be LexicallyDeclaredNames of ModuleItem.
    // 3. Return the list-concatenation of names1 and names2.
    // ModuleItem : StatementListItem
    // 1. Return LexicallyDeclaredNames of StatementListItem.
    script.body.lexically_declared_names(&mut |identifier| {
        lexically_declared_names.push(identifier.name.clone());
    });

    // NOTE 3
    // At the top level of a Module, function declarations are treated like lexical declarations rather than like var declarations.
    lexically_declared_names
}

// FunctionStatementList : [empty]
// 1. Return a new empty List.
// FunctionStatementList : StatementList
// 1. Return TopLevelLexicallyDeclaredNames of StatementList.
// ClassStaticBlockStatementList : [empty]
// 1. Return a new empty List.
// ClassStaticBlockStatementList : StatementList
// 1. Return the TopLevelLexicallyDeclaredNames of StatementList.
// ConciseBody : ExpressionBody
// 1. Return a new empty List.
// AsyncConciseBody : ExpressionBody
// 1. Return a new empty List.

impl<'a> LexicallyDeclaredNames<'a> for oxc_allocator::Vec<'_, Statement<'_>> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let names1 be LexicallyDeclaredNames of StatementList.
        // 2. Let names2 be LexicallyDeclaredNames of StatementListItem.
        // 3. Return the list-concatenation of names1 and names2.
        for ele in self {
            ele.lexically_declared_names(f);
        }
    }
}

impl<'a> LexicallyDeclaredNames<'a> for Statement<'_> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        match self {
            // Block : { }
            // 1. Return a new empty List.
            // StatementListItem : Statement
            // 1. If Statement is Statement : LabelledStatement , return LexicallyDeclaredNames of LabelledStatement.
            Statement::LabeledStatement(st) => st.lexically_declared_names(f),
            // 2. Return a new empty List.
            Statement::SwitchStatement(st) => {
                // CaseBlock : { }
                // 1. Return a new empty List.
                // CaseBlock : { CaseClausesopt DefaultClause CaseClausesopt }
                // 1. If the first CaseClauses is present, let names1 be the LexicallyDeclaredNames of the first CaseClauses.
                // 2. Else, let names1 be a new empty List.
                // 3. Let names2 be LexicallyDeclaredNames of DefaultClause.
                // 4. If the second CaseClauses is present, let names3 be the LexicallyDeclaredNames of the second CaseClauses.
                // 5. Else, let names3 be a new empty List.
                // 6. Return the list-concatenation of names1, names2, and names3.
                // CaseClauses : CaseClauses CaseClause
                // 1. Let names1 be LexicallyDeclaredNames of CaseClauses.
                // 2. Let names2 be LexicallyDeclaredNames of CaseClause.
                // 3. Return the list-concatenation of names1 and names2.
                // CaseClause : case Expression : StatementListopt
                // 1. If the StatementList is present, return the LexicallyDeclaredNames of StatementList.
                // 2. Return a new empty List.
                // DefaultClause : default : StatementListopt
                // 1. If the StatementList is present, return the LexicallyDeclaredNames of StatementList.
                // 2. Return a new empty List.
                for ele in &st.cases {
                    ele.consequent.lexically_declared_names(f);
                }
            }
            Statement::ModuleDeclaration(st) => {
                match &st.0 {
                    // ModuleItem : ImportDeclaration
                    // 1. Return the BoundNames of ImportDeclaration.
                    // NOTE 2
                    // The LexicallyDeclaredNames of a Module includes the names of all of its imported bindings.
                    ModuleDeclaration::ImportDeclaration(decl) => decl.bound_names(f),
                    ModuleDeclaration::ExportAllDeclaration(_decl) => {}
                    // ModuleItem : ExportDeclaration
                    ModuleDeclaration::ExportNamedDeclaration(decl) => {
                        if matches!(decl.declaration, Some(Declaration::VariableDeclaration(_))) {
                            // 1. If ExportDeclaration is export VariableStatement, return a new empty List.
                            return;
                        }
                        // 2. Return the BoundNames of ExportDeclaration.
                        decl.bound_names(f)
                    }
                    ModuleDeclaration::ExportDefaultDeclaration(decl) => {
                        // 2. Return the BoundNames of ExportDeclaration.
                        match &decl.declaration {
                            ExportDefaultDeclarationKind::Expression(_) => {}
                            ExportDefaultDeclarationKind::FunctionDeclaration(decl) => {
                                decl.bound_names(f)
                            }
                            ExportDefaultDeclarationKind::ClassDeclaration(decl) => {
                                decl.bound_names(f)
                            }
                            ExportDefaultDeclarationKind::TSInterfaceDeclaration(_)
                            | ExportDefaultDeclarationKind::TSEnumDeclaration(_) => unreachable!(),
                        }
                    }
                    ModuleDeclaration::TSExportAssignment(_)
                    | ModuleDeclaration::TSNamespaceExportDeclaration(_) => unreachable!(),
                }
            }
            // StatementListItem : Declaration
            // 1. Return the BoundNames of Declaration.
            Statement::Declaration(st) => st.bound_names(f),
            Statement::BlockStatement(_)
            | Statement::BreakStatement(_)
            | Statement::ContinueStatement(_)
            | Statement::DebuggerStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::EmptyStatement(_)
            | Statement::ExpressionStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::ForStatement(_)
            | Statement::IfStatement(_)
            | Statement::ReturnStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::TryStatement(_)
            | Statement::WhileStatement(_)
            | Statement::WithStatement(_) => {}
        }
    }
}

impl<'a> LexicallyDeclaredNames<'a> for LabeledStatement<'_> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        // LabelledStatement : LabelIdentifier : LabelledItem
        // 1. Return the LexicallyDeclaredNames of LabelledItem.
        // LabelledItem : Statement
        // 1. Return a new empty List.
        // LabelledItem : FunctionDeclaration
        // 1. Return BoundNames of FunctionDeclaration.
        self.body.lexically_declared_names(f);
    }
}

/// ### [8.2.6 Static Semantics: VarDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-vardeclarednames)
/// The syntax-directed operation VarDeclaredNames takes no arguments and returns a List of Strings.
pub(crate) trait VarDeclaredNames<'a> {
    fn var_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F);
}

pub(crate) fn script_var_declared_names(script: &Program<'_>) -> Vec<Atom> {
    let mut var_declared_names = Vec::new();
    // Script : [empty]
    // 1. Return a new empty List.
    // ScriptBody : StatementList
    // 1. Return TopLevelVarDeclaredNames of StatementList.
    script.body.top_level_var_declared_names(&mut |identifier| {
        var_declared_names.push(identifier.name.clone());
    });
    // NOTE 1
    // At the top level of a Script, function declarations are treated like var declarations rather than like lexical declarations.
    var_declared_names
}

pub(crate) fn module_var_declared_names(module: &Program<'_>) -> Vec<Atom> {
    let mut var_declared_names = Vec::new();
    // ModuleItemList : ModuleItemList ModuleItem
    // 1. Let names1 be VarDeclaredNames of ModuleItemList.
    // 2. Let names2 be VarDeclaredNames of ModuleItem.
    // 3. Return the list-concatenation of names1 and names2.
    module.body.var_declared_names(&mut |identifier| {
        var_declared_names.push(identifier.name.clone());
    });
    var_declared_names
}

pub(crate) fn function_var_declared_names(function: &FunctionBody<'_>) -> Vec<Atom> {
    let mut var_declared_names = Vec::new();
    // NOTE
    // This section is extended by Annex B.3.5.

    // FunctionStatementList : [empty]
    // 1. Return a new empty List.
    // FunctionStatementList : StatementList
    // 1. Return TopLevelVarDeclaredNames of StatementList.
    function
        .statements
        .top_level_var_declared_names(&mut |identifier| {
            var_declared_names.push(identifier.name.clone());
        });
    var_declared_names
}

pub(crate) fn class_static_block_var_declared_names(static_block: &StaticBlock<'_>) -> Vec<Atom> {
    let mut var_declared_names = Vec::new();
    // ClassStaticBlockStatementList : [empty]
    // 1. Return a new empty List.
    // ClassStaticBlockStatementList : StatementList
    // 1. Return the TopLevelVarDeclaredNames of StatementList.
    static_block
        .body
        .top_level_var_declared_names(&mut |identifier| {
            var_declared_names.push(identifier.name.clone());
        });
    var_declared_names
}

pub(crate) fn arrow_function_var_declared_names(arrow_function: &FunctionBody<'_>) -> Vec<Atom> {
    debug_assert!(arrow_function.statements.len() <= 1);
    if let Some(body) = arrow_function.statements.first() {
        debug_assert!(matches!(body, Statement::ExpressionStatement(_)));
    }
    // ConciseBody : ExpressionBody
    // 1. Return a new empty List.
    // AsyncConciseBody : ExpressionBody
    // 1. Return a new empty List.
    vec![]
}

impl<'a> VarDeclaredNames<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn var_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let names1 be VarDeclaredNames of StatementList.
        // 2. Let names2 be VarDeclaredNames of StatementListItem.
        // 3. Return the list-concatenation of names1 and names2.
        for ele in self {
            ele.var_declared_names(f);
        }
    }
}

impl<'a> VarDeclaredNames<'a> for Statement<'a> {
    fn var_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        match self {
            // Statement :
            // BreakStatement
            Statement::BreakStatement(_) |
            // ContinueStatement
            Statement::ContinueStatement(_) |
            // DebuggerStatement
            Statement::DebuggerStatement(_) |
            // EmptyStatement
            Statement::EmptyStatement(_) |
            // ExpressionStatement
            Statement::ExpressionStatement(_) |
            // ReturnStatement
            Statement::ReturnStatement(_) |
            // ThrowStatement
            Statement::ThrowStatement(_) => {
                // 1. Return a new empty List.
            }
            Statement::BlockStatement(st) => {
                if st.body.is_empty() {
                    // Block : { }
                    // 1. Return a new empty List.
                }
            }
            Statement::DoWhileStatement(st) => {
                // DoWhileStatement : do Statement while ( Expression ) ;
                // 1. Return the VarDeclaredNames of Statement.
                st.body.var_declared_names(f);
            },
            Statement::ForInStatement(st) => {
                // ForInOfStatement :
                // for ( LeftHandSideExpression in Expression ) Statement
                // for ( ForDeclaration in Expression ) Statement
                // for ( LeftHandSideExpression of AssignmentExpression ) Statement
                // for ( ForDeclaration of AssignmentExpression ) Statement
                // for await ( LeftHandSideExpression of AssignmentExpression ) Statement
                // for await ( ForDeclaration of AssignmentExpression ) Statement
                // 1. Return the VarDeclaredNames of Statement.
                // ForInOfStatement :
                // for ( var ForBinding in Expression ) Statement
                // for ( var ForBinding of AssignmentExpression ) Statement
                // for await ( var ForBinding of AssignmentExpression ) Statement
                // 1. Let names1 be the BoundNames of ForBinding.
                // 2. Let names2 be the VarDeclaredNames of Statement.
                // 3. Return the list-concatenation of names1 and names2.
                if !st.left.is_lexical_declaration() {
                    if let ForStatementLeft::VariableDeclaration(decl) = &st.left {
                        decl.bound_names(f);
                    }
                }
                st.body.var_declared_names(f);
            },
            Statement::ForOfStatement(st) => {
                if !st.left.is_lexical_declaration() {
                    if let ForStatementLeft::VariableDeclaration(decl) = &st.left {
                        decl.bound_names(f);
                    }
                }
                st.body.var_declared_names(f);
            },
            Statement::ForStatement(st) => {
                // ForStatement : for ( Expressionopt ; Expressionopt ; Expressionopt ) Statement
                // 1. Return the VarDeclaredNames of Statement.
                // ForStatement : for ( var VariableDeclarationList ; Expressionopt ; Expressionopt ) Statement
                // 2. Let names2 be VarDeclaredNames of Statement.
                // 3. Return the list-concatenation of names1 and names2.
                // ForStatement : for ( LexicalDeclaration Expressionopt ; Expressionopt ) Statement
                // 1. Return the VarDeclaredNames of Statement.
                if let Some(init) = &st.init {
                    match init {
                        ForStatementInit::VariableDeclaration(decl) => {
                            if decl.kind == VariableDeclarationKind::Var {
                                // 1. Let names1 be BoundNames of VariableDeclarationList.
                                decl.bound_names(f);
                            }
                        },
                        ForStatementInit::Expression(_) => todo!(),
                        ForStatementInit::UsingDeclaration(_) => todo!(),
                    }
                }
                st.body.var_declared_names(f);
            },
            Statement::IfStatement(st) => {
                // IfStatement : if ( Expression ) Statement else Statement
                // 1. Let names1 be VarDeclaredNames of the first Statement.
                // 2. Let names2 be VarDeclaredNames of the second Statement.
                // 3. Return the list-concatenation of names1 and names2.
                // IfStatement : if ( Expression ) Statement
                // 1. Return the VarDeclaredNames of Statement.
                st.consequent.var_declared_names(f);
                if let Some(st) = st.alternate.as_ref() { st.var_declared_names(f); }
            },
            Statement::LabeledStatement(st) => {
                // LabelledStatement : LabelIdentifier : LabelledItem
                // 1. Return the VarDeclaredNames of LabelledItem.
                if matches!(st.body, Statement::Declaration(Declaration::FunctionDeclaration(_))) {
                    // LabelledItem : FunctionDeclaration
                    // 1. Return a new empty List.
                    return;
                }
                st.body.var_declared_names(f);
            },
            Statement::SwitchStatement(st) => {
                // SwitchStatement : switch ( Expression ) CaseBlock
                // 1. Return the VarDeclaredNames of CaseBlock.
                for ele in &st.cases {
                    // CaseBlock : { }
                    // 1. Return a new empty List.
                    // CaseBlock : { CaseClausesopt DefaultClause CaseClausesopt }
                    // 1. If the first CaseClauses is present, let names1 be the VarDeclaredNames of the first CaseClauses.
                    // 2. Else, let names1 be a new empty List.
                    // 3. Let names2 be VarDeclaredNames of DefaultClause.
                    // 4. If the second CaseClauses is present, let names3 be the VarDeclaredNames of the second CaseClauses.
                    // 5. Else, let names3 be a new empty List.
                    // 6. Return the list-concatenation of names1, names2, and names3.
                    // CaseClauses : CaseClauses CaseClause
                    // 1. Let names1 be VarDeclaredNames of CaseClauses.
                    // 2. Let names2 be VarDeclaredNames of CaseClause.
                    // 3. Return the list-concatenation of names1 and names2.
                    // CaseClause : case Expression : StatementListopt
                    // 1. If the StatementList is present, return the VarDeclaredNames of StatementList.
                    // 2. Return a new empty List.
                    // DefaultClause : default : StatementListopt
                    // 1. If the StatementList is present, return the VarDeclaredNames of StatementList.
                    // 2. Return a new empty List.
                    ele.consequent.var_declared_names(f);
                }
            },
            Statement::TryStatement(st) => {
                // TryStatement : try Block Catch
                // 1. Let names1 be VarDeclaredNames of Block.
                st.block.body.var_declared_names(f);
                // 2. Let names2 be VarDeclaredNames of Catch.
                if let Some(catch) = &st.handler {
                    catch.body.body.var_declared_names(f);
                }
                // 3. Let names3 be VarDeclaredNames of Finally.
                if let Some(finally) = &st.finalizer {
                    finally.body.var_declared_names(f);
                }
                // 4. Return the list-concatenation of names1, names2, and names3.
            },
            Statement::WhileStatement(st) => {
                // WhileStatement : while ( Expression ) Statement
                // 1. Return the VarDeclaredNames of Statement.
                st.body.var_declared_names(f);
            },
            Statement::WithStatement(st) => {
                // WithStatement : with ( Expression ) Statement
                // 1. Return the VarDeclaredNames of Statement.
                st.body.var_declared_names(f);
            },
            Statement::ModuleDeclaration(decl) => {
                match &decl.0 {
                    // ModuleItem : ImportDeclaration
                    // 1. Return a new empty List.
                    ModuleDeclaration::ImportDeclaration(_) |
                    ModuleDeclaration::ExportAllDeclaration(_) |
                    ModuleDeclaration::ExportDefaultDeclaration(_) => {
                    },
                    // ModuleItem : ExportDeclaration
                    // 1. If ExportDeclaration is export VariableStatement, return BoundNames of ExportDeclaration.
                    // 2. Return a new empty List.
                    ModuleDeclaration::ExportNamedDeclaration(decl) => {
                        if let Some(Declaration::VariableDeclaration(decl)) = &decl.declaration {
                          decl.bound_names(f);
                        }
                    },
                    ModuleDeclaration::TSExportAssignment(_) |
                    ModuleDeclaration::TSNamespaceExportDeclaration(_) => unreachable!(),
                }
            },
            Statement::Declaration(decl) => {
                match decl {
                    Declaration::VariableDeclaration(var_decl) => {
                        // VariableStatement : var VariableDeclarationList ;
                        // 1. Return BoundNames of VariableDeclarationList.
                        var_decl.bound_names(f);
                    },
                    Declaration::FunctionDeclaration(_) => {
                    },
                    Declaration::ClassDeclaration(_) => todo!(),
                    Declaration::UsingDeclaration(_) => todo!(),
                    Declaration::TSTypeAliasDeclaration(_) => todo!(),
                    Declaration::TSInterfaceDeclaration(_) => todo!(),
                    Declaration::TSEnumDeclaration(_) => todo!(),
                    Declaration::TSModuleDeclaration(_) => todo!(),
                    Declaration::TSImportEqualsDeclaration(_) => todo!(),
                }
                // StatementListItem : Declaration
                // 1. Return a new empty List.
            },
        }
    }
}

/// ### [8.2.7 Static Semantics: VarScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-varscopeddeclarations)
/// The syntax-directed operation VarScopedDeclarations takes no arguments and returns a List of Parse Nodes.
pub(crate) trait VarScopedDeclarations<'a> {
    fn var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F);
}

pub(crate) fn script_var_scoped_declarations<'a>(
    script: &'a Program<'a>,
) -> Vec<VarScopedDeclaration<'a>> {
    let mut var_scoped_declarations = Vec::new();
    // Script : [empty]
    // 1. Return a new empty List.
    // ScriptBody : StatementList
    // 1. Return TopLevelVarScopedDeclarations of StatementList.
    script
        .body
        .top_level_var_scoped_declarations(&mut |declarator| {
            var_scoped_declarations.push(declarator);
        });
    var_scoped_declarations
}

pub(crate) fn module_var_scoped_declarations<'a>(
    module: &'a Program<'a>,
) -> Vec<VarScopedDeclaration<'a>> {
    let mut var_scoped_declarations = Vec::new();
    // Module : [empty]
    // 1. Return a new empty List.
    // ModuleItemList : ModuleItemList ModuleItem
    // 1. Let declarations1 be VarScopedDeclarations of ModuleItemList.
    // 2. Let declarations2 be VarScopedDeclarations of ModuleItem.
    // 3. Return the list-concatenation of declarations1 and declarations2.
    module.body.var_scoped_declarations(&mut |declarator| {
        var_scoped_declarations.push(declarator);
    });
    var_scoped_declarations
}
// FunctionStatementList : [empty]
// 1. Return a new empty List.
// FunctionStatementList : StatementList
// 1. Return the TopLevelVarScopedDeclarations of StatementList.
// ClassStaticBlockStatementList : [empty]
// 1. Return a new empty List.
// ClassStaticBlockStatementList : StatementList
// 1. Return the TopLevelVarScopedDeclarations of StatementList.
// ConciseBody : ExpressionBody
// 1. Return a new empty List.
// AsyncConciseBody : ExpressionBody
// 1. Return a new empty List.

#[derive(Debug, Clone, Copy)]
pub(crate) enum VarScopedDeclaration<'a> {
    VariableDeclaration(&'a VariableDeclarator<'a>),
    FunctionDeclaration(&'a Function<'a>),
}

impl<'a> VarScopedDeclarations<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let declarations1 be VarScopedDeclarations of StatementList.
        // 2. Let declarations2 be VarScopedDeclarations of StatementListItem.
        // 3. Return the list-concatenation of declarations1 and declarations2.
        for ele in self {
            ele.var_scoped_declarations(f);
        }
    }
}

impl<'a> VarScopedDeclarations<'a> for Statement<'a> {
    fn var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F) {
        match self {
            // Statement :
            // EmptyStatement
            Statement::EmptyStatement(_) |
            // ExpressionStatement
            Statement::ExpressionStatement(_) |
            // ContinueStatement
            Statement::ContinueStatement(_) |
            // BreakStatement
            Statement::BreakStatement(_) |
            // ReturnStatement
            Statement::ReturnStatement(_) |
            // ThrowStatement
            Statement::ThrowStatement(_) |
            // DebuggerStatement
            Statement::DebuggerStatement(_) => {
                // 1. Return a new empty List.
            }
            Statement::BlockStatement(st) => {
                // Block : { }
                if st.body.is_empty() {
                    // 1. Return a new empty List.
                }
            },
            // StatementListItem : Declaration
            Statement::Declaration(decl) => {
                match decl {
                    Declaration::VariableDeclaration(decl) => {
                        decl.var_scoped_declarations(f);
                    },
                    _ => {
                        // 1. Return a new empty List.
                    },
                }
            },
            Statement::IfStatement(st) => {
                // IfStatement : if ( Expression ) Statement else Statement
                // 1. Let declarations1 be VarScopedDeclarations of the first Statement.
                // 2. Let declarations2 be VarScopedDeclarations of the second Statement.
                // 3. Return the list-concatenation of declarations1 and declarations2.
                // IfStatement : if ( Expression ) Statement
                // 1. Return the VarScopedDeclarations of Statement.
                st.consequent.var_scoped_declarations(f);
                if let Some(alternate) = &st.alternate {
                    alternate.var_scoped_declarations(f);
                }
            },
            Statement::DoWhileStatement(st) => {
                // DoWhileStatement : do Statement while ( Expression ) ;
                // 1. Return the VarScopedDeclarations of Statement.
                st.body.var_scoped_declarations(f);
            },
            Statement::WhileStatement(st) => {
                // WhileStatement : while ( Expression ) Statement
                // 1. Return the VarScopedDeclarations of Statement.
                st.body.var_scoped_declarations(f);
            },
            Statement::ForStatement(st) => {
                // ForStatement : for ( var VariableDeclarationList ; Expressionopt ; Expressionopt ) Statement
                if let Some(ForStatementInit::VariableDeclaration(var_decl)) = &st.init {
                    // 1. Let declarations1 be VarScopedDeclarations of VariableDeclarationList.
                    var_decl.var_scoped_declarations(f);
                }
                // 2. Let declarations2 be VarScopedDeclarations of Statement.
                st.body.var_scoped_declarations(f);
                // 3. Return the list-concatenation of declarations1 and declarations2.
            },
            Statement::ForInStatement(st) => {
                if let ForStatementLeft::VariableDeclaration(var_decl) = &st.left {
                    // ForInOfStatement :
                    // for ( var ForBinding in Expression ) Statement
                    // for ( var ForBinding of AssignmentExpression ) Statement
                    // for await ( var ForBinding of AssignmentExpression ) Statement
                    // 1. Let declarations1 be « ForBinding ».
                    var_decl.var_scoped_declarations(f);
                }
                // 2. Let declarations2 be VarScopedDeclarations of Statement.
                st.body.var_scoped_declarations(f);
                // 3. Return the list-concatenation of declarations1 and declarations2.
            },
            Statement::ForOfStatement(st) => {
                if let ForStatementLeft::VariableDeclaration(var_decl) = &st.left {
                    // ForInOfStatement :
                    // for ( var ForBinding in Expression ) Statement
                    // for ( var ForBinding of AssignmentExpression ) Statement
                    // for await ( var ForBinding of AssignmentExpression ) Statement
                    // 1. Let declarations1 be « ForBinding ».
                    var_decl.var_scoped_declarations(f);
                }
                // 2. Let declarations2 be VarScopedDeclarations of Statement.
                st.body.var_scoped_declarations(f);
                // 3. Return the list-concatenation of declarations1 and declarations2.
            },
            Statement::WithStatement(st) => {
                // NOTE
                // This section is extended by Annex B.3.5.

                // WithStatement : with ( Expression ) Statement
                // 1. Return the VarScopedDeclarations of Statement.
                st.body.var_scoped_declarations(f);
            },
            Statement::SwitchStatement(st) => {
                // SwitchStatement : switch ( Expression ) CaseBlock
                // 1. Return the VarScopedDeclarations of CaseBlock.
                for ele in &st.cases {
                    // CaseBlock : { }
                    // 1. Return a new empty List.
                    // CaseBlock : { CaseClausesopt DefaultClause CaseClausesopt }
                    // 1. If the first CaseClauses is present, let declarations1 be the VarScopedDeclarations of the first CaseClauses.
                    // 2. Else, let declarations1 be a new empty List.
                    // 3. Let declarations2 be VarScopedDeclarations of DefaultClause.
                    // 4. If the second CaseClauses is present, let declarations3 be the VarScopedDeclarations of the second CaseClauses.
                    // 5. Else, let declarations3 be a new empty List.
                    // 6. Return the list-concatenation of declarations1, declarations2, and declarations3.
                    // CaseClauses : CaseClauses CaseClause
                    // 1. Let declarations1 be VarScopedDeclarations of CaseClauses.
                    // 2. Let declarations2 be VarScopedDeclarations of CaseClause.
                    // 3. Return the list-concatenation of declarations1 and declarations2.
                    // CaseClause : case Expression : StatementListopt
                    // 1. If the StatementList is present, return the VarScopedDeclarations of StatementList.
                    // 2. Return a new empty List.
                    // DefaultClause : default : StatementListopt
                    // 1. If the StatementList is present, return the VarScopedDeclarations of StatementList.
                    // 2. Return a new empty List.
                    ele.consequent.var_scoped_declarations(f);
                }
            },
            Statement::LabeledStatement(st) => {
                // LabelledStatement : LabelIdentifier : LabelledItem
                // 1. Return the VarScopedDeclarations of LabelledItem.
                // LabelledItem : FunctionDeclaration
                if let Statement::Declaration(Declaration::FunctionDeclaration(_)) = &st.body {
                    // 1. Return a new empty List.
                    return;
                }
                st.body.var_scoped_declarations(f);
            },
            Statement::TryStatement(st) => {
                // TryStatement : try Block Catch Finally
                // 1. Let declarations1 be VarScopedDeclarations of Block.
                st.block.body.var_scoped_declarations(f);
                // 2. Let declarations2 be VarScopedDeclarations of Catch.
                if let Some(handler) = &st.handler {
                    // Catch : catch ( CatchParameter ) Block
                    // 1. Return the VarScopedDeclarations of Block.
                    handler.body.body.var_scoped_declarations(f);
                }
                // 3. Let declarations3 be VarScopedDeclarations of Finally.
                if let Some(finally) = &st.finalizer {
                    finally.body.var_scoped_declarations(f);
                }
                // 4. Return the list-concatenation of declarations1, declarations2, and declarations3.
            },
            Statement::ModuleDeclaration(st) => {
                match &st.0 {
                    // ModuleItem : ImportDeclaration
                    ModuleDeclaration::ImportDeclaration(_) => {
                        // 1. Return a new empty List.
                    },
                    // ModuleItem : ExportDeclaration
                    ModuleDeclaration::ExportAllDeclaration(_) |
                    ModuleDeclaration::ExportDefaultDeclaration(_) => {}
                    ModuleDeclaration::ExportNamedDeclaration(decl) => {
                        // 1. If ExportDeclaration is export VariableStatement, return VarScopedDeclarations of VariableStatement.
                        if let Some(Declaration::VariableDeclaration(decl)) = &decl.declaration {
                            decl.var_scoped_declarations(f);
                        }
                    },
                    ModuleDeclaration::TSExportAssignment(_) |
                    ModuleDeclaration::TSNamespaceExportDeclaration(_) => unreachable!(),
                }
                // 2. Return a new empty List.
            },
        }
    }
}

impl<'a> VarScopedDeclarations<'a> for VariableDeclaration<'a> {
    fn var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F) {
        if self.kind != VariableDeclarationKind::Var {
            return;
        }
        // VariableDeclarationList : VariableDeclaration
        // 1. Return « VariableDeclaration ».
        // VariableDeclarationList : VariableDeclarationList , VariableDeclaration
        // 1. Let declarations1 be VarScopedDeclarations of VariableDeclarationList.
        // 2. Return the list-concatenation of declarations1 and « VariableDeclaration ».
        for declarator in &self.declarations {
            f(VarScopedDeclaration::VariableDeclaration(unsafe {
                std::mem::transmute(declarator)
            }));
        }
    }
}

/// ### [8.2.8 Static Semantics: TopLevelLexicallyDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-toplevellexicallydeclarednames)
/// The syntax-directed operation TopLevelLexicallyDeclaredNames takes no arguments and returns a List of Strings.
trait TopLevelLexicallyDeclaredNames<'a> {
    fn top_level_lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F);
}

impl<'a> TopLevelLexicallyDeclaredNames<'a> for oxc_allocator::Vec<'_, Statement<'_>> {
    fn top_level_lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let names1 be TopLevelLexicallyDeclaredNames of StatementList.
        // 2. Let names2 be TopLevelLexicallyDeclaredNames of StatementListItem.
        // 3. Return the list-concatenation of names1 and names2.
        for ele in self {
            ele.lexically_declared_names(f);
        }
    }
}

impl<'a> TopLevelLexicallyDeclaredNames<'a> for Statement<'_> {
    fn top_level_lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        // StatementListItem : Statement
        // 1. Return a new empty List.
        // NOTE
        // At the top level of a function, or script, function declarations are treated like var declarations rather than like lexical declarations.
        if let Statement::Declaration(decl) = self {
            match decl {
                // 1. If Declaration is Declaration : HoistableDeclaration , then
                // a. Return a new empty List.
                Declaration::FunctionDeclaration(_) => {}
                // 2. Return the BoundNames of Declaration.
                Declaration::VariableDeclaration(decl) => decl.bound_names(f),
                Declaration::ClassDeclaration(decl) => decl.bound_names(f),
                Declaration::UsingDeclaration(decl) => decl.bound_names(f),
                Declaration::TSTypeAliasDeclaration(_)
                | Declaration::TSInterfaceDeclaration(_)
                | Declaration::TSEnumDeclaration(_)
                | Declaration::TSModuleDeclaration(_)
                | Declaration::TSImportEqualsDeclaration(_) => unreachable!(),
            }
        }
    }
}

/// ### [8.2.9 Static Semantics: TopLevelLexicallyScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-toplevellexicallyscopeddeclarations)
/// The syntax-directed operation TopLevelLexicallyScopedDeclarations takes no arguments and returns a List of Parse Nodes.
trait TopLevelLexicallyScopedDeclarations<'a> {
    fn top_level_lexically_scoped_declarations<F: FnMut(&Declaration<'a>)>(&self, f: &mut F);
}

impl<'a> TopLevelLexicallyScopedDeclarations<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn top_level_lexically_scoped_declarations<F: FnMut(&Declaration<'a>)>(&self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let declarations1 be TopLevelLexicallyScopedDeclarations of StatementList.
        // 2. Let declarations2 be TopLevelLexicallyScopedDeclarations of StatementListItem.
        // 3. Return the list-concatenation of declarations1 and declarations2.
        for ele in self {
            ele.top_level_lexically_scoped_declarations(f);
        }
    }
}

impl<'a> TopLevelLexicallyScopedDeclarations<'a> for Statement<'a> {
    fn top_level_lexically_scoped_declarations<F: FnMut(&Declaration<'a>)>(&self, f: &mut F) {
        // StatementListItem : Declaration
        if let Statement::Declaration(decl) = self {
            // 1. If Declaration is Declaration : HoistableDeclaration , then
            if matches!(decl, Declaration::FunctionDeclaration(_)) {
                // a. Return a new empty List.
                return;
            }
            // 2. Return « Declaration ».
            f(decl);
        }
        // StatementListItem : Statement
        // 1. Return a new empty List.
    }
}

/// ### [8.2.10 Static Semantics: TopLevelVarDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-toplevelvardeclarednames)
/// The syntax-directed operation TopLevelVarDeclaredNames takes no arguments and returns a List of Strings.
trait TopLevelVarDeclaredNames<'a> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F);
}

impl<'a> TopLevelVarDeclaredNames<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let names1 be TopLevelVarDeclaredNames of StatementList.
        // 2. Let names2 be TopLevelVarDeclaredNames of StatementListItem.
        // 3. Return the list-concatenation of names1 and names2.
        for ele in self {
            ele.top_level_lexically_declared_names(f);
        }
    }
}

impl<'a> TopLevelVarDeclaredNames<'a> for Statement<'a> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        match self {
            Statement::LabeledStatement(st) => {
                // StatementListItem : Statement
                // 1. If Statement is Statement : LabelledStatement , return TopLevelVarDeclaredNames of Statement.
                st.top_level_var_declared_names(f);
                // NOTE
                // At the top level of a function or script, inner function declarations are treated like var declarations.
            }
            Statement::Declaration(decl) => {
                // StatementListItem : Declaration
                // 1. If Declaration is Declaration : HoistableDeclaration , then
                match decl {
                    // a. Return the BoundNames of HoistableDeclaration.
                    Declaration::FunctionDeclaration(decl) => decl.bound_names(f),
                    Declaration::TSTypeAliasDeclaration(_)
                    | Declaration::TSInterfaceDeclaration(_)
                    | Declaration::TSEnumDeclaration(_)
                    | Declaration::TSModuleDeclaration(_)
                    | Declaration::TSImportEqualsDeclaration(_) => unreachable!(),
                    // 2. Return a new empty List.
                    _ => {}
                }
            }
            _ => {
                // 2. Return VarDeclaredNames of Statement.
            }
        }
    }
}

impl<'a> TopLevelVarDeclaredNames<'a> for LabeledStatement<'a> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        // LabelledStatement : LabelIdentifier : LabelledItem
        // 1. Return the TopLevelVarDeclaredNames of LabelledItem.

        // LabelledItem : Statement
        // 1. If Statement is Statement : LabelledStatement , return TopLevelVarDeclaredNames of Statement.
        if let Statement::LabeledStatement(st) = &self.body {
            st.top_level_var_declared_names(f);
        }
        // 2. Return VarDeclaredNames of Statement.

        // LabelledItem : FunctionDeclaration
        if let Statement::Declaration(Declaration::FunctionDeclaration(decl)) = &self.body {
            // 1. Return BoundNames of FunctionDeclaration.´
            decl.bound_names(f);
        }
    }
}

/// ### [8.2.11 Static Semantics: TopLevelVarScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-toplevelvarscopeddeclarations)
///
/// The syntax-directed operation TopLevelVarScopedDeclarations takes no arguments and returns a List of Parse Nodes.
trait TopLevelVarScopedDeclarations<'a> {
    fn top_level_var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F);
}

impl<'a> TopLevelVarScopedDeclarations<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn top_level_var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let declarations1 be TopLevelVarScopedDeclarations of StatementList.
        // 2. Let declarations2 be TopLevelVarScopedDeclarations of StatementListItem.
        // 3. Return the list-concatenation of declarations1 and declarations2.
        for ele in self {
            ele.top_level_var_scoped_declarations(f);
        }
    }
}

impl<'a> TopLevelVarScopedDeclarations<'a> for Statement<'a> {
    fn top_level_var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F) {
        match self {
            // StatementListItem : Statement
            Statement::LabeledStatement(st) => {
                // 1. If Statement is Statement : LabelledStatement , return TopLevelVarScopedDeclarations of Statement.
                st.top_level_var_scoped_declarations(f);
            }
            // StatementListItem : Declaration
            Statement::Declaration(decl) => {
                // 1. If Declaration is Declaration : HoistableDeclaration , then
                match decl {
                    Declaration::VariableDeclaration(decl) => {
                        // VariableDeclarations are actually VariableStatements and fall into
                        // 2. Return VarScopedDeclarations of Statement.
                        decl.var_scoped_declarations(f);
                    }
                    Declaration::FunctionDeclaration(decl) => {
                        // a. Let declaration be DeclarationPart of HoistableDeclaration.
                        // b. Return « declaration ».
                        f(VarScopedDeclaration::FunctionDeclaration(decl));
                    }
                    _ => {
                        // 2. Return a new empty List.
                    }
                }
            }
            // 2. Return VarScopedDeclarations of Statement.
            Statement::BlockStatement(_)
            | Statement::BreakStatement(_)
            | Statement::ContinueStatement(_)
            | Statement::DebuggerStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::EmptyStatement(_)
            | Statement::ExpressionStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::ForStatement(_)
            | Statement::IfStatement(_)
            | Statement::ReturnStatement(_)
            | Statement::SwitchStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::TryStatement(_)
            | Statement::WhileStatement(_)
            | Statement::WithStatement(_)
            | Statement::ModuleDeclaration(_) => {
                self.var_scoped_declarations(f);
            }
        }
    }
}

impl<'a> TopLevelVarScopedDeclarations<'a> for LabeledStatement<'a> {
    fn top_level_var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F) {
        // 1. If Statement is Statement : LabelledStatement , return TopLevelVarScopedDeclarations of Statement.
        if let Statement::LabeledStatement(st) = &self.body {
            // LabelledStatement : LabelIdentifier : LabelledItem
            // 1. Return the TopLevelVarScopedDeclarations of LabelledItem.
            // LabelledItem : Statement
            // 1. If Statement is Statement : LabelledStatement , return TopLevelVarScopedDeclarations of Statement.
            st.top_level_var_scoped_declarations(f);
        } else if let Statement::Declaration(Declaration::FunctionDeclaration(decl)) = &self.body {
            // LabelledItem : FunctionDeclaration
            // 1. Return « FunctionDeclaration ».
            f(VarScopedDeclaration::FunctionDeclaration(decl));
        }
        // 2. Return VarScopedDeclarations of Statement.
        self.body.var_scoped_declarations(f);
    }
}
