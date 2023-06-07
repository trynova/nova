use boa_unicode::UnicodeProperties;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    EOF,
    Ident,
    Number,
    Semi,
    Equal,
    LBrack,
    RBrack,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Inc,
    Dec,
    Sub,
    Colon,
    Comma,
    Pow,
    Mul,
    Not,
    Gte,
    Gt,
    ShiftRight,
    ShiftRightAssign,
    UShiftRight,
    UShiftRightAssign,
    ShiftLeft,
    ShiftLeftAssign,
    Lt,
    Lte,
    BOrAssign,
    BOr,
    OrAssign,
    Or,
    AndAssign,
    And,
    BAndAssign,
    BAnd,
    Xor,
    XorAssign,
    BNot,
    Nullish,
    NullishAssign,
    Ternary,
    Div,
    DivAssign,
    Mod,
    ModAssign,
    AddAssign,
    Add,
    SubAssign,
    PowAssign,
    MulAssign,
    Equality,
    StrictEquality,
    StrictInequality,
    Inequality,
    OptionalChain,
    Dot,
    Spread,
    InvalidDotDot,
    InvalidString,
    String,
    Template,
    TemplateEnd,
    TemplatePart,
    TemplateStart,
    InvalidComment,
    Arrow,
    Invalid,

    KeywordAwait,
    KeywordAsync,
    KeywordBreak,
    KeywordCase,
    KeywordCatch,
    KeywordClass,
    KeywordContinue,
    KeywordConst,
    // this is a keyword?
    KeywordDebugger,
    KeywordDefault,
    KeywordDelete,
    KeywordDo,
    KeywordElse,
    KeywordExport,
    KeywordExtends,
    KeywordFalse,
    KeywordFinally,
    KeywordFor,
    KeywordFunction,
    KeywordIf,
    KeywordIn,
    KeywordInstanceOf,
    KeywordImport,
    KeywordLet,
    KeywordNew,
    KeywordNull,
    KeywordOf,
    KeywordReturn,
    KeywordSuper,
    KeywordSwitch,
    KeywordThis,
    KeywordThrow,
    KeywordTrue,
    KeywordTry,
    KeywordTypeOf,
    KeywordVar,
    KeywordVoid,
    KeywordWhile,
    KeywordYield,
}

impl Token {
    pub fn is_right_assoc(self) -> bool {
        matches!(
            self,
            Token::Pow
                | Token::Ternary
                | Token::Arrow
                | Token::Equal
                | Token::AddAssign
                | Token::PowAssign
                | Token::SubAssign
                | Token::MulAssign
                | Token::DivAssign
                | Token::ModAssign
                | Token::ShiftLeftAssign
                | Token::ShiftRightAssign
                | Token::UShiftRightAssign
                | Token::BAndAssign
                | Token::XorAssign
                | Token::BOrAssign
                | Token::AndAssign
                | Token::OrAssign
                | Token::NullishAssign
        )
    }

    /// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Operator_Precedence
    pub fn lbp(self) -> u8 {
        match self {
            // 18 is just grouping
            Token::Dot | Token::OptionalChain | Token::LBrack | Token::LParen => 17,
            // 16 is only prefix ops
            Token::Inc | Token::Dec => 15,
            // 14 is only prefix ops
            Token::Pow => 13,
            Token::Mul | Token::Mod | Token::Div => 12,
            Token::Add | Token::Sub => 11,
            Token::ShiftLeft | Token::ShiftRight | Token::UShiftRight => 10,
            Token::Lt | Token::Lte | Token::Gt | Token::Gte => 9,
            Token::Equality
            | Token::Inequality
            | Token::StrictEquality
            | Token::StrictInequality => 8,
            Token::BAnd => 7,
            Token::Xor => 6,
            Token::BOr => 5,
            Token::And => 4,
            Token::Or | Token::Nullish => 3,
            Token::Equal
            | Token::AddAssign
            | Token::PowAssign
            | Token::SubAssign
            | Token::MulAssign
            | Token::DivAssign
            | Token::ModAssign
            | Token::ShiftLeftAssign
            | Token::ShiftRightAssign
            | Token::UShiftRightAssign
            | Token::BAndAssign
            | Token::XorAssign
            | Token::BOrAssign
            | Token::AndAssign
            | Token::OrAssign
            | Token::NullishAssign
            | Token::Arrow
            | Token::Ternary => 2,
            Token::Comma => 1,
            _ => 0,
        }
    }
}

