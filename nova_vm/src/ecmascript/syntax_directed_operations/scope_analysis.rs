// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::Deref;

use oxc_ast::ast::{
    self, BindingIdentifier, BlockStatement, Class, Declaration, ExportDefaultDeclarationKind,
    ForStatementInit, ForStatementLeft, Function, FunctionBody, LabeledStatement, Program,
    Statement, StaticBlock, SwitchCase, SwitchStatement, VariableDeclaration,
    VariableDeclarationKind, VariableDeclarator,
};
use oxc_ecmascript::BoundNames;

use oxc_span::Atom;

/// ### [8.2.4 Static Semantics: LexicallyDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-lexicallydeclarednames)
///
/// The syntax-directed operation LexicallyDeclaredNames takes no arguments and
/// returns a List of Strings.
pub(crate) trait LexicallyDeclaredNames<'a> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F);
}

pub(crate) fn script_lexically_declared_names<'a, 'b: 'a>(
    script: &'b Program<'a>,
) -> Vec<Atom<'a>> {
    let mut lexically_declared_names: Vec<Atom<'a>> = vec![];
    // Script : [empty]
    // 1. Return a new empty List.
    // ScriptBody : StatementList
    // 1. Return TopLevelLexicallyDeclaredNames of StatementList.
    script
        .body
        .top_level_lexically_declared_names(&mut |identifier| {
            lexically_declared_names.push(identifier.name);
        });
    // NOTE 1
    // At the top level of a Script, function declarations are treated like var declarations rather than like lexical declarations.
    lexically_declared_names
}

pub(crate) fn module_lexically_declared_names<'a>(script: &'a Program<'a>) -> Vec<Atom<'a>> {
    let mut lexically_declared_names = vec![];
    // NOTE 2
    // The LexicallyDeclaredNames of a Module includes the names of all of its imported bindings.

    // ModuleItemList : ModuleItemList ModuleItem
    // 1. Let names1 be LexicallyDeclaredNames of ModuleItemList.
    // 2. Let names2 be LexicallyDeclaredNames of ModuleItem.
    // 3. Return the list-concatenation of names1 and names2.
    // ModuleItem : StatementListItem
    // 1. Return LexicallyDeclaredNames of StatementListItem.
    script.body.lexically_declared_names(&mut |identifier| {
        lexically_declared_names.push(identifier.name);
    });

    // NOTE 3
    // At the top level of a Module, function declarations are treated like lexical declarations rather than like var declarations.
    lexically_declared_names
}

pub(crate) fn function_body_lexically_declared_names<'a>(
    body: &'a FunctionBody<'a>,
) -> Vec<Atom<'a>> {
    let mut lexically_declared_names = vec![];
    // FunctionStatementList : [empty]
    // 1. Return a new empty List.
    // FunctionStatementList : StatementList
    // 1. Return TopLevelLexicallyDeclaredNames of StatementList.
    // ClassStaticBlockStatementList : [empty]
    // 1. Return a new empty List.
    // ClassStaticBlockStatementList : StatementList
    // 1. Return the TopLevelLexicallyDeclaredNames of StatementList.
    body.statements
        .top_level_lexically_declared_names(&mut |identifier| {
            lexically_declared_names.push(identifier.name);
        });
    lexically_declared_names
}

// ConciseBody : ExpressionBody
// 1. Return a new empty List.
// AsyncConciseBody : ExpressionBody
// 1. Return a new empty List.

impl<'a> LexicallyDeclaredNames<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let names1 be LexicallyDeclaredNames of StatementList.
        // 2. Let names2 be LexicallyDeclaredNames of StatementListItem.
        // 3. Return the list-concatenation of names1 and names2.
        for ele in self {
            ele.lexically_declared_names(f);
        }
    }
}

