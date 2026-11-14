use std::rc::Rc;

use crate::symbol::Symbol;
use crate::value::Value;

#[derive(Clone, Debug)]
pub enum Formals {
    Fixed(Vec<Symbol>),
    Variadic { fixed: Vec<Symbol>, rest: Symbol },
}

#[derive(Clone, Debug)]
pub enum ApplyArgs {
    Normal(Vec<Rc<Expr>>),
    Multiple(Rc<Expr>),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Constant(Value),
    Variable(Symbol),
    Conditional {
        test: Rc<Expr>,
        consequent: Rc<Expr>,
        alternate: Rc<Expr>,
    },
    Closure {
        formals: Formals,
        args_display: Value,
        body: Rc<Expr>,
    },
    Assign {
        id: Symbol,
        expr: Rc<Expr>,
    },
    Define {
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
