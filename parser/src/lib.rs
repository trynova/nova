use ast::{Call, Decl, Node, NodeRef, SourceRef};
use generational_arena::Arena;
use tokenizer::{Token, TokenStream};

pub mod ast;

pub struct Parser<'a> {
    lex: TokenStream<'a>,
    pub nodes: Arena<Node>,
}

type Result<T> = std::result::Result<T, ()>;

#[derive(Debug)]
#[repr(packed)]
struct ScopeState {
    pub is_loop: bool,
    pub is_function: bool,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut stream = TokenStream::new(source);
        stream.next();
        stream.has_newline_before = true;

        let mut arena = Arena::new();
        let empty_idx = arena.insert(Node::Empty);
        assert!(
            empty_idx == Node::empty(),
            "The empty index must be placed at 0."
        );

        Self {
            lex: stream,
            nodes: arena,
        }
    }

    pub fn take(&mut self) -> SourceRef {
        let source_ref = SourceRef {
            start: self.lex.start as u32,
            end: self.lex.index as u32,
        };
        self.lex.next();
        source_ref
    }

    pub fn eat(&mut self, token: Token) -> Result<SourceRef> {
        if self.lex.token != token {
            return Err(());
        }
        Ok(self.take())
    }

    pub fn expect(&mut self, token: Token) -> Result<()> {
        if self.lex.token != token {
            return Err(());
        }
        self.lex.next();
        Ok(())
    }

    fn parse_binding(&mut self) -> Result<NodeRef> {
        match self.lex.token {
            // TODO: implement destructuring
            Token::Ident => {
                let source_ref = self.take();
                Ok(self.nodes.insert(Node::Ident(source_ref)))
            }
            _ => Err(()),
        }
    }

    /// Takes in the highest power of the expression before.
    pub fn parse_expr(&mut self, hp: u8) -> Result<NodeRef> {
        let mut lhs = match self.lex.token {
            Token::String => {
                let source_ref = self.take();
                self.nodes.insert(Node::String(SourceRef {
                    start: source_ref.start + 1,
                    end: source_ref.end - 1,
                }))
            }
            Token::Ident => {
                let source_ref = self.take();
                self.nodes.insert(Node::Ident(source_ref))
            }
            Token::Number => {
                let source_ref = self.take();
                self.nodes.insert(Node::Number(source_ref))
            }
            Token::LParen => {
                self.lex.next();
                let node = self.parse_expr(0)?;
                self.expect(Token::RParen)?;
                self.nodes.insert(Node::Paren(node))
            }
            Token::LBrack => {
                self.lex.next();

                let mut values = Vec::new();

                if self.lex.token != Token::RBrack {
                    loop {
                        if self.lex.token == Token::RBrack {
                            break;
                        }

                        if self.lex.token == Token::Comma {
                            values.push(Node::empty());
                        } else {
                            values.push(self.parse_expr(1)?);
                            if self.lex.token != Token::Comma {
                                break;
                            }
                        }

                        self.lex.next();
                    }
                }
                self.expect(Token::RBrack)?;

                self.nodes.insert(Node::Array(ast::Array {
                    values: values.into_boxed_slice(),
                }))
            }
            // TODO: implement async scopes
            Token::KeywordAsync => {
                self.lex.next();

                if self.lex.token != Token::KeywordFunction {
                    self.expect(Token::KeywordFunction)?;
                }

                // Parse *just* the function.
                let func_idx = self.parse_expr(100)?;
                let Some(func) = self.nodes.get(func_idx) else {
                    unreachable!();
                };

                if let Node::Function(func) = func {
                    let func_data = func.clone();
                    self.nodes[func_idx] = Node::AsyncFunction(func_data);
                    func_idx
                } else {
                    unreachable!()
                }
            }
            Token::KeywordFunction => {
                self.lex.next();

                let name = if self.lex.token == Token::Ident {
                    let source_ref = self.take();
                    self.nodes.insert(Node::Ident(source_ref))
                } else {
                    Node::empty()
                };

                self.expect(Token::LParen)?;

                let mut params = Vec::new();
                let mut spread_count: usize = 0;

                loop {
                    if self.lex.token == Token::RParen {
                        break;
                    }

                    if self.lex.token == Token::Spread {
                        self.lex.next();
                        let value = self.parse_expr(1)?;
                        params.push(self.nodes.insert(Node::Spread(value)));

                        if spread_count > 0 {
                            eprintln!("Found more than 1 function param spread.");
                            return Err(());
                        }
                        spread_count += 1;
                    } else {
                        let name = self.parse_binding()?;

                        let default = if self.lex.token == Token::Equal {
                            self.lex.next();
                            self.parse_expr(1)?
                        } else {
                            Node::empty()
                        };

                        params.push(self.nodes.insert(Node::Param(ast::Param { name, default })));
                    }

                    if self.lex.token != Token::Comma {
                        break;
                    }
                    self.lex.next();
                }

                self.expect(Token::RParen)?;
                self.expect(Token::LBrace)?;

                let scope = self.parse_scope(ScopeState {
                    is_loop: false,
                    is_function: true,
                })?;

                self.expect(Token::RBrace)?;

                // ASI always terminates function expressions.
                self.lex.has_newline_before = true;

                self.nodes.insert(Node::Function(ast::Function {
                    name,
                    params: params.into_boxed_slice(),
                    scope: scope.into_boxed_slice(),
                }))
            }
            other => {
                eprintln!("Expected expression, found {other:?}.");
                return Err(());
            }
        };

        loop {
            let power = self.lex.token.lbp();
            if hp >= power || power == 0 {
                break;
            }

            match self.lex.token {
                // Implement `name =>` arrow syntax
                Token::Arrow => {
                    self.lex.next();

                    let lhs_value = self.nodes.get(lhs);

                    let Some(Node::Ident(_)) = lhs_value else {
                        break;
                    };

                    let scope = if self.lex.token == Token::LBrace {
                        self.lex.next();
                        let scope = self.parse_scope(ScopeState {
                            is_loop: false,
                            is_function: true,
                        })?;
                        self.expect(Token::RBrace)?;
                        scope.into_boxed_slice()
                    } else {
                        Box::new([self.parse_expr(1)?])
                    };

                    lhs = self.nodes.insert(Node::ArrowFunction(ast::Function {
                        name: Node::empty(),
                        params: Box::new([lhs]),
                        scope,
                    }));
                }
                Token::Equal => {
                    self.lex.next();
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Assign(ast::BinaryOp { lhs, rhs }));
                }
                Token::Comma => {
                    self.lex.next();
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Group(ast::BinaryOp { lhs, rhs }));
                }
                Token::Add => {
                    self.lex.next();
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Add(ast::BinaryOp { lhs, rhs }));
                }
                Token::Sub => {
                    self.lex.next();
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Sub(ast::BinaryOp { lhs, rhs }));
                }
                Token::Mul => {
                    self.lex.next();
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Mul(ast::BinaryOp { lhs, rhs }));
                }
                Token::Mod => {
                    self.lex.next();
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Mod(ast::BinaryOp { lhs, rhs }));
                }
                Token::Div => {
                    self.lex.next();
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Div(ast::BinaryOp { lhs, rhs }));
                }
                Token::LBrack => {
                    self.lex.next();
                    let index = self.parse_expr(1)?;
                    self.expect(Token::RBrack)?;
                    lhs = self
                        .nodes
                        .insert(Node::Index(ast::Index { root: lhs, index }));
                }
                Token::LParen => {
                    self.lex.next();
                    let mut args = Vec::new();

                    loop {
                        if self.lex.token == Token::RParen {
                            break;
                        }

                        if self.lex.token == Token::Spread {
                            self.lex.next();
                            let value = self.parse_expr(1)?;
                            args.push(self.nodes.insert(Node::Spread(value)));
                        } else {
                            args.push(self.parse_expr(1)?);
                        }

                        if self.lex.token != Token::Comma {
                            break;
                        }

                        self.lex.next();
                    }

                    self.expect(Token::RParen)?;

                    lhs = self.nodes.insert(Node::Call(Call {
                        callee: lhs,
                        args: args.into_boxed_slice(),
                    }));
                }
                _ => return Err(()),
            }
        }

        Ok(lhs)
    }

    #[inline]
    pub fn parse_global_scope(&mut self) -> Result<Vec<NodeRef>> {
        self.parse_scope(ScopeState {
            is_function: false,
            is_loop: false,
        })
    }

    #[inline]
    fn expect_valid_terminator(&mut self) -> Result<()> {
        if self.lex.token == Token::Semi {
            self.lex.next();
            self.lex.has_newline_before = true;
            Ok(())
        } else if !self.lex.has_newline_before {
            // Recoverable?
            eprintln!("Expected a line ending at {:?}.", self.lex.token);
            Err(())
        } else {
            Ok(())
        }
    }

    fn parse_scope(&mut self, state: ScopeState) -> Result<Vec<NodeRef>> {
        let mut scope = Vec::new();

        loop {
            if self.lex.token == Token::RBrace || self.lex.token == Token::EOF {
                break;
            }

            self.expect_valid_terminator()?;

            match self.lex.token {
                Token::KeywordLet => {
                    self.lex.next();
                    let name = self.eat(Token::Ident)?;
                    let decl = self.nodes.insert(Node::Decl(Decl::Ident(name)));
                    self.expect(Token::Equal)?;
                    let value = self.parse_expr(1)?;
                    scope.push(self.nodes.insert(Node::LetDecl { decl, value }));
                }
                Token::KeywordConst => {
                    self.lex.next();
                    let name = self.eat(Token::Ident)?;
                    let decl = self.nodes.insert(Node::Decl(Decl::Ident(name)));
                    self.expect(Token::Equal)?;
                    let value = self.parse_expr(1)?;
                    scope.push(self.nodes.insert(Node::ConstDecl { decl, value }));
                }
                Token::KeywordReturn => 'blk: {
                    self.lex.next();

                    if self.lex.has_newline_before {
                        scope.push(self.nodes.insert(Node::Return(Node::empty())));
                        break 'blk;
                    }

                    if self.lex.token == Token::Semi {
                        self.lex.next();
                        scope.push(self.nodes.insert(Node::Return(Node::empty())));
                        break 'blk;
                    }

                    let value = self.parse_expr(1)?;
                    scope.push(self.nodes.insert(Node::Return(value)));

                    // We can simply report this later.
                    if !state.is_function {
                        return Err(());
                    }
                }
                Token::RBrace | Token::EOF => break,
                Token::Semi => {
                    self.lex.next();
                    self.lex.has_newline_before = true;
                }
                _ => {
                    scope.push(self.parse_expr(1)?);
                }
            }
        }

        Ok(scope)
    }
}

#[cfg(test)]
mod tests {}