impl<'a> LexicallyDeclaredNames<'a> for Statement<'a> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
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
            // ModuleItem : ImportDeclaration
            // 1. Return the BoundNames of ImportDeclaration.
            // NOTE 2
            // The LexicallyDeclaredNames of a Module includes the names of all of its imported bindings.
            Statement::ImportDeclaration(decl) => decl.bound_names(f),
            Statement::ExportAllDeclaration(_decl) => {}
            // ModuleItem : ExportDeclaration
            Statement::ExportNamedDeclaration(decl) => {
                if matches!(decl.declaration, Some(Declaration::VariableDeclaration(_))) {
                    // 1. If ExportDeclaration is export VariableStatement, return a new empty List.
                    return;
                }
                // 2. Return the BoundNames of ExportDeclaration.
                decl.bound_names(f)
            }
            Statement::ExportDefaultDeclaration(decl) => {
                // 2. Return the BoundNames of ExportDeclaration.
                match &decl.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(decl) => decl.bound_names(f),
                    ExportDefaultDeclarationKind::ClassDeclaration(decl) => decl.bound_names(f),
                    ExportDefaultDeclarationKind::BooleanLiteral(_)
                    | ExportDefaultDeclarationKind::NullLiteral(_)
                    | ExportDefaultDeclarationKind::NumericLiteral(_)
                    | ExportDefaultDeclarationKind::BigIntLiteral(_)
                    | ExportDefaultDeclarationKind::RegExpLiteral(_)
                    | ExportDefaultDeclarationKind::StringLiteral(_)
                    | ExportDefaultDeclarationKind::TemplateLiteral(_)
                    | ExportDefaultDeclarationKind::Identifier(_)
                    | ExportDefaultDeclarationKind::MetaProperty(_)
                    | ExportDefaultDeclarationKind::Super(_)
                    | ExportDefaultDeclarationKind::ArrayExpression(_)
                    | ExportDefaultDeclarationKind::ArrowFunctionExpression(_)
                    | ExportDefaultDeclarationKind::AssignmentExpression(_)
                    | ExportDefaultDeclarationKind::AwaitExpression(_)
                    | ExportDefaultDeclarationKind::BinaryExpression(_)
                    | ExportDefaultDeclarationKind::CallExpression(_)
                    | ExportDefaultDeclarationKind::ChainExpression(_)
                    | ExportDefaultDeclarationKind::ClassExpression(_)
                    | ExportDefaultDeclarationKind::ConditionalExpression(_)
                    | ExportDefaultDeclarationKind::FunctionExpression(_)
                    | ExportDefaultDeclarationKind::ImportExpression(_)
                    | ExportDefaultDeclarationKind::LogicalExpression(_)
                    | ExportDefaultDeclarationKind::NewExpression(_)
                    | ExportDefaultDeclarationKind::ObjectExpression(_)
                    | ExportDefaultDeclarationKind::ParenthesizedExpression(_)
                    | ExportDefaultDeclarationKind::SequenceExpression(_)
                    | ExportDefaultDeclarationKind::TaggedTemplateExpression(_)
                    | ExportDefaultDeclarationKind::ThisExpression(_)
                    | ExportDefaultDeclarationKind::UnaryExpression(_)
                    | ExportDefaultDeclarationKind::UpdateExpression(_)
                    | ExportDefaultDeclarationKind::YieldExpression(_)
                    | ExportDefaultDeclarationKind::PrivateInExpression(_)
                    | ExportDefaultDeclarationKind::V8IntrinsicExpression(_)
                    | ExportDefaultDeclarationKind::ComputedMemberExpression(_)
                    | ExportDefaultDeclarationKind::StaticMemberExpression(_)
                    | ExportDefaultDeclarationKind::PrivateFieldExpression(_) => {}
                    #[cfg(feature = "typescript")]
                    ExportDefaultDeclarationKind::TSNonNullExpression(_) => {}
                    #[cfg(not(feature = "typescript"))]
                    ExportDefaultDeclarationKind::TSNonNullExpression(_) => unreachable!(),
                    ExportDefaultDeclarationKind::JSXElement(_)
                    | ExportDefaultDeclarationKind::JSXFragment(_)
                    | ExportDefaultDeclarationKind::TSAsExpression(_)
                    | ExportDefaultDeclarationKind::TSInstantiationExpression(_)
                    | ExportDefaultDeclarationKind::TSInterfaceDeclaration(_)
                    | ExportDefaultDeclarationKind::TSSatisfiesExpression(_)
                    | ExportDefaultDeclarationKind::TSTypeAssertion(_) => unreachable!(),
                }
            }
            // StatementListItem : Declaration
            // 1. Return the BoundNames of Declaration.
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
            Statement::VariableDeclaration(decl) => decl.bound_names(f),
            Statement::FunctionDeclaration(decl) => decl.bound_names(f),
            Statement::ClassDeclaration(decl) => decl.bound_names(f),
            Statement::TSEnumDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSInterfaceDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSNamespaceExportDeclaration(_)
            | Statement::TSTypeAliasDeclaration(_) => {
                unreachable!()
            }
        }
    }
}

impl<'a> LexicallyDeclaredNames<'a> for LabeledStatement<'a> {
    fn lexically_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
        // LabelledStatement : LabelIdentifier : LabelledItem
        // 1. Return the LexicallyDeclaredNames of LabelledItem.
        // LabelledItem : Statement
        // 1. Return a new empty List.
        // LabelledItem : FunctionDeclaration
        // 1. Return BoundNames of FunctionDeclaration.
        if let Statement::FunctionDeclaration(decl) = &self.body {
            decl.bound_names(f);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum LexicallyScopedDeclaration<'a> {
    Variable(&'a VariableDeclarator<'a>),
    Function(&'a Function<'a>),
    Class(&'a Class<'a>),
    DefaultExport,
}

/// ### [8.2.5 Static Semantics: LexicallyScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-lexicallyscopeddeclarations)
///
/// The syntax-directed operation LexicallyScopedDeclarations takes no
/// arguments and returns a List of Parse Nodes.
pub(crate) trait LexicallyScopedDeclarations<'a> {
    fn lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(&'a self, f: &mut F);
}

impl<'a> LexicallyScopedDeclarations<'a> for BlockStatement<'a> {
    fn lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
        for st in &self.body {
            st.lexically_scoped_declarations(f);
        }
    }
}

impl<'a> LexicallyScopedDeclarations<'a> for SwitchStatement<'a> {
    fn lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
        for case in &self.cases {
            case.lexically_scoped_declarations(f);
        }
    }
}

impl<'a> LexicallyScopedDeclarations<'a> for SwitchCase<'a> {
    fn lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
        self.consequent.lexically_scoped_declarations(f);
    }
}

