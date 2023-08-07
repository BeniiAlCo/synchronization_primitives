use crate::sync::atomic::{
    AtomicPtr,
    Ordering::{Acquire, Relaxed},
};
use std::ptr::{null_mut, NonNull};

pub struct Queue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
}

struct Node<T> {
    elem: T,
    next: Option<NonNull<Node<T>>>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(null_mut()),
            tail: AtomicPtr::new(null_mut()),
        }
    }

    pub fn push(&self, elem: T) {
        let new_tail = Box::into_raw(Box::new(Node { elem, next: None }));
        let mut current_tail = self.tail.load(Relaxed);
        loop {
            unsafe {
                if current_tail.is_null() {
                    // Tail has nothing -- we need to add a new tail, and make the head point to that
                    // tail as it is the only element in the queue.
                    // Possible states we are currently in:
                    // - There are no elements in the list, and no other threads are interacting with
                    // the list
                    // - There are no elements in the list, and other threads are also trying to add an
                    // element to the list -- hence we CAS
                    //
                    // That is, if Tail is null, then we assume there is no other state the queue can
                    // be in other than empty -- once we succesfully add the new tail, we can safely
                    // add the new head too
                    match self
                        .tail
                        .compare_exchange(current_tail, new_tail, Acquire, Relaxed)
                    {
                        Ok(_) => {
                            self.head.store(new_tail, Relaxed);
                            break;
                        }
                        Err(e) => {
                            current_tail = e;
                        }
                    }
                } else {
                    // Tail has something, so we need to update the old Tail
                    //let old_tail = current_tail;

                    match self
                        .tail
                        .compare_exchange(current_tail, new_tail, Acquire, Relaxed)
                    {
                        Ok(ptr) => {
                            (*ptr).next = Some(NonNull::new_unchecked(new_tail));
                            //self.tail.store(new_tail, Relaxed);
                            break;
                        }
                        Err(e) => {
                            current_tail = e;
                        }
                    }
                }
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        let current_head = self.head.load(Relaxed);
        let current_tail = self.tail.load(Relaxed);
        unsafe {
            if current_head.is_null() && current_tail.is_null() {
                None
            } else if !current_head.is_null() && !current_tail.is_null() {
                match self
                    .tail
                    .compare_exchange(current_head, null_mut(), Acquire, Relaxed)
                {
                    Ok(_) => {
                        // tail == head, and tail now == null, so need to update head to be null
                        self.head.store(null_mut(), Relaxed);
                        Some(Box::from_raw(current_head).elem)
                    }
                    Err(_) => {
                        // tail != head, so head needs to = head.next
                        self.head
                            .store((*current_head).next.unwrap().as_ptr(), Relaxed);
                        Some(Box::from_raw(current_head).elem)
                    }
                }
            } else {
                None
            }
        }
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::Queue;

    #[cfg(not(loom))]
    mod sequential {
        use super::Queue;

        #[test]
        fn basics() {
            let mut list = Queue::new();

            // Check empty list behaves right
            assert_eq!(list.pop(), None);

            // Populate list
            list.push(1);
            list.push(2);
            list.push(3);

            // Check normal removal
            assert_eq!(list.pop(), Some(1));
            assert_eq!(list.pop(), Some(2));

            // Push some more just to make sure nothing's corrupted
            list.push(4);
            list.push(5);

            // Check normal removal
            assert_eq!(list.pop(), Some(3));
            assert_eq!(list.pop(), Some(4));

            // Check exhaustion
            assert_eq!(list.pop(), Some(5));
            assert_eq!(list.pop(), None);

            // Check the exhaustion case fixed the pointer right
            list.push(6);
            list.push(7);

            // Check normal removal
            assert_eq!(list.pop(), Some(6));
            assert_eq!(list.pop(), Some(7));
            assert_eq!(list.pop(), None);

            // Check dropping the queue
            list.push(8);
            list.push(9);
            list.push(10);
            drop(list);
        }
    }

    //#[cfg(loom)]
    mod loom {
        use super::Queue;
        use crate::{sync::Arc, thread};

        #[test]
        fn push_from_multiple_threads() {
            loom::model(|| {
                const NUM_THREADS: usize = 3;

                let list = Arc::new(Queue::new());

                let mut handles = Vec::with_capacity(NUM_THREADS);

                for i in 0..NUM_THREADS {
                    let list = list.clone();

                    let handle = thread::spawn(move || {
                        list.push(thread::current().id());
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.join().unwrap();
                }

                for _ in 0..NUM_THREADS {
                    assert!(list.pop().is_some());
                    //dbg!(list.pop());
                }
            });
        }

        #[test]
        fn push_and_pop_from_multiple_threads() {
            loom::model(|| {
                const NUM_THREADS: usize = 3;

                let list = Arc::new(Queue::new());

                let mut handles = Vec::with_capacity(NUM_THREADS);

                let l = list.clone();
                let handle = thread::spawn(move || {
                    l.push(thread::current().id());
                });
                handles.push(handle);

                let l = list.clone();
                let handle = thread::spawn(move || {
                    l.push(thread::current().id());
                });
                handles.push(handle);

                let l = list;
                let handle = thread::spawn(move || {
                    l.pop();
                });
                handles.push(handle);

                for handle in handles {
                    handle.join().unwrap();
                }
            });
        }
    }
}

