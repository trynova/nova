pub type NodeRef = generational_arena::Index;

#[derive(Debug, Clone)]
pub struct SourceRef {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug)]
pub enum Decl {
    Ident(SourceRef),
}

#[derive(Debug)]
pub struct BinaryOp {
    pub lhs: NodeRef,
    pub rhs: NodeRef,
}

#[derive(Debug)]
pub struct Call {
    pub callee: NodeRef,
    /// `Node::Param` or `Node::Spread`
    pub args: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct Index {
    pub root: NodeRef,
    pub index: NodeRef,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<SourceRef>,
    /// `Node::Param` or `Node::Spread`
    pub params: Box<[NodeRef]>,
    pub scope: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct Param {
    pub name: NodeRef,
    pub default: Option<NodeRef>,
}

#[derive(Debug)]
pub enum Node {
    LetDecl { decl: NodeRef, value: NodeRef },
    ConstDecl { decl: NodeRef, value: NodeRef },
    String(SourceRef),
    Number(SourceRef),
    Decl(Decl),
    Ident(SourceRef),
    Assign(BinaryOp),
    Add(BinaryOp),
    Sub(BinaryOp),
    Mul(BinaryOp),
    Mod(BinaryOp),
    Div(BinaryOp),
    Call(Call),
    Index(Index),
    Paren(NodeRef),
    Group(BinaryOp),
    Return(Option<NodeRef>),
    Spread(NodeRef),
    Param(Param),
    Function(Function),
    AsyncFunction(Function),
}
