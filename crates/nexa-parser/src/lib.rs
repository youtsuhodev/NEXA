//! Parser: tokens → AST.

use nexa_ast::{BinOp, Block, Expr, FnDef, Item, Param, Program, Stmt, TypeExpr};
use nexa_errors::{Diagnostic, Diagnostics, Span};
use nexa_lexer::{Token, TokenKind};

pub fn parse(tokens: Vec<Token>, diags: &mut Diagnostics) -> Option<Program> {
    let mut p = Parser { tokens, idx: 0 };
    match p.parse_program() {
        Ok(prog) => Some(prog),
        Err(d) => {
            diags.push(d);
            None
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut items = Vec::new();
        while !self.at_eof() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, Diagnostic> {
        let fn_span = self.expect_fn()?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                params.push(self.parse_param()?);
                if self.eat(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }
        self.expect(TokenKind::RParen)?;
        let ret_ty = if self.eat(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = Span::cover(fn_span, body.span);
        Ok(Item::Fn(FnDef {
            name,
            params,
            ret_ty,
            body,
            span,
        }))
    }

    fn parse_param(&mut self) -> Result<Param, Diagnostic> {
        let (name, name_span) = self.expect_ident_spanned()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(Param {
            name,
            ty,
            span: name_span,
        })
    }

    fn parse_type(&mut self) -> Result<TypeExpr, Diagnostic> {
        match self.bump() {
            Some(t) => match t.kind {
                TokenKind::Int => Ok(TypeExpr::Int),
                TokenKind::String => Ok(TypeExpr::String),
                TokenKind::Void => Ok(TypeExpr::Void),
                _ => Err(Diagnostic::spanned(
                    format!("expected type Int, String, or Void, found {:?}", t.kind),
                    t.span,
                )),
            },
            None => Err(Diagnostic::new("expected type, found end of file")),
        }
    }

    fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let l = self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_eof() {
            stmts.push(self.parse_stmt()?);
        }
        let r = self.expect(TokenKind::RBrace)?;
        Ok(Block {
            stmts,
            span: Span::cover(l, r),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&TokenKind::Return) {
            let r = self.bump().unwrap().span;
            let expr = self.parse_expr()?;
            let semi = self.expect(TokenKind::Semicolon)?;
            return Ok(Stmt::Return {
                expr,
                span: Span::cover(r, semi),
            });
        }
        let expr = self.parse_expr()?;
        let _semi = self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Expr(expr))
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = if self.check(&TokenKind::Plus) {
                Some(BinOp::Add)
            } else if self.check(&TokenKind::Minus) {
                Some(BinOp::Sub)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_multiplicative()?;
            let span = Span::cover(lhs.span(), rhs.span());
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_primary()?;
        loop {
            let op = if self.check(&TokenKind::Star) {
                Some(BinOp::Mul)
            } else if self.check(&TokenKind::Slash) {
                Some(BinOp::Div)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_primary()?;
            let span = Span::cover(lhs.span(), rhs.span());
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let Some(t) = self.peek().cloned() else {
            return Err(Diagnostic::new("unexpected end of file in expression"));
        };
        match t.kind {
            TokenKind::IntLit(n) => {
                let span = t.span;
                self.bump();
                Ok(Expr::IntLit(n, span))
            }
            TokenKind::StringLit(s) => {
                let span = t.span;
                self.bump();
                Ok(Expr::StringLit(s, span))
            }
            TokenKind::Ident(name) => {
                self.bump();
                if self.check(&TokenKind::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.eat(&TokenKind::Comma) {
                                continue;
                            }
                            break;
                        }
                    }
                    let close = self.expect(TokenKind::RParen)?;
                    Ok(Expr::Call {
                        callee: name,
                        args,
                        span: Span::cover(t.span, close),
                    })
                } else {
                    Ok(Expr::Ident(name, t.span))
                }
            }
            TokenKind::LParen => {
                self.bump();
                let inner = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(inner)
            }
            _ => Err(Diagnostic::spanned(
                format!("unexpected token in expression: {:?}", t.kind),
                t.span,
            )),
        }
    }

    fn expect_fn(&mut self) -> Result<Span, Diagnostic> {
        match self.bump() {
            Some(t) if matches!(t.kind, TokenKind::Fn) => Ok(t.span),
            Some(t) => Err(Diagnostic::spanned(
                format!("expected `fn`, found {:?}", t.kind),
                t.span,
            )),
            None => Err(Diagnostic::new("expected `fn`, found end of file")),
        }
    }

    fn expect_ident(&mut self) -> Result<String, Diagnostic> {
        self.expect_ident_spanned().map(|(s, _)| s)
    }

    fn expect_ident_spanned(&mut self) -> Result<(String, Span), Diagnostic> {
        match self.bump() {
            Some(t) => match t.kind {
                TokenKind::Ident(s) => Ok((s, t.span)),
                _ => Err(Diagnostic::spanned(
                    format!("expected identifier, found {:?}", t.kind),
                    t.span,
                )),
            },
            None => Err(Diagnostic::new("expected identifier, found end of file")),
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Span, Diagnostic> {
        match self.bump() {
            Some(t) if token_eq(&t.kind, &kind) => Ok(t.span),
            Some(t) => Err(Diagnostic::spanned(
                format!("expected {:?}, found {:?}", kind, t.kind),
                t.span,
            )),
            None => Err(Diagnostic::new(format!(
                "expected {:?}, found end of file",
                kind
            ))),
        }
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek()
            .map(|t| token_eq(&t.kind, kind))
            .unwrap_or(false)
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Eof))
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.idx)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.idx).cloned()?;
        self.idx += 1;
        Some(t)
    }
}

fn token_eq(a: &TokenKind, b: &TokenKind) -> bool {
    match (a, b) {
        (TokenKind::Fn, TokenKind::Fn) => true,
        (TokenKind::Return, TokenKind::Return) => true,
        (TokenKind::Int, TokenKind::Int) => true,
        (TokenKind::String, TokenKind::String) => true,
        (TokenKind::Void, TokenKind::Void) => true,
        (TokenKind::Plus, TokenKind::Plus) => true,
        (TokenKind::Minus, TokenKind::Minus) => true,
        (TokenKind::Star, TokenKind::Star) => true,
        (TokenKind::Slash, TokenKind::Slash) => true,
        (TokenKind::LParen, TokenKind::LParen) => true,
        (TokenKind::RParen, TokenKind::RParen) => true,
        (TokenKind::LBrace, TokenKind::LBrace) => true,
        (TokenKind::RBrace, TokenKind::RBrace) => true,
        (TokenKind::Comma, TokenKind::Comma) => true,
        (TokenKind::Semicolon, TokenKind::Semicolon) => true,
        (TokenKind::Colon, TokenKind::Colon) => true,
        (TokenKind::Arrow, TokenKind::Arrow) => true,
        (TokenKind::Eof, TokenKind::Eof) => true,
        (TokenKind::Ident(x), TokenKind::Ident(y)) => x == y,
        (TokenKind::IntLit(x), TokenKind::IntLit(y)) => x == y,
        (TokenKind::StringLit(x), TokenKind::StringLit(y)) => x == y,
        _ => false,
    }
}

trait ExprSpan {
    fn span(&self) -> Span;
}

impl ExprSpan for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s) => *s,
            Expr::StringLit(_, s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::Binary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
        }
    }
}
