use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::compiler::Compiler;
use crate::continuation::PromptTag;
use crate::env::Env;
use crate::interpreter::{CaptureTarget, Interpreter, Signal};
use crate::symbol::{Symbol, SymbolTable};
use crate::value::{NativeFn, NativeProcedure, Procedure, Value};

pub fn register_standard_library(symbols: &mut SymbolTable) {
    let primitives: &[(&str, NativeFn)] = &[
        ("cons", proc_cons),
        ("car", proc_car),
        ("cdr", proc_cdr),
        ("display", proc_display),
        ("integer->char", proc_integer_to_char),
        ("+", proc_add),
        ("-", proc_subtract),
        ("*", proc_multiply),
        (">", proc_greater_than),
        ("<", proc_less_than),
        ("zero?", proc_zero_pred),
        ("load", proc_load),
        ("vector", proc_vector),
        ("vector-ref", proc_vector_ref),
        ("string-length", proc_string_length),
        ("set-car!", proc_set_car),
        ("set-cdr!", proc_set_cdr),
        ("procedure?", proc_procedure_pred),
        ("symbol?", proc_symbol_pred),
        ("pair?", proc_pair_pred),
        ("null?", proc_null_pred),
        ("eq?", proc_eq_pred),
        ("eqv?", proc_eqv_pred),
        ("boolean?", proc_boolean_pred),
        ("vector?", proc_vector_pred),
        ("void", proc_void),
        ("putprop", proc_putprop),
        ("getprop", proc_getprop),
        ("remprop", proc_remprop),
        ("values", proc_values),
        ("=", proc_numeric_equals),
        ("vector-length", proc_vector_length),
        ("append", proc_append),
        ("string?", proc_string_pred),
        ("string=?", proc_string_equals),
        ("apply", proc_apply),
        ("string", proc_string),
        ("list", proc_list),
        ("number?", proc_number_pred),
        ("_native-eval", proc_native_eval),
        ("gensym", proc_gensym),
        ("gensym?", proc_gensym_pred),
        ("list->vector", proc_list_to_vector),
        ("make-vector", proc_make_vector),
        ("vector-set!", proc_vector_set),
        ("_get-ticks", proc_get_ticks),
        ("/", proc_div),
        ("call-with-current-continuation", proc_call_cc),
        ("make-continuation-prompt-tag", proc_make_continuation_prompt_tag),
        ("default-continuation-prompt-tag", proc_default_continuation_prompt_tag),
        ("continuation-prompt-tag?", proc_continuation_prompt_tag_pred),
        ("call-with-continuation-prompt", proc_call_with_continuation_prompt),
        ("abort-current-continuation", proc_abort_current_continuation),
        ("call-with-composable-continuation", proc_call_with_composable_continuation),
        ("_compile", proc_compile),
        ("char?", proc_char_pred),
        ("error", proc_error),
    ];

    for &(name, func) in primitives {
        let sym = symbols.intern(name);
        let proc = Value::Procedure(Procedure::Native(Rc::new(NativeProcedure { name, func })));
        sym.define_global(proc);
    }
}

fn expect_arity(args: &[Value], expected: usize, name: &str) -> Result<(), Signal> {
    if args.len() != expected {
        Err(Signal::Error(format!(
            "{name} expects {expected} argument(s), got {}",
            args.len()
        )))
    } else {
        Ok(())
    }
}

fn expect_int(val: &Value, name: &str) -> Result<i64, Signal> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(Signal::Error(format!(
            "{name}: expected integer, got {other}"
        ))),
    }
}

fn expect_symbol(val: &Value, name: &str) -> Result<Symbol, Signal> {
    match val {
        Value::Symbol(s) => Ok(s.clone()),
        other => Err(Signal::Error(format!(
            "{name}: expected symbol, got {other}"
        ))),
    }
}

fn proc_cons(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "cons")?;
    Ok(Value::cons(args[0].clone(), args[1].clone()))
}

fn proc_car(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "car")?;
    match &args[0] {
        Value::EmptyList => Err(Signal::Error("Cannot take car of empty list".to_string())),
        Value::Pair(p) => Ok(p.borrow().head.clone()),
        _ => Err(Signal::Error("Error: cannot take car of non-pair".to_string())),
    }
}

