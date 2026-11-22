use std::rc::Rc;

use crate::instruction::{ApplyArgs, Arity, Expr};
use crate::symbol::{Symbol, SymbolCache};
use crate::value::Value;

struct CompileScope {
    names: Vec<Symbol>,
    /// While compiling letrec initializers synchronously, slots >= limit are not yet
    /// initialized unless accessed from inside a nested lambda.
    sync_visible_limit: Option<usize>,
    is_lambda: bool,
    pushes_runtime_frame: bool,
}

pub struct Compiler {
    cache: SymbolCache,
}

impl Compiler {
    pub fn new(cache: SymbolCache) -> Self {
        Self { cache }
    }

    pub fn compile(&self, expr: &Value) -> Result<Rc<Expr>, String> {
        let mut scopes = Vec::new();
        self.compile_in(expr, &mut scopes)
    }

    fn resolve(scopes: &[CompileScope], sym: &Symbol) -> Option<(u32, u32)> {
        let mut crossed_lambda = false;
        let mut runtime_depth: u32 = 0;
        for scope in scopes.iter().rev() {
            if scope.pushes_runtime_frame {
                let search_len = match scope.sync_visible_limit {
                    Some(limit) if !crossed_lambda => limit,
                    _ => scope.names.len(),
                };
                if let Some(slot) = scope.names[..search_len].iter().rposition(|s| s == sym) {
                    return Some((runtime_depth, slot as u32));
                }
                runtime_depth += 1;
            }
            if scope.is_lambda {
                crossed_lambda = true;
            }
        }
        None
    }