pub(crate) fn function_body_lexically_scoped_declarations<'body>(
    code: &'body FunctionBody<'body>,
) -> Vec<LexicallyScopedDeclaration<'body>> {
    let mut lexically_scoped_declarations = vec![];
    // FunctionStatementList : [empty]
    // 1. Return a new empty List.

    // FunctionStatementList : StatementList
    // 1. Return the TopLevelLexicallyScopedDeclarations of StatementList.

    code.statements
        .top_level_lexically_scoped_declarations(&mut |decl| {
            lexically_scoped_declarations.push(decl);
        });

    // Note: Concise bodies have no declarations and thus do not call this function.
    lexically_scoped_declarations
}

pub(crate) fn class_static_block_lexically_scoped_declarations<'body>(
    static_block: &'body StaticBlock<'body>,
) -> Vec<LexicallyScopedDeclaration<'body>> {
    let mut lexically_scoped_declarations = vec![];
    // ClassStaticBlockStatementList : [empty]
    // 1. Return a new empty List.

    // ClassStaticBlockStatementList : StatementList
    // 1. Return the TopLevelLexicallyScopedDeclarations of StatementList.
    static_block
        .body
        .top_level_lexically_scoped_declarations(&mut |decl| {
            lexically_scoped_declarations.push(decl);
        });

    lexically_scoped_declarations
}

pub(crate) fn script_lexically_scoped_declarations<'body>(
    script: &'body Program<'body>,
) -> Vec<LexicallyScopedDeclaration<'body>> {
    let mut lexically_scoped_declarations = vec![];
    // 1. Return TopLevelLexicallyScopedDeclarations of StatementList.
    script
        .body
        .top_level_lexically_scoped_declarations(&mut |decl| {
            lexically_scoped_declarations.push(decl);
        });

    lexically_scoped_declarations
}

pub(crate) fn module_lexically_scoped_declarations<'a>(
    module: &'a [ast::Statement<'a>],
) -> Vec<LexicallyScopedDeclaration<'a>> {
    let mut lexically_scoped_declarations = vec![];

    //  ModuleItemList : ModuleItemList ModuleItem
    // 1. Let declarations1 be LexicallyScopedDeclarations of ModuleItemList.
    let f = &mut |decl| {
        // 3. Return the list-concatenation of declarations[...]
        lexically_scoped_declarations.push(decl);
    };
    for statement in module {
        statement.lexically_scoped_declarations(f);
    }

    lexically_scoped_declarations
}

pub(crate) fn function_body_lexically_scoped_decarations<'body>(
    body: &'body FunctionBody<'body>,
) -> Vec<LexicallyScopedDeclaration<'body>> {
    let mut lexically_scoped_declarations = vec![];
    //  FunctionStatementList : StatementList

    // 1. Return the TopLevelLexicallyScopedDeclarations of StatementList.
    body.statements
        .top_level_lexically_scoped_declarations(&mut |decl| {
            lexically_scoped_declarations.push(decl);
        });
    lexically_scoped_declarations
}

impl<'a> LexicallyScopedDeclarations<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
        // StatementList : StatementList StatementListItem
        // 1. Let declarations1 be LexicallyScopedDeclarations of StatementList.
        // 2. Let declarations2 be LexicallyScopedDeclarations of StatementListItem.
        // 3. Return the list-concatenation of declarations1 and declarations2.
        for ele in self {
            ele.lexically_scoped_declarations(f);
        }
    }
}

