use crate::{
    ast::{AssignLevel, Binding, Expr, Function, FunctionParam, Span, Stmt},
    lexer::{Keyword, Lexer, Token},
};

pub type Result<T> = std::result::Result<T, ()>;

#[derive(Debug)]
pub struct Parser<'a> {
    pub lex: Lexer<'a>,
    pub error: String,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lex = Lexer::new(input);
        lex.next();
        Self {
            lex,
            error: "".into(),
        }
    }

    fn eat(&mut self, tok: Token) -> Result<Span> {
        if self.lex.token != tok {
            self.error = format!("expected {:?}, found {:?}", self.lex.token, tok);
            return Err(());
        }
        let span = self.lex.span();
        self.lex.next();
        Ok(span)
    }

    /// Parses a function expression. Assumes the `function` keyword has already
    /// been consumed.
    fn parse_function(&mut self) -> Result<Function> {
        let name = if self.lex.token == Token::Identifier {
            let span = self.lex.span();
            self.lex.next();
            Some(span)
        } else {
            None
        };

        self.eat(Token::LeftParen)?;

        let mut params = Vec::new();
        loop {
            if self.lex.token == Token::RightParen {
                break;
            }

            let name = self.parse_binding()?;

            match self.lex.token {
                Token::Comma => {
                    self.lex.next();
                    params.push(FunctionParam {
                        name,
                        default: None,
                    });
                }
                Token::Equal => {
                    self.lex.next();

                    let default = self.parse_expr(0)?;
                    params.push(FunctionParam {
                        name,
                        default: Some(Box::new(default)),
                    });

                    if self.lex.token != Token::Comma {
                        break;
                    }
                    self.lex.next();
                }
                _ => {
                    let Binding::Identifier(_) = name else {
						self.error = "Missing initializer in destructuring declaration".into();
						return Err(());
					};
                    params.push(FunctionParam {
                        name,
                        default: None,
                    });
                    break;
                }
            }
        }

        self.eat(Token::RightParen)?;
        self.eat(Token::LeftBrace)?;
        let scope = self.parse_scope(true)?;
        self.eat(Token::RightBrace)?;

        Ok(Function {
            name,
            params: params.into_boxed_slice(),
            scope,
        })
    }

    fn parse_simple_expr(&mut self) -> Result<Expr> {
        match self.lex.token {
            Token::Keyword(Keyword::Function) => {
                self.lex.next();
                Ok(Expr::Function(self.parse_function()?))
            }
            Token::LeftParen => {
                self.lex.next();
                let value = self.parse_expr(0)?;
                self.eat(Token::RightParen)?;
                Ok(value)
            }
            Token::LeftBrack => {
                self.lex.next();

                let mut values = Vec::new();

                loop {
                    if self.lex.token == Token::Comma {
                        self.lex.next();
                        if self.lex.token == Token::RightBrack {
                            break;
                        }
                        values.push(None);
                        continue;
                    }

                    if self.lex.token == Token::RightBrack {
                        break;
                    }

                    let value = self.parse_expr(0)?;
                    values.push(Some(value));

                    if self.lex.token == Token::Comma {
                        self.lex.next();
                    }
                }
                self.eat(Token::RightBrack)?;

                Ok(Expr::ArrayLiteral {
                    values: values.into_boxed_slice(),
                })
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
            Token::Plus
            | Token::Minus
            | Token::Not
            | Token::BitComplement
            | Token::Keyword(Keyword::Yield)
            | Token::Keyword(Keyword::Await) => {
                let kind = self.lex.token.into();
                self.lex.next();
                let value = self.parse_expr(140)?;
                Ok(Expr::UnaryOp {
                    kind,
                    value: Box::new(value),
                })
            }
            Token::Keyword(Keyword::Null) => {
                self.lex.next();
                Ok(Expr::Null)
            }
            tok => {
                self.error = format!("expected expression, found {tok:?}");
                return Err(());
            }
        }
    }

    pub fn parse_expr(&mut self, lbp: u8) -> Result<Expr> {
        let mut lhs = self.parse_simple_expr()?;

        loop {
            let prec = self.lex.token.lbp();

            if prec == 0 || prec <= lbp {
                break;
            }

            // TODO: We need custom logic here for ordering unary keywords
            //       because code like `a + yield 1` should fail to parse.

            if self.lex.token == Token::LeftBrack {
                self.lex.next();
                let index = self.parse_expr(0)?;
                self.eat(Token::RightBrack)?;
                lhs = Expr::Index {
                    root: Box::new(lhs),
                    index: Box::new(index),
                };
                continue;
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

                lhs = Expr::FunctionCall {
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
                Ok(Binding::Identifier(span))
            }
            _ => Err(()),
        }
    }

    fn expect_stmt_end(&mut self) -> Result<()> {
        if self.lex.token == Token::Semi {
            self.lex.next();
        } else if self.lex.token != Token::EOF
            && self.lex.token != Token::RightBrace
            && !self.lex.has_newline_before
        {
            self.error = format!(
                "expected new line or semi colon, found {:?}",
                self.lex.token
            );
            return Err(());
        }
        Ok(())
    }

    pub fn parse_scope(&mut self, is_fn_scope: bool) -> Result<Box<[Stmt]>> {
        let mut nodes = Vec::new();
        loop {
            match self.lex.token {
                Token::Semi => self.lex.next(),
                Token::Keyword(Keyword::Return) => {
                    if !is_fn_scope {
                        self.error =
                            "return statements are not allowed outside of functions".into();
                        return Err(());
                    }
                    self.lex.next();
                    nodes.push(Stmt::Return {
                        value: self.parse_expr(0)?,
                    });
                    self.expect_stmt_end()?;
                }
                Token::Keyword(Keyword::Function) => {
                    self.lex.next();
                    nodes.push(Stmt::Function(self.parse_function()?));
                }
                Token::Keyword(Keyword::Break) => {
                    self.lex.next();

                    nodes.push(Stmt::Break {
                        label: if self.lex.token == Token::Identifier {
                            let label = self.lex.span();
                            self.lex.next();
                            Some(label)
                        } else {
                            None
                        },
                    });
                    self.expect_stmt_end()?;
                }
                Token::Keyword(Keyword::Continue) => {
                    self.lex.next();

                    nodes.push(Stmt::Continue {
                        label: if self.lex.token == Token::Identifier {
                            let label = self.lex.span();
                            self.lex.next();
                            Some(label)
                        } else {
                            None
                        },
                    });
                    self.expect_stmt_end()?;
                }
                Token::Keyword(Keyword::Let | Keyword::Const | Keyword::Var) => 'blk: {
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
                            Token::Semi => {
                                self.lex.next();
                                nodes.push(Stmt::Declare { level, binding });
                                self.expect_stmt_end()?;
                                break 'blk;
                            }
                            Token::Equal => {
                                self.lex.next();
                                let value = self.parse_expr(0)?;
                                nodes.push(Stmt::Assign {
                                    level,
                                    binding,
                                    value,
                                });
                            }
                            _ => {
                                if !self.lex.has_newline_before {
                                    self.eat(Token::Semi)?; // this must fail
                                    unreachable!();
                                }

                                nodes.push(Stmt::Declare { level, binding });
                            }
                        }

                        if self.lex.token != Token::Comma {
                            if self.lex.token != Token::Semi
                                && self.lex.token != Token::EOF
                                && self.lex.token != Token::RightBrace
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

                    self.expect_stmt_end()?;
                }
                _ => {
                    let Ok(value) = self.parse_expr(0) else {
						break;
					};
                    self.expect_stmt_end()?;
                    nodes.push(Stmt::Expr { value });
                }
            }
        }
        Ok(nodes.into_boxed_slice())
    }

    pub fn parse_global_scope(&mut self) -> Result<Box<[Stmt]>> {
        let scope = self.parse_scope(true)?;
        if self.lex.token != Token::EOF {
            self.error = format!("expected statement, found {:?}", self.lex.token);
            return Err(());
        }
        Ok(scope)
    }
}
