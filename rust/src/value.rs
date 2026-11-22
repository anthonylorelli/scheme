use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::continuation::Continuation;
use crate::env::Env;
use crate::instruction::{Arity, Expr};
use crate::interpreter::{Interpreter, Signal};
use crate::symbol::Symbol;

#[derive(Clone, Debug)]
pub struct Pair {
    pub head: Value,
    pub tail: Value,
}

pub type NativeFn = fn(&mut Interpreter, &[Value], &Env) -> Result<Value, Signal>;

#[derive(Clone)]
pub struct NativeProcedure {
    pub name: &'static str,
    pub func: NativeFn,
}

#[derive(Clone)]
pub struct Closure {
    pub arity: Arity,
    pub extra_locals: usize,
    pub has_frame: bool,
    pub args_display: Value,
    pub body: Rc<Expr>,
    pub env: Env,
}

#[derive(Clone)]
pub enum Procedure {
    Native(Rc<NativeProcedure>),
    Closure(Rc<Closure>),
    Continuation(Continuation),
}

impl Procedure {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Native(a), Self::Native(b)) => Rc::ptr_eq(a, b),
            (Self::Closure(a), Self::Closure(b)) => Rc::ptr_eq(a, b),
            (Self::Continuation(a), Self::Continuation(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
}

impl fmt::Debug for Procedure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for Procedure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native(n) => write!(f, "<#primitive:{}>", n.name),
            Self::Closure(c) => write!(f, "<#function> Args: {}", c.args_display),
            Self::Continuation(_) => write!(f, "<#continuation>"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Void,
    EmptyList,
    Bool(bool),
    Char(char),
    Int(i64),
    Str(Rc<String>),
    Symbol(Symbol),
    Pair(Rc<RefCell<Pair>>),
    Vector(Rc<RefCell<Vec<Value>>>),
    Procedure(Procedure),
    Values(Box<Vec<Value>>),
    Compiled(Rc<Expr>),
    /// Internal carrier for evaluated argument ribs during continuation resumption.
    Rib(Box<Vec<Value>>),
}

const _: () = assert!(std::mem::size_of::<Value>() == 16);

impl Value {
    pub fn cons(head: Value, tail: Value) -> Self {
        Self::Pair(Rc::new(RefCell::new(Pair { head, tail })))
    }

    pub fn list_from_slice(items: &[Value]) -> Self {
        let mut acc = Self::EmptyList;
        for item in items.iter().rev() {
            acc = Self::cons(item.clone(), acc);
        }
        acc
    }

    pub fn list_from_vec(items: Vec<Value>) -> Self {
        let mut acc = Self::EmptyList;
        for item in items.into_iter().rev() {
            acc = Self::cons(item, acc);
        }
        acc
    }

    pub fn to_proper_vec(&self) -> Result<Vec<Value>, String> {
        let mut out = Vec::new();
        let mut cur = self.clone();
        loop {
            match cur {
                Self::EmptyList => return Ok(out),
                Self::Pair(p) => {
                    let borrowed = p.borrow();
                    out.push(borrowed.head.clone());
                    let next = borrowed.tail.clone();
                    drop(borrowed);
                    cur = next;
                }
                _ => return Err("Error: attempt to take length of non-list.".to_string()),
            }
        }
    }

    pub fn list_length(&self) -> Result<usize, String> {
        let mut len = 0usize;
        let mut cur = self.clone();
        loop {
            match cur {
                Self::EmptyList => return Ok(len),
                Self::Pair(p) => {
                    len += 1;
                    let next = p.borrow().tail.clone();
                    cur = next;
                }
                _ => return Err("Error: attempt to take length of non-list.".to_string()),
            }
        }
    }

    pub fn is_true(&self) -> bool {
        !matches!(self, Self::Bool(false))
    }

    pub fn is_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Void, Self::Void) => true,
            (Self::EmptyList, Self::EmptyList) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Char(a), Self::Char(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => Rc::ptr_eq(a, b),
            (Self::Symbol(a), Self::Symbol(b)) => a == b,
            (Self::Pair(a), Self::Pair(b)) => Rc::ptr_eq(a, b),
            (Self::Vector(a), Self::Vector(b)) => Rc::ptr_eq(a, b),
            (Self::Procedure(a), Self::Procedure(b)) => a.ptr_eq(b),
            (Self::Compiled(a), Self::Compiled(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    pub fn is_eqv(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Char(a), Self::Char(b)) => a == b,
            _ => self.is_eq(other),
        }
    }

    pub fn first(&self) -> Option<Value> {
        match self {
            Self::Pair(p) => Some(p.borrow().head.clone()),
            _ => None,
        }
    }

    pub fn rest(&self) -> Option<Value> {
        match self {
            Self::Pair(p) => Some(p.borrow().tail.clone()),
            _ => None,
        }
    }

    pub fn second(&self) -> Option<Value> {
        self.rest()?.first()
    }

    pub fn third(&self) -> Option<Value> {
        self.rest()?.second()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void => Ok(()),
            Self::EmptyList => f.write_str("()"),
            Self::Bool(true) => f.write_str("#t"),
            Self::Bool(false) => f.write_str("#f"),
            Self::Char(' ') => f.write_str("#\\space"),
            Self::Char('\n') => f.write_str("#\\newline"),
            Self::Char(c) => write!(f, "#\\{c}"),
            Self::Int(n) => write!(f, "{n}"),
            Self::Str(s) => f.write_str(s),
            Self::Symbol(sym) => write!(f, "{sym}"),
            Self::Pair(p) => {
                f.write_str("(")?;
                let borrowed = p.borrow();
                write!(f, "{}", borrowed.head)?;
                let mut rest = borrowed.tail.clone();
                drop(borrowed);
                loop {
                    match rest {
                        Self::EmptyList => break,
                        Self::Pair(next_p) => {
                            f.write_str(" ")?;
                            let nb = next_p.borrow();
                            write!(f, "{}", nb.head)?;
                            let next_tail = nb.tail.clone();
                            drop(nb);
                            rest = next_tail;
                        }
                        other => {
                            write!(f, " . {other}")?;
                            break;
                        }
                    }
                }
                f.write_str(")")
            }
            Self::Vector(vec) => {
                let borrowed = vec.borrow();
                write!(f, "#{}(", borrowed.len())?;
                for (idx, item) in borrowed.iter().enumerate() {
                    if idx > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str(")")
            }
            Self::Procedure(proc) => write!(f, "{proc}"),
            Self::Values(vals) => {
                for (idx, v) in vals.iter().enumerate() {
                    if idx > 0 {
                        f.write_str("\n")?;
                    }
                    write!(f, "{v}")?;
                }
                Ok(())
            }
            Self::Compiled(_) => f.write_str("<#compiled>"),
            Self::Rib(vals) => write!(f, "{}", Self::list_from_slice(vals)),
        }
    }
}
