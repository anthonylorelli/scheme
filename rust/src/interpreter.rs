use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::compiler::Compiler;
use crate::context::Context;
use crate::continuation::{Continuation, PromptTag};
use crate::env::{Env, EnvFrame, SMALL_SLOTS};
use crate::instruction::{ApplyArgs, Arity, Expr};
use crate::macro_expander::MacroExpander;
use crate::parser::Parser;
use crate::primitives::{
    EMBEDDED_INIT_SCM, EMBEDDED_PSYNTAX_PP, PRIMITIVES_SCM, RUST_EXTENSIONS_SCM,
};
use crate::scanner::Scanner;
use crate::stdlib::register_standard_library;
use crate::symbol::SymbolTable;
use crate::value::{Closure, Procedure, Value};

enum SmallArgs {
    Inline {
        len: usize,
        buf: [Value; SMALL_SLOTS],
    },
    Heap(Vec<Value>),
}

impl SmallArgs {
    #[inline]
    fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => *len,
            Self::Heap(v) => v.len(),
        }
    }

    #[inline]
    fn as_slice(&self) -> &[Value] {
        match self {
            Self::Inline { len, buf } => &buf[..*len],
            Self::Heap(v) => v.as_slice(),
        }
    }

    #[inline]
    fn into_vec(self) -> Vec<Value> {
        match self {
            Self::Inline { len, mut buf } => {
                let mut v = Vec::with_capacity(len);
                for slot in buf.iter_mut().take(len) {
                    v.push(std::mem::replace(slot, Value::Void));
                }
                v
            }
            Self::Heap(v) => v,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CaptureTarget {
    Undelimited,
    Composable(PromptTag),
}

#[derive(Debug)]
pub enum Signal {
    Capture {
        target: CaptureTarget,
        proc: Procedure,
        frames: Vec<Context>,
    },
    Invoke {
        k_frames: Rc<Vec<Context>>,
        val: Value,
    },
    Abort {
        tag: PromptTag,
        args: Vec<Value>,
    },
    Error(String),
}

enum PromptEntry<'a> {
    Thunk {
        proc: &'a Procedure,
        env: &'a Env,
    },
    ResumeValue {
        frames: &'a [Context],
        val: Value,
    },
    ResumeProcedure {
        frames: &'a [Context],
        proc: &'a Procedure,
        k_val: Value,
    },
}

enum OutputSink {
    Stdout,
    Buffer(Vec<u8>),
}

pub struct Interpreter {
    symbols: Rc<RefCell<SymbolTable>>,
    default_prompt_tag: PromptTag,
    output: OutputSink,
    current_dir_stack: Vec<PathBuf>,
}

impl Interpreter {
    /// Creates a bare interpreter with native `StandardLibrary` bindings (without `Primitives`,
    /// `psyntax.pp`, or `init.scm` loaded yet).
    pub fn new_bare() -> Self {
        let symbols = Rc::new(RefCell::new(SymbolTable::new()));
        register_standard_library(&mut symbols.borrow_mut());
        Self {
            symbols,
            default_prompt_tag: PromptTag::new(Some("default".to_string())),
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

    pub fn default_prompt_tag(&self) -> &PromptTag {
        &self.default_prompt_tag
    }

    /// Loads `Primitives`, `psyntax.pp`, `init.scm`, and Rust syntax extensions.
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

        self.eval_str(RUST_EXTENSIONS_SCM)?;

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
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let resolved = self.resolve_load_path(&path_str);
        let file = File::open(&resolved)
            .map_err(|e| format!("Could not open file '{}': {e}", resolved.display()))?;
        if let Some(parent) = resolved
            .canonicalize()
            .ok()
            .and_then(|c| c.parent().map(Path::to_path_buf))
        {
            self.current_dir_stack.push(parent);
        } else if let Some(parent) = resolved.parent() {
            self.current_dir_stack.push(parent.to_path_buf());
        } else {
            self.current_dir_stack.push(PathBuf::from("."));
        }

        let reader = BufReader::new(file);
        let res = self.eval_reader(reader);
        self.current_dir_stack.pop();
        res
    }

    pub fn eval_str(&mut self, source: &str) -> Result<Value, String> {
        self.eval_reader(Cursor::new(source.as_bytes()))
    }

    pub fn eval_reader<R: BufRead>(&mut self, reader: R) -> Result<Value, String> {
        let mut parser = Parser::new(Scanner::new(reader), self.symbols.clone());
        let mut last = Value::Void;
        while let Some(datum) = parser.read()? {
            last = self.eval_datum(datum)?;
        }
        Ok(last)
    }

    pub fn eval_next<R: BufRead>(&mut self, parser: &mut Parser<R>) -> Result<Option<Value>, String> {
        let Some(datum) = parser.read()? else {
            return Ok(None);
        };
        let val = self.eval_datum(datum)?;
        Ok(Some(val))
    }

    pub fn eval_datum(&mut self, datum: Value) -> Result<Value, String> {
        let expanded = MacroExpander::expand(self, datum)?;
        let cache = self.symbols.borrow().cache.clone();
        let compiler = Compiler::new(cache);
        let compiled = compiler.compile(&expanded)?;
        self.evaluate_compiled(compiled, EnvFrame::empty_top_level())
    }

    pub fn call_procedure_top_level(
        &mut self,
        proc: &Procedure,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let env = EnvFrame::empty_top_level();
        let mut pending_res = self.apply_procedure(proc, args, &env);
        self.drive_trampoline(&mut pending_res)
    }

    pub fn evaluate_compiled(&mut self, expr: Rc<Expr>, env: Env) -> Result<Value, String> {
        let mut pending_res = self.execute(expr, env);
        self.drive_trampoline(&mut pending_res)
    }

    pub fn call_with_prompt(
        &mut self,
        thunk: &Procedure,
        tag: &PromptTag,
        handler: Option<&Procedure>,
        env: &Env,
    ) -> Result<Value, Signal> {
        self.run_with_prompt_entry(PromptEntry::Thunk { proc: thunk, env }, tag, handler)
    }

    fn invoke_default_prompt_handler(&mut self, args: &[Value]) -> Result<Value, Signal> {
        if args.len() != 1 {
            return Err(Signal::Error(format!(
                "Default prompt handler expects 1 thunk argument, got {}",
                args.len()
            )));
        }
        let abort_proc = match &args[0] {
            Value::Procedure(p) => p.clone(),
            other => {
                return Err(Signal::Error(format!(
                    "Default prompt handler expects a procedure, got {other}"
                )))
            }
        };
        let top_env = EnvFrame::empty_top_level();
        self.apply_procedure(&abort_proc, Vec::new(), &top_env)
    }

    fn run_with_prompt_entry(
        &mut self,
        entry: PromptEntry<'_>,
        tag: &PromptTag,
        handler: Option<&Procedure>,
    ) -> Result<Value, Signal> {
        let mut pending = match entry {
            PromptEntry::Thunk { proc, env } => self.apply_procedure(proc, Vec::new(), env),
            PromptEntry::ResumeValue { frames, val } => self.resume_with_value(frames, val),
            PromptEntry::ResumeProcedure {
                frames,
                proc,
                k_val,
            } => self.resume_with_procedure(frames, proc, k_val),
        };

        loop {
            match pending {
                Ok(v) => return Ok(v),
                Err(Signal::Abort {
                    tag: abort_tag,
                    args,
                }) => {
                    if &abort_tag == tag {
                        if let Some(h) = handler {
                            let top_env = EnvFrame::empty_top_level();
                            return self.apply_procedure(h, args, &top_env);
                        }
                        pending = self.invoke_default_prompt_handler(&args);
                    } else {
                        return Err(Signal::Abort {
                            tag: abort_tag,
                            args,
                        });
                    }
                }
                Err(Signal::Capture {
                    target: CaptureTarget::Composable(cap_tag),
                    proc,
                    frames,
                }) if &cap_tag == tag => {
                    let k = Continuation::new(frames);
                    let k_frames = k.frames().clone();
                    let k_val = Value::Procedure(Procedure::ComposableContinuation(k));
                    pending = self.resume_with_procedure(&k_frames, &proc, k_val);
                }
                Err(Signal::Capture {
                    target,
                    proc,
                    mut frames,
                }) => {
                    frames.push(Context::Prompt {
                        tag: tag.clone(),
                        handler: handler.cloned(),
                    });
                    return Err(Signal::Capture {
                        target,
                        proc,
                        frames,
                    });
                }
                Err(other) => return Err(other),
            }
        }
    }

    fn drive_trampoline(&mut self, pending_res: &mut Result<Value, Signal>) -> Result<Value, String> {
        loop {
            let err = match pending_res {
                Ok(val) => return Ok(val.clone()),
                Err(sig) => std::mem::replace(sig, Signal::Error(String::new())),
            };

            match err {
                Signal::Capture {
                    target: CaptureTarget::Undelimited,
                    proc,
                    mut frames,
                } => {
                    if frames.is_empty() {
                        frames.push(Context::Halt);
                    }
                    let k = Continuation::new(frames);
                    let k_frames = k.frames().clone();
                    let k_val = Value::Procedure(Procedure::Continuation(k));
                    *pending_res = self.resume_with_procedure(&k_frames, &proc, k_val);
                }
                Signal::Capture {
                    target: CaptureTarget::Composable(tag),
                    proc,
                    frames,
                } => {
                    if tag == self.default_prompt_tag {
                        let k = Continuation::new(frames);
                        let k_frames = k.frames().clone();
                        let k_val = Value::Procedure(Procedure::ComposableContinuation(k));
                        *pending_res = self.resume_with_procedure(&k_frames, &proc, k_val);
                    } else {
                        return Err(format!("No continuation prompt found for tag {tag}"));
                    }
                }
                Signal::Invoke { k_frames, val } => {
                    *pending_res = self.resume_with_value(&k_frames, val);
                }
                Signal::Abort { tag, args } => {
                    if tag == self.default_prompt_tag {
                        *pending_res = self.invoke_default_prompt_handler(&args);
                    } else {
                        return Err(format!("No continuation prompt found for tag {tag}"));
                    }
                }
                Signal::Error(msg) => return Err(msg),
            }
        }
    }

    fn resume_through_prompt(
        &mut self,
        entry: PromptEntry<'_>,
        prompt: &Context,
        outside: &[Context],
    ) -> Result<Value, Signal> {
        let Context::Prompt { tag, handler } = prompt else {
            unreachable!("resume_through_prompt requires a Context::Prompt");
        };
        let prompt_val = match self.run_with_prompt_entry(entry, tag, handler.as_ref()) {
            Ok(v) => v,
            Err(Signal::Capture {
                target,
                proc,
                mut frames,
            }) => {
                frames.extend_from_slice(outside);
                return Err(Signal::Capture {
                    target,
                    proc,
                    frames,
                });
            }
            Err(other) => return Err(other),
        };
        self.resume_with_value(outside, prompt_val)
    }

    fn resume_with_procedure(
        &mut self,
        k_frames: &[Context],
        proc: &Procedure,
        k_val: Value,
    ) -> Result<Value, Signal> {
        if let Some(prompt_idx) = k_frames
            .iter()
            .rposition(|c| matches!(c, Context::Prompt { .. }))
        {
            let inside = &k_frames[..prompt_idx];
            let outside = &k_frames[prompt_idx + 1..];
            return self.resume_through_prompt(
                PromptEntry::ResumeProcedure {
                    frames: inside,
                    proc,
                    k_val,
                },
                &k_frames[prompt_idx],
                outside,
            );
        }

        let top_env = EnvFrame::empty_top_level();
        let val = match self.apply_procedure(proc, vec![k_val], &top_env) {
            Ok(v) => v,
            Err(Signal::Capture {
                target,
                proc: inner_proc,
                mut frames,
            }) => {
                frames.extend_from_slice(k_frames);
                return Err(Signal::Capture {
                    target,
                    proc: inner_proc,
                    frames,
                });
            }
            Err(other) => return Err(other),
        };

        self.resume_with_value(k_frames, val)
    }

    fn resume_with_value(&mut self, k_frames: &[Context], mut val: Value) -> Result<Value, Signal> {
        if let Some(prompt_idx) = k_frames
            .iter()
            .rposition(|c| matches!(c, Context::Prompt { .. }))
        {
            let inside = &k_frames[..prompt_idx];
            let outside = &k_frames[prompt_idx + 1..];
            return self.resume_through_prompt(
                PromptEntry::ResumeValue {
                    frames: inside,
                    val,
                },
                &k_frames[prompt_idx],
                outside,
            );
        }

        for (idx, ctx) in k_frames.iter().enumerate() {
            match self.invoke_context(ctx, val) {
                Ok(next_val) => val = next_val,
                Err(Signal::Capture {
                    target,
                    proc,
                    mut frames,
                }) => {
                    frames.extend_from_slice(&k_frames[idx + 1..]);
                    return Err(Signal::Capture {
                        target,
                        proc,
                        frames,
                    });
                }
                Err(other) => return Err(other),
            }
        }
        Ok(val)
    }

    fn unpack_multiple_values(val: Value) -> Vec<Value> {
        match val {
            Value::Values(vs) => *vs,
            Value::Void => Vec::new(),
            other => vec![other],
        }
    }

    #[inline]
    fn unpack_continuation_vec(mut args: Vec<Value>) -> Value {
        match args.len() {
            0 => Value::Void,
            1 => args.pop().unwrap(),
            _ => Value::Values(Box::new(args)),
        }
    }

    #[inline]
    fn unpack_continuation_args(args_rib: SmallArgs) -> Value {
        match args_rib {
            SmallArgs::Inline { len: 0, .. } => Value::Void,
            SmallArgs::Inline { len: 1, mut buf } => std::mem::replace(&mut buf[0], Value::Void),
            SmallArgs::Heap(v) => Self::unpack_continuation_vec(v),
            other => Value::Values(Box::new(other.into_vec())),
        }
    }

    fn invoke_context(&mut self, ctx: &Context, val: Value) -> Result<Value, Signal> {
        match ctx {
            Context::Halt => Ok(val),
            Context::Prompt { .. } => {
                unreachable!("Prompt frames are handled by resume_with_value")
            }
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
            Context::AssignLocal { depth, slot, env } => {
                EnvFrame::assign_local(env, *depth, *slot, val);
                Ok(Value::Void)
            }
            Context::AssignGlobal { id } => {
                if id.assign_global(val) {
                    Ok(Value::Void)
                } else {
                    Err(Signal::Error(format!(
                        "Error: cannot set undefined identifier {id}"
                    )))
                }
            }
            Context::DefineGlobal { id } => {
                id.define_global(val);
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
                Ok(Value::Rib(Box::new(args)))
            }
            Context::Func { rator, env } => {
                let args_rib = match val {
                    Value::Rib(r) => *r,
                    other => vec![other],
                };
                let fn_val = match self.execute(rator.clone(), env.clone()) {
                    Ok(v) => v,
                    Err(Signal::Capture {
                        target,
                        proc,
                        mut frames,
                    }) => {
                        frames.push(Context::Apply {
                            args_rib,
                            env: env.clone(),
                        });
                        return Err(Signal::Capture {
                            target,
                            proc,
                            frames,
                        });
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
            Context::MultArg => Ok(Value::Rib(Box::new(Self::unpack_multiple_values(val)))),
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
                Err(Signal::Capture {
                    target,
                    proc,
                    mut frames,
                }) => {
                    frames.push(Context::ArgEval {
                        remaining_left: exprs[..idx].to_vec(),
                        evaluated_rev,
                        env: env.clone(),
                    });
                    return Err(Signal::Capture {
                        target,
                        proc,
                        frames,
                    });
                }
                Err(other) => return Err(other),
            }
        }
        evaluated_rev.reverse();
        Ok(evaluated_rev)
    }

    #[inline]
    fn bind_closure_env(closure: &Closure, mut args: Vec<Value>) -> Result<Env, Signal> {
        match closure.arity {
            Arity::Fixed(expected) => {
                if args.len() != expected {
                    return Err(Signal::Error(format!(
                        "Arity mismatch: expected {expected} argument(s), got {}",
                        args.len()
                    )));
                }
                if !closure.has_frame {
                    return Ok(closure.env.clone());
                }
                if closure.extra_locals > 0 {
                    args.resize(expected + closure.extra_locals, Value::Void);
                }
                Ok(EnvFrame::from_slots(args, closure.env.clone()))
            }
            Arity::Variadic { fixed } => {
                if args.len() < fixed {
                    return Err(Signal::Error(format!(
                        "Arity mismatch: expected at least {fixed} argument(s), got {}",
                        args.len()
                    )));
                }
                let mut rest_list = Value::EmptyList;
                for item in args.drain(fixed..).rev() {
                    rest_list = Value::cons(item, rest_list);
                }
                args.push(rest_list);
                if closure.extra_locals > 0 {
                    args.resize(fixed + 1 + closure.extra_locals, Value::Void);
                }
                Ok(EnvFrame::from_slots(args, closure.env.clone()))
            }
        }
    }

    #[inline]
    fn bind_closure_env_small(closure: &Closure, args: SmallArgs) -> Result<Env, Signal> {
        match closure.arity {
            Arity::Fixed(expected) => {
                let argc = args.len();
                if argc != expected {
                    return Err(Signal::Error(format!(
                        "Arity mismatch: expected {expected} argument(s), got {argc}"
                    )));
                }
                if !closure.has_frame {
                    return Ok(closure.env.clone());
                }
                let total = expected + closure.extra_locals;
                match args {
                    SmallArgs::Inline { buf, .. } if total <= SMALL_SLOTS => {
                        Ok(EnvFrame::from_inline_slots(buf, closure.env.clone()))
                    }
                    other => {
                        let mut v = other.into_vec();
                        if closure.extra_locals > 0 {
                            v.resize(total, Value::Void);
                        }
                        Ok(EnvFrame::from_slots(v, closure.env.clone()))
                    }
                }
            }
            Arity::Variadic { fixed } => {
                let argc = args.len();
                if argc < fixed {
                    return Err(Signal::Error(format!(
                        "Arity mismatch: expected at least {fixed} argument(s), got {argc}"
                    )));
                }
                let total = fixed + 1 + closure.extra_locals;
                match args {
                    SmallArgs::Inline { len, mut buf } if total <= SMALL_SLOTS => {
                        let mut rest_list = Value::EmptyList;
                        for idx in (fixed..len).rev() {
                            let item = std::mem::replace(&mut buf[idx], Value::Void);
                            rest_list = Value::cons(item, rest_list);
                        }
                        buf[fixed] = rest_list;
                        Ok(EnvFrame::from_inline_slots(buf, closure.env.clone()))
                    }
                    other => Self::bind_closure_env(closure, other.into_vec()),
                }
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
                let val = Self::unpack_continuation_vec(args);
                Err(Signal::Invoke {
                    k_frames: k.frames().clone(),
                    val,
                })
            }
            Procedure::ComposableContinuation(k) => {
                let val = Self::unpack_continuation_vec(args);
                self.resume_with_value(k.frames(), val)
            }
        }
    }

    pub fn execute(&mut self, mut expr: Rc<Expr>, mut env: Env) -> Result<Value, Signal> {
        loop {
            match expr.as_ref() {
                Expr::Constant(val) => return Ok(val.clone()),
                Expr::LocalVar { depth, slot } => {
                    return Ok(EnvFrame::lookup_local(&env, *depth, *slot));
                }
                Expr::GlobalVar(sym) => {
                    return sym.get_global().ok_or_else(|| {
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
                        Err(Signal::Capture {
                            target,
                            proc,
                            mut frames,
                        }) => {
                            frames.push(Context::Cond {
                                consequent: consequent.clone(),
                                alternate: alternate.clone(),
                                env: env.clone(),
                            });
                            return Err(Signal::Capture {
                                target,
                                proc,
                                frames,
                            });
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
                    arity,
                    extra_locals,
                    has_frame,
                    args_display,
                    body,
                } => {
                    return Ok(Value::Procedure(Procedure::Closure(Rc::new(Closure {
                        arity: *arity,
                        extra_locals: *extra_locals,
                        has_frame: *has_frame,
                        args_display: args_display.clone(),
                        body: body.clone(),
                        env: env.clone(),
                    }))));
                }
                Expr::PushFrame { slot_count, body } => {
                    env = EnvFrame::with_void_slots(*slot_count, env);
                    expr = body.clone();
                }
                Expr::AssignLocal {
                    depth,
                    slot,
                    expr: val_expr,
                } => {
                    let val = match self.execute(val_expr.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture {
                            target,
                            proc,
                            mut frames,
                        }) => {
                            frames.push(Context::AssignLocal {
                                depth: *depth,
                                slot: *slot,
                                env: env.clone(),
                            });
                            return Err(Signal::Capture {
                                target,
                                proc,
                                frames,
                            });
                        }
                        Err(other) => return Err(other),
                    };
                    EnvFrame::assign_local(&env, *depth, *slot, val);
                    return Ok(Value::Void);
                }
                Expr::AssignGlobal { id, expr: val_expr } => {
                    let val = match self.execute(val_expr.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture {
                            target,
                            proc,
                            mut frames,
                        }) => {
                            frames.push(Context::AssignGlobal { id: id.clone() });
                            return Err(Signal::Capture {
                                target,
                                proc,
                                frames,
                            });
                        }
                        Err(other) => return Err(other),
                    };
                    if id.assign_global(val) {
                        return Ok(Value::Void);
                    } else {
                        return Err(Signal::Error(format!(
                            "Error: cannot set undefined identifier {id}"
                        )));
                    }
                }
                Expr::DefineGlobal { id, expr: val_expr } => {
                    let val = match self.execute(val_expr.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture {
                            target,
                            proc,
                            mut frames,
                        }) => {
                            frames.push(Context::DefineGlobal { id: id.clone() });
                            return Err(Signal::Capture {
                                target,
                                proc,
                                frames,
                            });
                        }
                        Err(other) => return Err(other),
                    };
                    id.define_global(val);
                    return Ok(Value::Void);
                }
                Expr::Sequence { first, rest } => {
                    match self.execute(first.clone(), env.clone()) {
                        Ok(_) => {}
                        Err(Signal::Capture {
                            target,
                            proc,
                            mut frames,
                        }) => {
                            frames.push(Context::Seq {
                                next: rest.clone(),
                                env: env.clone(),
                            });
                            return Err(Signal::Capture {
                                target,
                                proc,
                                frames,
                            });
                        }
                        Err(other) => return Err(other),
                    }
                    expr = rest.clone();
                }
                Expr::Apply { rator, args } => {
                    let args_rib = match args {
                        ApplyArgs::Normal(rand_exprs) if rand_exprs.len() <= SMALL_SLOTS => {
                            let n = rand_exprs.len();
                            let mut buf = [const { Value::Void }; SMALL_SLOTS];
                            for idx in (0..n).rev() {
                                match self.execute(rand_exprs[idx].clone(), env.clone()) {
                                    Ok(v) => buf[idx] = v,
                                    Err(Signal::Capture {
                                        target,
                                        proc,
                                        mut frames,
                                    }) => {
                                        let mut evaluated_rev =
                                            Vec::with_capacity(n - idx - 1);
                                        for done_idx in ((idx + 1)..n).rev() {
                                            evaluated_rev.push(std::mem::replace(
                                                &mut buf[done_idx],
                                                Value::Void,
                                            ));
                                        }
                                        frames.push(Context::ArgEval {
                                            remaining_left: rand_exprs[..idx].to_vec(),
                                            evaluated_rev,
                                            env: env.clone(),
                                        });
                                        frames.push(Context::Func {
                                            rator: rator.clone(),
                                            env: env.clone(),
                                        });
                                        return Err(Signal::Capture {
                                            target,
                                            proc,
                                            frames,
                                        });
                                    }
                                    Err(other) => return Err(other),
                                }
                            }
                            SmallArgs::Inline { len: n, buf }
                        }
                        ApplyArgs::Normal(rand_exprs) => {
                            match self.eval_normal_args_from(
                                rand_exprs,
                                Vec::with_capacity(rand_exprs.len()),
                                &env,
                            ) {
                                Ok(rib) => SmallArgs::Heap(rib),
                                Err(Signal::Capture {
                                    target,
                                    proc,
                                    mut frames,
                                }) => {
                                    frames.push(Context::Func {
                                        rator: rator.clone(),
                                        env: env.clone(),
                                    });
                                    return Err(Signal::Capture {
                                        target,
                                        proc,
                                        frames,
                                    });
                                }
                                Err(other) => return Err(other),
                            }
                        }
                        ApplyArgs::Multiple(producer) => {
                            match self.execute(producer.clone(), env.clone()) {
                                Ok(mult_val) => {
                                    SmallArgs::Heap(Self::unpack_multiple_values(mult_val))
                                }
                                Err(Signal::Capture {
                                    target,
                                    proc,
                                    mut frames,
                                }) => {
                                    frames.push(Context::MultArg);
                                    frames.push(Context::Func {
                                        rator: rator.clone(),
                                        env: env.clone(),
                                    });
                                    return Err(Signal::Capture {
                                        target,
                                        proc,
                                        frames,
                                    });
                                }
                                Err(other) => return Err(other),
                            }
                        }
                    };

                    let fn_val = match self.execute(rator.clone(), env.clone()) {
                        Ok(v) => v,
                        Err(Signal::Capture {
                            target,
                            proc,
                            mut frames,
                        }) => {
                            frames.push(Context::Apply {
                                args_rib: args_rib.into_vec(),
                                env: env.clone(),
                            });
                            return Err(Signal::Capture {
                                target,
                                proc,
                                frames,
                            });
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
                            let cenv = Self::bind_closure_env_small(&closure, args_rib)?;
                            expr = closure.body.clone();
                            env = cenv;
                        }
                        Procedure::Native(native) => {
                            return (native.func)(self, args_rib.as_slice(), &env);
                        }
                        Procedure::Continuation(k) => {
                            let val = Self::unpack_continuation_args(args_rib);
                            return Err(Signal::Invoke {
                                k_frames: k.frames().clone(),
                                val,
                            });
                        }
                        Procedure::ComposableContinuation(k) => {
                            let val = Self::unpack_continuation_args(args_rib);
                            return self.resume_with_value(k.frames(), val);
                        }
                    }
                }
            }
        }
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        let symbols = self.symbols.borrow();
        symbols.clear_all_properties();
        symbols.clear_all_globals();
    }
}
