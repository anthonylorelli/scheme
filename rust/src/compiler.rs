use std::rc::Rc;

use crate::instruction::{ApplyArgs, Expr, Formals};
use crate::symbol::{Symbol, SymbolCache};
use crate::value::Value;

pub struct Compiler {
    cache: SymbolCache,
}

impl Compiler {
    pub fn new(cache: SymbolCache) -> Self {
        Self { cache }
    }

    pub fn compile(&self, expr: &Value) -> Result<Rc<Expr>, String> {
        match expr {
            Value::Pair(p) => {
                let borrowed = p.borrow();
                let head = borrowed.head.clone();
                let tail = borrowed.tail.clone();
                drop(borrowed);

                if let Value::Symbol(ref sym) = head {
                    if *sym == self.cache.r#if {
                        return self.compile_if(&tail);
                    } else if *sym == self.cache.lambda {
                        return self.compile_lambda(&tail);
                    } else if *sym == self.cache.set {
                        return self.compile_set(&tail);
                    } else if *sym == self.cache.define {
                        return self.compile_define(&tail);
                    } else if *sym == self.cache.quote {
                        let quoted = tail
                            .first()
                            .ok_or_else(|| "quote requires an argument".to_string())?;
                        return Ok(Rc::new(Expr::Constant(quoted)));
                    } else if *sym == self.cache.begin {
                        let items = tail.to_proper_vec()?;
                        return self.compile_sequence(&items);
                    } else if *sym == self.cache.letrec {
                        return self.compile_letrec(&tail);
                    } else if *sym == self.cache.call_with_values {
                        return self.compile_call_with_values(&tail);
                    }
                }

                self.compile_application(&head, &tail)
            }
            Value::Symbol(sym) => Ok(Rc::new(Expr::Variable(sym.clone()))),
            other => Ok(Rc::new(Expr::Constant(other.clone()))),
        }
    }

    fn compile_if(&self, tail: &Value) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.len() < 2 || items.len() > 3 {
            return Err(format!("if expects 2 or 3 arguments, got {}", items.len()));
        }
        let test = self.compile(&items[0])?;
        let consequent = self.compile(&items[1])?;
        let alternate = if items.len() == 3 {
            self.compile(&items[2])?
        } else {
            Rc::new(Expr::Constant(Value::Void))
        };
        Ok(Rc::new(Expr::Conditional {
            test,
            consequent,
            alternate,
        }))
    }

    fn parse_formals(raw: &Value) -> Result<(Formals, Value), String> {
        match raw {
            Value::EmptyList => Ok((Formals::Fixed(Vec::new()), Value::EmptyList)),
            Value::Symbol(sym) => Ok((
                Formals::Variadic {
                    fixed: Vec::new(),
                    rest: sym.clone(),
                },
                Value::list_from_vec(vec![Value::Symbol(sym.clone())]),
            )),
            Value::Pair(_) => {
                let mut fixed = Vec::new();
                let mut display_syms = Vec::new();
                let mut cur = raw.clone();
                loop {
                    match cur {
                        Value::EmptyList => {
                            return Ok((Formals::Fixed(fixed), Value::list_from_vec(display_syms)));
                        }
                        Value::Symbol(rest_sym) => {
                            display_syms.push(Value::Symbol(rest_sym.clone()));
                            return Ok((
                                Formals::Variadic {
                                    fixed,
                                    rest: rest_sym,
                                },
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
                                    fixed.push(s);
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

    fn compile_lambda(&self, tail: &Value) -> Result<Rc<Expr>, String> {
        let raw_formals = tail
            .first()
            .ok_or_else(|| "lambda missing parameter list".to_string())?;
        let body_list = tail
            .rest()
            .ok_or_else(|| "lambda missing body".to_string())?
            .to_proper_vec()?;
        let (formals, args_display) = Self::parse_formals(&raw_formals)?;
        let body = self.compile_sequence(&body_list)?;
        Ok(Rc::new(Expr::Closure {
            formals,
            args_display,
            body,
        }))
    }

    fn expect_symbol(val: &Value, form: &str) -> Result<Symbol, String> {
        match val {
            Value::Symbol(s) => Ok(s.clone()),
            other => Err(format!("{form} expected symbol identifier, got {other}")),
        }
    }

    fn compile_set(&self, tail: &Value) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.len() != 2 {
            return Err("set! expects 2 arguments".to_string());
        }
        let id = Self::expect_symbol(&items[0], "set!")?;
        let expr = self.compile(&items[1])?;
        Ok(Rc::new(Expr::Assign { id, expr }))
    }

    fn compile_define(&self, tail: &Value) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.is_empty() {
            return Err("define expects arguments".to_string());
        }
        // Also support shorthand `(define (f x ...) body ...)` if unexpanded
        if let Value::Pair(sig_pair) = &items[0] {
            let borrowed = sig_pair.borrow();
            let name_val = borrowed.head.clone();
            let params_val = borrowed.tail.clone();
            drop(borrowed);
            let id = Self::expect_symbol(&name_val, "define")?;
            let (formals, args_display) = Self::parse_formals(&params_val)?;
            let body = self.compile_sequence(&items[1..])?;
            let closure = Rc::new(Expr::Closure {
                formals,
                args_display,
                body,
            });
            return Ok(Rc::new(Expr::Define { id, expr: closure }));
        }

        if items.len() != 2 {
            return Err("define expects 2 arguments".to_string());
        }
        let id = Self::expect_symbol(&items[0], "define")?;
        let expr = self.compile(&items[1])?;
        Ok(Rc::new(Expr::Define { id, expr }))
    }

    fn compile_letrec(&self, tail: &Value) -> Result<Rc<Expr>, String> {
        let bindings_val = tail
            .first()
            .ok_or_else(|| "letrec missing bindings".to_string())?;
        let body_vals = tail
            .rest()
            .ok_or_else(|| "letrec missing body".to_string())?
            .to_proper_vec()?;

        let binding_items = bindings_val.to_proper_vec()?;
        let mut seq_exprs = Vec::with_capacity(binding_items.len() + body_vals.len());

        for binding in binding_items {
            if matches!(binding, Value::EmptyList) {
                break;
            }
            let pair_items = binding.to_proper_vec()?;
            if pair_items.len() != 2 {
                return Err("Invalid letrec binding".to_string());
            }
            let def_expr = Value::list_from_vec(vec![
                Value::Symbol(self.cache.define.clone()),
                pair_items[0].clone(),
                pair_items[1].clone(),
            ]);
            seq_exprs.push(def_expr);
        }
        seq_exprs.extend(body_vals);

        let compiled_body = self.compile_sequence(&seq_exprs)?;
        let closure = Rc::new(Expr::Closure {
            formals: Formals::Fixed(Vec::new()),
            args_display: Value::EmptyList,
            body: compiled_body,
        });
        Ok(Rc::new(Expr::Apply {
            rator: closure,
            args: ApplyArgs::Normal(Vec::new()),
        }))
    }

    fn compile_call_with_values(&self, tail: &Value) -> Result<Rc<Expr>, String> {
        let items = tail.to_proper_vec()?;
        if items.len() != 2 {
            return Err("call-with-values expects 2 arguments".to_string());
        }
        let producer_fn = self.compile(&items[0])?;
        let consumer_fn = self.compile(&items[1])?;

        let producer_call = Rc::new(Expr::Apply {
            rator: producer_fn,
            args: ApplyArgs::Normal(Vec::new()),
        });
        Ok(Rc::new(Expr::Apply {
            rator: consumer_fn,
            args: ApplyArgs::Multiple(producer_call),
        }))
    }

    fn compile_sequence(&self, exprs: &[Value]) -> Result<Rc<Expr>, String> {
        if exprs.is_empty() {
            return Ok(Rc::new(Expr::Constant(Value::Void)));
        }
        let mut compiled = Vec::with_capacity(exprs.len());
        for e in exprs {
            compiled.push(self.compile(e)?);
        }
        let mut iter = compiled.into_iter().rev();
        let mut acc = iter.next().expect("non-empty sequence");
        for prev in iter {
            acc = Rc::new(Expr::Sequence {
                first: prev,
                rest: acc,
            });
        }
        Ok(acc)
    }

    fn compile_application(&self, rator: &Value, rands: &Value) -> Result<Rc<Expr>, String> {
        let compiled_rator = self.compile(rator)?;
        let rand_vals = rands.to_proper_vec()?;
        let mut compiled_rands = Vec::with_capacity(rand_vals.len());
        for rand in &rand_vals {
            compiled_rands.push(self.compile(rand)?);
        }
        Ok(Rc::new(Expr::Apply {
            rator: compiled_rator,
            args: ApplyArgs::Normal(compiled_rands),
        }))
    }
}
