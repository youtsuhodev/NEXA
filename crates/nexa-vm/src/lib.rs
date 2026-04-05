//! Stack VM and bytecode for NEXA (MVP).

mod compile;

pub use compile::compile;

use nexa_errors::Diagnostic;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
    Unit,
}

#[derive(Clone, Debug)]
pub enum Op {
    PushInt(i64),
    PushStr(usize),
    PushUnit,
    AddI64,
    SubI64,
    MulI64,
    DivI64,
    Pop,
    /// Load function parameter by index (0 = first param).
    LoadParam(u8),
    BuiltinPrint,
    Call { func: u16, argc: u8 },
    Ret,
    Halt,
}

#[derive(Clone, Debug, Default)]
pub struct Chunk {
    pub ops: Vec<Op>,
    pub strings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub chunks: Vec<Chunk>,
    /// Index of `main` in `chunks`.
    pub entry: usize,
    pub func_names: Vec<String>,
}

pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<Frame>,
    program: Program,
}

#[derive(Clone, Debug)]
struct Frame {
    func: usize,
    ip: usize,
    /// Slot for each parameter (MVP: params only).
    locals: Vec<Value>,
}

impl Vm {
    pub fn new(program: Program) -> Self {
        Self {
            stack: Vec::new(),
            frames: Vec::new(),
            program,
        }
    }

    pub fn run(&mut self) -> Result<(), Diagnostic> {
        self.frames.push(Frame {
            func: self.program.entry,
            ip: 0,
            locals: Vec::new(),
        });

        loop {
            let Some(frame) = self.frames.last_mut() else {
                return Ok(());
            };
            let chunk = &self.program.chunks[frame.func];
            let Some(op) = chunk.ops.get(frame.ip).cloned() else {
                return Err(Diagnostic::new(format!(
                    "instruction pointer out of bounds in function {}",
                    self.program.func_names.get(frame.func).map(String::as_str).unwrap_or("?")
                )));
            };
            frame.ip += 1;

            match op {
                Op::PushInt(n) => self.stack.push(Value::Int(n)),
                Op::PushUnit => self.stack.push(Value::Unit),
                Op::PushStr(i) => {
                    let s = chunk
                        .strings
                        .get(i)
                        .cloned()
                        .ok_or_else(|| Diagnostic::new("invalid string constant index"))?;
                    self.stack.push(Value::Str(s));
                }
                Op::AddI64 => self.binop_i64(|a, b| a + b)?,
                Op::SubI64 => self.binop_i64(|a, b| a - b)?,
                Op::MulI64 => self.binop_i64(|a, b| a * b)?,
                Op::DivI64 => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    if b == 0 {
                        return Err(Diagnostic::new("division by zero at runtime"));
                    }
                    self.stack.push(Value::Int(a / b));
                }
                Op::Pop => {
                    self.stack.pop();
                }
                Op::LoadParam(i) => {
                    let v = frame
                        .locals
                        .get(i as usize)
                        .cloned()
                        .ok_or_else(|| Diagnostic::new("invalid local index"))?;
                    self.stack.push(v);
                }
                Op::BuiltinPrint => {
                    let v = self.stack.pop().ok_or_else(|| Diagnostic::new("stack underflow"))?;
                    match v {
                        Value::Str(s) => println!("{s}"),
                        other => {
                            return Err(Diagnostic::new(format!(
                                "print expected string, got {:?}",
                                other
                            )))
                        }
                    }
                }
                Op::Call { func, argc } => {
                    let argc = argc as usize;
                    if self.stack.len() < argc {
                        return Err(Diagnostic::new("stack underflow at call"));
                    }
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        args.push(
                            self.stack
                                .pop()
                                .ok_or_else(|| Diagnostic::new("stack underflow"))?,
                        );
                    }
                    args.reverse();
                    self.frames.push(Frame {
                        func: func as usize,
                        ip: 0,
                        locals: args,
                    });
                }
                Op::Ret => {
                    let ret = self.stack.pop();
                    let Some(_finished) = self.frames.pop() else {
                        return Err(Diagnostic::new("return with no active frame"));
                    };
                    if let Some(v) = ret {
                        if v != Value::Unit {
                            self.stack.push(v);
                        }
                    }
                    if self.frames.is_empty() {
                        return Ok(());
                    }
                }
                Op::Halt => return Ok(()),
            }
        }
    }

    fn pop_int(&mut self) -> Result<i64, Diagnostic> {
        match self.stack.pop() {
            Some(Value::Int(n)) => Ok(n),
            Some(other) => Err(Diagnostic::new(format!("expected Int, got {:?}", other))),
            None => Err(Diagnostic::new("stack underflow")),
        }
    }

    fn binop_i64(&mut self, f: impl FnOnce(i64, i64) -> i64) -> Result<(), Diagnostic> {
        let b = self.pop_int()?;
        let a = self.pop_int()?;
        self.stack.push(Value::Int(f(a, b)));
        Ok(())
    }
}

impl Program {
    pub fn from_ast(program: &nexa_ast::Program) -> Result<Self, Diagnostic> {
        compile::compile(program)
    }
}
