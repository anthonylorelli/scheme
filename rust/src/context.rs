use std::rc::Rc;

use crate::env::Env;
use crate::instruction::Expr;
use crate::symbol::Symbol;
use crate::value::Value;

#[derive(Clone, Debug)]
pub enum Context {
    Halt,
    Cond {
        consequent: Rc<Expr>,
        alternate: Rc<Expr>,
        env: Env,
    },
    Assign {
        id: Symbol,
        env: Env,
    },
    Define {
        id: Symbol,
        env: Env,
    },
    ArgEval {
        remaining_left: Vec<Rc<Expr>>,
        evaluated_rev: Vec<Value>,
        env: Env,
    },
    Func {
        rator: Rc<Expr>,
        env: Env,
    },
    Apply {
        args_rib: Vec<Value>,
        env: Env,
    },
    Seq {
        next: Rc<Expr>,
        env: Env,
    },
    MultArg,
}