fn proc_cdr(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "cdr")?;
    match &args[0] {
        Value::EmptyList => Err(Signal::Error("Cannot take cdr of empty list".to_string())),
        Value::Pair(p) => Ok(p.borrow().tail.clone()),
        _ => Err(Signal::Error("Error: cannot take cdr of non-pair".to_string())),
    }
}

fn proc_display(i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "display")?;
    i.write_output(&format!("{}", args[0]))
        .map_err(Signal::Error)?;
    Ok(Value::Void)
}

fn proc_integer_to_char(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "integer->char")?;
    let n = expect_int(&args[0], "integer->char")?;
    let code = u32::try_from(n)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| Signal::Error(format!("integer->char: invalid character code {n}")))?;
    Ok(Value::Char(code))
}

fn proc_add(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    let mut sum = 0i64;
    for arg in args {
        let n = expect_int(arg, "+")?;
        sum = sum.wrapping_add(n);
    }
    Ok(Value::Int(sum))
}

fn proc_subtract(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    if args.is_empty() {
        return Err(Signal::Error("- expects at least 1 argument".to_string()));
    }
    let first = expect_int(&args[0], "-")?;
    if args.len() == 1 {
        return Ok(Value::Int(first.wrapping_neg()));
    }
    let mut acc = first;
    for arg in &args[1..] {
        acc = acc.wrapping_sub(expect_int(arg, "-")?);
    }
    Ok(Value::Int(acc))
}

fn proc_multiply(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    let mut prod = 1i64;
    for arg in args {
        prod = prod.wrapping_mul(expect_int(arg, "*")?);
    }
    Ok(Value::Int(prod))
}

fn proc_greater_than(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, ">")?;
    let a = expect_int(&args[0], ">")?;
    let b = expect_int(&args[1], ">")?;
    Ok(Value::Bool(a > b))
}

fn proc_less_than(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "<")?;
    let a = expect_int(&args[0], "<")?;
    let b = expect_int(&args[1], "<")?;
    Ok(Value::Bool(a < b))
}

fn proc_zero_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "zero?")?;
    let n = expect_int(&args[0], "zero?")?;
    Ok(Value::Bool(n == 0))
}

fn proc_load(i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "load")?;
    let path_str = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        other => other.to_string(),
    };
    i.load_file(&path_str).map_err(Signal::Error)?;
    Ok(Value::Void)
}

fn proc_vector(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    Ok(Value::Vector(Rc::new(RefCell::new(args.to_vec()))))
}

fn proc_vector_ref(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "vector-ref")?;
    let idx = expect_int(&args[1], "vector-ref")?;
    match &args[0] {
        Value::Vector(v) => {
            let borrowed = v.borrow();
            if idx >= 0 && (idx as usize) < borrowed.len() {
                Ok(borrowed[idx as usize].clone())
            } else {
                Err(Signal::Error(format!(
                    "Index {idx} out of range [0, {}] in vector {}",
                    borrowed.len() as isize - 1,
                    args[0]
                )))
            }
        }
        other => Err(Signal::Error(format!(
            "Attempt to index non-vector value: {other}"
        ))),
    }
}

fn proc_string_length(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "string-length")?;
    match &args[0] {
        Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
        _ => Err(Signal::Error(
            "Error: cannot apply string-length to non-string value".to_string(),
        )),
    }
}

fn proc_set_car(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "set-car!")?;
    match &args[0] {
        Value::Pair(p) => {
            p.borrow_mut().head = args[1].clone();
            Ok(Value::Void)
        }
        _ => Err(Signal::Error(
            "Error: attempt to set car of non-pair argument".to_string(),
        )),
    }
}

fn proc_set_cdr(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "set-cdr!")?;
    match &args[0] {
        Value::Pair(p) => {
            p.borrow_mut().tail = args[1].clone();
            Ok(Value::Void)
        }
        _ => Err(Signal::Error(
            "Error: attempt to set cdr of non-pair argument".to_string(),
        )),
    }
}

fn proc_procedure_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "procedure?")?;
    Ok(Value::Bool(matches!(args[0], Value::Procedure(_))))
}

