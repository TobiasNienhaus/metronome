use std::hint;
use std::sync::atomic::{
    AtomicU64,
    Ordering
};

static mut ID: AtomicU64 = AtomicU64::new(0);

pub fn new() -> u64 {
    unsafe {
        *ID.get_mut() += 1;
        ID.load(Ordering::SeqCst)
    }
}