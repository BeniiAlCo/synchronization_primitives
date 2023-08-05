use crate::{
    semaphore::binary_semaphore::BinarySemaphore,
    sync::atomic::{AtomicPtr, AtomicUsize},
};
use std::{
    ptr::null_mut,
    sync::atomic::Ordering::{Acquire, Relaxed, Release},
};

pub struct List<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
    len: AtomicUsize,
    semaphore: BinarySemaphore,
}

struct Node<T> {
    elem: T,
    next: AtomicPtr<Node<T>>,
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<T> Sync for List<T> {}

impl<T> List<T> {
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(null_mut()),
            tail: AtomicPtr::new(null_mut()),
            len: AtomicUsize::new(0),
            semaphore: BinarySemaphore::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push_front(&self, elem: T) {
        let permit = self.semaphore.aquire();
        let mut new_head = Box::new(Node {
            elem,
            next: AtomicPtr::new(null_mut()),
        });
        let mut current_head = self.head.load(Relaxed);
        let mut current_tail = self.tail.load(Relaxed);
        //loop {
        unsafe {
            if current_head.is_null() {
                //if !current_tail.is_null() {
                // if head is null and tail is not null, a thread is in the middle of
                // `push_tail`, and we should assume that there is a head that hasn't been
                // updated yet, so we know this attempt at replacing the head will fail
                //continue;
                //}
                // Head is null
                // If the tail is also null, then we can proceed
                // but we should also enter a binary semaphore here, as if we are changing the
                // head atomically, we cannot guaruntee that no other thread won't change the
                // tail while we do that... but is that a problem?
                // What if the tail is null here, but when we update the head, the tail is not
                // null?
                // We could say, if the head is null, the tail ought be null, if that's not the
                // case, then restart the loop as the head is about to be updated to be the
                // tail
                // if the tail is null, then we swap in the new head, and if the head's swap
                // has been successful, we then check the tail again, and if it is still null,
                // then we're good, and the tail should be set to the head, and if it isn't,
                // the head's next needs to be set to the tail
            } else {
                new_head.next = AtomicPtr::new(current_head);
            }
            let x = Box::into_raw(new_head);

            self.head.store(x, Relaxed);
            //match self
            //    .head
            //    .compare_exchange(current_head, x, Acquire, Relaxed)
            //{
            //    Ok(old_head) => {
            self.len.fetch_add(1, Relaxed);
            // the head is now irreversibly in place -- if the head was null when we
            // started, and the tail is still null, then we want to update the tail to
            // be the same as the head;
            // if the head was null and the tail is now not null, then it was up to the
            // `push_tail` call to update any `Node` `next` fields, so we don't need to
            // do any additional bookkeeping.
            //if old_head.is_null() {
            //    let _ = self.tail.compare_exchange(
            //        current_tail,
            //        self.head.load(Acquire),
            //        Acquire,
            //        Relaxed,
            //    );
            //}
            //break;
            //    }
            //    Err(_e) => {
            //new_head = Box::from_raw(x);
            //current_head = self.head.load(Relaxed);
            //    }
            //}
        }
        //}
        BinarySemaphore::release(permit);
    }

    pub fn pop_front(&self) -> Option<T> {
        let permit = self.semaphore.aquire();
        unsafe {
            let current_head = self.head.load(Relaxed);
            let current_tail = self.tail.load(Relaxed);
            if current_head.is_null() {
                None
            } else {
                let old_head = Box::from_raw(current_head);
                let new_head = old_head.next.load(Relaxed);
                self.head.store(new_head, Relaxed);
                //if self
                //    .head
                //    .compare_exchange(current_head, new_head, Acquire, Relaxed)
                //    .is_ok()
                //{
                self.len.fetch_sub(1, Release);
                BinarySemaphore::release(permit);
                Some(old_head.elem)
                //} else {
                //    BinarySemaphore::release(permit);
                //    None
                //}
            }
        }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        while self.pop_front().is_some() {}
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let mut v = vec![];
            let mut current = self.head.load(Relaxed);
            while !current.is_null() {
                v.push(format!("{:?}", (*current).elem));
                current = (*current).next.load(Relaxed);
            }
            f.debug_struct("List")
                .field("head", &v)
                .field("len", &v.len())
                .finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::List;

    #[cfg(not(loom))]
    mod sequential {
        use super::*;

        #[test]
        fn push_and_drop() {
            let list = List::new();
            list.push_front(1);
            list.push_front(2);
            dbg!(&list);
            drop(list);
        }

        #[test]
        fn push_and_pop() {
            let list = List::new();
            list.push_front(1);
            list.pop_front();
        }
    }

    mod concurrent {
        use crate::{sync::Arc, thread};

        use super::*;

        #[test]
        fn push_from_multiple_threads() {
            loom::model(|| {
                const NUM_THREADS: usize = 2;

                let list = Arc::new(List::new());

                let mut handles = Vec::with_capacity(NUM_THREADS);

                for i in 0..NUM_THREADS {
                    let list = list.clone();

                    let handle = thread::spawn(move || {
                        list.push_front(thread::current().id());
                        assert_ne!(0, list.len());
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.join().unwrap();
                }

                assert_eq!(NUM_THREADS, list.len());
            });
        }

        #[test]
        fn push_and_pop_from_multiple_threads() {
            loom::model(|| {
                const NUM_THREADS: usize = 2;

                let list = Arc::new(List::new());

                let mut handles = Vec::with_capacity(NUM_THREADS);

                let l = list.clone();
                let handle = thread::spawn(move || {
                    l.push_front(thread::current().id());
                    //assert_ne!(0, l.len());
                });
                handles.push(handle);

                let l = list.clone();
                let handle = thread::spawn(move || {
                    l.pop_front();
                });
                handles.push(handle);

                for handle in handles {
                    handle.join().unwrap();
                }

                assert_eq!(1, list.len());
            });
        }
    }
}
