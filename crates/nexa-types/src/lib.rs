//! Minimal type checking (MVP).

use nexa_ast::{BinOp, Expr, FnDef, Item, Program, Stmt, TypeExpr};
use nexa_errors::{Diagnostic, Diagnostics, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Int,
    String,
    Void,
}

impl From<&TypeExpr> for Ty {
    fn from(t: &TypeExpr) -> Self {
        match t {
            TypeExpr::Int => Ty::Int,
            TypeExpr::String => Ty::String,
            TypeExpr::Void => Ty::Void,
        }
    }
}

pub fn check_program(program: &Program, diags: &mut Diagnostics) -> bool {
    let mut funcs: Vec<&FnDef> = Vec::new();
    for Item::Fn(f) in &program.items {
        funcs.push(f);
    }

    let mut names: std::collections::HashMap<String, Span> = std::collections::HashMap::new();
    for f in &funcs {
        if let Some(prev) = names.insert(f.name.clone(), f.span) {
            diags.push(Diagnostic::spanned(
                format!("duplicate function `{}`", f.name),
                f.span,
            ));
            diags.push(Diagnostic::spanned("previous definition here", prev));
            return false;
        }
    }

    let Some(main_def) = funcs.iter().find(|f| f.name == "main") else {
        diags.error("program must define `fn main()`");
        return false;
    };

    if !main_def.params.is_empty() {
        diags.push(Diagnostic::spanned(
            "`main` must take no parameters (MVP)",
            main_def.span,
        ));
        return false;
    }

    let main_ret = main_def
        .ret_ty
        .as_ref()
        .map(Ty::from)
        .unwrap_or(Ty::Void);

    if main_ret != Ty::Void {
        diags.push(Diagnostic::spanned(
            "`main` must return Void (MVP)",
            main_def.span,
        ));
        return false;
    }

    let mut ok = true;
    for &f in &funcs {
        if !check_fn(f, &funcs, diags) {
            ok = false;
        }
    }
    ok
}

fn check_fn(f: &FnDef, all: &[&FnDef], diags: &mut Diagnostics) -> bool {
    let ret = f.ret_ty.as_ref().map(Ty::from).unwrap_or(Ty::Void);
    let mut env: std::collections::HashMap<String, Ty> = std::collections::HashMap::new();
    for p in &f.params {
        env.insert(p.name.clone(), Ty::from(&p.ty));
    }

    let mut ok = true;
    for stmt in &f.body.stmts {
        if !check_stmt(stmt, ret, &env, all, diags) {
            ok = false;
        }
    }
    ok
}

fn check_stmt(
    stmt: &Stmt,
    fn_ret: Ty,
    env: &std::collections::HashMap<String, Ty>,
    funcs: &[&FnDef],
    diags: &mut Diagnostics,
) -> bool {
    match stmt {
        Stmt::Return { expr, span } => {
            let t = match check_expr(expr, env, funcs, diags) {
                Some(t) => t,
                None => return false,
            };
            if t != fn_ret {
                diags.push(Diagnostic::spanned(
                    format!(
                        "return type mismatch: expected {:?}, found {:?}",
                        fn_ret, t
                    ),
                    *span,
                ));
                return false;
            }
            true
        }
        Stmt::Expr(e) => {
            let Some(t) = check_expr(e, env, funcs, diags) else {
                return false;
            };
            if t != Ty::Void {
                diags.push(Diagnostic::spanned(
                    format!(
                        "expression statement has type {:?}; only Void calls are allowed (MVP)",
                        t
                    ),
                    e.span(),
                ));
                return false;
            }
            true
        }
    }
}

fn check_expr(
    expr: &Expr,
    env: &std::collections::HashMap<String, Ty>,
    funcs: &[&FnDef],
    diags: &mut Diagnostics,
) -> Option<Ty> {
    match expr {
        Expr::IntLit(_, _) => Some(Ty::Int),
        Expr::StringLit(_, _) => Some(Ty::String),
        Expr::Ident(name, span) => match env.get(name) {
            Some(t) => Some(*t),
            None => {
                diags.push(Diagnostic::spanned(
                    format!("unknown variable `{}`", name),
                    *span,
                ));
                None
            }
        },
        Expr::Binary {
            op,
            lhs,
            rhs,
            span,
        } => {
            let lt = check_expr(lhs, env, funcs, diags)?;
            let rt = check_expr(rhs, env, funcs, diags)?;
            if lt != Ty::Int || rt != Ty::Int {
                diags.push(Diagnostic::spanned(
                    format!(
                        "binary operator {:?} requires Int operands (got {:?} and {:?})",
                        op, lt, rt
                    ),
                    *span,
                ));
                return None;
            }
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => Some(Ty::Int),
            }
        }
        Expr::Call {
            callee,
            args,
            span,
        } => check_call(callee, args, *span, env, funcs, diags),
    }
}

fn check_call(
    callee: &str,
    args: &[Expr],
    span: Span,
    env: &std::collections::HashMap<String, Ty>,
    funcs: &[&FnDef],
    diags: &mut Diagnostics,
) -> Option<Ty> {
    if callee == "print" {
        if args.len() != 1 {
            diags.push(Diagnostic::spanned(
                format!("`print` expects 1 argument, got {}", args.len()),
                span,
            ));
            return None;
        }
        let t = check_expr(&args[0], env, funcs, diags)?;
        if t != Ty::String {
            diags.push(Diagnostic::spanned(
                format!("`print` expects String, got {:?}", t),
                span,
            ));
            return None;
        }
        return Some(Ty::Void);
    }

    let Some(def) = funcs.iter().find(|f| f.name == callee) else {
        diags.push(Diagnostic::spanned(
            format!("unknown function `{}`", callee),
            span,
        ));
        return None;
    };
    if args.len() != def.params.len() {
        diags.push(Diagnostic::spanned(
            format!(
                "function `{}` expects {} arguments, got {}",
                callee,
                def.params.len(),
                args.len()
            ),
            span,
        ));
        return None;
    }
    for (a, p) in args.iter().zip(def.params.iter()) {
        let got = check_expr(a, env, funcs, diags)?;
        let exp = Ty::from(&p.ty);
        if got != exp {
            diags.push(Diagnostic::spanned(
                format!(
                    "argument type mismatch for `{}`: expected {:?}, got {:?}",
                    p.name, exp, got
                ),
                span,
            ));
            return None;
        }
    }
    Some(def.ret_ty.as_ref().map(Ty::from).unwrap_or(Ty::Void))
}
