use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

pub fn now_ts_milli() -> u64 {
    now_ts_micro() / 1000 // milliseconds
}

pub fn now_ts_micro() -> u64 {
    let anchor = minstant::Anchor::new();
    let now = minstant::Instant::now();
    let ts = now.as_unix_nanos(&anchor); // nanoseconds
    ts / 1000 // microseconds
}

lazy_static! {
    static ref MAX_LOCK_DURATION: Duration = Duration::from_secs(20);
}

/// shortcut wl and rl.
pub trait HandyRwLock<T> {
    fn wl(&self) -> RwLockWriteGuard<'_, T>;
    fn rl(&self) -> RwLockReadGuard<'_, T>;
}

impl<T> HandyRwLock<T> for RwLock<T> {
    fn wl(&self) -> RwLockWriteGuard<'_, T> {
        match self.try_write_for(*MAX_LOCK_DURATION) {
            Some(guard) => guard,
            None => panic!("lock max duration exceed, maybe deadlocked!!"),
        }
    }

    fn rl(&self) -> RwLockReadGuard<'_, T> {
        match self.try_read_for(*MAX_LOCK_DURATION) {
            Some(guard) => guard,
            None => panic!("lock max duration exceed, maybe deadlocked!!"),
        }
    }
}

pub trait HandyMutex<T> {
    fn l(&self) -> MutexGuard<'_, T>;
}

impl<T> HandyMutex<T> for Mutex<T> {
    fn l(&self) -> MutexGuard<'_, T> {
        match self.try_lock_for(*MAX_LOCK_DURATION) {
            Some(guard) => guard,
            None => panic!("lock max duration exceed, maybe deadlocked!!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_ts() {
        assert!(now_ts_milli() > 0);
    }
}
