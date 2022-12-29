use crate::lexer::Token;

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
    Ident(Span),
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    BinaryOp {
        kind: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    StringLiteral {
        span: Span,
    },
    NumberLiteral {
        span: Span,
    },
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
            _ => unreachable!(),
        }
    }
}

/// A statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    Assign {
        level: AssignLevel,
        binding: Binding,
        value: Expr,
    },
    Declare {
        binding: Binding,
    },
    Label(Box<Stmt>),
}

#[derive(Debug, Clone)]
pub enum AssignLevel {
    Let,
    Const,
    Var,
}
