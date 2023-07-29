use std::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering::Relaxed},
};

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
pub struct List<T> {
    head: AtomicPtr<Node<T>>, //*mut Node<T>,
                              //tail: AtomicPtr<Node<T>>, //*mut Node<T>,
}

struct Node<T> {
    elem: T,
    next: *mut Node<T>,
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> List<T> {
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            //tail: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn push(&self, elem: T) {
        let mut new_head = Box::new(Node {
            elem,
            next: ptr::null_mut(),
        });
        let current_head = self.head.load(Relaxed);

        let new_head = if current_head.is_null() {
            Box::into_raw(new_head)
        } else {
            new_head.next = current_head;
            Box::into_raw(new_head)
        };

        if self
            .head
            .compare_exchange(
                current_head,
                new_head,
                std::sync::atomic::Ordering::Acquire,
                Relaxed,
            )
            .is_ok()
        {
        } else {
            unsafe {
                drop(Box::from_raw(new_head));
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        unsafe {
            let current_head = self.head.load(Relaxed);
            if current_head.is_null() {
                None
            } else {
                let old_head = Box::from_raw(current_head);
                let new_head = old_head.next;
                if self
                    .head
                    .compare_exchange(
                        current_head,
                        new_head,
                        std::sync::atomic::Ordering::Acquire,
                        Relaxed,
                    )
                    .is_ok()
                {
                    Some(old_head.elem)
                } else {
                    None
                }
            }
        }
    }
}

impl<T: std::fmt::Debug + Clone> std::fmt::Debug for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let mut v = vec![];
            let mut current = self.head.load(Relaxed);
            while !current.is_null() {
                v.push((*current).elem.clone());
                current = (*current).next; //.load(Relaxed);
            }
            f.debug_struct("List").field("head", &v).finish()
        }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

#[test]
fn rcu_() {
    let q = List::new();
    q.push(1);
    q.push(2);
    dbg!(q);
}

#[test]
fn rcu() {
    let q = std::sync::Arc::new(List::new());
    std::thread::scope(|s| {
        for _ in 0..3 {
            s.spawn(|| {
                let q = q.clone();
                q.push(1);
            });
            s.spawn(|| {
                let q = q.clone();
                q.push(2);
            });
            s.spawn(|| {
                let q = q.clone();
                q.pop();
            });
        }
    });
    dbg!(q);
}
