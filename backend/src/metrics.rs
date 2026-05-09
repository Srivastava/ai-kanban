use std::sync::atomic::{AtomicU64, Ordering};

pub static DB_POOL_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub static ZOMBIE_SESSIONS_RECOVERED: AtomicU64 = AtomicU64::new(0);

pub fn record_pool_timeout() {
    DB_POOL_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_zombie_recovered(count: u64) {
    ZOMBIE_SESSIONS_RECOVERED.fetch_add(count, Ordering::Relaxed);
}
