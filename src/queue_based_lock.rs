// Mutex that is a single AtomicPtr that points to a list of waiting threads.

use std::{collections::VecDeque, sync::atomic::AtomicPtr, thread, time::Duration};

pub struct QueueLock {
    ptr: AtomicPtr<VecDeque<i32>>,
}

impl QueueLock {
    fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(Box::into_raw(Box::default())),
        }
    }

    fn add(&self, t: i32) {
        unsafe {
            let old = self.ptr.load(std::sync::atomic::Ordering::Relaxed);
            let mut new = Box::new(old.as_ref().unwrap().clone());
            new.push_back(t);
            if let Err(e) = self.ptr.compare_exchange(
                old,
                Box::into_raw(new),
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            ) {
                dbg!(Box::from_raw(e));
                panic!()
            } else {
                //thread::sleep(Duration::from_secs(1));
                drop(Box::from_raw(old));
            }
        }
    }
}

// RCU
// Large chunk of data that is read/writeable via multiple threads
// Instead of sharing the data structure, we can save and share an atomic pointer to it.
// Instead of modifying the structure, with an atomic pointer, we can replace it entirely.
// RCU (Read,Copy,Update) is the process necessary to replace the data.
// - Read the pointer
// - Copy it to a new allocation that can be modified on the acting thread
// - (Modify) the copied data, to the required state
// - Update with a Compare-and-exchange operation (only succeeds if no other thread has updated)
// - (Deallocate) the exchanged data

#[test]
fn another_test() {
    let q = std::sync::Arc::new(QueueLock::new());
    std::thread::scope(|s| {
        s.spawn(|| {
            let q = q.clone();
            q.add(1);
            q.add(2);
        });
    });
    std::thread::scope(|s| {
        s.spawn(|| {
            let q = q.clone();
            for _ in 0..3 {
                q.add(3);
            }

            //unsafe {
            //let c = Box::from_raw(q.ptr.load(std::sync::atomic::Ordering::Relaxed));
            //dbg!(c.as_ref());
            //drop(c);
            //}
        });
        s.spawn(|| {
            let q = q.clone();
            for _ in 0..3 {
                q.add(2);
            }
            //unsafe {
            //    dbg!(Box::from_raw(
            //        q.ptr.load(std::sync::atomic::Ordering::Relaxed)
            //    ));
            //}
        });
    });
    std::thread::scope(|s| {
        s.spawn(|| {
            let q = q.clone();
            for _ in 0..3 {
                q.add(2);
            }
            //unsafe {
            //    dbg!(Box::from_raw(
            //        q.ptr.load(std::sync::atomic::Ordering::Relaxed)
            //    ));
            //}
        });

        //s.spawn(|| {
        //    let q = q.clone();
        //    q.add(3);
        //});
    });
}

#[test]
fn test() {
    let q = std::sync::Arc::new(QueueLock::new());
    std::thread::scope(|_| {
        let _t = std::thread::spawn(|| {
            std::thread::park();
            println!("there")
        });

        let _s = std::thread::spawn(|| {
            std::thread::park();
            println!("helloo");
        });
        //q.clone().add(t.thread().clone());
        //q.clone().add(s.thread().clone());
    });
    unsafe {
        for _t in (*q.ptr.load(std::sync::atomic::Ordering::Relaxed)).iter() {
            //t.unpark();
        }
    }
}
