use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::compiler::Compiler;
use crate::context::Context;
use crate::continuation::Continuation;
use crate::env::{Env, EnvFrame};
use crate::instruction::{ApplyArgs, Expr, Formals};
use crate::macro_expander::MacroExpander;
use crate::parser::Parser;
use crate::primitives::{EMBEDDED_INIT_SCM, EMBEDDED_PSYNTAX_PP, PRIMITIVES_SCM};
use crate::scanner::Scanner;
use crate::stdlib::register_standard_library;
use crate::symbol::SymbolTable;
use crate::value::{Closure, Procedure, Value};

#[derive(Debug)]
pub enum Signal {
    Capture {
        proc: Procedure,
        frames: Vec<Context>,
    },
    Invoke {
        k_frames: Rc<Vec<Context>>,
        val: Value,
    },
    Error(String),
}

enum OutputSink {
    Stdout,
    Buffer(Vec<u8>),
}

pub struct Interpreter {
    symbols: Rc<RefCell<SymbolTable>>,
    global_env: Env,
    output: OutputSink,
    current_dir_stack: Vec<PathBuf>,
}

impl Interpreter {
    /// Creates a bare interpreter with native `StandardLibrary` bindings (without `Primitives`,
    /// `psyntax.pp`, or `init.scm` loaded yet).
    pub fn new_bare() -> Self {
        let symbols = Rc::new(RefCell::new(SymbolTable::new()));
        let global_env = EnvFrame::new_root();
        register_standard_library(&mut symbols.borrow_mut(), &global_env);
        Self {
            symbols,
            global_env,
            output: OutputSink::Stdout,
            current_dir_stack: Vec::new(),
        }
    }

    /// Creates a fully initialized Scheme interpreter with `Primitives`, `psyntax.pp`,
    /// and `init.scm` loaded.
    pub fn new() -> Result<Self, String> {
        let mut interp = Self::new_bare();
        interp.load_standard_runtime(None)?;
        Ok(interp)
    }

    /// Creates a fully initialized interpreter that captures `(display ...)` output into
    /// an internal buffer for programmatic inspection and testing.
    pub fn with_captured_output() -> Result<Self, String> {
        let mut interp = Self::new_bare();
        interp.output = OutputSink::Buffer(Vec::new());
        interp.load_standard_runtime(None)?;
        Ok(interp)
    }

    pub fn enable_output_capture(&mut self) {
        self.output = OutputSink::Buffer(Vec::new());
    }

    pub fn take_captured_output(&mut self) -> String {
        match &mut self.output {
            OutputSink::Buffer(buf) => {
                let bytes = std::mem::take(buf);
                String::from_utf8_lossy(&bytes).into_owned()
            }
            OutputSink::Stdout => String::new(),
        }
    }

    pub fn write_output(&mut self, text: &str) -> Result<(), String> {
        match &mut self.output {
            OutputSink::Stdout => {
                let mut out = io::stdout();
                out.write_all(text.as_bytes())
                    .and_then(|_| out.flush())
                    .map_err(|e| format!("Output error: {e}"))
            }
            OutputSink::Buffer(buf) => {
                buf.extend_from_slice(text.as_bytes());
                Ok(())
            }
        }
    }

    pub fn symbols(&self) -> &Rc<RefCell<SymbolTable>> {
        &self.symbols
    }

    pub fn global_env(&self) -> &Env {
        &self.global_env
    }

