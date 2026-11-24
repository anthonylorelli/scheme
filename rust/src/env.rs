use std::cell::RefCell;
use std::rc::Rc;

use crate::value::Value;

pub const SMALL_SLOTS: usize = 4;

pub type Env = Option<Rc<EnvFrame>>;

#[derive(Debug)]
enum FrameSlots {
    Inline([Value; SMALL_SLOTS]),
    Heap(Vec<Value>),
}

#[derive(Debug)]
pub struct EnvFrame {
    slots: RefCell<FrameSlots>,
    parent: Env,
}

impl EnvFrame {
    #[inline]
    pub fn empty_top_level() -> Env {
        None
    }

    #[inline]
    pub fn from_inline_slots(buf: [Value; SMALL_SLOTS], parent: Env) -> Env {
        Some(Rc::new(Self {
            slots: RefCell::new(FrameSlots::Inline(buf)),
            parent,
        }))
    }

    #[inline]
    pub fn from_slots(slots: Vec<Value>, parent: Env) -> Env {
        Some(Rc::new(Self {
            slots: RefCell::new(FrameSlots::Heap(slots)),
            parent,
        }))
    }

    #[inline]
    pub fn with_void_slots(count: usize, parent: Env) -> Env {
        let slots = if count <= SMALL_SLOTS {
            FrameSlots::Inline([const { Value::Void }; SMALL_SLOTS])
        } else {
            FrameSlots::Heap(vec![Value::Void; count])
        };
        Some(Rc::new(Self {
            slots: RefCell::new(slots),
            parent,
        }))
    }

    #[inline]
    fn frame_at(env: &Env, depth: u32) -> &EnvFrame {
        let mut cur = env
            .as_deref()
            .expect("lexical address depth exceeded environment chain");
        for _ in 0..depth {
            cur = cur
                .parent
                .as_deref()
                .expect("lexical address depth exceeded environment chain");
        }
        cur
    }

    #[inline]
    pub fn lookup_local(env: &Env, depth: u32, slot: u32) -> Value {
        let frame = Self::frame_at(env, depth);
        match &*frame.slots.borrow() {
            FrameSlots::Inline(buf) => buf[slot as usize].clone(),
            FrameSlots::Heap(vec) => vec[slot as usize].clone(),
        }
    }

    #[inline]
    pub fn assign_local(env: &Env, depth: u32, slot: u32, val: Value) {
        let frame = Self::frame_at(env, depth);
        match &mut *frame.slots.borrow_mut() {
            FrameSlots::Inline(buf) => buf[slot as usize] = val,
            FrameSlots::Heap(vec) => vec[slot as usize] = val,
        }
    }
}
