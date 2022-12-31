use codespan_reporting::diagnostic::{Diagnostic, Label};
use hashbrown::{hash_set, HashMap, HashSet};
use nova_parser::ast::{BinaryOp, Binding, BindingLevel, Expr, Stmt, UnaryOp};
use std::{cell::RefCell, fmt::Display, rc::Rc};

#[derive(Debug)]
pub struct Env<'a> {
    pub parent: Option<Rc<RefCell<Env<'a>>>>,
    pub entries: HashMap<&'a str, Type>,
}

impl<'a> Env<'a> {
    pub fn new() -> Self {
        Self {
            parent: None,
            entries: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Checker<'a> {
    pub source: &'a str,
    pub diagnostics: Vec<Diagnostic<usize>>,
}

#[derive(Debug, Clone)]
pub enum Error {
    InvalidBinaryOperator {
        expr: Expr,
        lhs_t: Type,
        rhs_t: Type,
    },
}

impl Error {
    pub fn into_diagnostic(&self, fid: usize) -> Diagnostic<usize> {
        match self {
            Self::InvalidBinaryOperator {
                expr:
                    Expr::BinaryOp {
                        op_index,
                        kind,
                        lhs,
                        rhs,
                    },
                lhs_t,
                rhs_t,
            } => Diagnostic::error()
                .with_code("E2365")
                .with_message(format!(
                    "Operator '{}' cannot be applied to types '{}' and '{}'.",
                    kind.as_ref(),
                    lhs_t,
                    rhs_t,
                ))
                .with_labels(vec![
                    Label::primary(fid, *op_index as usize..(*op_index as usize + 1)),
                    Label::secondary(fid, lhs.span()).with_message(format!("{lhs_t}")),
                    Label::secondary(fid, rhs.span()).with_message(format!("{rhs_t}")),
                ]),

            _ => panic!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Undefined,
    Number,
    Boolean,
    String,
    Null,
    Union(HashSet<Type>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Undefined => write!(f, "undefined"),
            Self::Number => write!(f, "number"),
            Self::Boolean => write!(f, "boolean"),
            Self::String => write!(f, "string"),
            Self::Null => write!(f, "null"),
            _ => panic!(),
        }
    }
}

impl std::hash::Hash for Type {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl Type {
    pub fn new_union<const LEN: usize>(values: [Type; LEN]) -> Type {
        Type::Union(HashSet::from_iter(values.into_iter()))
    }
}

type Result<T> = std::result::Result<T, ()>;

impl<'a> Checker<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            diagnostics: Vec::new(),
        }
    }

    fn is_assignable(lhs: &Type, rhs: &Type) -> bool {
        match (lhs, rhs) {
            (Type::Undefined, Type::Undefined) => true,
            (Type::Boolean, Type::Boolean) => true,
            (Type::Number, Type::Number) => true,
            (Type::String, Type::String) => true,
            (Type::Null, Type::Null) => true,
            (Type::Union(members), t) => {
                for member in members.iter() {
                    if !Self::is_assignable(member, t) {
                        return false;
                    }
                }
                true
            }
            (t, Type::Union(members)) => {
                for member in members.iter() {
                    if Self::is_assignable(member, t) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn infer_type(&mut self, env: &mut Env<'a>, expr: &Expr) -> Result<Type> {
        Ok(match expr {
            Expr::True { .. } | Expr::False { .. } => Type::Boolean,
            Expr::NumberLiteral { .. } => Type::Number,
            Expr::StringLiteral { .. } => Type::String,
            Expr::Null { .. } => Type::Null,
            Expr::Identifier { span } => {
                let name = &self.source[span.into_range()];
                let Some(ty) = env.entries.get(name) else {
					return Err(());
				};
                ty.clone()
            }
            Expr::UnaryOp { kind, value, .. } => {
                let value = self.infer_type(env, &value)?;

                match kind {
                    UnaryOp::Not => {
                        if !Self::is_assignable(
                            &value,
                            &Type::new_union([Type::Number, Type::Boolean]),
                        ) {
                            return Err(());
                        };

                        Type::Boolean
                    }
                    UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitComplement => {
                        let Type::Number = value else {
							return Err(());
						};

                        Type::Number
                    }
                    _ => panic!(),
                }
            }
            Expr::BinaryOp { kind, lhs, rhs, .. } => 'blk: {
                let lhs = self.infer_type(env, lhs)?;
                let rhs = self.infer_type(env, rhs)?;

                if let BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Mod
                | BinaryOp::Div = kind
                {
                    let (Type::Number, Type::Number) = (&lhs, &rhs) else {
						self.diagnostics.push(Error::InvalidBinaryOperator { expr: expr.clone(), lhs_t: lhs, rhs_t: rhs }.into_diagnostic(0));
						return Err(());
					};

                    break 'blk Type::Number;
                }

                panic!();
            }
            _ => panic!(),
        })
    }

    pub fn check_scope(&mut self, env: &mut Env<'a>, scope: &[Stmt]) -> Result<()> {
        for stmt in scope.iter() {
            match stmt {
                Stmt::Assign {
                    level,
                    binding,
                    value,
                } => {
                    let inferred = self.infer_type(env, value)?;
                    env.entries.insert(
                        if let Binding::Identifier(span) = binding {
                            &self.source[span.into_range()]
                        } else {
                            panic!();
                        },
                        inferred,
                    );
                }
                _ => panic!(),
            }
        }

        Ok(())
    }
}
