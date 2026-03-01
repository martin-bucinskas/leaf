use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub includes: Vec<String>,
    pub globals: Vec<GlobalVar>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type {
    Int,
    Str,
    Bool,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalVar {
    pub ty: Type,
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVar {
    pub ty: Type,
    pub name: String,
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_ty: Type,
    pub body: Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    LocalVar(LocalVar),
    If {
        condition: Expression,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        condition: Expression,
        block: Block,
    },
    Return(Option<Expression>),
    Assignment {
        name: String,
        value: Expression,
    },
    Expression(Expression),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Literal(Literal),
    Identifier(String),
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Literal {
    Int(i64),
    Str(String),
    Bool(bool),
}
