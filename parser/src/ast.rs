use crate::lexer::{Keyword, Token};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
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
    Null,
    Index {
        root: Box<Expr>,
        index: Box<Expr>,
    },
    UnaryOp {
        kind: UnaryOp,
        value: Box<Expr>,
    },
    BinaryOp {
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
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Pos,
    Neg,
    Not,
    BitComplement,
    Yield,
    Await,
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

#[derive(Debug, Clone)]
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
    MemberAccess,
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
        level: AssignLevel,
        binding: Binding,
        value: Expr,
    },
    Declare {
        level: AssignLevel,
        binding: Binding,
    },
    Label(Box<Stmt>),
    Break {
        label: Option<Span>,
    },
    Continue {
        label: Option<Span>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum AssignLevel {
    Let,
    Const,
    Var,
}
