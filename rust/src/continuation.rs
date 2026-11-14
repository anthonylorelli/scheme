use std::rc::Rc;

use crate::context::Context;

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
