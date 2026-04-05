//! Lower AST → bytecode (MVP; no MIR).

use nexa_ast::{BinOp, Block, Expr, FnDef, Item, Program, Stmt, TypeExpr};
use nexa_errors::Diagnostic;
use std::collections::HashMap;

use crate::{Chunk, Op, Program as VmProgram};

pub fn compile(program: &Program) -> Result<VmProgram, Diagnostic> {
    let mut funcs: Vec<&FnDef> = Vec::new();
    for Item::Fn(f) in &program.items {
        funcs.push(f);
    }

    let indices: HashMap<String, u16> = funcs
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.clone(), i as u16))
        .collect();

    let Some(entry) = indices.get("main").copied().map(|i| i as usize) else {
        return Err(Diagnostic::new("missing `main` (should be caught earlier)"));
    };

    let mut chunks = Vec::new();
    let mut names = Vec::new();

    for f in &funcs {
        names.push(f.name.clone());
        let mut c = Chunk::default();
        let mut env: HashMap<String, u8> = HashMap::new();
        for (i, p) in f.params.iter().enumerate() {
            if i > u8::MAX as usize {
                return Err(Diagnostic::new("too many parameters (MVP limit 256)"));
            }
            env.insert(p.name.clone(), i as u8);
        }
        compile_block(&mut c, &f.body, &env, &indices)?;
        let ends_return = matches!(f.body.stmts.last(), Some(Stmt::Return { .. }));
        if f.name == "main" {
            c.ops.push(Op::Halt);
        } else if !ends_return {
            let void_ret = f
                .ret_ty
                .as_ref()
                .map(|t| matches!(t, TypeExpr::Void))
                .unwrap_or(true);
            if void_ret {
                c.ops.push(Op::PushUnit);
            }
            c.ops.push(Op::Ret);
        }
        chunks.push(c);
    }

    Ok(VmProgram {
        chunks,
        entry,
        func_names: names,
    })
}

fn compile_block(
    chunk: &mut Chunk,
    block: &Block,
    env: &HashMap<String, u8>,
    funcs: &HashMap<String, u16>,
) -> Result<(), Diagnostic> {
    for stmt in &block.stmts {
        compile_stmt(chunk, stmt, env, funcs)?;
    }
    Ok(())
}

fn compile_stmt(
    chunk: &mut Chunk,
    stmt: &Stmt,
    env: &HashMap<String, u8>,
    funcs: &HashMap<String, u16>,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Expr(e) => {
            compile_expr(chunk, e, env, funcs)?;
            chunk.ops.push(Op::Pop);
            Ok(())
        }
        Stmt::Return { expr, .. } => {
            compile_expr(chunk, expr, env, funcs)?;
            chunk.ops.push(Op::Ret);
            Ok(())
        }
    }
}

fn compile_expr(
    chunk: &mut Chunk,
    expr: &Expr,
    env: &HashMap<String, u8>,
    funcs: &HashMap<String, u16>,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::IntLit(n, _) => {
            chunk.ops.push(Op::PushInt(*n));
        }
        Expr::StringLit(s, _) => {
            let idx = intern_string(chunk, s);
            chunk.ops.push(Op::PushStr(idx));
        }
        Expr::Ident(name, span) => {
            let Some(&slot) = env.get(name) else {
                return Err(Diagnostic::spanned(
                    format!("unknown local `{}` in codegen", name),
                    *span,
                ));
            };
            chunk.ops.push(Op::LoadParam(slot));
        }
        Expr::Binary {
            op,
            lhs,
            rhs,
            ..
        } => {
            compile_expr(chunk, lhs, env, funcs)?;
            compile_expr(chunk, rhs, env, funcs)?;
            match op {
                BinOp::Add => chunk.ops.push(Op::AddI64),
                BinOp::Sub => chunk.ops.push(Op::SubI64),
                BinOp::Mul => chunk.ops.push(Op::MulI64),
                BinOp::Div => chunk.ops.push(Op::DivI64),
            }
        }
        Expr::Call {
            callee,
            args,
            span,
        } => {
            if callee == "print" {
                if args.len() != 1 {
                    return Err(Diagnostic::spanned("print arity mismatch", *span));
                }
                compile_expr(chunk, &args[0], env, funcs)?;
                chunk.ops.push(Op::BuiltinPrint);
                chunk.ops.push(Op::PushUnit);
                return Ok(());
            }
            let Some(&fid) = funcs.get(callee) else {
                return Err(Diagnostic::spanned(
                    format!("unknown function `{}` in codegen", callee),
                    *span,
                ));
            };
            for a in args {
                compile_expr(chunk, a, env, funcs)?;
            }
            chunk.ops.push(Op::Call {
                func: fid,
                argc: args.len().min(255) as u8,
            });
        }
    }
    Ok(())
}

fn intern_string(chunk: &mut Chunk, s: &str) -> usize {
    if let Some(i) = chunk.strings.iter().position(|x| x == s) {
        return i;
    }
    let i = chunk.strings.len();
    chunk.strings.push(s.to_string());
    i
}
