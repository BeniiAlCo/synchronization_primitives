use std::sync::atomic::Ordering::Acquire;
use std::sync::atomic::{AtomicPtr, Ordering::Relaxed};
use std::time::Duration;

/// RCU: **R**ead, **C**opy, **U**pdate; A Pattern for Concurrent Access to large Structs
///
/// # An Overview
///
/// We can use existing synchronization primatives to safely access small variables concurrently.
/// If the variable were to be much larger, however, we don't have an atomic type that allows for
/// lock-free atomic operations on the entire object.
/// RCU is a process that allows for this kind of interaction.
///
/// RCU is a pattern in which we take a(n atomic) pointer to an object.
/// From here, a given thread can read in the object, make changes, then replace the entire object
/// atomically (give there have been no changes since it was read).
///
/// # The Plan
///
/// 1. **R**ead the data
/// 2. **C**opy the data to a new (not-concurrently-accessible) location
/// 3. Modify that data if necessary
/// 4. **U**pdate the data by swapping the new data with the old data (if the old data hasn't already
///    been updated by some other thread in the meantime)
/// 5. Deallocate the old data
///
/// # The Problem
///
/// At t=1, Thread 1 Reads the data protected by the RCU, and modifies it. Even though it copies
/// the data into a local variable to modify it, it still must keep a version of the original data,
/// as to compare to the RCU data when it tries to update it.
/// At t=2, Thread 2 Reads the data protected by the RCU, and modifies it.
/// at t=3, Thread 1 Replaces the data in the RCU and drops the original RCU data. Thread 2 still
/// has a reference to the original data, that Thread 1 does not know about. We have dropped data
/// that we still have a reference to. This is an error.
pub struct Rcu {
    ptr: AtomicPtr<Vec<i32>>,
}

impl Default for Rcu {
    fn default() -> Self {
        Self::new()
    }
}

impl Rcu {
    pub fn new() -> Rcu {
        Rcu {
            ptr: AtomicPtr::new(Box::into_raw(Box::default())),
        }
    }

    pub fn push(&self, elem: i32) {
        unsafe {
            // Read
            let old = self.ptr.load(Relaxed);

            // Copy
            let mut new = Box::new((*old).clone());

            // Modify
            (*new).push(elem);
            let new = Box::into_raw(new);
            std::thread::sleep(Duration::from_secs(1));

            match self.ptr.compare_exchange(old, new, Acquire, Relaxed) {
                Ok(old) => {
                    drop(Box::from_raw(old));
                }
                Err(_) => drop(Box::from_raw(new)),
            }
        }
    }
}

#[test]
fn rcu_test() {
    let q = std::sync::Arc::new(Rcu::new());
    std::thread::scope(|s| {
        for _ in 0..3 {
            s.spawn(|| {
                let q = q.clone();
                q.push(1);
            });
        }
    });
}
