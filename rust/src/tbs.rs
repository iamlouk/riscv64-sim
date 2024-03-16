use crate::insts::*;

#[allow(dead_code)]
pub struct TranslationBlock {
    pub start:      u64,
    pub size:       u64,
    pub exec_count: std::sync::atomic::AtomicI64,
    pub valid:      bool,
    pub label:      Option<std::rc::Rc<str>>,
    pub instrs:     Vec<(Inst, u8)>,
}

pub struct JIT {
    pub tbs: std::collections::HashMap<i64, TranslationBlock>,
    pub buffer: Vec<(Inst, u8)>,
}

impl JIT {
    pub fn new() -> Self {
        Self {
            tbs: std::collections::HashMap::with_capacity(1024),
            buffer : Vec::with_capacity(32),
        }
    }
}


