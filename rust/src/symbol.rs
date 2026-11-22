use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::value::Value;

struct SymbolInner {
    name: String,
    is_gensym: bool,
    props: RefCell<HashMap<Symbol, Value>>,
    global_val: RefCell<Option<Value>>,
}

#[derive(Clone)]
pub struct Symbol(Rc<SymbolInner>);

impl Symbol {
    fn new(name: impl Into<String>, is_gensym: bool) -> Self {
        Self(Rc::new(SymbolInner {
            name: name.into(),
            is_gensym,
            props: RefCell::new(HashMap::new()),
            global_val: RefCell::new(None),
        }))
    }

    pub fn as_str(&self) -> &str {
        &self.0.name
    }

    pub fn is_gensym(&self) -> bool {
        self.0.is_gensym
    }

    #[inline]
    pub fn get_global(&self) -> Option<Value> {
        self.0.global_val.borrow().clone()
    }

    #[inline]
    pub fn define_global(&self, val: Value) {
        *self.0.global_val.borrow_mut() = Some(val);
    }

    #[inline]
    pub fn assign_global(&self, val: Value) -> bool {
        let mut slot = self.0.global_val.borrow_mut();
        if slot.is_some() {
            *slot = Some(val);
            true
        } else {
            false
        }
    }

    pub fn clear_global(&self) {
        *self.0.global_val.borrow_mut() = None;
    }

    pub fn put_prop(&self, key: Symbol, val: Value) {
        self.0.props.borrow_mut().insert(key, val);
    }

    pub fn get_prop(&self, key: &Symbol) -> Option<Value> {
        self.0.props.borrow().get(key).cloned()
    }

    pub fn rem_prop(&self, key: &Symbol) {
        self.0.props.borrow_mut().remove(key);
    }

    pub fn clear_props(&self) {
        self.0.props.borrow_mut().clear();
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Symbol {}

impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0) as usize).hash(state);
    }
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Symbol({})", self.0.name)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.name)
    }
}

#[derive(Clone)]
pub struct SymbolCache {
    pub r#if: Symbol,
    pub set: Symbol,
    pub define: Symbol,
    pub lambda: Symbol,
    pub quote: Symbol,
    pub letrec: Symbol,
    pub begin: Symbol,
    pub and: Symbol,
    pub or: Symbol,
    pub expander: Symbol,
    pub quasiquote: Symbol,
    pub unquote: Symbol,
    pub unquote_splicing: Symbol,
    pub call_with_values: Symbol,
    pub call_with_current_continuation: Symbol,
    pub call_cc: Symbol,
}

pub struct SymbolTable {
    table: HashMap<String, Symbol>,
    gensym_counter: usize,
    pub cache: SymbolCache,
}

impl SymbolTable {
    const GENSYM_PREFIX: &'static str = "__%sym";

    pub fn new() -> Self {
        let mut table: HashMap<String, Symbol> = HashMap::new();
        let mut intern_init = |s: &str| -> Symbol {
            if let Some(existing) = table.get(s) {
                existing.clone()
            } else {
                let sym = Symbol::new(s, false);
                table.insert(s.to_string(), sym.clone());
                sym
            }
        };

        let cache = SymbolCache {
            r#if: intern_init("if"),
            set: intern_init("set!"),
            define: intern_init("define"),
            lambda: intern_init("lambda"),
            quote: intern_init("quote"),
            letrec: intern_init("letrec"),
            begin: intern_init("begin"),
            and: intern_init("and"),
            or: intern_init("or"),
            expander: intern_init("_expander"),
            quasiquote: intern_init("quasiquote"),
            unquote: intern_init("unquote"),
            unquote_splicing: intern_init("unquote-splicing"),
            call_with_values: intern_init("call-with-values"),
            call_with_current_continuation: intern_init("call-with-current-continuation"),
            call_cc: intern_init("call/cc"),
        };

        Self {
            table,
            gensym_counter: 0,
            cache,
        }
    }

    pub fn intern(&mut self, name: &str) -> Symbol {
        if let Some(sym) = self.table.get(name) {
            sym.clone()
        } else {
            let sym = Symbol::new(name, false);
            self.table.insert(name.to_string(), sym.clone());
            sym
        }
    }

    pub fn gensym(&mut self) -> Symbol {
        loop {
            let candidate = format!("{}{}", Self::GENSYM_PREFIX, self.gensym_counter);
            self.gensym_counter += 1;
            if let std::collections::hash_map::Entry::Vacant(entry) = self.table.entry(candidate) {
                let sym = Symbol::new(entry.key().clone(), true);
                entry.insert(sym.clone());
                return sym;
            }
        }
    }

    pub fn clear_all_properties(&self) {
        for sym in self.table.values() {
            sym.clear_props();
        }
    }

    pub fn clear_all_globals(&self) {
        for sym in self.table.values() {
            sym.clear_global();
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
