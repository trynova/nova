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
    /// `Node::Ident` or `Node::Empty`
    pub name: NodeRef,
    /// `Node::Param` or `Node::Spread`
    pub params: Box<[NodeRef]>,
    pub scope: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct Param {
    pub name: NodeRef,
    pub default: NodeRef,
}

#[derive(Debug)]
pub struct Array {
    pub values: Box<[NodeRef]>,
}

#[derive(Debug)]
pub enum Node {
    /// Do not construct manually. Achieve a [`NodeRef`] with [`Node::empty()`].
    Empty,
    LetDecl {
        decl: NodeRef,
        value: NodeRef,
    },
    ConstDecl {
        decl: NodeRef,
        value: NodeRef,
    },
    True(SourceRef),
    False(SourceRef),
    Null(SourceRef),
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
    Array(Array),
    Call(Call),
    Index(Index),
    Paren(NodeRef),
    Group(BinaryOp),
    /// May be empty.
    Return(NodeRef),
    Spread(NodeRef),
    Param(Param),
    Function(Function),
    AsyncFunction(Function),
    ArrowFunction(Function),
}

impl Node {
    /// A reference to the `Node::Empty` node in the arena.
    pub fn empty() -> NodeRef {
        // This is ensured to be at index 0 in the parser.
        generational_arena::Index::from_raw_parts(0, 0)
    }
}
