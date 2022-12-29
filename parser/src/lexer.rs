use boa_unicode::UnicodeProperties;

use crate::ast::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    EOF,

    // Entities
    Identifier,
    StringLiteral,
    InvalidStringLiteral,
    NumberLiteral,
    InvalidNumberLiteral,
    Keyword(Keyword),

    // Unary
    Not,
    BitComplement,

    // These could be unary or binary.
    Plus,
    Minus,

    // Binary
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
    Unknown,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBrack,
    RightBrack,
    Semi,
    Question,
    Dot,
    Comma,
    // TODO: DotDotDot
}

impl Token {
    /// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Operator_Precedence
    pub fn lbp(self) -> u8 {
        match self {
            Self::LeftParen | Self::LeftBrack => 180,
            // Unary ops are not left-binding
            Self::Pow => 130,
            Self::Mul | Self::Div | Self::Mod => 120,
            // these are binary at this point
            Self::Plus | Self::Minus => 110,
            Self::BitShiftLeft | Self::BitShiftRight | Self::BitUnsignedShiftRight => 100,
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => 90,
            Self::EqualEqual | Self::NotEqual | Self::EqualEqualEqual | Self::NotEqualEqual => 80,
            Self::BitAnd => 70,
            Self::BitXor => 60,
            Self::BitOr => 50,
            Self::And => 40,
            Self::Nullish | Self::Or => 30,
            Self::Equal | Self::OrAssign => 20,
            // Self::Comma => 10,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Keyword {
    Break,
    Case,
    Catch,
    Class,
    Const,
    Continue,
    Debugger,
    Default,
    Delete,
    Do,
    Else,
    Export,
    Extends,
    False,
    Finally,
    For,
    Function,
    If,
    Import,
    In,
    Instanceof,
    New,
    Null,
    Return,
    Super,
    Switch,
    This,
    Throw,
    True,
    Try,
    Typeof,
    Var,
    Void,
    While,
    With,

    // Only reserved in strict mode.
    Let,
    Static,
    Yield,

    // Only reserved in modules or async function bodies.
    Await,

    // Reserved in strict mode.
    Implements,
    Interface,
    Package,
    Private,
    Protected,
    Public,
}

static KEYWORDS: phf::Map<&'static str, Token> = phf::phf_map! {
    "break" => Token::Keyword(Keyword::Break),
    "case" => Token::Keyword(Keyword::Case),
    "catch" => Token::Keyword(Keyword::Catch),
    "class" => Token::Keyword(Keyword::Class),
    "const" => Token::Keyword(Keyword::Const),
    "continue" => Token::Keyword(Keyword::Continue),
    "debugger" => Token::Keyword(Keyword::Debugger),
    "default" => Token::Keyword(Keyword::Default),
    "delete" => Token::Keyword(Keyword::Delete),
    "do" => Token::Keyword(Keyword::Do),
    "else" => Token::Keyword(Keyword::Else),
    "export" => Token::Keyword(Keyword::Export),
    "extends" => Token::Keyword(Keyword::Extends),
    "false" => Token::Keyword(Keyword::False),
    "finally" => Token::Keyword(Keyword::Finally),
    "for" => Token::Keyword(Keyword::For),
    "function" => Token::Keyword(Keyword::Function),
    "if" => Token::Keyword(Keyword::If),
    "import" => Token::Keyword(Keyword::Import),
    "in" => Token::Keyword(Keyword::In),
    "instanceof" => Token::Keyword(Keyword::Instanceof),
    "new" => Token::Keyword(Keyword::New),
    "null" => Token::Keyword(Keyword::Null),
    "return" => Token::Keyword(Keyword::Return),
    "super" => Token::Keyword(Keyword::Super),
    "switch" => Token::Keyword(Keyword::Switch),
    "this" => Token::Keyword(Keyword::This),
    "throw" => Token::Keyword(Keyword::Throw),
    "true" => Token::Keyword(Keyword::True),
    "try" => Token::Keyword(Keyword::Try),
    "typeof" => Token::Keyword(Keyword::Typeof),
    "var" => Token::Keyword(Keyword::Var),
    "void" => Token::Keyword(Keyword::Void),
    "while" => Token::Keyword(Keyword::While),
    "with" => Token::Keyword(Keyword::With),

    // Only reserved in strict mode.
    "let" => Token::Keyword(Keyword::Let),
    "static" => Token::Keyword(Keyword::Static),
    "yield" => Token::Keyword(Keyword::Yield),

    // Only reserved in modules or async function bodies.
    "await" => Token::Keyword(Keyword::Await),

    // Reserved in strict mode.
    "implements" => Token::Keyword(Keyword::Implements),
    "interface" => Token::Keyword(Keyword::Interface),
    "package" => Token::Keyword(Keyword::Package),
    "private" => Token::Keyword(Keyword::Private),
    "protected" => Token::Keyword(Keyword::Protected),
    "public" => Token::Keyword(Keyword::Public),
};

enum NumberParseState {
    Number { seen_exp: bool },
    Float { seen_exp: bool },
}

#[derive(Debug, Clone, Copy)]
pub struct Lexer<'a> {
    buffer: &'a [u8],
    pub index: usize,
    pub start: usize,
    pub token: Token,
    pub has_newline_before: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            buffer: input.as_bytes(),
            index: 0,
            start: 0,
            token: Token::EOF,
            has_newline_before: false,
        }
    }

    #[inline]
    pub fn source(&'a self) -> &'a str {
        // SAFETY: the API ensures that the buffer is already an &str
        unsafe { std::str::from_utf8_unchecked(self.buffer) }
    }

    pub fn span(&self) -> Span {
        Span {
            start: self.start as u32,
            end: self.index as u32,
        }
    }

    fn continue_number(&mut self, mut state: NumberParseState) {
        // TODO: support underscore separators
        loop {
            match state {
                NumberParseState::Float { seen_exp: false } => match self.buffer.get(self.index) {
                    Some(b'e') => {
                        self.index += 1;
                        state = NumberParseState::Float { seen_exp: true };
                    }
                    Some(b'.') => {
                        self.token = Token::InvalidNumberLiteral;
                        return;
                    }
                    Some(b'0'..=b'9') => self.index += 1,
                    _ => break,
                },
                NumberParseState::Float { seen_exp: true } => match self.buffer.get(self.index) {
                    Some(b'e') => {
                        self.token = Token::InvalidNumberLiteral;
                        return;
                    }
                    Some(b'.') => {
                        self.token = Token::InvalidNumberLiteral;
                        return;
                    }
                    Some(b'0'..=b'9') => self.index += 1,
                    Some(b'_') => panic!(),
                    _ => break,
                },
                NumberParseState::Number { seen_exp } => match self.buffer.get(self.index) {
                    Some(b'e') => {
                        self.index += 1;
                        state = NumberParseState::Number { seen_exp: true };
                    }
                    Some(b'_') => panic!(),
                    Some(b'.') if seen_exp => {
                        self.token = Token::InvalidNumberLiteral;
                        return;
                    }
                    Some(b'.') => {
                        self.index += 1;
                        state = NumberParseState::Number { seen_exp: false };
                    }
                    Some(b'0'..=b'9') => {
                        self.index += 1;
                    }
                    _ => break,
                },
            }
        }
    }

    pub fn next(&mut self) {
        self.has_newline_before = false;

        // The main lexer loop. This is used to restart at the initial lexing
        // phase in order to continue after whitespace without invoking another
        // call stack. The default case in any initial branch is to exit.
        // Falling through is explicit in the form of `continue 'main`.
        'main: loop {
            self.start = self.index;

            match self.buffer.get(self.index) {
                Some(b'+') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        self.index += 1;
                        Token::AddAssign
                    } else {
                        Token::Plus
                    };
                }
                Some(b'-') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        self.index += 1;
                        Token::SubAssign
                    } else {
                        Token::Minus
                    };
                }
                Some(b'*') => {
                    self.index += 1;
                    self.token = match self.buffer.get(self.index) {
                        Some(b'*') => {
                            self.index += 1;
                            if let Some(b'=') = self.buffer.get(self.index) {
                                self.index += 1;
                                Token::PowAssign
                            } else {
                                Token::Pow
                            }
                        }
                        Some(b'=') => {
                            self.index += 1;
                            Token::PowAssign
                        }
                        _ => Token::Mul,
                    }
                }
                Some(b'/') => {
                    self.index += 1;
                    self.token = match self.buffer.get(self.index) {
                        // line comment
                        Some(b'/') => loop {
                            self.index += 1;

                            if let Some(b'\n' | b'\r') | None = self.buffer.get(self.index) {
                                continue 'main;
                            }
                        },
                        // block comment
                        Some(b'*') => loop {
                            self.index += 1;
                            if let Some(b'*') = self.buffer.get(self.index) {
                                self.index += 1;
                                if let Some(b'/') = self.buffer.get(self.index) {
                                    self.index += 1;
                                    continue 'main;
                                }
                            }
                        },
                        Some(b'=') => {
                            self.index += 1;
                            Token::DivAssign
                        }
                        _ => Token::Div,
                    }
                }
                Some(b'<') => {
                    self.index += 1;
                    // TODO: handle `<!--` here?
                    self.token = match self.buffer.get(self.index) {
                        Some(b'=') => {
                            self.index += 1;
                            Token::LessEqual
                        }
                        Some(b'<') => {
                            self.index += 1;
                            if let Some(b'=') = self.buffer.get(self.index) {
                                self.index += 1;
                                Token::BitShiftLeftAssign
                            } else {
                                Token::BitShiftLeft
                            }
                        }
                        _ => Token::Less,
                    };
                }
                Some(b'>') => {
                    self.index += 1;
                    self.token = match self.buffer.get(self.index) {
                        Some(b'=') => {
                            self.index += 1;
                            Token::GreaterEqual
                        }
                        Some(b'>') => {
                            self.index += 1;
                            match self.buffer.get(self.index) {
                                Some(b'>') => {
                                    self.index += 1;
                                    if let Some(b'=') = self.buffer.get(self.index) {
                                        self.index += 1;
                                        Token::BitUnsignedShiftRightAssign
                                    } else {
                                        Token::BitUnsignedShiftRight
                                    }
                                }
                                Some(b'=') => {
                                    self.index += 1;
                                    Token::BitShiftRightAssign
                                }
                                _ => Token::BitShiftRight,
                            }
                        }
                        _ => Token::Greater,
                    }
                }
                Some(b'!') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        self.index += 1;
                        if let Some(b'=') = self.buffer.get(self.index) {
                            self.index += 1;
                            Token::NotEqualEqual
                        } else {
                            Token::NotEqual
                        }
                    } else {
                        Token::Not
                    }
                }
                Some(b'=') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        self.index += 1;
                        if let Some(b'=') = self.buffer.get(self.index) {
                            self.index += 1;
                            Token::EqualEqualEqual
                        } else {
                            Token::EqualEqual
                        }
                    } else {
                        Token::Equal
                    }
                }
                Some(b'"') => {
                    self.token = Token::StringLiteral;

                    loop {
                        self.index += 1;

                        match self.buffer.get(self.index) {
                            Some(b'\\') => {
                                self.index += 1;
                                if let Some(b'\n' | b'\r') | None = self.buffer.get(self.index) {
                                    self.token = Token::InvalidStringLiteral;
                                    break;
                                }
                                self.index += 1;
                            }
                            Some(b'\n' | b'\r') | None => {
                                self.token = Token::InvalidStringLiteral;
                                break;
                            }
                            Some(b'"') => {
                                self.index += 1;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Some(b'\'') => {
                    self.token = Token::StringLiteral;

                    loop {
                        self.index += 1;

                        match self.buffer.get(self.index) {
                            Some(b'\\') => {
                                self.index += 1;
                                if let Some(b'\n' | b'\r') | None = self.buffer.get(self.index) {
                                    self.token = Token::InvalidStringLiteral;
                                    break;
                                }
                                self.index += 1;
                            }
                            Some(b'\n' | b'\r') | None => {
                                self.token = Token::InvalidStringLiteral;
                                break;
                            }
                            Some(b'\'') => {
                                self.index += 1;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Some(b'^') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        self.index += 1;
                        Token::BitXorAssign
                    } else {
                        Token::BitXor
                    }
                }
                Some(b'&') => {
                    self.index += 1;
                    self.token = match self.buffer.get(self.index) {
                        Some(b'&') => {
                            self.index += 1;
                            if let Some(b'=') = self.buffer.get(self.index) {
                                self.index += 1;
                                Token::AndAssign
                            } else {
                                Token::And
                            }
                        }
                        Some(b'=') => {
                            self.index += 1;
                            Token::BitAndAssign
                        }
                        _ => Token::BitAnd,
                    }
                }
                Some(b'|') => {
                    self.index += 1;
                    self.token = match self.buffer.get(self.index) {
                        Some(b'|') => {
                            self.index += 1;
                            if let Some(b'=') = self.buffer.get(self.index) {
                                self.index += 1;
                                Token::Or
                            } else {
                                Token::OrAssign
                            }
                        }
                        Some(b'=') => {
                            self.index += 1;
                            Token::BitOrAssign
                        }
                        _ => Token::BitOr,
                    }
                }
                Some(b'~') => {
                    self.index += 1;
                    self.token = Token::BitComplement;
                }
                Some(b'%') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        self.index += 1;
                        Token::ModAssign
                    } else {
                        Token::Mod
                    }
                }
                Some(b',') => {
                    self.index += 1;
                    self.token = Token::Comma;
                }
                Some(b'\r' | b'\n') => {
                    self.has_newline_before = true;
                    loop {
                        self.index += 1;
                        let Some(b' ' | b'\t' | b'\r' | b'\n') = self.buffer.get(self.index) else {
							break;
						};
                    }
                    continue 'main;
                }
                Some(b' ' | b'\t') => {
                    loop {
                        self.index += 1;
                        match self.buffer.get(self.index) {
                            Some(b' ' | b'\t') => {}
                            Some(b'\r' | b'\n') => break,
                            _ => continue 'main,
                        }
                    }

                    self.has_newline_before = true;

                    loop {
                        self.index += 1;
                        let Some(b' ' | b'\t' | b'\r' | b'\n') = self.buffer.get(self.index) else {
							break;
						};
                    }

                    continue 'main;
                }
                Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$') => loop {
                    self.token = Token::Identifier;
                    self.index += 1;
                    let Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'$') = self.buffer.get(self.index) else {
						let start_utf8 = self.index;
						let mut chars = self.source()[start_utf8..].char_indices();
						while let Some((idx, c)) = chars.next() {
							if !boa_unicode::UnicodeProperties::is_id_continue(c) {
								self.index = start_utf8 + idx;
								break;
							}
						}
						self.token = *KEYWORDS.get(&self.source()[self.start..self.index]).unwrap_or(&Token::Identifier);
						break;
					};
                },
                Some(b'(') => {
                    self.index += 1;
                    self.token = Token::LeftParen;
                }
                Some(b')') => {
                    self.index += 1;
                    self.token = Token::RightParen;
                }
                Some(b'{') => {
                    self.index += 1;
                    self.token = Token::LeftBrace;
                }
                Some(b'}') => {
                    self.index += 1;
                    self.token = Token::RightBrace;
                }
                Some(b'[') => {
                    self.index += 1;
                    self.token = Token::LeftBrack;
                }
                Some(b']') => {
                    self.index += 1;
                    self.token = Token::RightBrack;
                }
                Some(b'.') => 'blk: {
                    self.index += 1;
                    if let Some(b'0'..=b'9') = self.buffer.get(self.index) {
                        self.token = Token::NumberLiteral;
                        self.continue_number(NumberParseState::Float { seen_exp: true });
                        break 'blk;
                    }
                    self.token = Token::Dot;
                }
                Some(b';') => {
                    self.index += 1;
                    self.token = Token::Semi;
                }
                Some(b'?') => {
                    self.index += 1;
                    self.token = if let Some(b'?') = self.buffer.get(self.index) {
                        self.index += 1;
                        if let Some(b'=') = self.buffer.get(self.index) {
                            self.index += 1;
                            Token::NullishAssign
                        } else {
                            Token::Nullish
                        }
                    } else {
                        Token::Question
                    };
                }
                Some(b'1'..=b'9') => {
                    self.token = Token::NumberLiteral;
                    self.index += 1;
                    self.continue_number(NumberParseState::Number { seen_exp: false });
                }
                None => self.token = Token::EOF,
                _ => 'blk: {
                    let mut chars = self.source()[self.index..].char_indices();

                    let start_utf8 = self.index;
                    if let Some((_, c)) = chars.next() {
                        if !boa_unicode::UnicodeProperties::is_id_start(c) {
                            self.index = start_utf8 + chars.next().map(|(idx, _)| idx).unwrap_or(1);
                            self.token = Token::Unknown;
                            break 'blk;
                        }

                        while let Some((_, c)) = chars.next() {
                            if !boa_unicode::UnicodeProperties::is_id_continue(c) {
                                self.index =
                                    start_utf8 + chars.next().map(|(idx, _)| idx).unwrap_or(1);
                                self.token = Token::Identifier;
                                break 'blk;
                            }
                        }
                    }

                    self.index += 1;
                    self.token = Token::Unknown;
                }
            }

            break;
        }
    }
}
