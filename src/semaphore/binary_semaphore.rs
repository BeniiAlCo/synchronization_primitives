use std::sync::atomic::{
    AtomicU32,
    Ordering::{Acquire, Relaxed, Release},
};

use atomic_wait::{wait, wake_one};

pub struct BinarySemaphore {
    state: AtomicU32,
}

pub struct BinaryPermit<'a> {
    binary_semaphore: &'a BinarySemaphore,
}

impl BinarySemaphore {
    pub const fn new() -> Self {
        Self {
            // 2: Permit remaining
            // 1: No Permits remaining; No threads waiting
            // 0: No Permits remaining; Threads waiting
            state: AtomicU32::new(2),
        }
    }

    pub fn aquire(&self) -> BinaryPermit {
        if self.state.compare_exchange(2, 1, Acquire, Relaxed).is_err() {
            lock_contended(&self.state);
        }
        BinaryPermit {
            binary_semaphore: self,
        }
    }

    pub fn release(permit: BinaryPermit) {
        drop(permit);
    }

    fn get_count(&self) -> u32 {
        match self.state.load(Relaxed) {
            2 => 1,
            _ => 0,
        }
    }
}

fn lock_contended(state: &AtomicU32) {
    let mut spin_count = 0;
    while state.load(Relaxed) == 1 && spin_count < 100 {
        spin_count += 1;
        loom::hint::spin_loop();
        std::hint::spin_loop();
    }

    if state.compare_exchange(2, 1, Acquire, Relaxed).is_ok() {
        return;
    }

    while state.swap(0, Acquire) != 2 {
        loom::hint::spin_loop();
        wait(state, 0);
    }
}

impl Drop for BinaryPermit<'_> {
    fn drop(&mut self) {
        if self.binary_semaphore.state.swap(2, Release) == 0 {
            wake_one(&self.binary_semaphore.state);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;

    #[test]
    fn new() {
        let semaphore = Arc::new(BinarySemaphore::new());
        assert_eq!(semaphore.get_count(), 1);
    }

    #[test]
    fn test_concurrency() {
        let semaphore = Arc::new(BinarySemaphore::new());
        thread::scope(|s| {
            for _ in 0..30 {
                s.spawn(|| {
                    let semaphore = semaphore.clone();
                    {
                        let permit = semaphore.aquire();
                        std::hint::black_box(&semaphore);
                        BinarySemaphore::release(permit);
                    }
                });
            }
        });
        assert_eq!(semaphore.get_count(), 1);
    }
}
