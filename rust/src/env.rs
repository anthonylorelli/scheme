use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::symbol::Symbol;
use crate::value::Value;

pub type Env = Rc<RefCell<EnvFrame>>;

#[derive(Debug)]
pub struct EnvFrame {
    bindings: HashMap<Symbol, Value>,
    parent: Option<Env>,
}

impl EnvFrame {
    pub fn new_root() -> Env {
        Rc::new(RefCell::new(Self {
            bindings: HashMap::new(),
            parent: None,
        }))
    }

    pub fn with_capacity_and_parent(cap: usize, parent: Option<Env>) -> Env {
        Rc::new(RefCell::new(Self {
            bindings: HashMap::with_capacity(cap),
            parent,
        }))
    }

    pub fn lookup(env: &Env, sym: &Symbol) -> Option<Value> {
        let mut cur = Some(env.clone());
        while let Some(frame_rc) = cur {
            let frame = frame_rc.borrow();
            if let Some(val) = frame.bindings.get(sym) {
                return Some(val.clone());
            }
            cur = frame.parent.clone();
        }
        None
    }

    pub fn assign(env: &Env, sym: &Symbol, val: Value) -> bool {
        let mut cur = Some(env.clone());
        while let Some(frame_rc) = cur {
            let mut frame = frame_rc.borrow_mut();
            if let Some(slot) = frame.bindings.get_mut(sym) {
                *slot = val;
                return true;
            }
            cur = frame.parent.clone();
        }
        false
    }

    pub fn define(env: &Env, sym: Symbol, val: Value) {
        env.borrow_mut().bindings.insert(sym, val);
    }

    pub fn define_in_place(&mut self, sym: Symbol, val: Value) {
        self.bindings.insert(sym, val);
    }

    pub fn clear(env: &Env) {
        env.borrow_mut().bindings.clear();
    }
}
