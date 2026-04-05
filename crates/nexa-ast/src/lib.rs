//! Abstract syntax tree for NEXA (.nxa).

use nexa_errors::Span;

#[derive(Clone, Debug)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Fn(FnDef),
}

#[derive(Clone, Debug)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<TypeExpr>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpr {
    Int,
    String,
    Void,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Expr(Expr),
    Return { expr: Expr, span: Span },
}

#[derive(Clone, Debug)]
pub enum Expr {
    IntLit(i64, Span),
    StringLit(String, Span),
    Ident(String, Span),
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s) => *s,
            Expr::StringLit(_, s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::Binary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
        }
    }
}
