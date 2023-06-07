use crate::{
    ast::{self, Call, Decl, Node, NodeRef, SourceRef},
    Lexer, Token,
};
use generational_arena::Arena;

pub struct Parser<'a> {
    lex: Lexer<'a>,
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
        let mut lex = Lexer::new(source);
        lex.next();

        let mut arena: Arena<Node> = Arena::new();
        let empty_idx = arena.insert(Node::Empty);
        assert!(
            empty_idx == Node::empty(),
            "The empty index must be placed at 0."
        );

        Self { lex, nodes: arena }
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
            Token::Ident => {
                let source_ref = self.take();
                Ok(self.nodes.insert(Node::Ident(source_ref)))
            }
            Token::LBrack => {
                self.lex.next();
                let mut values = Vec::new();

                loop {
                    match self.lex.token {
                        Token::RBrack => break,
                        Token::Comma => {
                            self.lex.next();
                            values.push(Node::empty());
                        }
                        Token::Spread => {
                            self.lex.next();
                            let value = self.parse_binding()?;
                            values.push(self.nodes.insert(Node::Spread(value)));

                            if self.lex.token != Token::RBrack {
                                eprintln!("Rest element must be last element.");
                                return Err(());
                            }
                            break;
                        }
                        _ => {
                            let binding = self.parse_binding()?;
                            if self.lex.token == Token::Equal {
                                self.lex.next();
                                let default = self.parse_expr(1)?;
                                values.push(self.nodes.insert(Node::Assign(ast::BinaryOp {
                                    lhs: binding,
                                    rhs: default,
                                })));
                            } else {
                                values.push(binding);
                            }

                            if self.lex.token != Token::Comma {
                                break;
                            }
                            self.lex.next();
                        }
                    }
                }

                self.expect(Token::RBrack)?;

                Ok(self.nodes.insert(Node::Array(ast::Array {
                    values: values.into_boxed_slice(),
                })))
            }
            // TODO: implement object destructuring
            _ => {
                eprintln!("Invalid left-hand side in assignment");
                Err(())
            }
        }
    }

    #[inline]
    fn parse_call_args(&mut self) -> Result<Box<[NodeRef]>> {
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

        Ok(args.into_boxed_slice())
    }

    fn validate_binding(&self, lhs: NodeRef) -> Result<()> {
        match self.nodes.get(lhs).unwrap() {
            // TODO: add object destructuring here once implemented
            Node::Array(arr) => {
                for &item in arr.values.iter() {
                    match self.nodes.get(item).unwrap() {
                        Node::Assign(assign) => self.validate_binding(assign.lhs)?,
                        _ => self.validate_binding(item)?,
                    }
                }
            }
            Node::Ident(_) => {}
            _ => {
                eprintln!("Invalid destructuring assignment target");
                return Err(());
            }
        }

        Ok(())
    }

    fn validate_arrow_func_params(&mut self, maybe_group_idx: NodeRef) -> Result<Box<[NodeRef]>> {
        let mut params = Vec::new();
        let mut cur_id = maybe_group_idx;
        let mut has_rest = false;
        loop {
            let param = match self.nodes.get(cur_id).unwrap() {
                Node::Group(inner) => {
                    cur_id = inner.lhs;
                    inner.rhs
                }
                _ => {
                    let old_id = cur_id;
                    cur_id = Node::empty();
                    old_id
                }
            };

            match self.nodes.get(param).unwrap() {
                Node::Spread(value) => {
                    let value = self.nodes.get(*value).unwrap();

                    if let Node::Ident(_) = value {
                    } else {
                        eprintln!("Invalid arrow function parameters");
                        return Err(());
                    }

                    if has_rest {
                        eprintln!("Rest parameter must be last formal parameter");
                        return Err(());
                    }

                    has_rest = true;
                }
                Node::Ident(_) | Node::Assign(_) => {}
                _ => {
                    eprintln!("Invalid arrow function parameters");
                    return Err(());
                }
            }

            params.push(param);

            if cur_id == Node::empty() {
                break;
            }
        }

        params.reverse();
        Ok(params.into_boxed_slice())
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

                if self.lex.token == Token::RParen {
                    self.lex.next();
                    self.expect(Token::Arrow)?;

                    let nodes = if self.lex.token == Token::LBrace {
                        self.lex.next();
                        let nodes = self.parse_scope(ScopeState {
                            is_loop: false,
                            is_function: true,
                        })?;
                        self.expect(Token::RBrace)?;
                        nodes
                    } else {
                        Box::new([self.parse_expr(1)?])
                    };

                    self.nodes.insert(ast::Node::ArrowFunction(ast::Function {
                        name: Node::empty(),
                        params: Box::new([]),
                        scope: nodes,
                    }))
                } else {
                    let node = self.parse_expr(0)?;
                    self.expect(Token::RParen)?;
                    self.nodes.insert(Node::Paren(node))
                }
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
            Token::KeywordNew => 'blk: {
                // TODO: this does not currently work for `new Foo.Bar()`
                self.lex.next();
                let callee = self.parse_expr(17)?;

                if self.lex.token != Token::LParen {
                    break 'blk self.nodes.insert(Node::New(callee));
                }
                self.lex.next();

                let args = self.parse_call_args()?;
                self.nodes.insert(Node::NewCall(ast::Call { callee, args }))
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
            Token::LBrace => {
                self.lex.next();
                let mut entries = Vec::new();

                loop {
                    match self.lex.token {
                        Token::Spread => {
                            self.lex.next();
                            let value = self.parse_expr(1)?;
                            entries.push(self.nodes.insert(Node::Spread(value)));
                        }
                        Token::String => {
                            let name = {
                                let full = self.take();
                                SourceRef {
                                    start: full.start + 1,
                                    end: full.end - 1,
                                }
                            };
                            self.expect(Token::Colon)?;
                            let value = self.parse_expr(1)?;
                            let name = self.nodes.insert(Node::String(name));
                            entries.push(
                                self.nodes
                                    .insert(Node::Entry(ast::ObjectEntry { name, value })),
                            );
                        }
                        Token::Ident => {
                            let name = self.take();
                            let value = if let Token::RBrace | Token::Comma = self.lex.token {
                                Node::empty()
                            } else {
                                self.expect(Token::Colon)?;
                                self.parse_expr(1)?
                            };
                            let name = self.nodes.insert(Node::String(name));
                            entries.push(
                                self.nodes
                                    .insert(Node::Entry(ast::ObjectEntry { name, value })),
                            )
                        }
                        Token::LBrack => {
                            self.lex.next();
                            let name = self.parse_expr(1)?;
                            self.expect(Token::RBrack)?;
                            self.expect(Token::Colon)?;
                            let value = self.parse_expr(1)?;
                            entries.push(
                                self.nodes
                                    .insert(Node::Entry(ast::ObjectEntry { name, value })),
                            );
                        }
                        _ => break,
                    }

                    if self.lex.token != Token::Comma {
                        break;
                    }
                    self.lex.next();
                }

                self.expect(Token::RBrace)?;
                self.nodes.insert(Node::Object(ast::Object {
                    entries: entries.into_boxed_slice(),
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
                Token::Arrow => {
                    self.lex.next();

                    let Some(lhs_value) = self.nodes.get(lhs) else {
                        unreachable!();
                    };

                    match lhs_value {
                        Node::Ident(_) => {
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
                        Node::Paren(group_idx) => {
                            let params = self.validate_arrow_func_params(*group_idx)?;

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
                                params,
                                scope,
                            }));
                        }
                        _ => {
                            eprintln!("Invalid arrow function parameters");
                            return Err(());
                        }
                    }
                }
                Token::Equal => {
                    self.lex.next();
                    // TODO: validate lhs as assignment expr
                    self.validate_binding(lhs)?;
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
                    self.lex.next();

                    // foo?.()

                    let args = self.parse_call_args()?;
                    lhs = self
                        .nodes
                        .insert(Node::OptionalCall(Call { callee: lhs, args }));
                }
                Token::Comma => {
                    self.lex.next();
                    if self.lex.token == Token::Spread {
                        self.lex.next();
                        let value = self.parse_expr(power)?;
                        let spread = self.nodes.insert(Node::Spread(value));
                        lhs = self
                            .nodes
                            .insert(Node::Group(ast::BinaryOp { lhs, rhs: spread }));
                    } else {
                        let rhs = self.parse_expr(power)?;
                        lhs = self.nodes.insert(Node::Group(ast::BinaryOp { lhs, rhs }));
                    }
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
                    let args = self.parse_call_args()?;
                    lhs = self.nodes.insert(Node::Call(Call { callee: lhs, args }));
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

    pub fn parse_global_scope(&mut self) -> Result<Box<[NodeRef]>> {
        let nodes = self.parse_scope(ScopeState {
            is_function: false,
            is_loop: false,
        })?;

        if self.lex.token != Token::EOF {
            eprintln!("Expected statement, found {:?}.", self.lex.token);
            return Err(());
        }

        Ok(nodes)
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
            Token::LBrace => {
                self.lex.next();
                let nodes = self.parse_scope(state)?;
                self.expect(Token::RBrace)?;
                self.nodes.insert(Node::Block(ast::Block { nodes }))
            }
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
                    self.lex.start = source_ref.start as usize;
                    self.lex.index = source_ref.end as usize;
                    self.lex.token = Token::Ident;
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
            Token::KeywordIf => {
                let head = self.nodes.insert(Node::Empty);
                let mut tail = head;

                loop {
                    if self.lex.token != Token::KeywordIf {
                        break;
                    }
                    self.lex.next();
                    self.expect(Token::LParen)?;
                    let condition = self.parse_expr(1)?;
                    self.expect(Token::RParen)?;
                    self.expect(Token::LBrace)?;
                    let nodes = self.parse_scope(state)?;
                    self.expect(Token::RBrace)?;

                    let new_tail = self.nodes.insert(Node::Empty);
                    self.nodes[tail] = Node::If(ast::If {
                        condition,
                        nodes,
                        next: new_tail,
                    });
                    tail = new_tail;

                    if self.lex.token != Token::KeywordElse {
                        break;
                    }
                    self.lex.next();

                    if self.lex.token == Token::LBrace {
                        self.lex.next();
                        let nodes = self.parse_scope(state)?;
                        self.expect(Token::RBrace)?;
                        self.nodes[tail] = Node::Else(ast::Else { nodes });
                        break;
                    }
                }

                self.lex.has_newline_before = true;
                head
            }
            Token::KeywordSwitch => {
                self.lex.next();
                self.expect(Token::LParen)?;
                let expr = self.parse_expr(1)?;
                self.expect(Token::RParen)?;
                self.expect(Token::LBrace)?;

                let mut cases = Vec::new();
                let mut seen_default = false;
                loop {
                    let value = match self.lex.token {
                        Token::KeywordDefault => {
                            self.lex.next();
                            if seen_default {
                                eprintln!("More than one default clause in switch statement");
                                return Err(());
                            }
                            seen_default = true;
                            Node::empty()
                        }
                        Token::KeywordCase => {
                            self.lex.next();
                            self.parse_expr(1)?
                        }
                        _ => break,
                    };

                    self.expect(Token::Colon)?;
                    let nodes = self.parse_scope(state)?;
                    cases.push(ast::Case { value, nodes });
                }

                self.lex.has_newline_before = true;
                self.expect(Token::RBrace)?;

                self.nodes.insert(Node::Switch(ast::Switch {
                    expr,
                    cases: cases.into_boxed_slice(),
                }))
            }
            Token::RBrace | Token::KeywordDefault | Token::KeywordCase | Token::EOF => {
                Node::empty()
            }
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
            if let Token::RBrace | Token::KeywordCase | Token::KeywordDefault | Token::EOF =
                self.lex.token
            {
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
