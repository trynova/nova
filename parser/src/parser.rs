use crate::{
    ast::{AssignLevel, Binding, Expr, Span, Stmt, UnaryOp},
    lexer::{Keyword, Lexer, Token},
};

pub type Result<T> = std::result::Result<T, ()>;

#[derive(Debug)]
pub struct Parser<'a> {
    pub lex: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lex = Lexer::new(input);
        lex.next();
        Self { lex }
    }

    fn eat(&mut self, tok: Token) -> Result<Span> {
        if self.lex.token != tok {
            return Err(());
        }
        let span = self.lex.span();
        self.lex.next();
        Ok(span)
    }

    fn parse_simple_expr(&mut self) -> Result<Expr> {
        match self.lex.token {
            Token::LeftParen => {
                self.lex.next();
                let value = self.parse_expr(0)?;
                self.eat(Token::RightParen)?;
                Ok(value)
            }
            Token::NumberLiteral => {
                let span = self.lex.span();
                self.lex.next();
                Ok(Expr::NumberLiteral { span })
            }
            Token::StringLiteral => {
                let span = self.lex.span();
                self.lex.next();
                Ok(Expr::StringLiteral { span })
            }
            Token::Identifier => {
                let span = self.lex.span();
                self.lex.next();
                Ok(Expr::Identifier { span })
            }
            Token::Plus => {
                self.lex.next();
                let value = self.parse_expr(140)?;
                Ok(Expr::UnaryOp {
                    kind: UnaryOp::Pos,
                    value: Box::new(value),
                })
            }
            Token::Minus => {
                self.lex.next();
                let value = self.parse_expr(140)?;
                Ok(Expr::UnaryOp {
                    kind: UnaryOp::Neg,
                    value: Box::new(value),
                })
            }
            Token::Not => {
                self.lex.next();
                let value = self.parse_expr(140)?;
                Ok(Expr::UnaryOp {
                    kind: UnaryOp::Not,
                    value: Box::new(value),
                })
            }
            Token::BitComplement => {
                self.lex.next();
                let value = self.parse_expr(140)?;
                Ok(Expr::UnaryOp {
                    kind: UnaryOp::BitComplement,
                    value: Box::new(value),
                })
            }
            tok => panic!("{tok:?}"),
        }
    }

    pub fn parse_expr(&mut self, lbp: u8) -> Result<Expr> {
        let mut lhs = self.parse_simple_expr()?;

        loop {
            let prec = self.lex.token.lbp();

            if prec == 0 || prec < lbp {
                break;
            }

            if self.lex.token == Token::LeftParen {
                self.lex.next();
                let mut args = Vec::new();

                loop {
                    if self.lex.token == Token::RightParen {
                        break;
                    }

                    let value = self.parse_expr(0)?;
                    args.push(value);

                    if self.lex.token != Token::Comma {
                        break;
                    }
                    self.lex.next();
                }

                self.eat(Token::RightParen)?;

                lhs = Expr::FnCall {
                    calle: Box::new(lhs),
                    args: args.into_boxed_slice(),
                };
                continue;
            }

            let kind = self.lex.token.into();
            self.lex.next();

            lhs = Expr::BinaryOp {
                kind,
                lhs: Box::new(lhs),
                rhs: Box::new(self.parse_expr(prec)?),
            };
        }

        Ok(lhs)
    }

    pub fn parse_binding(&mut self) -> Result<Binding> {
        match self.lex.token {
            Token::Identifier => {
                let span = self.lex.span();
                self.lex.next();
                Ok(Binding::Ident(span))
            }
            _ => Err(()),
        }
    }

    pub fn parse_scope(&mut self, is_fn_scope: bool) -> Result<Box<[Stmt]>> {
        let mut nodes = Vec::new();
        loop {
            match self.lex.token {
                Token::Keyword(Keyword::Let | Keyword::Const | Keyword::Var) => {
                    let level = match self.lex.token {
                        Token::Keyword(Keyword::Let) => AssignLevel::Let,
                        Token::Keyword(Keyword::Const) => AssignLevel::Const,
                        Token::Keyword(Keyword::Var) => AssignLevel::Var,
                        _ => unreachable!(),
                    };

                    self.lex.next();
                    loop {
                        let binding = self.parse_binding()?;

                        match self.lex.token {
                            Token::Equal => {
                                self.lex.next();
                                let value = self.parse_expr(0)?;
                                nodes.push(Stmt::Assign {
                                    level,
                                    binding,
                                    value,
                                });
                            }
                            _ => {}
                        }

                        if self.lex.token != Token::Comma {
                            if self.lex.token != Token::Semi
                                && self.lex.token != Token::EOF
                                && self.lex.has_newline_before == false
                            {
                                return Err(());
                            }

                            if self.lex.token == Token::Semi {
                                self.lex.next();
                            }
                            break;
                        }
                        self.lex.next();
                    }
                }
                _ => break,
            }
        }
        Ok(nodes.into_boxed_slice())
    }
}