fn proc_symbol_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "symbol?")?;
    Ok(Value::Bool(matches!(args[0], Value::Symbol(_))))
}

fn proc_pair_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "pair?")?;
    Ok(Value::Bool(matches!(args[0], Value::Pair(_))))
}

fn proc_null_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "null?")?;
    Ok(Value::Bool(matches!(args[0], Value::EmptyList)))
}

fn proc_eq_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "eq?")?;
    Ok(Value::Bool(args[0].is_eq(&args[1])))
}

fn proc_eqv_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "eqv?")?;
    Ok(Value::Bool(args[0].is_eqv(&args[1])))
}

fn proc_boolean_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "boolean?")?;
    Ok(Value::Bool(matches!(args[0], Value::Bool(_))))
}

fn proc_vector_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "vector?")?;
    Ok(Value::Bool(matches!(args[0], Value::Vector(_))))
}

fn proc_void(_i: &mut Interpreter, _args: &[Value], _env: &Env) -> Result<Value, Signal> {
    Ok(Value::Void)
}

fn proc_putprop(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 3, "putprop")?;
    let sym = expect_symbol(&args[0], "putprop")?;
    let prop = expect_symbol(&args[1], "putprop")?;
    sym.put_prop(prop, args[2].clone());
    Ok(Value::Void)
}

fn proc_getprop(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "getprop")?;
    let sym = expect_symbol(&args[0], "getprop")?;
    let prop = expect_symbol(&args[1], "getprop")?;
    Ok(sym.get_prop(&prop).unwrap_or(Value::Bool(false)))
}

fn proc_remprop(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "remprop")?;
    let sym = expect_symbol(&args[0], "remprop")?;
    let prop = expect_symbol(&args[1], "remprop")?;
    sym.rem_prop(&prop);
    Ok(Value::Void)
}

fn proc_values(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    match args.len() {
        0 => Ok(Value::Void),
        1 => Ok(args[0].clone()),
        _ => Ok(Value::Values(Box::new(args.to_vec()))),
    }
}

fn proc_numeric_equals(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    if args.len() < 2 {
        return Err(Signal::Error("= expects at least 2 arguments".to_string()));
    }
    let first = expect_int(&args[0], "=")?;
    for arg in &args[1..] {
        if expect_int(arg, "=")? != first {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn proc_vector_length(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "vector-length")?;
    match &args[0] {
        Value::Vector(v) => Ok(Value::Int(v.borrow().len() as i64)),
        other => Err(Signal::Error(format!(
            "vector-length expected vector, got {other}"
        ))),
    }
}

fn proc_append(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    if args.is_empty() {
        return Ok(Value::EmptyList);
    }
    let mut acc = args[args.len() - 1].clone();
    for list in args[..args.len() - 1].iter().rev() {
        let items = list.to_proper_vec().map_err(Signal::Error)?;
        for item in items.into_iter().rev() {
            acc = Value::cons(item, acc);
        }
    }
    Ok(acc)
}

fn proc_string_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "string?")?;
    Ok(Value::Bool(matches!(args[0], Value::Str(_))))
}

fn proc_string_equals(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "string=?")?;
    match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a.as_str() == b.as_str())),
        _ => Err(Signal::Error("string=? expects two strings".to_string())),
    }
}

fn proc_apply(i: &mut Interpreter, args: &[Value], env: &Env) -> Result<Value, Signal> {
    if args.len() < 2 {
        return Err(Signal::Error(
            "apply expects procedure and argument list".to_string(),
        ));
    }
    let proc = match &args[0] {
        Value::Procedure(p) => p.clone(),
        other => {
            return Err(Signal::Error(format!(
                "apply expected procedure, got {other}"
            )))
        }
    };

    let mut flat_args = Vec::new();
    for item in &args[1..args.len() - 1] {
        flat_args.push(item.clone());
    }
    let tail_items = args[args.len() - 1]
        .to_proper_vec()
        .map_err(Signal::Error)?;
    flat_args.extend(tail_items);

    i.apply_procedure(&proc, flat_args, env)
}

