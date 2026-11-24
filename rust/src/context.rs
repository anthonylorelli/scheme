use std::rc::Rc;

use crate::continuation::PromptTag;
use crate::env::Env;
use crate::instruction::Expr;
use crate::symbol::Symbol;
use crate::value::{Procedure, Value};

#[derive(Clone, Debug)]
pub enum Context {
    Halt,
    Prompt {
        tag: PromptTag,
        handler: Option<Procedure>,
    },
    Cond {
        consequent: Rc<Expr>,
        alternate: Rc<Expr>,
        env: Env,
    },
    AssignLocal {
        depth: u32,
        slot: u32,
        env: Env,
    },
    AssignGlobal {
        id: Symbol,
    },
    DefineGlobal {
        id: Symbol,
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
