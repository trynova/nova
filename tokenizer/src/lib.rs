use boa_unicode::UnicodeProperties;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    EOF,

    // Entities
    Identifier,
    StringLiteral,
    InvalidStringLiteral,
    IntegerLiteral,
    FloatLiteral,

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
    // TODO: DotDotDot
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

                            if let Some(b'\n') | None = self.buffer.get(self.index) {
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
                Some(b'.') => {
                    self.index += 1;
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
                None => self.token = Token::EOF,
                _ => {
                    self.index += 1;
                    self.token = Token::Unknown;
                }
            }

            break;
        }
    }
}