fn proc_string(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    let mut buf = String::with_capacity(args.len());
    for arg in args {
        match arg {
            Value::Char(c) => buf.push(*c),
            other => {
                return Err(Signal::Error(format!(
                    "Error: non-character value passed to string function: {other}"
                )))
            }
        }
    }
    Ok(Value::Str(Rc::new(buf)))
}

fn proc_list(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    Ok(Value::list_from_slice(args))
}

fn proc_number_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "number?")?;
    Ok(Value::Bool(matches!(args[0], Value::Int(_))))
}

fn proc_native_eval(i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "_native-eval")?;
    let expr = match &args[0] {
        Value::Compiled(c) => c.clone(),
        other => {
            let cache = i.symbols().borrow().cache.clone();
            let compiler = Compiler::new(cache);
            compiler.compile(other).map_err(Signal::Error)?
        }
    };
    i.execute(expr, crate::env::EnvFrame::empty_top_level())
}

fn proc_gensym(i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 0, "gensym")?;
    let sym = i.symbols().borrow_mut().gensym();
    Ok(Value::Symbol(sym))
}

fn proc_gensym_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "gensym?")?;
    let sym = expect_symbol(&args[0], "gensym?")?;
    Ok(Value::Bool(sym.is_gensym()))
}

fn proc_list_to_vector(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "list->vector")?;
    let vec = args[0].to_proper_vec().map_err(Signal::Error)?;
    Ok(Value::Vector(Rc::new(RefCell::new(vec))))
}

fn proc_make_vector(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    if args.is_empty() || args.len() > 2 {
        return Err(Signal::Error("make-vector expects 1 or 2 arguments".to_string()));
    }
    let n = expect_int(&args[0], "make-vector")?;
    if n < 0 {
        return Err(Signal::Error("make-vector: negative length".to_string()));
    }
    let fill = if args.len() == 2 {
        args[1].clone()
    } else {
        Value::Void
    };
    Ok(Value::Vector(Rc::new(RefCell::new(vec![fill; n as usize]))))
}

fn proc_vector_set(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 3, "vector-set!")?;
    let idx = expect_int(&args[1], "vector-set!")?;
    match &args[0] {
        Value::Vector(v) => {
            let mut borrowed = v.borrow_mut();
            if idx >= 0 && (idx as usize) < borrowed.len() {
                borrowed[idx as usize] = args[2].clone();
                Ok(Value::Void)
            } else {
                Err(Signal::Error(format!(
                    "Index {idx} out of range [0, {}]",
                    borrowed.len() as isize - 1
                )))
            }
        }
        other => Err(Signal::Error(format!(
            "vector-set! expected vector, got {other}"
        ))),
    }
}

fn proc_get_ticks(_i: &mut Interpreter, _args: &[Value], _env: &Env) -> Result<Value, Signal> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // Match .NET DateTime.Ticks unit (1 tick = 100 nanoseconds)
    Ok(Value::Int((nanos / 100) as i64))
}

fn proc_div(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 2, "/")?;
    let a = expect_int(&args[0], "/")?;
    let b = expect_int(&args[1], "/")?;
    if b == 0 {
        return Err(Signal::Error("Division by zero".to_string()));
    }
    Ok(Value::Int(a.wrapping_div(b)))
}

fn expect_procedure(val: &Value, name: &str) -> Result<Procedure, Signal> {
    match val {
        Value::Procedure(p) => Ok(p.clone()),
        other => Err(Signal::Error(format!(
            "{name} expected procedure, got {other}"
        ))),
    }
}

fn expect_prompt_tag(val: &Value, name: &str) -> Result<PromptTag, Signal> {
    match val {
        Value::PromptTag(tag) => Ok(tag.clone()),
        other => Err(Signal::Error(format!(
            "{name} expected continuation prompt tag, got {other}"
        ))),
    }
}

fn proc_call_cc(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "call-with-current-continuation")?;
    let proc = expect_procedure(&args[0], "call/cc")?;
    Err(Signal::Capture {
        target: CaptureTarget::Undelimited,
        proc,
        frames: Vec::new(),
    })
}

