use std::sync::atomic::{
    AtomicU32,
    Ordering::{Acquire, Relaxed, Release},
};

use atomic_wait::{wait, wake_one};

pub mod binary_semaphore;

pub struct Semaphore {
    state: AtomicU32,
    max_permits: u32,
}

pub struct Permit<'a> {
    state: u32,
    semaphore: &'a Semaphore,
}

impl Semaphore {
    pub const fn new(value: u32) -> Self {
        Self {
            state: AtomicU32::new(value),
            max_permits: value,
        }
    }

    pub fn aquire(&self) -> Permit {
        let mut s = self.state.load(Relaxed);
        loop {
            if s > 0 {
                match self.state.compare_exchange_weak(s, s - 1, Acquire, Relaxed) {
                    Ok(_) => {
                        return Permit {
                            state: 1,
                            semaphore: self,
                        }
                    }
                    Err(e) => s = e,
                }
            }
            if s == 0 {
                while self.state.load(Relaxed) == 0 {
                    wait(&self.state, 0);
                }
            }
        }
    }

    pub fn aquire_n(&self, n: u32) -> Permit {
        assert!(
            n <= self.max_permits,
            "Cannot aquire more than the maximum number of permits."
        );
        let mut s = self.state.load(Relaxed);
        loop {
            if s >= n {
                match self.state.compare_exchange_weak(s, s - n, Acquire, Relaxed) {
                    Ok(_) => {
                        return Permit {
                            state: n,
                            semaphore: self,
                        }
                    }
                    Err(e) => s = e,
                }
            }
            if s < n {
                wait(&self.state, s);
                s = self.state.load(Relaxed);
            }
        }
    }

    pub fn release(permit: Permit) {
        drop(permit);
    }

    fn get_count(&self) -> u32 {
        self.state.load(Relaxed)
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.semaphore.state.fetch_add(self.state, Release);
        wake_one(&self.semaphore.state);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;

    #[test]
    fn new() {
        let semaphore = Arc::new(Semaphore::new(3));
        assert_eq!(semaphore.get_count(), 3);
    }

    #[test]
    fn test_concurrency() {
        let semaphore = Arc::new(Semaphore::new(30));
        thread::scope(|s| {
            for i in 1..=30 {
                let i = AtomicU32::new(i);
                s.spawn(|| {
                    let semaphore = semaphore.clone();
                    {
                        let permit = semaphore.aquire_n(i.into_inner());
                        std::hint::black_box(&semaphore);
                        assert!(semaphore.state.load(Relaxed) <= 30);
                        Semaphore::release(permit);
                    }
                });
            }
        });
        assert_eq!(semaphore.get_count(), 30);
    }
}