    /// Loads `Primitives`, `psyntax.pp`, and `init.scm`.
    /// Checks `runtime_dir` (or the executable directory / `SCHEME_LIB_DIR`) on disk first,
    /// falling back to the embedded copies from `lib/`.
    pub fn load_standard_runtime(&mut self, runtime_dir: Option<&Path>) -> Result<(), String> {
        self.eval_str(PRIMITIVES_SCM)?;

        let candidate_dirs: Vec<PathBuf> = {
            let mut dirs = Vec::new();
            if let Some(d) = runtime_dir {
                dirs.push(d.to_path_buf());
            }
            if let Ok(env_dir) = std::env::var("SCHEME_LIB_DIR") {
                dirs.push(PathBuf::from(env_dir));
            }
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    dirs.push(parent.to_path_buf());
                }
            }
            dirs
        };

        if let Some(psyntax_path) = candidate_dirs
            .iter()
            .map(|d| d.join("psyntax.pp"))
            .find(|p| p.is_file())
        {
            self.load_file(&psyntax_path)?;
        } else {
            self.eval_str(EMBEDDED_PSYNTAX_PP)?;
        }

        if let Some(init_path) = candidate_dirs
            .iter()
            .map(|d| d.join("init.scm"))
            .find(|p| p.is_file())
        {
            self.load_file(&init_path)?;
        } else {
            self.eval_str(EMBEDDED_INIT_SCM)?;
        }

        Ok(())
    }

    fn resolve_load_path(&self, raw_path: &str) -> PathBuf {
        let p = Path::new(raw_path);
        if p.exists() {
            return p.to_path_buf();
        }
        if let Some(base) = self.current_dir_stack.last() {
            let joined = base.join(p);
            if joined.exists() {
                return joined;
            }
        }
        p.to_path_buf()
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<Value, String> {
        let env = self.global_env.clone();
        let path_str = path.as_ref().to_string_lossy().into_owned();
        self.load_file_in_env(&path_str, env)
    }

    pub fn load_file_in_env(&mut self, raw_path: &str, env: Env) -> Result<Value, String> {
        let resolved = self.resolve_load_path(raw_path);
        let file = File::open(&resolved)
            .map_err(|e| format!("Could not open file '{}': {e}", resolved.display()))?;
        if let Some(parent) = resolved.canonicalize().ok().and_then(|c| c.parent().map(Path::to_path_buf)) {
            self.current_dir_stack.push(parent);
        } else if let Some(parent) = resolved.parent() {
            self.current_dir_stack.push(parent.to_path_buf());
        } else {
            self.current_dir_stack.push(PathBuf::from("."));
        }

        let reader = BufReader::new(file);
        let res = self.eval_reader_in_env(reader, env);
        self.current_dir_stack.pop();
        res
    }

    pub fn eval_str(&mut self, source: &str) -> Result<Value, String> {
        let env = self.global_env.clone();
        self.eval_reader_in_env(Cursor::new(source.as_bytes()), env)
    }

    pub fn eval_reader_in_env<R: BufRead>(&mut self, reader: R, env: Env) -> Result<Value, String> {
        let mut parser = Parser::new(Scanner::new(reader), self.symbols.clone());
        let mut last = Value::Void;
        while let Some(datum) = parser.read()? {
            last = self.eval_datum_in_env(datum, env.clone())?;
        }
        Ok(last)
    }

    pub fn eval_next<R: BufRead>(&mut self, parser: &mut Parser<R>) -> Result<Option<Value>, String> {
        let Some(datum) = parser.read()? else {
            return Ok(None);
        };
        let env = self.global_env.clone();
        let val = self.eval_datum_in_env(datum, env)?;
        Ok(Some(val))
    }

    pub fn eval_datum_in_env(&mut self, datum: Value, env: Env) -> Result<Value, String> {
        let expanded = MacroExpander::expand(self, datum)?;
        let cache = self.symbols.borrow().cache.clone();
        let compiler = Compiler::new(cache);
        let compiled = compiler.compile(&expanded)?;
        self.evaluate_compiled(compiled, env)
    }

    pub fn call_procedure_top_level(
        &mut self,
        proc: &Procedure,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let env = self.global_env.clone();
        let mut pending_res = self.apply_procedure(proc, args, &env);
        self.drive_trampoline(&mut pending_res)
    }

    pub fn evaluate_compiled(&mut self, expr: Rc<Expr>, env: Env) -> Result<Value, String> {
        let mut pending_res = self.execute(expr, env);
        self.drive_trampoline(&mut pending_res)
    }

    fn drive_trampoline(&mut self, pending_res: &mut Result<Value, Signal>) -> Result<Value, String> {
        loop {
            let err = match pending_res {
                Ok(val) => return Ok(val.clone()),
                Err(sig) => std::mem::replace(sig, Signal::Error(String::new())),
            };

            match err {
                Signal::Capture { proc, mut frames } => {
                    if frames.is_empty() {
                        frames.push(Context::Halt);
                    }
                    let k = Continuation::new(frames);
                    let k_frames = k.frames().clone();
                    *pending_res = self.resume_with_procedure(&k_frames, &proc, k);
                }
                Signal::Invoke { k_frames, val } => {
                    *pending_res = self.resume_with_value(&k_frames, val);
                }
                Signal::Error(msg) => return Err(msg),
            }
        }
    }

    fn resume_with_procedure(
        &mut self,
        k_frames: &[Context],
        proc: &Procedure,
        k: Continuation,
    ) -> Result<Value, Signal> {
        let k_val = Value::Procedure(Procedure::Continuation(k));
        let global = self.global_env.clone();
        let val = match self.apply_procedure(proc, vec![k_val], &global) {
            Ok(v) => v,
            Err(Signal::Capture {
                proc: inner_proc,
                mut frames,
            }) => {
                frames.extend_from_slice(k_frames);
                return Err(Signal::Capture {
                    proc: inner_proc,
                    frames,
                });
            }
            Err(other) => return Err(other),
        };

        self.resume_with_value(k_frames, val)
    }

    fn resume_with_value(&mut self, k_frames: &[Context], mut val: Value) -> Result<Value, Signal> {
        for (idx, ctx) in k_frames.iter().enumerate() {
            match self.invoke_context(ctx, val) {
                Ok(next_val) => val = next_val,
                Err(Signal::Capture { proc, mut frames }) => {
                    frames.extend_from_slice(&k_frames[idx + 1..]);
                    return Err(Signal::Capture { proc, frames });
                }
                Err(other) => return Err(other),
            }
        }
        Ok(val)
    }

    fn unpack_multiple_values(val: Value) -> Vec<Value> {
        match val {
            Value::Values(vs) => vs,
            Value::Void => Vec::new(),
            other => vec![other],
        }
    }

    fn invoke_context(&mut self, ctx: &Context, val: Value) -> Result<Value, Signal> {
        match ctx {
            Context::Halt => Ok(val),
            Context::Cond {
                consequent,
                alternate,
                env,
            } => {
                let branch = if val.is_true() {
                    consequent.clone()
                } else {
                    alternate.clone()
                };
                self.execute(branch, env.clone())
            }
            Context::Assign { id, env } => {
                if EnvFrame::assign(env, id, val) {
                    Ok(Value::Void)
                } else {
                    Err(Signal::Error(format!(
                        "Error: cannot set undefined identifier {id}"
                    )))
                }
            }
            Context::Define { id, env } => {
                EnvFrame::define(env, id.clone(), val);
                Ok(Value::Void)
            }
            Context::ArgEval {
                remaining_left,
                evaluated_rev,
                env,
            } => {
                let mut rev = evaluated_rev.clone();
                rev.push(val);
                let args = self.eval_normal_args_from(remaining_left, rev, env)?;
                Ok(Value::Rib(args))
            }
            Context::Func { rator, env } => {
                let args_rib = match val {
                    Value::Rib(r) => r,
                    other => vec![other],
                };
                let fn_val = match self.execute(rator.clone(), env.clone()) {
                    Ok(v) => v,
                    Err(Signal::Capture { proc, mut frames }) => {
                        frames.push(Context::Apply {
                            args_rib,
                            env: env.clone(),
                        });
                        return Err(Signal::Capture { proc, frames });
                    }
                    Err(other) => return Err(other),
                };
                let proc = match fn_val {
                    Value::Procedure(p) => p,
                    other => {
                        return Err(Signal::Error(format!(
                            "Attempt to apply non-procedure: {other}"
                        )))
                    }
                };
                self.apply_procedure(&proc, args_rib, env)
            }
            Context::Apply { args_rib, env } => {
                let proc = match val {
                    Value::Procedure(p) => p,
                    other => {
                        return Err(Signal::Error(format!(
                            "Attempt to apply non-procedure: {other}"
                        )))
                    }
                };
                self.apply_procedure(&proc, args_rib.clone(), env)
            }
            Context::Seq { next, env } => self.execute(next.clone(), env.clone()),
            Context::MultArg => Ok(Value::Rib(Self::unpack_multiple_values(val))),
        }
    }

    fn eval_normal_args_from(
        &mut self,
        exprs: &[Rc<Expr>],
        mut evaluated_rev: Vec<Value>,
        env: &Env,
    ) -> Result<Vec<Value>, Signal> {
        for idx in (0..exprs.len()).rev() {
            match self.execute(exprs[idx].clone(), env.clone()) {
                Ok(v) => evaluated_rev.push(v),
                Err(Signal::Capture { proc, mut frames }) => {
                    frames.push(Context::ArgEval {
                        remaining_left: exprs[..idx].to_vec(),
                        evaluated_rev,
                        env: env.clone(),
                    });
                    return Err(Signal::Capture { proc, frames });
                }
                Err(other) => return Err(other),
            }
        }
        evaluated_rev.reverse();
        Ok(evaluated_rev)
    }

    fn bind_closure_env(closure: &Closure, args: Vec<Value>) -> Result<Env, Signal> {
        match &closure.formals {
            Formals::Fixed(params) => {
                if args.len() != params.len() {
                    return Err(Signal::Error(format!(
                        "Arity mismatch: expected {} argument(s), got {}",
                        params.len(),
                        args.len()
                    )));
                }
                let cenv =
                    EnvFrame::with_capacity_and_parent(params.len(), Some(closure.env.clone()));
                {
                    let mut frame = cenv.borrow_mut();
                    for (sym, val) in params.iter().zip(args) {
                        frame.define_in_place(sym.clone(), val);
                    }
                }
                Ok(cenv)
            }
            Formals::Variadic { fixed, rest } => {
                if args.len() < fixed.len() {
                    return Err(Signal::Error(format!(
                        "Arity mismatch: expected at least {} argument(s), got {}",
                        fixed.len(),
                        args.len()
                    )));
                }
                let cenv =
                    EnvFrame::with_capacity_and_parent(fixed.len() + 1, Some(closure.env.clone()));
                let mut iter = args.into_iter();
                {
                    let mut frame = cenv.borrow_mut();
                    for sym in fixed {
                        frame.define_in_place(sym.clone(), iter.next().unwrap());
                    }
                    let rest_list = Value::list_from_vec(iter.collect());
                    frame.define_in_place(rest.clone(), rest_list);
                }
                Ok(cenv)
            }
        }
    }

    pub fn apply_procedure(
        &mut self,
        proc: &Procedure,
        args: Vec<Value>,
        env: &Env,
    ) -> Result<Value, Signal> {
        match proc {
            Procedure::Native(native) => (native.func)(self, &args, env),
            Procedure::Closure(closure) => {
                let cenv = Self::bind_closure_env(closure, args)?;
                self.execute(closure.body.clone(), cenv)
            }
            Procedure::Continuation(k) => {
                let val = match args.len() {
                    0 => Value::Void,
                    1 => args.into_iter().next().unwrap(),
                    _ => Value::Values(args),
                };
                Err(Signal::Invoke {
                    k_frames: k.frames().clone(),
                    val,
                })
            }
        }
    }

    pub fn execute(&mut self, mut expr: Rc<Expr>, mut env: Env) -> Result<Value, Signal> {
        loop {
            match expr.as_ref() {
                Expr::Constant(val) => return Ok(val.clone()),
                Expr::Variable(sym) => {
                    return EnvFrame::lookup(&env, sym).ok_or_else(|| {
                        Signal::Error(format!("Error: no value associated with symbol {sym}"))
                    });
                }
                Expr::Conditional {
                    test,
                    consequent,
                    alternate,
                } => {
                    let cond = match self.execute(test.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture { proc, mut frames }) => {
                            frames.push(Context::Cond {
                                consequent: consequent.clone(),
                                alternate: alternate.clone(),
                                env: env.clone(),
                            });
                            return Err(Signal::Capture { proc, frames });
                        }
                        Err(other) => return Err(other),
                    };
                    expr = if cond.is_true() {
                        consequent.clone()
                    } else {
                        alternate.clone()
                    };
                }
                Expr::Closure {
                    formals,
                    args_display,
                    body,
                } => {
                    return Ok(Value::Procedure(Procedure::Closure(Rc::new(Closure {
                        formals: formals.clone(),
                        args_display: args_display.clone(),
                        body: body.clone(),
                        env: env.clone(),
                    }))));
                }
                Expr::Assign {
                    id,
                    expr: val_expr,
                } => {
                    let val = match self.execute(val_expr.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture { proc, mut frames }) => {
                            frames.push(Context::Assign {
                                id: id.clone(),
                                env: env.clone(),
                            });
                            return Err(Signal::Capture { proc, frames });
                        }
                        Err(other) => return Err(other),
                    };
                    if EnvFrame::assign(&env, id, val) {
                        return Ok(Value::Void);
                    } else {
                        return Err(Signal::Error(format!(
                            "Error: cannot set undefined identifier {id}"
                        )));
                    }
                }
                Expr::Define {
                    id,
                    expr: val_expr,
                } => {
                    let val = match self.execute(val_expr.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture { proc, mut frames }) => {
                            frames.push(Context::Define {
                                id: id.clone(),
                                env: env.clone(),
                            });
                            return Err(Signal::Capture { proc, frames });
                        }
                        Err(other) => return Err(other),
                    };
                    EnvFrame::define(&env, id.clone(), val);
                    return Ok(Value::Void);
                }
                Expr::Sequence { first, rest } => {
                    match self.execute(first.clone(), env.clone()) {
                        Ok(_) => {}
                        Err(Signal::Capture { proc, mut frames }) => {
                            frames.push(Context::Seq {
                                next: rest.clone(),
                                env: env.clone(),
                            });
                            return Err(Signal::Capture { proc, frames });
                        }
                        Err(other) => return Err(other),
                    }
                    expr = rest.clone();
                }
                Expr::Apply { rator, args } => {
                    let args_rib = match args {
                        ApplyArgs::Normal(rand_exprs) => {
                            match self.eval_normal_args_from(
                                rand_exprs,
                                Vec::with_capacity(rand_exprs.len()),
                                &env,
                            ) {
                                Ok(rib) => rib,
                                Err(Signal::Capture { proc, mut frames }) => {
                                    frames.push(Context::Func {
                                        rator: rator.clone(),
                                        env: env.clone(),
                                    });
                                    return Err(Signal::Capture { proc, frames });
                                }
                                Err(other) => return Err(other),
                            }
                        }
                        ApplyArgs::Multiple(producer) => {
                            match self.execute(producer.clone(), env.clone()) {
                                Ok(mult_val) => Self::unpack_multiple_values(mult_val),
                                Err(Signal::Capture { proc, mut frames }) => {
                                    frames.push(Context::MultArg);
                                    frames.push(Context::Func {
                                        rator: rator.clone(),
                                        env: env.clone(),
                                    });
                                    return Err(Signal::Capture { proc, frames });
                                }
                                Err(other) => return Err(other),
                            }
                        }
                    };

                    let fn_val = match self.execute(rator.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture { proc, mut frames }) => {
                            frames.push(Context::Apply {
                                args_rib,
                                env: env.clone(),
                            });
                            return Err(Signal::Capture { proc, frames });
                        }
                        Err(other) => return Err(other),
                    };

                    let proc = match fn_val {
                        Value::Procedure(p) => p,
                        other => {
                            return Err(Signal::Error(format!(
                                "Attempt to apply non-procedure: {other}"
                            )))
                        }
                    };

                    match proc {
                        Procedure::Closure(closure) => {
                            let cenv = Self::bind_closure_env(&closure, args_rib)?;
                            expr = closure.body.clone();
                            env = cenv;
                        }
                        other_proc => {
                            return self.apply_procedure(&other_proc, args_rib, &env);
                        }
                    }
                }
            }
        }
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        self.symbols.borrow().clear_all_properties();
        EnvFrame::clear(&self.global_env);
    }
}
