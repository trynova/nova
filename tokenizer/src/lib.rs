use boa_unicode::UnicodeProperties;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Eof,
    String,
    NonTerminatedString,
    HexLit,
    BinLit,
    NumLit,
    Junk,
    JunkNewline,
    InvalidNonTerminatedComment,
    InvalidNewlineString,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LAngle,
    RAngle,
    Bang,
    Caret,
    Asterisk,
    Amp,
    And,
    Pipe,
    Or,
    Plus,
    AddAssign,
    Minus,
    SubAssign,
    Div,
    DivAssign,
    Equal,
    EqualEqual,
    EqualEqualEqual,
    LessOrEqual,
    GreaterOrEqual,
    FatArrow,
    Ident,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Token {
    pub kind: TokenKind,
    pub start: u32,
}

#[derive(Debug, Clone, Copy)]
enum State {
    Init,
    StringSingleContinue,
    StringDoubleContinue,
    StringSingleEscape,
    StringDoubleEscape,
    Zero,
    HexContinue,
    BinContinue,
    NumContinue,
    NumExpContinue,
    NumFloatContinue,
    Amp,
    Pipe,
    Plus,
    Minus,
    FwdSlash,
    Equal,
    EqualEqual,

    RAngle,
    LAngle,

    Junk,
    JunkNewline,

    JunkSlash,
    JunkNewlineSlash,

    JunkCommentContinue,
    JunkNewlineCommentContinue,

    JunkCommentAsterisk,
    JunkNewlineCommentAsterisk,
}

pub struct TokenStream<'a> {
    buffer: &'a [u8],
    index: u32,
    len: u32,
}

