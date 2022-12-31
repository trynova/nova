use crate::lexer::{Keyword, Token};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Into<std::ops::Range<usize>> for Span {
    fn into(self) -> std::ops::Range<usize> {
        self.into_range()
    }
}

impl Span {
    #[inline]
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    // We do this because deriving Into<_> has some inference issues for range
    // indices.
    pub fn into_range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

/// A binding for some data.
#[derive(Debug, Clone)]
pub enum Binding {
    Identifier(Span),
    _TheRestOfThemTM,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<Span>,
    pub params: Box<[FunctionParam]>,
    pub scope: Box<[Stmt]>,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub name: Binding,
    pub default: Option<Box<Expr>>,
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Null {
        start: u32,
    },
    True {
        start: u32,
    },
    False {
        start: u32,
    },
    Undefined {
        start: u32,
    },
    Index {
        span: Span,
        root: Box<Expr>,
        index: Box<Expr>,
    },
    UnaryOp {
        start: u32,
        kind: UnaryOp,
        value: Box<Expr>,
    },
    BinaryOp {
        op_index: u32,
        kind: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Function(Function),
    FunctionCall {
        calle: Box<Expr>,
        args: Box<[Expr]>,
        // TODO: support function spreading
    },
    ArrayLiteral {
        span: Span,
        values: Box<[Option<Expr>]>,
    },
    StringLiteral {
        span: Span,
    },
    NumberLiteral {
        span: Span,
    },
    Identifier {
        span: Span,
    },
    ObjectLiteral(ObjectLiteral),
    Ternary {
        condition: Box<Expr>,
        truthy: Box<Expr>,
        falsy: Box<Expr>,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            &Self::Null { start } => Span::new(start, start + 4),
            &Self::True { start } => Span::new(start, start + 4),
            &Self::False { start } => Span::new(start, start + 5),
            &Self::Undefined { start } => Span::new(start, start + 5),
            Self::UnaryOp { start, kind, value } => Span::new(*start, value.span().end),
            Self::BinaryOp { lhs, rhs, .. } => Span::new(lhs.span().start, rhs.span().end),
            &Self::StringLiteral { span }
            | &Self::NumberLiteral { span }
            | &Self::Index { span, .. }
            | &Self::ArrayLiteral { span, .. } => span,
            Self::Ternary {
                condition,
                truthy,
                falsy,
            } => Span::new(condition.span().start, falsy.span().end),
            _ => panic!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectLiteral {
    pub entries: Box<[ObjectEntry]>,
}

#[derive(Debug, Clone)]
pub struct ObjectEntry {
    pub name: Span,
    pub value: Option<Box<Expr>>,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Pos,
    Neg,
    BitComplement,
    Not,
    Yield,
    Await,
}

impl AsRef<str> for UnaryOp {
    fn as_ref(&self) -> &str {
        match self {
            Self::Pos => "+",
            Self::Neg => "-",
            Self::BitComplement => "~",
            Self::Not => "!",
            Self::Yield => "yield",
            Self::Await => "await",
        }
    }
}

impl From<Token> for UnaryOp {
    fn from(value: Token) -> Self {
        match value {
            Token::Plus => Self::Pos,
            Token::Minus => Self::Neg,
            Token::Not => Self::Not,
            Token::BitComplement => Self::BitComplement,
            Token::Keyword(Keyword::Yield) => Self::Yield,
            Token::Keyword(Keyword::Await) => Self::Await,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    // Binary
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Equal,
    EqualEqual,
    EqualEqualEqual,
    NotEqual,
    NotEqualEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    BitShiftLeft,
    BitShiftRight,
    BitUnsignedShiftRight,
    BitAnd,
    BitOr,
    BitXor,
    Or,
    And,
    Nullish,
    Mod,

    // Binary Assign
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    PowAssign,
    BitShiftLeftAssign,
    BitShiftRightAssign,
    BitUnsignedShiftRightAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    OrAssign,
    AndAssign,
    NullishAssign,
    ModAssign,

    // Misc.
    MemberAccess,
    Sequence,
}

impl AsRef<str> for BinaryOp {
    fn as_ref(&self) -> &str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Pow => "**",
            Self::Equal => "=",
            Self::EqualEqual => "==",
            Self::EqualEqualEqual => "===",
            Self::NotEqual => "!=",
            Self::NotEqualEqual => "!==",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::BitShiftLeft => "<<",
            Self::BitShiftRight => ">>",
            Self::BitUnsignedShiftRight => ">>>",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Or => "||",
            Self::And => "&&",
            Self::Nullish => "??",
            Self::Mod => "%",

            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
            Self::PowAssign => "**=",
            Self::BitShiftLeftAssign => "<<=",
            Self::BitShiftRightAssign => ">>=",
            Self::BitUnsignedShiftRightAssign => ">>>=",
            Self::BitAndAssign => "&=",
            Self::BitOrAssign => "|=",
            Self::BitXorAssign => "^=",
            Self::OrAssign => "||=",
            Self::AndAssign => "&&=",
            Self::NullishAssign => "??=",
            Self::ModAssign => "%=",

            Self::MemberAccess => ".",
            Self::Sequence => ",",
        }
    }
}

impl From<Token> for BinaryOp {
    fn from(value: Token) -> Self {
        match value {
            Token::Plus => Self::Add,
            Token::Minus => Self::Sub,
            Token::Mul => Self::Mul,
            Token::Mod => Self::Mod,
            Token::Div => Self::Div,
            Token::Pow => Self::Pow,
            Token::Equal => Self::Equal,
            Token::EqualEqual => Self::EqualEqual,
            Token::EqualEqualEqual => Self::EqualEqualEqual,
            Token::NotEqual => Self::NotEqual,
            Token::NotEqualEqual => Self::NotEqualEqual,
            Token::Less => Self::Less,
            Token::LessEqual => Self::LessEqual,
            Token::Greater => Self::Greater,
            Token::GreaterEqual => Self::GreaterEqual,
            Token::BitShiftLeft => Self::BitShiftLeft,
            Token::BitShiftRight => Self::BitShiftRight,
            Token::BitUnsignedShiftRight => Self::BitUnsignedShiftRight,
            Token::BitAnd => Self::BitAnd,
            Token::BitOr => Self::BitOr,
            Token::BitXor => Self::BitXor,
            Token::Or => Self::Or,
            Token::And => Self::And,
            Token::Nullish => Self::Nullish,
            Token::AddAssign => Self::AddAssign,
            Token::SubAssign => Self::SubAssign,
            Token::MulAssign => Self::MulAssign,
            Token::DivAssign => Self::DivAssign,
            Token::PowAssign => Self::PowAssign,
            Token::BitShiftLeftAssign => Self::BitShiftLeftAssign,
            Token::BitShiftRightAssign => Self::BitShiftRightAssign,
            Token::BitUnsignedShiftRightAssign => Self::BitUnsignedShiftRightAssign,
            Token::BitAndAssign => Self::BitAndAssign,
            Token::BitOrAssign => Self::BitOrAssign,
            Token::BitXorAssign => Self::BitXorAssign,
            Token::OrAssign => Self::OrAssign,
            Token::AndAssign => Self::AndAssign,
            Token::NullishAssign => Self::NullishAssign,
            Token::ModAssign => Self::ModAssign,
            Token::Dot => Self::MemberAccess,
            Token::Comma => Self::Sequence,
            _ => unreachable!(),
        }
    }
}

/// A statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    Function(Function),
    Return {
        value: Expr,
    },
    Yield {
        value: Expr,
    },
    Await {
        value: Expr,
    },
    Assign {
        level: BindingLevel,
        binding: Binding,
        value: Expr,
    },
    Declare {
        level: BindingLevel,
        binding: Binding,
    },
    Label(Box<Stmt>),
    Break {
        label: Option<Span>,
    },
    Continue {
        label: Option<Span>,
    },
    Expr {
        value: Expr,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BindingLevel {
    None,
    Let,
    Const,
    Var,
}
