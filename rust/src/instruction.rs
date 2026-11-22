use std::rc::Rc;

use crate::symbol::Symbol;
use crate::value::Value;

#[derive(Clone, Copy, Debug)]
pub enum Arity {
    Fixed(usize),
    Variadic { fixed: usize },
}

#[derive(Clone, Debug)]
pub enum ApplyArgs {
    Normal(Vec<Rc<Expr>>),
    Multiple(Rc<Expr>),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Constant(Value),
    LocalVar {
        depth: u32,
        slot: u32,
    },
    GlobalVar(Symbol),
    Conditional {
        test: Rc<Expr>,
        consequent: Rc<Expr>,
        alternate: Rc<Expr>,
    },
    Closure {
        arity: Arity,
        extra_locals: usize,
        has_frame: bool,
        args_display: Value,
        body: Rc<Expr>,
    },
    PushFrame {
        slot_count: usize,
        body: Rc<Expr>,
    },
    AssignLocal {
        depth: u32,
        slot: u32,
        expr: Rc<Expr>,
    },
    AssignGlobal {
        id: Symbol,
        expr: Rc<Expr>,
    },
    DefineGlobal {
        id: Symbol,
        expr: Rc<Expr>,
    },
    Apply {
        rator: Rc<Expr>,
        args: ApplyArgs,
    },
    Sequence {
        first: Rc<Expr>,
        rest: Rc<Expr>,
    },
}