impl<'a> TokenStream<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            index: 0,
            len: u32::try_from(buffer.len()).expect("[todo: better error]"),
        }
    }

    pub fn next(&mut self) -> Token {
        let start = self.index;
        let mut state: State = State::Init;
        let mut kind: TokenKind = TokenKind::Eof;

        loop {
            let c = if self.index < self.len {
                // TODO: find way to avoid upcast on 64bit machines
                self.buffer[self.index as usize]
            } else {
                0
            };
            self.index += 1;

            use State::*;
            use TokenKind::*;

            match state {
                Init => match c {
                    b'\'' => {
                        state = StringSingleContinue;
                        kind = String;
                    }
                    b'"' => {
                        state = StringDoubleContinue;
                        kind = String;
                    }
                    b'0' => state = Zero,
                    b'1'..=b'9' => state = NumContinue,
                    b'\n' => {
                        kind = TokenKind::JunkNewline;
                        state = State::JunkNewline;
                    }
                    b'{' => {
                        kind = LBrace;
                        break;
                    }
                    b'}' => {
                        kind = RBrace;
                        break;
                    }
                    b'(' => {
                        kind = LParen;
                        break;
                    }
                    b')' => {
                        kind = RParen;
                        break;
                    }
                    b'<' => {
                        state = State::LAngle;
                    }
                    b'>' => {
                        state = State::RAngle;
                    }
                    b'!' => {
                        kind = Bang;
                        break;
                    }
                    b'^' => {
                        kind = Caret;
                        break;
                    }
                    b'*' => {
                        kind = Asterisk;
                        break;
                    }
                    b'&' => {
                        state = State::Amp;
                    }
                    b'|' => {
                        state = State::Pipe;
                    }
                    b'+' => {
                        state = State::Plus;
                    }
                    b'-' => {
                        state = State::Minus;
                    }
                    b'/' => {
                        state = State::FwdSlash;
                    }
                    b'=' => {
                        state = State::Equal;
                    }
                    b' ' | b'\r' | b'\t' => {
                        kind = TokenKind::Junk;
                        state = State::Junk;
                    }
                    0 => break,
                    _ => {
                        self.index -= 1;
                        let mut chars = unsafe {
                            std::str::from_utf8_unchecked(&self.buffer[self.index as usize..])
                        }
                        .char_indices();

                        // we know there's at least one
                        let (offset0, cp0) = chars.next().unwrap();

                        if cp0.is_id_start() {
                            self.index += offset0 as u32 + 1;

                            for (offset, cp) in chars {
                                if !cp.is_id_continue() {
                                    break;
                                }
                                self.index += offset as u32;
                            }

                            kind = Ident;
                            break;
                        }

                        panic!("Unknown character '{}'.", char::from(c));
                    }
                },
                State::Equal => match c {
                    b'=' => state = State::EqualEqual,
                    b'>' => {
                        kind = TokenKind::FatArrow;
                        break;
                    }
                    _ => {
                        kind = TokenKind::Equal;
                        self.index -= 1;
                        break;
                    }
                },
                State::EqualEqual => match c {
                    b'=' => {
                        kind = EqualEqualEqual;
                        break;
                    }
                    _ => {
                        kind = TokenKind::EqualEqual;
                        self.index -= 1;
                        break;
                    }
                },
                State::FwdSlash => {
                    kind = if c == b'=' {
                        DivAssign
                    } else {
                        self.index -= 1;
                        TokenKind::Div
                    };
                    break;
                }
                State::Plus => {
                    kind = if c == b'=' {
                        AddAssign
                    } else {
                        self.index -= 1;
                        TokenKind::Plus
                    };
                    break;
                }
                State::Minus => {
                    kind = if c == b'=' {
                        SubAssign
                    } else {
                        self.index -= 1;
                        TokenKind::Minus
                    };
                    break;
                }
                State::LAngle => {
                    kind = if c == b'=' {
                        LessOrEqual
                    } else {
                        self.index -= 1;
                        TokenKind::LAngle
                    };
                    break;
                }
                State::RAngle => {
                    kind = if c == b'=' {
                        GreaterOrEqual
                    } else {
                        self.index -= 1;
                        TokenKind::RAngle
                    };
                    break;
                }
                State::Amp => match c {
                    b'&' => {
                        kind = And;
                        break;
                    }
                    _ => {
                        kind = TokenKind::Amp;
                        self.index -= 1;
                        break;
                    }
                },
                State::Pipe => match c {
                    b'|' => {
                        kind = Or;
                        break;
                    }
                    _ => {
                        kind = TokenKind::Pipe;
                        self.index -= 1;
                        break;
                    }
                },
                State::Junk => match c {
                    b'\n' => state = State::JunkNewline,
                    b' ' | b'\r' | b'\t' => {}
                    b'/' => state = JunkSlash,
                    _ => {
                        self.index -= 1;
                        break;
                    }
                },
                State::JunkNewline => match c {
                    b' ' | b'\t' | b'\n' | b'\r' => {}
                    b'/' => state = JunkNewlineSlash,
                    _ => {
                        self.index -= 1;
                        break;
                    }
                },
                JunkSlash => match c {
                    b'*' => state = JunkCommentContinue,
                    _ => {
                        self.index -= 2;
                        break;
                    }
                },
                JunkNewlineSlash => match c {
                    b'*' => state = JunkNewlineCommentContinue,
                    _ => {
                        self.index -= 2;
                        break;
                    }
                },
                JunkCommentContinue => match c {
                    b'*' => state = JunkCommentAsterisk,
                    b'\n' => {
                        state = JunkNewlineCommentContinue;
                        kind = TokenKind::JunkNewline;
                    }
                    0 => {
                        kind = InvalidNonTerminatedComment;
                        break;
                    }
                    _ => {}
                },
                JunkNewlineCommentContinue => match c {
                    b'*' => state = JunkCommentAsterisk,
                    0 => {
                        kind = InvalidNonTerminatedComment;
                        break;
                    }
                    _ => {}
                },
                JunkCommentAsterisk => match c {
                    b'/' => {
                        state = State::Junk;
                    }
                    0 => {
                        kind = InvalidNonTerminatedComment;
                        break;
                    }
                    _ => {
                        self.index -= 2;
                        break;
                    }
                },
                JunkNewlineCommentAsterisk => match c {
                    b'/' => {
                        state = State::JunkNewline;
                    }
                    0 => {
                        kind = InvalidNonTerminatedComment;
                        break;
                    }
                    _ => {
                        self.index -= 2;
                        break;
                    }
                },
                StringSingleContinue => match c {
                    b'\\' => state = StringSingleEscape,
                    b'\'' => break,
                    b'\n' => kind = InvalidNewlineString,
                    0 => {
                        kind = NonTerminatedString;
                        break;
                    }
                    _ => {}
                },
                StringSingleEscape => match c {
                    0 => {
                        kind = NonTerminatedString;
                        break;
                    }
                    // TODO: make an actual validator
                    _ => state = StringSingleContinue,
                },
                StringDoubleContinue => match c {
                    b'\\' => state = StringDoubleEscape,
                    b'"' => break,
                    b'\n' => kind = InvalidNewlineString,
                    0 => {
                        kind = NonTerminatedString;
                        break;
                    }
                    _ => {}
                },
                StringDoubleEscape => match c {
                    0 => {
                        kind = NonTerminatedString;
                        break;
                    }
                    // TODO: make an actual validator
                    _ => state = StringDoubleContinue,
                },
                Zero => match c {
                    b'B' | b'b' => state = BinContinue,
                    b'X' | b'x' | b'0' => state = HexContinue,
                    0 => {
                        kind = NumLit;
                        self.index -= 1;
                        break;
                    }
                    _ => {}
                },
                BinContinue => match c {
                    b'0' | b'1' => {}
                    b'_' => todo!("Underscore literal support."),
                    _ => {
                        kind = BinLit;
                        self.index -= 1;
                        break;
                    }
                },
                HexContinue => match c {
                    b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => {}
                    b'_' => todo!("Underscore literal support."),
                    _ => {
                        kind = HexLit;
                        self.index -= 1;
                        break;
                    }
                },
                NumContinue => match c {
                    b'0'..=b'9' => {}
                    b'_' => todo!("Underscore literal support."),
                    b'e' => state = NumExpContinue,
                    b'.' => state = NumFloatContinue,
                    _ => {
                        kind = NumLit;
                        self.index -= 1;
                        break;
                    }
                },
                NumExpContinue => match c {
                    b'0'..=b'9' => {}
                    b'_' => todo!("Underscore literal support."),
                    _ => {
                        kind = NumLit;
                        self.index -= 1;
                        break;
                    }
                },
                NumFloatContinue => match c {
                    b'0'..=b'9' => {}
                    b'_' => todo!("Underscore literal support."),
                    b'e' => state = NumExpContinue,
                    b'.' => state = NumFloatContinue,
                    _ => {
                        kind = NumLit;
                        self.index -= 1;
                        break;
                    }
                },
            }
        }

        Token { start, kind }
    }
}