impl<'a> LexicallyScopedDeclarations<'a> for Statement<'a> {
    fn lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
        match self {
            Statement::LabeledStatement(st) => {
                // StatementListItem : Statement

                // 1. If Statement is Statement : LabelledStatement , return LexicallyScopedDeclarations of LabelledStatement.
                st.deref().lexically_scoped_declarations(f);
                // 2. Return a new empty List.
            },
            Statement::BlockStatement(_) |
            Statement::EmptyStatement(_) |
            Statement::ExpressionStatement(_) |
            Statement::IfStatement(_) |
            // BreakableStatement
            // > IterationStatement
            Statement::DoWhileStatement(_) |
            Statement::WhileStatement(_) |
            Statement::ForStatement(_) |
            Statement::ForInStatement(_) |
            Statement::ForOfStatement(_) |
            // > SwitchStatement
            Statement::SwitchStatement(_) |
            Statement::ContinueStatement(_) |
            Statement::BreakStatement(_) |
            Statement::ReturnStatement(_) |
            Statement::WithStatement(_) |
            Statement::ThrowStatement(_) |
            Statement::TryStatement(_) |
            Statement::DebuggerStatement(_) => {},
            // StatementListItem : Declaration
            Statement::VariableDeclaration(decl) => {
                // VariableStatement
                if decl.kind.is_var() {
                    // 2. Return a new empty List.
                    return;
                }
                // 1. Return a List whose sole element is DeclarationPart of Declaration.
                for decl in &decl.declarations {
                    f(LexicallyScopedDeclaration::Variable(decl));
                }
            }
            Statement::FunctionDeclaration(decl) => {
                f(LexicallyScopedDeclaration::Function(decl));
            },
            Statement::ClassDeclaration(decl) => {
                f(LexicallyScopedDeclaration::Class(decl));
            },
            // ExportDeclaration :
            // ModuleItem : ImportDeclaration
            // 1. Return a new empty List.
            Statement::ImportDeclaration(_) |
            // export ExportFromClause FromClause ;
            Statement::ExportAllDeclaration(_) => {
                // 1. Return a new empty List.
            },
            Statement::ExportNamedDeclaration(decl) => {
                // export NamedExports ;
                // export VariableStatement
                // 1. Return a new empty List.
                if let Some(Declaration::VariableDeclaration(decl)) = &decl.declaration {
                    if decl.kind.is_var() {
                        return;
                    }
                    // ExportDeclaration : export Declaration
                    // 1. Return a List whose sole element is DeclarationPart of Declaration.
                    debug_assert_eq!(decl.declarations.len(), 1);
                    f(LexicallyScopedDeclaration::Variable(decl.declarations.first().unwrap()));
                }
                // No declaration means this is NamedExports (possibly in an ExportFromClause)
            },
            Statement::ExportDefaultDeclaration(decl) => {
                match &decl.declaration {
                    // ExportDeclaration : export default HoistableDeclaration
                    ExportDefaultDeclarationKind::FunctionDeclaration(decl) => {
                        // 1. Return a List whose sole element is DeclarationPart of HoistableDeclaration.
                        f(LexicallyScopedDeclaration::Function(decl));
                    },
                    ExportDefaultDeclarationKind::FunctionExpression(expr) => {
                        f(LexicallyScopedDeclaration::Function(expr));
                    },
                    // ExportDeclaration : export default ClassDeclaration
                    // 1. Return a List whose sole element is ClassDeclaration.
                    ExportDefaultDeclarationKind::ClassDeclaration(decl) => {
                        f(LexicallyScopedDeclaration::Class(decl));
                    },
                    ExportDefaultDeclarationKind::ClassExpression(expr) => {
                        f(LexicallyScopedDeclaration::Class(expr));
                    }
                    // ExportDeclaration : export default AssignmentExpression ;
                    ExportDefaultDeclarationKind::AssignmentExpression(_) => {
                        // 1. Return a List whose sole element is this ExportDeclaration.
                        f(LexicallyScopedDeclaration::DefaultExport);
                    }
                    ExportDefaultDeclarationKind::BooleanLiteral(_) |
                    ExportDefaultDeclarationKind::NullLiteral(_) |
                    ExportDefaultDeclarationKind::NumericLiteral(_) |
                    ExportDefaultDeclarationKind::BigIntLiteral(_) |
                    ExportDefaultDeclarationKind::RegExpLiteral(_) |
                    ExportDefaultDeclarationKind::StringLiteral(_) |
                    ExportDefaultDeclarationKind::TemplateLiteral(_) |
                    ExportDefaultDeclarationKind::Identifier(_) |
                    ExportDefaultDeclarationKind::MetaProperty(_) |
                    ExportDefaultDeclarationKind::Super(_) |
                    ExportDefaultDeclarationKind::ArrayExpression(_) |
                    ExportDefaultDeclarationKind::ArrowFunctionExpression(_) |
                    ExportDefaultDeclarationKind::AwaitExpression(_) |
                    ExportDefaultDeclarationKind::BinaryExpression(_) |
                    ExportDefaultDeclarationKind::CallExpression(_) |
                    ExportDefaultDeclarationKind::ChainExpression(_) |
                    ExportDefaultDeclarationKind::ConditionalExpression(_) |
                    ExportDefaultDeclarationKind::ImportExpression(_) |
                    ExportDefaultDeclarationKind::LogicalExpression(_) |
                    ExportDefaultDeclarationKind::NewExpression(_) |
                    ExportDefaultDeclarationKind::ObjectExpression(_) |
                    ExportDefaultDeclarationKind::ParenthesizedExpression(_) |
                    ExportDefaultDeclarationKind::SequenceExpression(_) |
                    ExportDefaultDeclarationKind::TaggedTemplateExpression(_) |
                    ExportDefaultDeclarationKind::ThisExpression(_) |
                    ExportDefaultDeclarationKind::UnaryExpression(_) |
                    ExportDefaultDeclarationKind::UpdateExpression(_) |
                    ExportDefaultDeclarationKind::YieldExpression(_) |
                    ExportDefaultDeclarationKind::PrivateInExpression(_) |
                    ExportDefaultDeclarationKind::V8IntrinsicExpression(_) |                    ExportDefaultDeclarationKind::ComputedMemberExpression(_) |
                    ExportDefaultDeclarationKind::StaticMemberExpression(_) |
                    ExportDefaultDeclarationKind::PrivateFieldExpression(_) => {}
                    #[cfg(feature = "typescript")]
                    ExportDefaultDeclarationKind::TSNonNullExpression(_) => {}
                    #[cfg(not(feature = "typescript"))]
                    ExportDefaultDeclarationKind::TSNonNullExpression(_) => unreachable!(),
                    ExportDefaultDeclarationKind::JSXElement(_) |
                    ExportDefaultDeclarationKind::JSXFragment(_) |
                    ExportDefaultDeclarationKind::TSAsExpression(_) |
                    ExportDefaultDeclarationKind::TSInstantiationExpression(_) |
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) |
                    ExportDefaultDeclarationKind::TSSatisfiesExpression(_) |
                    ExportDefaultDeclarationKind::TSTypeAssertion(_) => unreachable!(),
                }
            }
            Statement::TSEnumDeclaration(_) |
            Statement::TSExportAssignment(_) |
            Statement::TSImportEqualsDeclaration(_) |
            Statement::TSInterfaceDeclaration(_) |
            Statement::TSModuleDeclaration(_) |
            Statement::TSNamespaceExportDeclaration(_) |
            Statement::TSTypeAliasDeclaration(_) => unreachable!(),
        }
        // 2. Return a new empty List.
    }
}