fn proc_make_continuation_prompt_tag(
    _i: &mut Interpreter,
    args: &[Value],
    _env: &Env,
) -> Result<Value, Signal> {
    if args.len() > 1 {
        return Err(Signal::Error(format!(
            "make-continuation-prompt-tag expects 0 or 1 argument(s), got {}",
            args.len()
        )));
    }
    let name = match args.first() {
        None => None,
        Some(Value::Symbol(s)) => Some(s.as_str().to_string()),
        Some(Value::Str(s)) => Some(s.as_str().to_string()),
        Some(other) => {
            return Err(Signal::Error(format!(
                "make-continuation-prompt-tag expected symbol or string name, got {other}"
            )))
        }
    };
    Ok(Value::PromptTag(PromptTag::new(name)))
}

fn proc_default_continuation_prompt_tag(
    i: &mut Interpreter,
    args: &[Value],
    _env: &Env,
) -> Result<Value, Signal> {
    expect_arity(args, 0, "default-continuation-prompt-tag")?;
    Ok(Value::PromptTag(i.default_prompt_tag().clone()))
}

fn proc_continuation_prompt_tag_pred(
    _i: &mut Interpreter,
    args: &[Value],
    _env: &Env,
) -> Result<Value, Signal> {
    expect_arity(args, 1, "continuation-prompt-tag?")?;
    Ok(Value::Bool(matches!(args[0], Value::PromptTag(_))))
}

fn proc_call_with_continuation_prompt(
    i: &mut Interpreter,
    args: &[Value],
    env: &Env,
) -> Result<Value, Signal> {
    if args.is_empty() || args.len() > 3 {
        return Err(Signal::Error(format!(
            "call-with-continuation-prompt expects 1 to 3 argument(s), got {}",
            args.len()
        )));
    }
    let thunk = expect_procedure(&args[0], "call-with-continuation-prompt")?;
    let tag = if args.len() >= 2 {
        expect_prompt_tag(&args[1], "call-with-continuation-prompt")?
    } else {
        i.default_prompt_tag().clone()
    };
    let handler = if args.len() == 3 {
        match &args[2] {
            Value::Bool(false) => None,
            other => Some(expect_procedure(other, "call-with-continuation-prompt")?),
        }
    } else {
        None
    };
    i.call_with_prompt(&thunk, &tag, handler.as_ref(), env)
}

fn proc_abort_current_continuation(
    _i: &mut Interpreter,
    args: &[Value],
    _env: &Env,
) -> Result<Value, Signal> {
    if args.is_empty() {
        return Err(Signal::Error(
            "abort-current-continuation expects at least a prompt tag".to_string(),
        ));
    }
    let tag = expect_prompt_tag(&args[0], "abort-current-continuation")?;
    Err(Signal::Abort {
        tag,
        args: args[1..].to_vec(),
    })
}

fn proc_call_with_composable_continuation(
    i: &mut Interpreter,
    args: &[Value],
    _env: &Env,
) -> Result<Value, Signal> {
    if args.is_empty() || args.len() > 2 {
        return Err(Signal::Error(format!(
            "call-with-composable-continuation expects 1 or 2 argument(s), got {}",
            args.len()
        )));
    }
    let proc = expect_procedure(&args[0], "call-with-composable-continuation")?;
    let tag = if args.len() == 2 {
        expect_prompt_tag(&args[1], "call-with-composable-continuation")?
    } else {
        i.default_prompt_tag().clone()
    };
    Err(Signal::Capture {
        target: CaptureTarget::Composable(tag),
        proc,
        frames: Vec::new(),
    })
}

fn proc_compile(i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "_compile")?;
    let cache = i.symbols().borrow().cache.clone();
    let compiler = Compiler::new(cache);
    let compiled = compiler.compile(&args[0]).map_err(Signal::Error)?;
    Ok(Value::Compiled(compiled))
}

fn proc_char_pred(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    expect_arity(args, 1, "char?")?;
    Ok(Value::Bool(matches!(args[0], Value::Char(_))))
}

fn proc_error(_i: &mut Interpreter, args: &[Value], _env: &Env) -> Result<Value, Signal> {
    let parts: Vec<String> = args.iter().map(ToString::to_string).collect();
    Err(Signal::Error(format!("Error: {}", parts.join(" "))))
}
