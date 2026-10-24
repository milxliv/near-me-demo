use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Fixed-window rate limiter, per client and in total.
///
/// Every window clears all counts, so the map never holds more than `total`
/// entries and nothing outlives one window. Keys are client IPs, kept in
/// memory only.
pub struct RateLimiter {
    per_client: u32,
    total: u32,
    window: Duration,
    state: Mutex<Window>,
}

struct Window {
    started: Instant,
    total: u32,
    counts: HashMap<String, u32>,
}

impl RateLimiter {
    pub fn new(per_client: u32, total: u32, window: Duration) -> Self {
        Self {
            per_client,
            total,
            window,
            state: Mutex::new(Window {
                started: Instant::now(),
                total: 0,
                counts: HashMap::new(),
            }),
        }
    }

    /// Counts one request for `client` and says whether it may proceed.
    pub fn allow(&self, client: &str) -> bool {
        // A poisoned lock only means another request panicked mid-update;
        // the counts are still usable.
        let mut w = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        if now.duration_since(w.started) >= self.window {
            w.started = now;
            w.total = 0;
            w.counts.clear();
        }
        if w.total >= self.total {
            return false;
        }
        let count = w.counts.entry(client.to_owned()).or_insert(0);
        if *count >= self.per_client {
            return false;
        }
        *count += 1;
        w.total += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_each_client_separately() {
        let limiter = RateLimiter::new(2, 100, Duration::from_secs(60));
        assert!(limiter.allow("a"));
        assert!(limiter.allow("a"));
        assert!(!limiter.allow("a"));
        assert!(limiter.allow("b"));
    }

    #[test]
    fn limits_the_total() {
        let limiter = RateLimiter::new(10, 3, Duration::from_secs(60));
        assert!(limiter.allow("a"));
        assert!(limiter.allow("b"));
        assert!(limiter.allow("c"));
        assert!(!limiter.allow("d"));
    }

    #[test]
    fn a_new_window_resets_counts() {
        let limiter = RateLimiter::new(1, 100, Duration::from_millis(20));
        assert!(limiter.allow("a"));
        assert!(!limiter.allow("a"));
        std::thread::sleep(Duration::from_millis(30));
        assert!(limiter.allow("a"));
    }
}