impl<'a> LexicallyScopedDeclarations<'a> for LabeledStatement<'a> {
    fn lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
        // LabelledStatement : LabelIdentifier : LabelledItem
        // 1. Return the LexicallyScopedDeclarations of LabelledItem.
        // LabelledItem : Statement
        // 1. Return a new empty List.
        // LabelledItem : FunctionDeclaration
        // 1. Return « FunctionDeclaration ».
        if let Statement::FunctionDeclaration(decl) = &self.body {
            f(LexicallyScopedDeclaration::Function(decl));
        } else if let Statement::LabeledStatement(decl) = &self.body {
            decl.body.lexically_scoped_declarations(f);
        }
    }
}

/// ### [8.2.6 Static Semantics: VarDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-vardeclarednames)
///
/// The syntax-directed operation VarDeclaredNames takes no arguments and
/// returns a List of Strings.
pub(crate) trait VarDeclaredNames<'a> {
    fn var_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&self, f: &mut F);
}

pub(crate) fn script_var_declared_names<'a>(script: &'a Program<'a>) -> Vec<Atom<'a>> {
    let mut var_declared_names = vec![];
    // Script : [empty]
    // 1. Return a new empty List.
    // ScriptBody : StatementList
    // 1. Return TopLevelVarDeclaredNames of StatementList.
    script.body.top_level_var_declared_names(&mut |identifier| {
        var_declared_names.push(identifier.name);
    });
    // NOTE 1
    // At the top level of a Script, function declarations are treated like var declarations rather than like lexical declarations.
    var_declared_names
}

pub(crate) fn module_var_declared_names<'a>(module: &Program<'a>) -> Vec<Atom<'a>> {
    let mut var_declared_names = vec![];
    // ModuleItemList : ModuleItemList ModuleItem
    // 1. Let names1 be VarDeclaredNames of ModuleItemList.
    // 2. Let names2 be VarDeclaredNames of ModuleItem.
    // 3. Return the list-concatenation of names1 and names2.
    module.body.var_declared_names(&mut |identifier| {
        var_declared_names.push(identifier.name);
    });
    var_declared_names
}

pub(crate) fn function_body_var_declared_names<'a>(
    function: &'a FunctionBody<'a>,
) -> Vec<Atom<'a>> {
    let mut var_declared_names = vec![];
    // NOTE
    // This section is extended by Annex B.3.5.

    // FunctionStatementList : [empty]
    // 1. Return a new empty List.
    // FunctionStatementList : StatementList
    // 1. Return TopLevelVarDeclaredNames of StatementList.
    function
        .statements
        .top_level_var_declared_names(&mut |identifier| {
            var_declared_names.push(identifier.name);
        });
    var_declared_names
}

pub(crate) fn class_static_block_var_declared_names<'a>(
    static_block: &'a StaticBlock<'a>,
) -> Vec<Atom<'a>> {
    let mut var_declared_names = vec![];
    // ClassStaticBlockStatementList : [empty]
    // 1. Return a new empty List.
    // ClassStaticBlockStatementList : StatementList
    // 1. Return the TopLevelVarDeclaredNames of StatementList.
    static_block
        .body
        .top_level_var_declared_names(&mut |identifier| {
            var_declared_names.push(identifier.name);
        });
    var_declared_names
}

pub(crate) fn arrow_function_var_declared_names<'a>(
    arrow_function: &FunctionBody<'a>,
) -> Vec<Atom<'a>> {
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
    fn var_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&self, f: &mut F) {
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
    fn var_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&self, f: &mut F) {
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
                // Block : { }
                // 1. Return a new empty List.
                st.body.var_declared_names(f);
            }
            // StatementListItem : Declaration
            // 1. Return a new empty List.
            Statement::FunctionDeclaration(_) |
            Statement::ClassDeclaration(_) => {}
            Statement::VariableDeclaration(decl) if decl.kind.is_lexical() => {}
            // VariableStatement : var VariableDeclarationList ;
            Statement::VariableDeclaration(decl) => {
                // 1. Return the BoundNames of VariableDeclarationList
                decl.bound_names(f);
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
                if let Some(ForStatementInit::VariableDeclaration(decl)) = &st.init {
                    if decl.kind.is_var() {
                        // 1. Let names1 be BoundNames of VariableDeclarationList.
                        decl.bound_names(f);
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
                if matches!(st.body, Statement::FunctionDeclaration(_)) {
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
            // ModuleItem : ImportDeclaration
            // 1. Return a new empty List.
            Statement::ImportDeclaration(_) |
            // ModuleItem : ExportDeclaration
            Statement::ExportAllDeclaration(_) |
            Statement::ExportDefaultDeclaration(_) => {
            },
            // 1. If ExportDeclaration is export VariableStatement, return BoundNames of ExportDeclaration.
            // 2. Return a new empty List.
            Statement::ExportNamedDeclaration(decl) => {
                if let Some(Declaration::VariableDeclaration(decl)) = &decl.declaration {
                    if decl.kind.is_var() {
                        decl.bound_names(f);
                    }
                }
            },
            Statement::TSEnumDeclaration(_) |
            Statement::TSExportAssignment(_) |
            Statement::TSImportEqualsDeclaration(_) |
            Statement::TSInterfaceDeclaration(_) |
            Statement::TSModuleDeclaration(_) |
            Statement::TSNamespaceExportDeclaration(_) |
            Statement::TSTypeAliasDeclaration(_) => unreachable!(),
        }
    }
}

/// ### [8.2.7 Static Semantics: VarScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-varscopeddeclarations)
///
/// The syntax-directed operation VarScopedDeclarations takes no arguments and
/// returns a List of Parse Nodes.
pub(crate) trait VarScopedDeclarations<'a> {
    fn var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F);
}

