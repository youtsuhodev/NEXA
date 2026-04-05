use clap::{Parser, Subcommand};
use nexa_errors::{format_diagnostic, Diagnostics};
use nexa_lexer::Lexer;
use nexa_parser::parse;
use nexa_resolve::resolve;
use nexa_session::SourceFile;
use nexa_types::check_program;
use nexa_vm::Vm;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "nexa", version, about = "NEXA (.nxa) compiler CLI (MVP)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse, type-check, and run on the VM.
    Run { path: PathBuf },
    /// Parse and type-check only.
    Check { path: PathBuf },
    /// Placeholder: native codegen is not implemented yet.
    Build { path: PathBuf },
    /// Placeholder: formatter not implemented yet.
    Fmt { path: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Run { path } => match drive(&path, true) {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE,
        },
        Command::Check { path } => match drive(&path, false) {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE,
        },
        Command::Build { path } => {
            eprintln!(
                "nexa build: not implemented in MVP ({}). Use `nexa run`.",
                path.display()
            );
            ExitCode::FAILURE
        }
        Command::Fmt { path } => {
            eprintln!(
                "nexa fmt: not implemented in MVP ({}).",
                path.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn drive(path: &PathBuf, run: bool) -> Result<(), ()> {
    let mut diags = Diagnostics::default();
    let file = match SourceFile::load(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}", path.display(), e);
            return Err(());
        }
    };

    let tokens = match Lexer::new(&file.contents).tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: lex error: {}", path.display(), e);
            return Err(());
        }
    };

    let Some(ast) = parse(tokens, &mut diags) else {
        print_diags(&file.path, &file.contents, &diags);
        return Err(());
    };

    resolve(&ast);

    if !check_program(&ast, &mut diags) || !diags.is_empty() {
        print_diags(&file.path, &file.contents, &diags);
        return Err(());
    }

    let program = match nexa_vm::Program::from_ast(&ast) {
        Ok(p) => p,
        Err(d) => {
            diags.push(d);
            print_diags(&file.path, &file.contents, &diags);
            return Err(());
        }
    };

    if !run {
        return Ok(());
    }

    let mut vm = Vm::new(program);
    if let Err(d) = vm.run() {
        diags.push(d);
        print_diags(&file.path, &file.contents, &diags);
        return Err(());
    }

    Ok(())
}

fn print_diags(path: &std::path::Path, source: &str, diags: &Diagnostics) {
    for d in diags.iter() {
        eprint!("{}", format_diagnostic(path, source, d));
    }
}
