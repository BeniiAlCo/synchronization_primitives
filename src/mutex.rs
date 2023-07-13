use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

use crate::semaphore::binary_semaphore::{BinaryPermit, BinarySemaphore};

pub struct Mutex<T> {
    semaphore: BinarySemaphore,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

pub struct MutexGuard<'a, T> {
    permit: BinaryPermit<'a>,
    mutex: &'a Mutex<T>,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            semaphore: BinarySemaphore::new(),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        MutexGuard {
            mutex: self,
            permit: self.semaphore.wait(),
        }
    }

    pub fn unlock(guard: MutexGuard<'_, T>) {
        drop(guard);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Barrier},
        thread,
        time::Instant,
    };

    use super::*;

    #[test]
    fn mutex() {
        let m = Mutex::new(0);
        std::hint::black_box(&m);
        let start = Instant::now();
        for _ in 0..5_000_000 {
            *m.lock() += 1;
        }
        let duration = start.elapsed();
        dbg!(format!("locked {} times in {:?}", *m.lock(), duration));
    }

    #[test]
    fn test_mutex_concurrency() {
        const NUM_THREADS: usize = 10;
        const NUM_ITERATIONS: usize = 1000;

        let mutex = Arc::new(Mutex::new(0));
        let barrier = Arc::new(Barrier::new(NUM_THREADS));

        let mut handles = Vec::with_capacity(NUM_THREADS);

        for _ in 0..NUM_THREADS {
            let mutex_clone = Arc::clone(&mutex);
            let barrier_clone = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                barrier_clone.wait();

                for _ in 0..NUM_ITERATIONS {
                    let mut guard = mutex_clone.lock();
                    *guard += 1;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let guard = mutex.lock();
        assert_eq!(*guard, NUM_THREADS * NUM_ITERATIONS);
    }
}