static KEYWORDS: phf::Map<&'static str, Token> = phf::phf_map! {
    "await" => Token::KeywordAwait,
    "async" => Token::KeywordAsync,
    "break" => Token::KeywordBreak,
    "case" => Token::KeywordCase,
    "catch" => Token::KeywordCatch,
    "class" => Token::KeywordClass,
    "continue" => Token::KeywordContinue,
    "const" => Token::KeywordConst,
    // this is a keyword?
    "debugger" => Token::KeywordDebugger,
    "default" => Token::KeywordDefault,
    "delete" => Token::KeywordDelete,
    "do" => Token::KeywordDo,
    "else" => Token::KeywordElse,
    "export" => Token::KeywordExport,
    "extends" => Token::KeywordExtends,
    "false" => Token::KeywordFalse,
    "finally" => Token::KeywordFinally,
    "for" => Token::KeywordFor,
    "function" => Token::KeywordFunction,
    "if" => Token::KeywordIf,
    "in" => Token::KeywordIn,
    "instanceof" => Token::KeywordInstanceOf,
    "import" => Token::KeywordImport,
    "let" => Token::KeywordLet,
    "new" => Token::KeywordNew,
    "null" => Token::KeywordNull,
    "of" => Token::KeywordOf,
    "return" => Token::KeywordReturn,
    "super" => Token::KeywordSuper,
    "switch" => Token::KeywordSwitch,
    "this" => Token::KeywordThis,
    "throw" => Token::KeywordThrow,
    "true" => Token::KeywordTrue,
    "try" => Token::KeywordTry,
    "typeof" => Token::KeywordTypeOf,
    "var" => Token::KeywordVar,
    "void" => Token::KeywordVoid,
    "while" => Token::KeywordWhile,
    "yield" => Token::KeywordYield,
};

