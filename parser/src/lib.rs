use ast::{Call, Decl, Node, NodeRef, SourceRef};
use generational_arena::Arena;
use tokenizer::{Token, TokenStream};

pub mod ast;

pub struct Parser<'a> {
    lex: TokenStream<'a>,
    pub nodes: Arena<Node>,
}

type Result<T> = std::result::Result<T, ()>;

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut stream = TokenStream::new(source);
        stream.next();
        stream.has_newline_before = true;
        Self {
            lex: stream,
            nodes: Arena::new(),
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
            other => {
                eprintln!("{other:?}");
                return Err(());
            }
        };

        loop {
            let power = self.lex.token.lbp();
            if hp >= power || power == 0 {
                break;
            }

            match self.lex.token {
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

                        args.push(self.parse_expr(1)?);

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

    pub fn parse_scope(&mut self) -> Result<Vec<NodeRef>> {
        let mut scope = Vec::new();

        loop {
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
                Token::Semi => self.lex.next(),
                Token::EOF | Token::RBrace => break,
                _ => break,
            }
        }

        Ok(scope)
    }
}

#[cfg(test)]
mod tests {}
