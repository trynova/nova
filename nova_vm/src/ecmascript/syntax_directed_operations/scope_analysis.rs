use oxc_ast::{
    ast::{
        BindingIdentifier, Declaration, Expression, ForStatementInit, LabeledStatement,
        ModuleDeclaration, Program, Statement, VariableDeclarationKind,
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
            Statement::BlockStatement(st) => {}
            Statement::BreakStatement(st) => {}
            Statement::ContinueStatement(st) => {}
            Statement::DebuggerStatement(st) => {}
            Statement::DoWhileStatement(st) => {}
            Statement::EmptyStatement(st) => {}
            Statement::ExpressionStatement(st) => {
                st.expression.lexically_declared_names(f);
            }
            Statement::ForInStatement(st) => {}
            Statement::ForOfStatement(st) => {}
            Statement::ForStatement(st) => {}
            Statement::IfStatement(st) => {}
            // StatementListItem : Statement
            // 1. If Statement is Statement : LabelledStatement , return LexicallyDeclaredNames of LabelledStatement.
            Statement::LabeledStatement(st) => st.lexically_declared_names(f),
            // 2. Return a new empty List.
            Statement::ReturnStatement(st) => {}
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
            Statement::ThrowStatement(st) => {}
            Statement::TryStatement(st) => {}
            Statement::WhileStatement(st) => {}
            Statement::WithStatement(st) => {}
            Statement::ModuleDeclaration(st) => {
                match &st.0 {
                    // ModuleItem : ImportDeclaration
                    // 1. Return the BoundNames of ImportDeclaration.
                    // NOTE 2
                    // The LexicallyDeclaredNames of a Module includes the names of all of its imported bindings.
                    ModuleDeclaration::ImportDeclaration(decl) => decl.bound_names(f),
                    ModuleDeclaration::ExportAllDeclaration(decl) => {}
                    // ModuleItem : ExportDeclaration
                    ModuleDeclaration::ExportDefaultDeclaration(decl) => {
                        // TODO: We should bind *default* and the declaration's bound names here I think
                    }
                    ModuleDeclaration::ExportNamedDeclaration(decl) => {
                        if matches!(
                            decl.0.declaration,
                            Some(Declaration::VariableDeclaration(_))
                        ) {
                            // 1. If ExportDeclaration is export VariableStatement, return a new empty List.
                            return;
                        }
                        // 2. Return the BoundNames of ExportDeclaration.
                        decl.0.bound_names(f)
                    }
                    ModuleDeclaration::TSExportAssignment(_)
                    | ModuleDeclaration::TSNamespaceExportDeclaration(_) => unreachable!(),
                }
            }
            // StatementListItem : Declaration
            // 1. Return the BoundNames of Declaration.
            Statement::Declaration(st) => st.bound_names(f),
        }
    }
}

impl<'a> LexicallyDeclaredNames<'a> for Expression<'_> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier)>(&self, f: &mut F) {
        match self {
            Expression::BooleanLiteral(_) => todo!(),
            Expression::NullLiteral(_) => todo!(),
            Expression::NumberLiteral(_) => todo!(),
            Expression::BigintLiteral(_) => todo!(),
            Expression::RegExpLiteral(_) => todo!(),
            Expression::StringLiteral(_) => todo!(),
            Expression::TemplateLiteral(_) => todo!(),
            Expression::Identifier(_) => todo!(),
            Expression::MetaProperty(_) => todo!(),
            Expression::Super(_) => todo!(),
            Expression::ArrayExpression(_) => todo!(),
            Expression::ArrowExpression(_) => todo!(),
            Expression::AssignmentExpression(_) => todo!(),
            Expression::AwaitExpression(_) => todo!(),
            Expression::BinaryExpression(_) => todo!(),
            Expression::CallExpression(_) => todo!(),
            Expression::ChainExpression(_) => todo!(),
            Expression::ClassExpression(_) => todo!(),
            Expression::ConditionalExpression(_) => todo!(),
            Expression::FunctionExpression(_) => todo!(),
            Expression::ImportExpression(_) => todo!(),
            Expression::LogicalExpression(_) => todo!(),
            Expression::MemberExpression(_) => todo!(),
            Expression::NewExpression(_) => todo!(),
            Expression::ObjectExpression(_) => todo!(),
            Expression::ParenthesizedExpression(_) => todo!(),
            Expression::SequenceExpression(_) => todo!(),
            Expression::TaggedTemplateExpression(_) => todo!(),
            Expression::ThisExpression(_) => todo!(),
            Expression::UnaryExpression(_) => todo!(),
            Expression::UpdateExpression(_) => todo!(),
            Expression::YieldExpression(_) => todo!(),
            Expression::PrivateInExpression(_) => todo!(),
            Expression::JSXElement(_) => todo!(),
            Expression::JSXFragment(_) => todo!(),
            Expression::TSAsExpression(_) => todo!(),
            Expression::TSSatisfiesExpression(_) => todo!(),
            Expression::TSTypeAssertion(_) => todo!(),
            Expression::TSNonNullExpression(_) => todo!(),
            Expression::TSInstantiationExpression(_) => todo!(),
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

/// 8.2.6 Static Semantics: VarDeclaredNames
/// The syntax-directed operation VarDeclaredNames takes no arguments and returns a List of Strings. It is defined piecewise over the following productions:
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
                    return;
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
                    match &st.left {
                        oxc_ast::ast::ForStatementLeft::VariableDeclaration(decl) => decl.bound_names(f),
                        _ => {},
                    }
                }
                st.body.var_declared_names(f);
            },
            Statement::ForOfStatement(st) => {
                if !st.left.is_lexical_declaration() {
                    match &st.left {
                        oxc_ast::ast::ForStatementLeft::VariableDeclaration(decl) => decl.bound_names(f),
                        _ => {},
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
                st.alternate.as_ref().map(|st| {
                    st.var_declared_names(f);
                });
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
                        if let Some(decl) = &decl.declaration {
                            match decl {
                                Declaration::VariableDeclaration(decl) => decl.bound_names(f),
                                _ => {},
                            }
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

// NOTE
// This section is extended by Annex B.3.5.

// FunctionStatementList : [empty]
// 1. Return a new empty List.
// FunctionStatementList : StatementList
// 1. Return TopLevelVarDeclaredNames of StatementList.
// ClassStaticBlockStatementList : [empty]
// 1. Return a new empty List.
// ClassStaticBlockStatementList : StatementList
// 1. Return the TopLevelVarDeclaredNames of StatementList.
// ConciseBody : ExpressionBody
// 1. Return a new empty List.
// AsyncConciseBody : ExpressionBody
// 1. Return a new empty List.

/// ### [8.2.8 Static Semantics: TopLevelLexicallyDeclaredNames]()
/// The syntax-directed operation TopLevelLexicallyDeclaredNames takes no arguments and returns a List of Strings.
pub(crate) trait TopLevelLexicallyDeclaredNames<'a> {
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
        match self {
            // StatementListItem : Declaration
            Statement::Declaration(decl) => {
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
            _ => {}
        }
    }
}

/// ### [8.2.9 Static Semantics: TopLevelLexicallyScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-toplevellexicallyscopeddeclarations)
/// The syntax-directed operation TopLevelLexicallyScopedDeclarations takes no arguments and returns a List of Parse Nodes.
pub(crate) trait TopLevelLexicallyScopedDeclarations<'a> {
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
/// The syntax-directed operation TopLevelVarDeclaredNames takes no arguments and returns a List of Strings. It is defined piecewise over the following productions:
pub(crate) trait TopLevelVarDeclaredNames<'a> {
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