    fn collect_internal_defines(&self, expr: &Value, out: &mut Vec<Symbol>) {
        let Value::Pair(p) = expr else {
            return;
        };
        let (head, tail) = {
            let b = p.borrow();
            (b.head.clone(), b.tail.clone())
        };
        if let Value::Symbol(ref sym) = head {
            if *sym == self.cache.quote || *sym == self.cache.lambda || *sym == self.cache.letrec {
                return;
            }
            if *sym == self.cache.define {
                if let Ok(items) = tail.to_proper_vec() {
                    if let Some(first) = items.first() {
                        match first {
                            Value::Symbol(id) => {
                                if !out.contains(id) {
                                    out.push(id.clone());
                                }
                                for rhs in &items[1..] {
                                    self.collect_internal_defines(rhs, out);
                                }
                            }
                            Value::Pair(sig_pair) => {
                                let name = sig_pair.borrow().head.clone();
                                if let Value::Symbol(id) = name {
                                    if !out.contains(&id) {
                                        out.push(id);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                return;
            }
            if *sym == self.cache.begin {
                if let Ok(items) = tail.to_proper_vec() {
                    for item in &items {
                        self.collect_internal_defines(item, out);
                    }
                }
                return;
            }
        }
        if let Ok(items) = tail.to_proper_vec() {
            self.collect_internal_defines(&head, out);
            for item in &items {
                self.collect_internal_defines(item, out);
            }
        }
    }

    fn compile_in(&self, expr: &Value, scopes: &mut Vec<CompileScope>) -> Result<Rc<Expr>, String> {
        match expr {
            Value::Pair(p) => {
                let borrowed = p.borrow();
                let head = borrowed.head.clone();
                let tail = borrowed.tail.clone();
                drop(borrowed);

                if let Value::Symbol(ref sym) = head {
                    if *sym == self.cache.r#if {
                        return self.compile_if(&tail, scopes);
                    } else if *sym == self.cache.lambda {
                        return self.compile_lambda(&tail, scopes);
                    } else if *sym == self.cache.set {
                        return self.compile_set(&tail, scopes);
                    } else if *sym == self.cache.define {
                        return self.compile_define(&tail, scopes);
                    } else if *sym == self.cache.quote {
                        let quoted = tail
                            .first()
                            .ok_or_else(|| "quote requires an argument".to_string())?;
                        return Ok(Rc::new(Expr::Constant(quoted)));
                    } else if *sym == self.cache.begin {
                        let items = tail.to_proper_vec()?;
                        return self.compile_sequence(&items, scopes);
                    } else if *sym == self.cache.letrec {
                        return self.compile_letrec(&tail, scopes);
                    } else if *sym == self.cache.call_with_values {
                        return self.compile_call_with_values(&tail, scopes);
                    }
                }

                self.compile_application(&head, &tail, scopes)
            }
            Value::Symbol(sym) => {
                if let Some((depth, slot)) = Self::resolve(scopes, sym) {
                    Ok(Rc::new(Expr::LocalVar { depth, slot }))
                } else {
                    Ok(Rc::new(Expr::GlobalVar(sym.clone())))
                }
            }
            other => Ok(Rc::new(Expr::Constant(other.clone()))),
        }
    }

    fn compile_if(&self, tail: &Value, scopes: &mut Vec<CompileScope>) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.len() < 2 || items.len() > 3 {
            return Err(format!("if expects 2 or 3 arguments, got {}", items.len()));
        }
        let test = self.compile_in(&items[0], scopes)?;
        let consequent = self.compile_in(&items[1], scopes)?;
        let alternate = if items.len() == 3 {
            self.compile_in(&items[2], scopes)?
        } else {
            Rc::new(Expr::Constant(Value::Void))
        };
        Ok(Rc::new(Expr::Conditional {
            test,
            consequent,
            alternate,
        }))
    }

    fn parse_formals(raw: &Value) -> Result<(Arity, Vec<Symbol>, Value), String> {
        match raw {
            Value::EmptyList => Ok((Arity::Fixed(0), Vec::new(), Value::EmptyList)),
            Value::Symbol(sym) => Ok((
                Arity::Variadic { fixed: 0 },
                vec![sym.clone()],
                Value::list_from_vec(vec![Value::Symbol(sym.clone())]),
            )),
            Value::Pair(_) => {
                let mut params = Vec::new();
                let mut display_syms = Vec::new();
                let mut cur = raw.clone();
                loop {
                    match cur {
                        Value::EmptyList => {
                            let arity = Arity::Fixed(params.len());
                            return Ok((arity, params, Value::list_from_vec(display_syms)));
                        }
                        Value::Symbol(rest_sym) => {
                            let fixed = params.len();
                            display_syms.push(Value::Symbol(rest_sym.clone()));
                            params.push(rest_sym);
                            return Ok((
                                Arity::Variadic { fixed },
                                params,
                                Value::list_from_vec(display_syms),
                            ));
                        }
                        Value::Pair(p) => {
                            let borrowed = p.borrow();
                            let h = borrowed.head.clone();
                            let t = borrowed.tail.clone();
                            drop(borrowed);
                            match h {
                                Value::Symbol(s) => {
                                    display_syms.push(Value::Symbol(s.clone()));
                                    params.push(s);
                                    cur = t;
                                }
                                other => {
                                    return Err(format!(
                                        "Invalid lambda parameter (expected symbol): {other}"
                                    ));
                                }
                            }
                        }
                        other => {
                            return Err(format!("Invalid lambda formals tail: {other}"));
                        }
                    }
                }
            }
            other => Err(format!("Invalid lambda formals: {other}")),
        }
    }

    fn compile_lambda_parts(
        &self,
        raw_formals: &Value,
        body_list: &[Value],
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        let (arity, mut names, args_display) = Self::parse_formals(raw_formals)?;
        let param_count = names.len();

        let mut internal_defs = Vec::new();
        for expr in body_list {
            self.collect_internal_defines(expr, &mut internal_defs);
        }
        for def_sym in internal_defs {
            if !names.contains(&def_sym) {
                names.push(def_sym);
            }
        }

        let extra_locals = names.len() - param_count;
        let has_frame = !names.is_empty();

        scopes.push(CompileScope {
            names,
            sync_visible_limit: None,
            is_lambda: true,
            pushes_runtime_frame: has_frame,
        });
        let body = self.compile_sequence(body_list, scopes)?;
        scopes.pop();

        Ok(Rc::new(Expr::Closure {
            arity,
            extra_locals,
            has_frame,
            args_display,
            body,
        }))
    }

    fn compile_lambda(
        &self,
        tail: &Value,
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        let raw_formals = tail
            .first()
            .ok_or_else(|| "lambda missing parameter list".to_string())?;
        let body_list = tail
            .rest()
            .ok_or_else(|| "lambda missing body".to_string())?
            .to_proper_vec()?;
        self.compile_lambda_parts(&raw_formals, &body_list, scopes)
    }

    fn expect_symbol(val: &Value, form: &str) -> Result<Symbol, String> {
        match val {
            Value::Symbol(s) => Ok(s.clone()),
            other => Err(format!("{form} expected symbol identifier, got {other}")),
        }
    }

    fn compile_set(
        &self,
        tail: &Value,
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.len() != 2 {
            return Err("set! expects 2 arguments".to_string());
        }
        let id = Self::expect_symbol(&items[0], "set!")?;
        let expr = self.compile_in(&items[1], scopes)?;
        if let Some((depth, slot)) = Self::resolve(scopes, &id) {
            Ok(Rc::new(Expr::AssignLocal { depth, slot, expr }))
        } else {
            Ok(Rc::new(Expr::AssignGlobal { id, expr }))
        }
    }

    fn compile_define(
        &self,
        tail: &Value,
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.is_empty() {
            return Err("define expects arguments".to_string());
        }

        // Also support shorthand `(define (f x ...) body ...)` if unexpanded
        let (id, val_expr) = if let Value::Pair(sig_pair) = &items[0] {
            let (name_val, params_val) = {
                let borrowed = sig_pair.borrow();
                (borrowed.head.clone(), borrowed.tail.clone())
            };
            let id = Self::expect_symbol(&name_val, "define")?;
            let closure = self.compile_lambda_parts(&params_val, &items[1..], scopes)?;
            (id, closure)
        } else {
            if items.len() != 2 {
                return Err("define expects 2 arguments".to_string());
            }
            let id = Self::expect_symbol(&items[0], "define")?;
            let expr = self.compile_in(&items[1], scopes)?;
            (id, expr)
        };

        if let Some(scope) = scopes.last() {
            debug_assert!(scope.pushes_runtime_frame);
            let slot = scope
                .names
                .iter()
                .rposition(|s| s == &id)
                .expect("internal define was not pre-collected into enclosing frame")
                as u32;
            Ok(Rc::new(Expr::AssignLocal {
                depth: 0,
                slot,
                expr: val_expr,
            }))
        } else {
            Ok(Rc::new(Expr::DefineGlobal { id, expr: val_expr }))
        }
    }

    fn compile_letrec(
        &self,
        tail: &Value,
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        let bindings_val = tail
            .first()
            .ok_or_else(|| "letrec missing bindings".to_string())?;
        let body_vals = tail
            .rest()
            .ok_or_else(|| "letrec missing body".to_string())?
            .to_proper_vec()?;

        let binding_items = bindings_val.to_proper_vec()?;
        let mut bindings: Vec<(Symbol, Value)> = Vec::with_capacity(binding_items.len());
        let mut names: Vec<Symbol> = Vec::with_capacity(binding_items.len());
        let mut slot_indices: Vec<usize> = Vec::with_capacity(binding_items.len());

        for binding in binding_items {
            if matches!(binding, Value::EmptyList) {
                break;
            }
            let pair_items = binding.to_proper_vec()?;
            if pair_items.len() != 2 {
                return Err("Invalid letrec binding".to_string());
            }
            let id = Self::expect_symbol(&pair_items[0], "letrec")?;
            let slot = if let Some(existing) = names.iter().position(|s| s == &id) {
                existing
            } else {
                let idx = names.len();
                names.push(id.clone());
                idx
            };
            slot_indices.push(slot);
            bindings.push((id, pair_items[1].clone()));
        }

        let mut internal_defs = Vec::new();
        for (_, init_val) in &bindings {
            self.collect_internal_defines(init_val, &mut internal_defs);
        }
        for expr in &body_vals {
            self.collect_internal_defines(expr, &mut internal_defs);
        }
        for def_sym in internal_defs {
            if !names.contains(&def_sym) {
                names.push(def_sym);
            }
        }

        if names.is_empty() {
            return self.compile_sequence(&body_vals, scopes);
        }

        let slot_count = names.len();
        scopes.push(CompileScope {
            names,
            sync_visible_limit: Some(0),
            is_lambda: false,
            pushes_runtime_frame: true,
        });

        let mut seq_compiled: Vec<Rc<Expr>> = Vec::with_capacity(bindings.len() + body_vals.len());
        for (idx, (_, init_val)) in bindings.iter().enumerate() {
            if let Some(top) = scopes.last_mut() {
                top.sync_visible_limit = Some(slot_indices[idx]);
            }
            let init_expr = self.compile_in(init_val, scopes)?;
            seq_compiled.push(Rc::new(Expr::AssignLocal {
                depth: 0,
                slot: slot_indices[idx] as u32,
                expr: init_expr,
            }));
        }

        if let Some(top) = scopes.last_mut() {
            top.sync_visible_limit = None;
        }
        for expr in &body_vals {
            seq_compiled.push(self.compile_in(expr, scopes)?);
        }

        scopes.pop();

        let body = Self::link_sequence(seq_compiled);
        Ok(Rc::new(Expr::PushFrame { slot_count, body }))
    }

    fn compile_call_with_values(
        &self,
        tail: &Value,
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.len() != 2 {
            return Err("call-with-values expects 2 arguments".to_string());
        }
        let producer_fn = self.compile_in(&items[0], scopes)?;
        let consumer_fn = self.compile_in(&items[1], scopes)?;

        let producer_call = Rc::new(Expr::Apply {
            rator: producer_fn,
            args: ApplyArgs::Normal(Vec::new()),
        });
        Ok(Rc::new(Expr::Apply {
            rator: consumer_fn,
            args: ApplyArgs::Multiple(producer_call),
        }))
    }

    fn link_sequence(compiled: Vec<Rc<Expr>>) -> Rc<Expr> {
        if compiled.is_empty() {
            return Rc::new(Expr::Constant(Value::Void));
        }
        let mut iter = compiled.into_iter().rev();
        let mut acc = iter.next().expect("non-empty sequence");
        for prev in iter {
            acc = Rc::new(Expr::Sequence {
                first: prev,
                rest: acc,
            });
        }
        acc
    }

    fn compile_sequence(
        &self,
        exprs: &[Value],
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        if exprs.is_empty() {
            return Ok(Rc::new(Expr::Constant(Value::Void)));
        }
        let mut compiled = Vec::with_capacity(exprs.len());
        for e in exprs {
            compiled.push(self.compile_in(e, scopes)?);
        }
        Ok(Self::link_sequence(compiled))
    }

    fn compile_application(
        &self,
        rator: &Value,
        rands: &Value,
        scopes: &mut Vec<CompileScope>,
    ) -> Result<Rc<Expr>, String> {
        let compiled_rator = self.compile_in(rator, scopes)?;
        let rand_vals = rands.to_proper_vec()?;
        let mut compiled_rands = Vec::with_capacity(rand_vals.len());
        for rand in &rand_vals {
            compiled_rands.push(self.compile_in(rand, scopes)?);
        }
        Ok(Rc::new(Expr::Apply {
            rator: compiled_rator,
            args: ApplyArgs::Normal(compiled_rands),
        }))
    }
}
