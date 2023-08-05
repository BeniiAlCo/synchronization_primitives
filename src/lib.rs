#![allow(unused)]

#[allow(dead_code)]
mod mutex;
#[allow(dead_code)]
mod queue_based_lock;
pub mod rcu;
#[allow(dead_code)]
mod semaphore;

#[cfg(loom)]
pub(crate) use loom::sync;
#[cfg(loom)]
pub(crate) use loom::thread;

#[cfg(not(loom))]
pub(crate) use std::sync;
#[cfg(not(loom))]
pub(crate) use std::thread;