pub(crate) fn script_var_scoped_declarations<'a>(
    script: &'a Program<'a>,
) -> Vec<VarScopedDeclaration<'a>> {
    let mut var_scoped_declarations = vec![];
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
    body: &'a [ast::Statement<'a>],
) -> Vec<VarScopedDeclaration<'a>> {
    let mut var_scoped_declarations = vec![];
    // Module : [empty]
    // 1. Return a new empty List.
    // ModuleItemList : ModuleItemList ModuleItem
    // 1. Let declarations1 be VarScopedDeclarations of ModuleItemList.
    // 2. Let declarations2 be VarScopedDeclarations of ModuleItem.
    // 3. Return the list-concatenation of declarations1 and declarations2.
    let f = &mut |declarator| {
        var_scoped_declarations.push(declarator);
    };
    for statement in body {
        statement.var_scoped_declarations(f);
    }
    var_scoped_declarations
}

pub(crate) fn function_body_var_scoped_declarations<'a>(
    code: &'a FunctionBody<'a>,
) -> Vec<VarScopedDeclaration<'a>> {
    let mut var_scoped_declarations = vec![];
    // FunctionStatementList : [empty]
    // 1. Return a new empty List.
    // FunctionStatementList : StatementList
    // 1. Return the TopLevelVarScopedDeclarations of StatementList.
    // ClassStaticBlockStatementList : [empty]
    // 1. Return a new empty List.
    // ClassStaticBlockStatementList : StatementList
    // 1. Return the TopLevelVarScopedDeclarations of StatementList.
    code.statements
        .top_level_var_scoped_declarations(&mut |declarator| {
            var_scoped_declarations.push(declarator);
        });
    var_scoped_declarations
}
// ConciseBody : ExpressionBody
// 1. Return a new empty List.
// AsyncConciseBody : ExpressionBody
// 1. Return a new empty List.

pub(crate) fn class_static_block_var_scoped_declarations<'a>(
    static_block: &'a StaticBlock<'a>,
) -> Vec<VarScopedDeclaration<'a>> {
    let mut var_scoped_declarations = vec![];
    //  ClassStaticBlockStatementList : [empty]
    //     1. Return a new empty List.
    // ClassStaticBlockStatementList : StatementList
    //     1. Return the TopLevelVarScopedDeclarations of StatementList.
    static_block
        .body
        .top_level_var_scoped_declarations(&mut |declarator| {
            var_scoped_declarations.push(declarator);
        });
    var_scoped_declarations
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum VarScopedDeclaration<'a> {
    Variable(&'a VariableDeclarator<'a>),
    Function(&'a Function<'a>),
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
                // 1. Return a new empty List.
                for ele in &st.body {
                    ele.var_scoped_declarations(f);
                }
            },
            // StatementListItem : Declaration
            Statement::VariableDeclaration(decl) => {
                decl.var_scoped_declarations(f);
            },
            // 1. Return a new empty List.
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
                if let Statement::FunctionDeclaration(_) = &st.body {
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
            // ModuleItem : ImportDeclaration
            Statement::ImportDeclaration(_) => {
                // 1. Return a new empty List.
            },
            // ModuleItem : ExportDeclaration
            Statement::ExportAllDeclaration(_) |
            Statement::ExportDefaultDeclaration(_) => {}
            Statement::ExportNamedDeclaration(decl) => {
                // 1. If ExportDeclaration is export VariableStatement, return VarScopedDeclarations of VariableStatement.
                if let Some(Declaration::VariableDeclaration(decl)) = &decl.declaration {
                    decl.var_scoped_declarations(f);
                }
            },
            Statement::FunctionDeclaration(_) |
            Statement::ClassDeclaration(_) => {}
            Statement::TSEnumDeclaration(_) |
            Statement::TSExportAssignment(_) |
            Statement::TSImportEqualsDeclaration(_) |
            Statement::TSInterfaceDeclaration(_) |
            Statement::TSModuleDeclaration(_) |
            Statement::TSNamespaceExportDeclaration(_) |
            Statement::TSTypeAliasDeclaration(_) => unreachable!(),
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
            f(VarScopedDeclaration::Variable(declarator));
        }
    }
}

/// ### [8.2.8 Static Semantics: TopLevelLexicallyDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-toplevellexicallydeclarednames)
///
/// The syntax-directed operation TopLevelLexicallyDeclaredNames takes no
/// arguments and returns a List of Strings.
trait TopLevelLexicallyDeclaredNames<'a> {
    fn top_level_lexically_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F);
}

impl<'a> TopLevelLexicallyDeclaredNames<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn top_level_lexically_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let names1 be TopLevelLexicallyDeclaredNames of StatementList.
        // 2. Let names2 be TopLevelLexicallyDeclaredNames of StatementListItem.
        // 3. Return the list-concatenation of names1 and names2.
        for ele in self {
            ele.top_level_lexically_declared_names(f);
        }
    }
}

