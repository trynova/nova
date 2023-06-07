pub type NodeRef = generational_arena::Index;

#[derive(Debug, Clone)]
pub struct SourceRef {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug)]
pub struct Decl {
    pub binding: NodeRef,
    // May be empty.
    pub value: NodeRef,
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
pub struct Ternary {
    pub condition: NodeRef,
    pub positive: NodeRef,
    pub negative: NodeRef,
}

#[derive(Debug)]
pub struct For {
    pub init: NodeRef,
    pub condition: NodeRef,
    pub action: NodeRef,
    pub nodes: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct While {
    pub condition: NodeRef,
    pub nodes: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct Label {
    pub name: SourceRef,
    pub stmt: NodeRef,
}

#[derive(Debug)]
pub struct If {
    pub condition: NodeRef,
    pub nodes: Box<[NodeRef]>,
    /// [`Node::If`] or [`Node::Else`] or empty
    pub next: NodeRef,
}

#[derive(Debug)]
pub struct Else {
    pub nodes: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct Switch {
    pub expr: NodeRef,
    pub cases: Box<[Case]>,
}

#[derive(Debug)]
pub struct Case {
    /// Empty if default case
    pub value: NodeRef,
    pub nodes: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct Block {
    pub nodes: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct Object {
    /// [`Node::ObjectEntry`] or [`Node::Spread`]
    pub entries: Box<[NodeRef]>,
}

#[derive(Debug)]
pub struct ObjectEntry {
    /// [`Node::String`] or any other expression if computed. Idents are
    /// normalized into strings due to ambiguity between binding idents or the
    /// property name.
    pub name: NodeRef,
    /// Empty when using shorthand `{ foo }`.
    pub value: NodeRef,
}

#[derive(Debug)]
pub struct Try {
    pub nodes: Box<[NodeRef]>,
    /// [`Node::Catch`] or empty.
    pub catch: NodeRef,
    pub finally: Option<Box<[NodeRef]>>,
}

#[derive(Debug)]
pub struct Catch {
    /// A binding or empty.
    pub capture: NodeRef,
    pub nodes: Box<[NodeRef]>,
}

#[derive(Debug)]
pub enum Node {
    /// Do not construct manually. Obtain a [`NodeRef`] with [`Node::empty()`].
    Empty,
    VarDecl(Decl),
    LetDecl(Decl),
    ConstDecl(Decl),
    VarDeclGroup(Box<[NodeRef]>),
    LetDeclGroup(Box<[NodeRef]>),
    ConstDeclGroup(Box<[NodeRef]>),
    True(SourceRef),
    False(SourceRef),
    Null(SourceRef),
    String(SourceRef),
    Number(SourceRef),
    Decl(Decl),
    Ident(SourceRef),
    Assign(BinaryOp),
    AddAssign(BinaryOp),
    SubAssign(BinaryOp),
    MulAssign(BinaryOp),
    ModAssign(BinaryOp),
    DivAssign(BinaryOp),
    PowAssign(BinaryOp),
    Pow(BinaryOp),
    ShiftLeftAssign(BinaryOp),
    ShiftLeft(BinaryOp),
    ShiftRightAssign(BinaryOp),
    ShiftRight(BinaryOp),
    UShiftRightAssign(BinaryOp),
    UShiftRight(BinaryOp),
    BAndAssign(BinaryOp),
    BAnd(BinaryOp),
    BOrAssign(BinaryOp),
    BOr(BinaryOp),
    XorAssign(BinaryOp),
    Xor(BinaryOp),
    AndAssign(BinaryOp),
    And(BinaryOp),
    OrAssign(BinaryOp),
    Or(BinaryOp),
    NullishAssign(BinaryOp),
    Nullish(BinaryOp),
    Add(BinaryOp),
    Sub(BinaryOp),
    Mul(BinaryOp),
    Mod(BinaryOp),
    Div(BinaryOp),
    Lt(BinaryOp),
    Gt(BinaryOp),
    Lte(BinaryOp),
    Gte(BinaryOp),
    Equality(BinaryOp),
    StrictEquality(BinaryOp),
    Inequality(BinaryOp),
    StrictInequality(BinaryOp),
    Member(BinaryOp),
    /// a?.b
    OptionalChain(BinaryOp),
    OptionalCall(Call),
    Ternary(Ternary),
    Array(Array),
    Call(Call),
    NewCall(Call),
    New(NodeRef),
    Index(Index),
    Paren(NodeRef),
    ParenList(Box<[NodeRef]>),
    Group(BinaryOp),
    /// May be empty.
    Return(NodeRef),
    Label(Label),
    Throw(NodeRef),
    Delete(NodeRef),
    TypeOf(NodeRef),
    InstanceOf(NodeRef),
    Void(NodeRef),
    /// [`Node::Ident`] for the label or empty
    Continue(NodeRef),
    /// [`Node::Ident`] for the label or empty
    Break(NodeRef),
    Spread(NodeRef),
    Param(Param),
    Function(Function),
    AsyncFunction(Function),
    ArrowFunction(Function),
    For(For),
    While(While),
    If(If),
    Else(Else),
    Switch(Switch),
    Block(Block),
    Object(Object),
    Entry(ObjectEntry),
    Try(Try),
    Catch(Catch),
}

impl Node {
    /// A reference to the `Node::Empty` node in the arena.
    #[inline]
    pub fn empty() -> NodeRef {
        // This is ensured to be at index 0 in the parser.
        generational_arena::Index::from_raw_parts(0, 0)
    }
}
