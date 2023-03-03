use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Keyword {
    Await,
    Async,
    Break,
    Case,
    Catch,
    Class,
    Continue,
    Const,
    // this is a keyword?
    Debugger,
    Default,
    Delete,
    Do,
    Else,
    Export,
    Extends,
    Finally,
    For,
    Function,
    Get,
    If,
    In,
    InstanceOf,
    Import,
    Let,
    New,
    Of,
    Return,
    Set,
    Super,
    Static,
    Switch,
    This,
    Throw,
    Try,
    TypeOf,
    Var,
    Void,
    While,
    Yield,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Reserved {
    False,
    Null,
    True,

    // future reserved words
    Enum,
    Implements,
    Interface,
    Package,
    Private,
    Protected,
    Public,

    // deprecated reserved words
    With,
}