impl<'a> TopLevelLexicallyDeclaredNames<'a> for Statement<'a> {
    fn top_level_lexically_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
        // NOTE
        // At the top level of a function, or script, function declarations are treated like var declarations rather than like lexical declarations.
        match self {
            // StatementListItem : Statement
            // 1. Return a new empty List.
            Statement::VariableDeclaration(decl) if decl.kind.is_var() => {
                // Note: This is VariableStatement.
            }
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
            | Statement::LabeledStatement(_)
            | Statement::ReturnStatement(_)
            | Statement::SwitchStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::TryStatement(_)
            | Statement::WhileStatement(_)
            | Statement::WithStatement(_) => {}
            // 1. If Declaration is Declaration : HoistableDeclaration , then
            // a. Return a new empty List.
            Statement::FunctionDeclaration(_) => {}
            // 2. Return the BoundNames of Declaration.
            Statement::ClassDeclaration(decl) => decl.bound_names(f),
            Statement::VariableDeclaration(decl) => decl.bound_names(f),
            Statement::ImportDeclaration(decl) => decl.bound_names(f),
            Statement::ExportNamedDeclaration(decl) => decl.bound_names(f),
            #[cfg(feature = "typescript")]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {}
            #[cfg(not(feature = "typescript"))]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {
                unreachable!()
            }
            // Note: No bounds names for export all and export default declarations.
            Statement::ExportAllDeclaration(_) | Statement::ExportDefaultDeclaration(_) => {}
            Statement::TSEnumDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSNamespaceExportDeclaration(_) => unreachable!(),
        }
    }
}

/// ### [8.2.9 Static Semantics: TopLevelLexicallyScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-toplevellexicallyscopeddeclarations)
///
/// The syntax-directed operation TopLevelLexicallyScopedDeclarations takes no
/// arguments and returns a List of Parse Nodes.
trait TopLevelLexicallyScopedDeclarations<'a> {
    fn top_level_lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    );
}

impl<'a> TopLevelLexicallyScopedDeclarations<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn top_level_lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
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
    fn top_level_lexically_scoped_declarations<F: FnMut(LexicallyScopedDeclaration<'a>)>(
        &'a self,
        f: &mut F,
    ) {
        // StatementListItem : Declaration
        match self {
            // StatementListItem : Statement
            // 1. Return a new empty List.
            Statement::VariableDeclaration(decl) if decl.kind.is_var() => {}
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
            | Statement::LabeledStatement(_)
            | Statement::ReturnStatement(_)
            | Statement::SwitchStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::TryStatement(_)
            | Statement::WhileStatement(_)
            | Statement::WithStatement(_) => {}
            // 1. If Declaration is Declaration : HoistableDeclaration , then
            Statement::FunctionDeclaration(_) => {
                // a. Return a new empty List.
            }
            // 2. Return « Declaration ».
            Statement::VariableDeclaration(decl) => {
                for decl in &decl.declarations {
                    f(LexicallyScopedDeclaration::Variable(decl));
                }
            }
            Statement::ClassDeclaration(decl) => f(LexicallyScopedDeclaration::Class(decl)),
            #[cfg(feature = "typescript")]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {}
            #[cfg(not(feature = "typescript"))]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {
                unreachable!()
            }
            Statement::TSEnumDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSNamespaceExportDeclaration(_) => unreachable!(),
            // Note: TopLevelLexicallScopedDeclarations should only be reached
            // from Function body, Class static fields, and Script body. Module
            // declarations should never be reached.
            Statement::ImportDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportDefaultDeclaration(_)
            | Statement::ExportNamedDeclaration(_) => unreachable!(),
        }
    }
}

/// ### [8.2.10 Static Semantics: TopLevelVarDeclaredNames](https://tc39.es/ecma262/#sec-static-semantics-toplevelvardeclarednames)
///
/// The syntax-directed operation TopLevelVarDeclaredNames takes no arguments
/// and returns a List of Strings.
trait TopLevelVarDeclaredNames<'a> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F);
}

impl<'a> TopLevelVarDeclaredNames<'a> for oxc_allocator::Vec<'a, Statement<'a>> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
        // StatementList : StatementList StatementListItem
        // 1. Let names1 be TopLevelVarDeclaredNames of StatementList.
        // 2. Let names2 be TopLevelVarDeclaredNames of StatementListItem.
        // 3. Return the list-concatenation of names1 and names2.
        for ele in self {
            ele.top_level_var_declared_names(f);
        }
    }
}

