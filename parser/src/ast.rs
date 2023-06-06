pub type NodeRef = generational_arena::Index;

#[derive(Debug)]
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
    pub args: Box<[NodeRef]>,
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
}
