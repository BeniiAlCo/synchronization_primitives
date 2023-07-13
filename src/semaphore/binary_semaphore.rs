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
            state: AtomicU32::new(1),
        }
    }

    pub fn wait(&self) -> BinaryPermit {
        let mut s = self.state.load(Relaxed);
        loop {
            if s == 1 {
                match self.state.compare_exchange_weak(1, 0, Acquire, Relaxed) {
                    Ok(_) => {
                        return BinaryPermit {
                            binary_semaphore: self,
                        }
                    }
                    Err(e) => s = e,
                }
            }
            if s == 0 {
                wait(&self.state, 0);
                s = self.state.load(Relaxed);
            }
        }
    }

    pub fn signal(permit: BinaryPermit) {
        drop(permit);
    }

    fn get_count(&self) -> u32 {
        self.state.load(Relaxed)
    }
}

impl Drop for BinaryPermit<'_> {
    fn drop(&mut self) {
        self.binary_semaphore.state.fetch_add(1, Release);
        wake_one(&self.binary_semaphore.state);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;

    #[test]
    fn new() {
        let semaphore = BinarySemaphore::new();
        assert_eq!(semaphore.get_count(), 1);
    }

    #[test]
    fn test_concurrency() {
        let semaphore = Arc::new(BinarySemaphore::new());

        thread::scope(|s| {
            for _ in 0..3 {
                s.spawn(|| {
                    let semaphore = semaphore.clone();
                    {
                        let permit = semaphore.wait();
                        std::hint::black_box(&semaphore);
                        BinarySemaphore::signal(permit);
                    }
                });
            }
        });

        assert_eq!(semaphore.get_count(), 1);
    }
}
