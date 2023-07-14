// Mutex that is a single AtomicPtr that points to a list of waiting threads.

use std::{
    collections::VecDeque,
    sync::{atomic::AtomicPtr, Arc},
    thread::{self, Thread},
};

pub struct Queue {
    ptr: AtomicPtr<VecDeque<Thread>>,
}

impl Queue {
    fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(Box::leak(Box::default())),
        }
    }

    fn add(&self, t: Thread) {
        let ptr = self.ptr.load(std::sync::atomic::Ordering::Relaxed);
        unsafe { (*ptr).push_back(t) };
    }
    fn print(&self) {
        unsafe { dbg!((*self.ptr.load(std::sync::atomic::Ordering::Relaxed)).iter()) };
    }
}

#[test]
fn test() {
    let q = Arc::new(Queue::new());
    thread::scope(|s| {
        let builder = thread::Builder::new();
        let t = thread::spawn(|| {
            thread::park();
            println!("there")
        });

        let s = thread::spawn(|| {
            thread::park();
            println!("helloo");
        });
        q.clone().add(t.thread().clone());
        q.clone().add(s.thread().clone());
    });
    unsafe {
        for t in (*q.ptr.load(std::sync::atomic::Ordering::Relaxed)).iter() {
            t.unpark();
        }
    }
    //q.print();
}