impl<'a> TopLevelVarDeclaredNames<'a> for Statement<'a> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
        match self {
            // StatementListItem : Declaration
            // 1. If Declaration is Declaration : HoistableDeclaration , then
            Statement::FunctionDeclaration(decl) => {
                // a. Return the BoundNames of HoistableDeclaration.
                decl.bound_names(f)
            }
            // 2. Return a new empty List.
            Statement::VariableDeclaration(decl) if decl.kind.is_lexical() => {
                // LexicalDeclaration : LetOrConst BindingList
            }
            Statement::ClassDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportDefaultDeclaration(_)
            | Statement::ExportNamedDeclaration(_)
            | Statement::ImportDeclaration(_) => {}
            // StatementListItem : Statement
            Statement::LabeledStatement(st) => {
                // 1. If Statement is Statement : LabelledStatement , return TopLevelVarDeclaredNames of Statement.
                st.top_level_var_declared_names(f);
                // NOTE
                // At the top level of a function or script, inner function declarations are treated like var declarations.
            }
            // 2. Return the VarDeclaredNames of Statement
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
            | Statement::VariableDeclaration(_)
            | Statement::WhileStatement(_)
            | Statement::WithStatement(_) => self.var_declared_names(f),
            #[cfg(feature = "typescript")]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {}
            #[cfg(not(feature = "typescript"))]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {
                unreachable!()
            }
            Statement::TSEnumDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSNamespaceExportDeclaration(_) => unreachable!(),
        }
    }
}

impl<'a> TopLevelVarDeclaredNames<'a> for LabeledStatement<'a> {
    fn top_level_var_declared_names<F: FnMut(&BindingIdentifier<'a>)>(&'a self, f: &mut F) {
        // LabelledStatement : LabelIdentifier : LabelledItem
        // 1. Return the TopLevelVarDeclaredNames of LabelledItem.

        match &self.body {
            // LabelledItem : Statement
            Statement::LabeledStatement(st) => {
                // 1. If Statement is Statement : LabelledStatement , return TopLevelVarDeclaredNames of Statement.
                st.top_level_var_declared_names(f);
            }
            // 2. Return VarDeclaredNames of Statement.
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
            | Statement::VariableDeclaration(_)
            | Statement::WhileStatement(_)
            | Statement::WithStatement(_) => {
                self.body.var_declared_names(f);
            }
            // LabelledItem : FunctionDeclaration
            Statement::FunctionDeclaration(decl) => {
                // 1. Return BoundNames of FunctionDeclaration.
                decl.bound_names(f);
            }
            Statement::ClassDeclaration(_)
            | Statement::ImportDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportDefaultDeclaration(_)
            | Statement::ExportNamedDeclaration(_) => self.body.var_declared_names(f),
            Statement::TSEnumDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSInterfaceDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSNamespaceExportDeclaration(_)
            | Statement::TSTypeAliasDeclaration(_) => unreachable!(),
        }
    }
}

/// ### [8.2.11 Static Semantics: TopLevelVarScopedDeclarations](https://tc39.es/ecma262/#sec-static-semantics-toplevelvarscopeddeclarations)
///
/// The syntax-directed operation TopLevelVarScopedDeclarations takes no
/// arguments and returns a List of Parse Nodes.
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
            // 2. Return VarScopedDeclarations of Statement.
            Statement::VariableDeclaration(decl) if decl.kind.is_var() => {
                // VariableStatement
                decl.var_scoped_declarations(f);
            }
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
            | Statement::WithStatement(_) => {
                self.var_scoped_declarations(f);
            }
            Statement::VariableDeclaration(decl) if decl.kind.is_var() => {
                // VariableStatements:
                // 2. Return VarScopedDeclarations of Statement.
                decl.var_scoped_declarations(f);
            }
            // StatementListItem : Declaration
            // 1. If Declaration is Declaration : HoistableDeclaration , then
            Statement::FunctionDeclaration(decl) => {
                // a. Let declaration be DeclarationPart of HoistableDeclaration.
                // b. Return « declaration ».
                f(VarScopedDeclaration::Function(decl));
            }
            // 2. Return a new empty List.
            Statement::ClassDeclaration(_)
            | Statement::ImportDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportDefaultDeclaration(_)
            | Statement::ExportNamedDeclaration(_)
            | Statement::VariableDeclaration(_) => {
                // 2. Return a new empty List.
            }
            #[cfg(feature = "typescript")]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {}
            #[cfg(not(feature = "typescript"))]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {
                unreachable!()
            }
            Statement::TSEnumDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSNamespaceExportDeclaration(_) => unreachable!(),
        }
    }
}

impl<'a> TopLevelVarScopedDeclarations<'a> for LabeledStatement<'a> {
    fn top_level_var_scoped_declarations<F: FnMut(VarScopedDeclaration<'a>)>(&'a self, f: &mut F) {
        // LabelledStatement : LabelIdentifier : LabelledItem
        // 1. Return the TopLevelVarScopedDeclarations of LabelledItem.
        match &self.body {
            // LabelledItem : Statement
            Statement::LabeledStatement(st) => {
                // 1. If Statement is Statement : LabelledStatement , return TopLevelVarScopedDeclarations of Statement.
                st.top_level_var_scoped_declarations(f);
            }
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
            | Statement::WithStatement(_) => {
                // 2. Return VarScopedDeclarations of Statement.
                self.body.var_scoped_declarations(f);
            }
            Statement::VariableDeclaration(decl) if decl.kind.is_var() => {
                decl.var_scoped_declarations(f);
            }
            Statement::FunctionDeclaration(decl) => {
                // LabelledItem : FunctionDeclaration
                // 1. Return « FunctionDeclaration ».
                f(VarScopedDeclaration::Function(decl));
            }
            Statement::TSTypeAliasDeclaration(_)
            | Statement::TSInterfaceDeclaration(_)
            | Statement::TSEnumDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSNamespaceExportDeclaration(_) => unreachable!(),
            _ => {
                // Other declarations are not mentioned
            }
        }
    }
}
