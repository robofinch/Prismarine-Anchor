use std::sync::{Mutex, MutexGuard};


/// Consolidate where `.unwrap()` is called; in the context of mutexes, panicking when a mutex
/// is poisoned is usually the preferred behavior.
///
/// The implementation provided for `Mutex<T>` simply calls `.lock().unwrap()`.
pub trait LockOrPanic<T> {
    /// Consolidate where `.unwrap()` is called; in the context of mutexes, panicking when a mutex
    /// is poisoned is usually the preferred behavior.
    fn lock_or_panic(&self) -> MutexGuard<'_, T>;
}

impl<T> LockOrPanic<T> for Mutex<T> {
    /// Simply calls `.lock().unwrap()` on a `Mutex`, in order to consolidate where
    /// `.unwrap()` is called.
    /// # Panics
    /// Panics if the mutex is poisoned.
    #[inline]
    fn lock_or_panic(&self) -> MutexGuard<'_, T> {
        #[expect(
            clippy::unwrap_used,
            reason = "we want to panic if a mutex is poisoned",
        )]
        self.lock().unwrap()
    }
}
