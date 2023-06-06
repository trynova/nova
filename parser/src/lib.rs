use ast::{Call, Decl, Node, NodeRef, SourceRef};
use generational_arena::Arena;
use tokenizer::{Token, TokenStream};

pub mod ast;

pub struct Parser<'a> {
    lex: TokenStream<'a>,
    pub nodes: Arena<Node>,
}

type Result<T> = std::result::Result<T, ()>;

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
struct ScopeState {
    pub is_loop: bool,
    pub is_function: bool,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut stream = TokenStream::new(source);
        stream.next();

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
            Token::KeywordTrue => {
                let source_ref = self.take();
                self.nodes.insert(Node::True(source_ref))
            }
            Token::KeywordFalse => {
                let source_ref = self.take();
                self.nodes.insert(Node::False(source_ref))
            }
            Token::KeywordNull => {
                let source_ref = self.take();
                self.nodes.insert(Node::Null(source_ref))
            }
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
                    scope,
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

                    let Some(lhs_value) = self.nodes.get(lhs) else {
                        unreachable!();
                    };

                    let Node::Ident(_) = lhs_value else {
                        break;
                    };

                    let scope = if self.lex.token == Token::LBrace {
                        self.lex.next();
                        let scope = self.parse_scope(ScopeState {
                            is_loop: false,
                            is_function: true,
                        })?;
                        self.expect(Token::RBrace)?;
                        scope
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
                    // TODO: validate lhs as assignment expr
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Assign(ast::BinaryOp { lhs, rhs }));
                }
                Token::Dot => {
                    self.lex.next();
                    // TODO: validate lhs as member expr
                    let rhs = self.parse_expr(power)?;
                    lhs = self.nodes.insert(Node::Member(ast::BinaryOp { lhs, rhs }));
                }
                Token::OptionalChain => 'blk: {
                    self.lex.next();
                    // TODO: validate lhs as member expr

                    if self.lex.token != Token::LParen {
                        // foo?.b
                        let rhs = self.parse_expr(power)?;
                        lhs = self
                            .nodes
                            .insert(Node::OptionalChain(ast::BinaryOp { lhs, rhs }));
                        break 'blk;
                    }

                    // foo?.()
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

                    lhs = self.nodes.insert(Node::OptionalCall(Call {
                        callee: lhs,
                        args: args.into_boxed_slice(),
                    }));
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
                Token::Ternary => {
                    self.lex.next();

                    let positive = self.parse_expr(1)?;
                    self.expect(Token::Colon)?;
                    let negative = self.parse_expr(1)?;

                    lhs = self.nodes.insert(Node::Ternary(ast::Ternary {
                        condition: lhs,
                        positive,
                        negative,
                    }));
                }
                _ => return Err(()),
            }
        }

        Ok(lhs)
    }

    #[inline]
    pub fn parse_global_scope(&mut self) -> Result<Box<[NodeRef]>> {
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

    #[inline]
    fn parse_decl_body(&mut self) -> Result<Decl> {
        let binding = self.parse_binding()?;
        let value = if self.lex.token == Token::Equal {
            self.lex.next();
            self.parse_expr(1)?
        } else {
            Node::empty()
        };
        // TODO: support commas separating declarations
        Ok(Decl { binding, value })
    }

    fn parse_stmt(&mut self, state: ScopeState) -> Result<NodeRef> {
        Ok(match self.lex.token {
            Token::Ident => {
                let source_ref = self.take();

                if self.lex.token == Token::Colon {
                    self.lex.next();

                    if let Token::KeywordLet | Token::KeywordConst = self.lex.token {
                        eprintln!(
                            "Lexical declaration cannot appear in a single-statement context."
                        );
                        return Err(());
                    }

                    let stmt = self.parse_stmt(state)?;
                    self.nodes.insert(Node::Label(ast::Label {
                        name: source_ref,
                        stmt,
                    }))
                } else {
                    // We need to backstep and parse as expression.
                    self.lex.index = source_ref.start as usize;
                    self.lex.next();
                    self.lex.has_newline_before = true;
                    self.parse_expr(1)?
                }
            }
            Token::KeywordVar => {
                self.lex.next();
                let decl = self.parse_decl_body()?;
                self.nodes.insert(Node::VarDecl(decl))
            }
            Token::KeywordLet => {
                self.lex.next();
                let decl = self.parse_decl_body()?;
                self.nodes.insert(Node::LetDecl(decl))
            }
            Token::KeywordConst => {
                self.lex.next();
                let decl = self.parse_decl_body()?;
                self.nodes.insert(Node::ConstDecl(decl))
            }
            Token::KeywordThrow => {
                self.lex.next();
                let value = self.parse_expr(1)?;
                self.nodes.insert(Node::Throw(value))
            }
            Token::KeywordContinue => {
                self.lex.next();
                let label = if !self.lex.has_newline_before && self.lex.token == Token::Ident {
                    let source_ref = self.take();
                    self.nodes.insert(Node::Ident(source_ref))
                } else {
                    Node::empty()
                };
                self.nodes.insert(Node::Continue(label))
            }
            Token::KeywordBreak => {
                self.lex.next();
                let label = if !self.lex.has_newline_before && self.lex.token == Token::Ident {
                    let source_ref = self.take();
                    self.nodes.insert(Node::Ident(source_ref))
                } else {
                    Node::empty()
                };
                self.nodes.insert(Node::Break(label))
            }
            Token::KeywordReturn => 'blk: {
                self.lex.next();

                // We can simply report this later.
                if !state.is_function {
                    return Err(());
                }

                if self.lex.has_newline_before {
                    break 'blk self.nodes.insert(Node::Return(Node::empty()));
                }

                if self.lex.token == Token::Semi {
                    self.lex.next();
                    break 'blk self.nodes.insert(Node::Return(Node::empty()));
                }

                let value = self.parse_expr(1)?;
                self.nodes.insert(Node::Return(value))
            }
            Token::KeywordFor => {
                self.lex.next();
                self.expect(Token::LParen)?;

                let init = match self.lex.token {
                    Token::Semi => Node::empty(),
                    Token::KeywordVar => {
                        self.lex.next();
                        let decl = self.parse_decl_body()?;
                        self.nodes.insert(Node::VarDecl(decl))
                    }
                    Token::KeywordLet => {
                        self.lex.next();
                        let decl = self.parse_decl_body()?;
                        self.nodes.insert(Node::LetDecl(decl))
                    }
                    Token::KeywordConst => {
                        self.lex.next();
                        let decl = self.parse_decl_body()?;
                        self.nodes.insert(Node::ConstDecl(decl))
                    }
                    _ => self.parse_expr(0)?,
                };
                self.expect(Token::Semi)?;

                let condition = if self.lex.token == Token::Semi {
                    Node::empty()
                } else {
                    self.parse_expr(0)?
                };
                self.expect(Token::Semi)?;

                let action = if self.lex.token == Token::RParen {
                    Node::empty()
                } else {
                    self.parse_expr(0)?
                };
                self.expect(Token::RParen)?;

                self.expect(Token::LBrace)?;
                let nodes = self.parse_scope(ScopeState {
                    is_loop: true,
                    ..state
                })?;
                self.expect(Token::RBrace)?;
                self.lex.has_newline_before = true;

                self.nodes.insert(Node::For(ast::For {
                    init,
                    condition,
                    action,
                    nodes,
                }))
            }
            Token::KeywordWhile => {
                self.lex.next();
                self.expect(Token::LParen)?;
                let condition = self.parse_expr(1)?;
                self.expect(Token::RParen)?;
                self.expect(Token::LBrace)?;
                let nodes = self.parse_scope(ScopeState {
                    is_loop: true,
                    ..state
                })?;
                self.expect(Token::RBrace)?;

                self.nodes
                    .insert(Node::While(ast::While { condition, nodes }))
            }
            Token::RBrace | Token::EOF => Node::empty(),
            Token::Semi => {
                self.lex.next();
                self.lex.has_newline_before = true;
                return self.parse_stmt(state);
            }
            _ => return self.parse_expr(1),
        })
    }

    fn parse_scope(&mut self, state: ScopeState) -> Result<Box<[NodeRef]>> {
        let mut scope = Vec::new();
        self.lex.has_newline_before = true;

        loop {
            if self.lex.token == Token::RBrace || self.lex.token == Token::EOF {
                break;
            }

            self.expect_valid_terminator()?;

            let stmt = self.parse_stmt(state)?;
            if stmt == Node::empty() {
                break;
            }
            scope.push(stmt);
        }

        Ok(scope.into_boxed_slice())
    }
}

#[cfg(test)]
mod tests {}
