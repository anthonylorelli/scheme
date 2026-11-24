use std::fmt;
use std::rc::Rc;

use crate::context::Context;

struct PromptTagInner {
    name: Option<String>,
}

#[derive(Clone)]
pub struct PromptTag(Rc<PromptTagInner>);

impl PromptTag {
    pub fn new(name: Option<String>) -> Self {
        Self(Rc::new(PromptTagInner { name }))
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq for PromptTag {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl Eq for PromptTag {}

impl fmt::Debug for PromptTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for PromptTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.name {
            Some(name) => write!(f, "<#prompt-tag:{name}>"),
            None => f.write_str("<#prompt-tag>"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Continuation {
    frames: Rc<Vec<Context>>,
}

impl Continuation {
    pub fn new(frames: Vec<Context>) -> Self {
        Self {
            frames: Rc::new(frames),
        }
    }

    pub fn frames(&self) -> &Rc<Vec<Context>> {
        &self.frames
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.frames, &other.frames)
    }
}