#[derive(Debug)]
pub struct Lexer<'a> {
    pub source: &'a str,
    /// `Option<char>` is memory optimized to only 4 bytes because of UTF-8
    /// codepoint limits.
    pub codepoint: Option<char>,
    pub index: usize,
    pub token: Token,
    pub start: usize,
    pub has_newline_before: bool,
    pub open_template_count: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            codepoint: source.chars().next().or(None),
            index: 0,
            token: Token::EOF,
            start: 0,
            has_newline_before: true,
            open_template_count: 0,
        }
    }

    /// Resets the parser at the given index.
    pub fn reset(&mut self, index: usize) {
        self.index = index;
        self.codepoint = self.source[self.index..].chars().next().or(None);
        self.next();
    }

    /// Steps a unicode codepoint forwards.
    fn step(&mut self) {
        let Some(cp) = self.codepoint else {
            return;
        };

        self.index += cp.len_utf8();
        self.codepoint = self.source[self.index..].chars().next();
    }

    #[inline]
    fn continue_ident_fast(&mut self) {
        loop {
            match self.codepoint {
                Some('a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$') => {
                    self.step();
                }
                Some(ch) => {
                    // We know it's just a non-ident continue ASCII character.
                    if ch.is_ascii() {
                        break;
                    }

                    // We can return here and wave the keyword check because
                    // keywords are only ASCII.
                    return self.continue_ident_slow();
                }
                None => break,
            }
        }

        // Check if the identifier is a keyword.
        if let Some(keyword) = KEYWORDS.get(&self.source[self.start..self.index]) {
            self.token = *keyword;
        }
    }

    #[inline]
    fn continue_ident_slow(&mut self) {
        loop {
            let Some(ch) = self.codepoint else {
                break;
            };

            if !ch.is_id_continue() {
                break;
            }

            self.step();
        }
    }

    #[inline]
    fn continue_zero(&mut self) {
        // TODO: actually implement this
        self.continue_number();
    }

    #[inline]
    fn continue_number(&mut self) {
        // TODO: actually implement this
        loop {
            match self.codepoint {
                Some('0'..='9') => {
                    self.step();
                }
                _ => break,
            }
        }
    }

    #[inline]
    fn continue_string(&mut self, end: char) {
        let mut escaped = false;

        loop {
            match (escaped, self.codepoint) {
                (_, None | Some('\r' | '\n')) => {
                    self.token = Token::InvalidString;
                    break;
                }
                (false, ch) if ch == Some(end) => {
                    self.step();
                    break;
                }
                (false, Some('\\')) => escaped = true,
                _ => escaped = false,
            }
            self.step();
        }
    }

    #[inline]
    fn continue_template(&mut self) {
        let mut escaped = false;
        loop {
            match (escaped, self.codepoint) {
                (_, None) => {
                    self.token = Token::InvalidString;
                    break;
                }
                (false, Some('`')) => {
                    self.step();
                    self.token = if self.token == Token::TemplateStart {
                        Token::Template
                    } else {
                        Token::TemplateEnd
                    };
                    break;
                }
                (false, Some('$')) => {
                    self.step();
                    if let Some('{') = self.codepoint {
                        self.step();
                        self.open_template_count += 1;
                        break;
                    }
                }
                _ => {
                    self.step();
                    escaped = false;
                }
            }
        }
    }

    pub fn next(&mut self) {
        self.has_newline_before = false;

        'main: loop {
            self.start = self.index;

            match self.codepoint {
                None => self.token = Token::EOF,
                Some(' ' | '\t') => {
                    self.step();
                    continue 'main;
                }
                Some('\r' | '\n') => {
                    self.step();
                    self.has_newline_before = true;
                    continue 'main;
                }
                Some('a'..='z' | 'A'..='Z' | '_' | '$') => {
                    self.step();
                    self.token = Token::Ident;
                    self.continue_ident_fast();
                }
                Some('0') => {
                    self.step();
                    self.continue_zero();
                }
                Some('1'..='9') => {
                    self.step();
                    self.token = Token::Number;
                    self.continue_number();
                }
                Some('\'') => {
                    self.step();
                    self.token = Token::String;
                    self.continue_string('\'');
                }
                Some('"') => {
                    self.step();
                    self.token = Token::String;
                    self.continue_string('"');
                }
                Some('`') => {
                    self.step();
                    self.token = Token::TemplateStart;
                    self.continue_template();
                }
                Some('[') => {
                    self.step();
                    self.token = Token::LBrack;
                }
                Some(']') => {
                    self.step();
                    self.token = Token::RBrack;
                }
                Some('(') => {
                    self.step();
                    self.token = Token::LParen;
                }
                Some(')') => {
                    self.step();
                    self.token = Token::RParen;
                }
                Some('{') => {
                    self.step();
                    self.token = Token::LBrace;
                }
                Some('}') => {
                    self.step();
                    self.token = if self.open_template_count > 0 {
                        self.token = Token::TemplatePart;
                        self.continue_template();
                        self.open_template_count -= 1;
                        break;
                    } else {
                        Token::RBrace
                    };
                }
                Some('+') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('+') => {
                            self.step();
                            Token::Inc
                        }
                        Some('=') => {
                            self.step();
                            Token::AddAssign
                        }
                        _ => Token::Add,
                    };
                }
                Some('-') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('-') => {
                            self.step();
                            Token::Dec
                        }
                        Some('=') => {
                            self.step();
                            Token::SubAssign
                        }
                        _ => Token::Sub,
                    };
                }
                Some('*') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('*') => {
                            self.step();
                            if let Some('=') = self.codepoint {
                                self.step();
                                Token::PowAssign
                            } else {
                                Token::Pow
                            }
                        }
                        Some('=') => {
                            self.step();
                            Token::MulAssign
                        }
                        _ => Token::Mul,
                    };
                }
                Some('%') => {
                    self.step();
                    self.token = if let Some('=') = self.codepoint {
                        self.step();
                        Token::ModAssign
                    } else {
                        Token::Mod
                    };
                }
                Some('/') => 'blk: {
                    self.step();
                    self.token = match self.codepoint {
                        Some('/') => loop {
                            self.step();
                            match self.codepoint {
                                None | Some('\r' | '\n') => continue 'main,
                                _ => {}
                            }
                        },
                        Some('*') => loop {
                            self.step();
                            match self.codepoint {
                                None => {
                                    self.token = Token::InvalidComment;
                                    break 'blk;
                                }
                                Some('*') => {
                                    if let Some('/') = self.source[self.index + 1..].chars().next()
                                    {
                                        self.step();
                                        self.step();
                                        continue 'main;
                                    }
                                }
                                _ => {}
                            }
                        },
                        Some('=') => {
                            self.step();
                            Token::DivAssign
                        }
                        _ => Token::Div,
                    };
                }
                Some('=') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('>') => {
                            self.step();
                            Token::Arrow
                        }
                        Some('=') => {
                            self.step();
                            if let Some('=') = self.codepoint {
                                self.step();
                                Token::StrictEquality
                            } else {
                                Token::Equality
                            }
                        }
                        _ => Token::Equal,
                    };
                }
                Some('!') => {
                    self.step();
                    self.token = if let Some('=') = self.codepoint {
                        self.step();
                        if let Some('=') = self.codepoint {
                            self.step();
                            Token::StrictInequality
                        } else {
                            Token::Inequality
                        }
                    } else {
                        Token::Not
                    };
                }
                Some('>') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('=') => {
                            self.step();
                            Token::Gte
                        }
                        Some('>') => {
                            self.step();
                            match self.codepoint {
                                Some('>') => {
                                    self.step();
                                    if let Some('=') = self.codepoint {
                                        self.step();
                                        Token::UShiftRightAssign
                                    } else {
                                        Token::UShiftRight
                                    }
                                }
                                Some('=') => {
                                    self.step();
                                    Token::ShiftRightAssign
                                }
                                _ => Token::ShiftRight,
                            }
                        }
                        _ => Token::Gt,
                    };
                }
                Some('<') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('=') => {
                            self.step();
                            Token::Lte
                        }
                        Some('<') => {
                            self.step();
                            if let Some('=') = self.codepoint {
                                self.step();
                                Token::ShiftLeftAssign
                            } else {
                                Token::ShiftLeft
                            }
                        }
                        _ => Token::Lt,
                    };
                }
                Some('|') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('|') => {
                            self.step();
                            if let Some('=') = self.codepoint {
                                self.step();
                                Token::OrAssign
                            } else {
                                Token::Or
                            }
                        }
                        Some('=') => {
                            self.step();
                            Token::BOrAssign
                        }
                        _ => Token::BOr,
                    };
                }
                Some('&') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('&') => {
                            self.step();
                            if let Some('=') = self.codepoint {
                                self.step();
                                Token::AndAssign
                            } else {
                                Token::And
                            }
                        }
                        Some('=') => {
                            self.step();
                            Token::BAndAssign
                        }
                        _ => Token::BAnd,
                    };
                }
                Some('^') => {
                    self.step();
                    self.token = if let Some('=') = self.codepoint {
                        self.step();
                        Token::XorAssign
                    } else {
                        Token::Xor
                    };
                }
                Some('~') => {
                    self.step();
                    self.token = Token::BNot;
                }
                Some('?') => {
                    self.step();
                    self.token = match self.codepoint {
                        Some('?') => {
                            self.step();
                            if let Some('=') = self.codepoint {
                                self.step();
                                Token::NullishAssign
                            } else {
                                Token::Nullish
                            }
                        }
                        Some('.') => {
                            self.step();
                            Token::OptionalChain
                        }
                        _ => Token::Ternary,
                    };
                }
                Some('.') => {
                    self.step();
                    self.token = if let Some('.') = self.codepoint {
                        if let Some('.') = self.source[self.index + 1..].chars().next() {
                            self.step();
                            self.step();
                            Token::Spread
                        } else {
                            Token::Dot
                        }
                    } else {
                        Token::Dot
                    };
                }
                Some(';') => {
                    self.step();
                    self.token = Token::Semi;
                }
                Some(':') => {
                    self.step();
                    self.token = Token::Colon;
                }
                Some(',') => {
                    self.step();
                    self.token = Token::Comma;
                }
                Some(ch) => 'blk: {
                    // Skip unicode whitespace characters.
                    if ch.is_pattern_whitespace() {
                        self.step();
                        continue 'main;
                    }

                    // Eat unicode identifiers.
                    if ch.is_id_continue() {
                        self.step();
                        self.token = Token::Ident;
                        self.continue_ident_slow();
                        break 'blk;
                    }

                    self.step();
                    self.token = Token::Invalid;
                }
            }

            break;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_tokens {
        ($source: expr, $tokens: expr) => {{
            let expected: &[Token] = $tokens;
            let mut out = Vec::<Token>::with_capacity(expected.len());
            let source: &str = $source;
            let mut stream = Lexer::new(source);

            loop {
                stream.next();
                if stream.token == Token::EOF {
                    break;
                }
                out.push(stream.token);
            }

            if stream.token != Token::EOF {
                assert!(
                    false,
                    "Expected end of file to end token stream. Found: {:?}",
                    stream.token
                );
            }

            assert_eq!(out.as_slice(), expected);
        }};
    }

    #[test]
    fn unicode_identifiers() {
        assert_tokens!(
            "ሀ zቐ ኂd bꡅa",
            &[Token::Ident, Token::Ident, Token::Ident, Token::Ident]
        );
    }

    #[test]
    fn operators() {
        assert_tokens!(
            "+ += ++ - -= -- * *= % %= / /= ** **= . ...",
            &[
                Token::Add,
                Token::AddAssign,
                Token::Inc,
                Token::Sub,
                Token::SubAssign,
                Token::Dec,
                Token::Mul,
                Token::MulAssign,
                Token::Mod,
                Token::ModAssign,
                Token::Div,
                Token::DivAssign,
                Token::Pow,
                Token::PowAssign,
                Token::Dot,
                Token::Spread,
            ]
        );
    }
}
