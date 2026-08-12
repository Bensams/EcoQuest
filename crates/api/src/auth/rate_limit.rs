//! Fixed-window rate limiting for authentication endpoints.
//!
//! ponytail: in-process counters, so limits are per API instance and reset on
//! restart. Move to a shared Redis/Postgres counter when the API runs more than
//! one replica.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Fixed-window counter keyed by client IP.
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    buckets: Mutex<HashMap<IpAddr, Bucket>>,
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    /// Builds a limiter allowing `max_requests` per `window_secs`.
    #[must_use]
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Records an attempt; returns `false` when the caller is over the limit.
    ///
    /// Callers without a resolvable IP are always allowed: failing closed here
    /// would let a proxy misconfiguration lock out every user.
    pub fn check(&self, ip: Option<IpAddr>) -> bool {
        let Some(ip) = ip else {
            return true;
        };
        let now = Instant::now();
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Opportunistic cleanup keeps the map from growing without bound.
        if buckets.len() > 10_000 {
            buckets.retain(|_, b| now.duration_since(b.window_start) < self.window);
        }

        let bucket = buckets.entry(ip).or_insert(Bucket {
            count: 0,
            window_start: now,
        });
        if now.duration_since(bucket.window_start) >= self.window {
            *bucket = Bucket {
                count: 0,
                window_start: now,
            };
        }
        bucket.count += 1;
        bucket.count <= self.max_requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn ip(last: u8) -> Option<IpAddr> {
        Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, last)))
    }

    #[test]
    fn allows_up_to_the_limit_then_blocks() {
        let limiter = RateLimiter::new(3, 60);
        assert!(limiter.check(ip(1)));
        assert!(limiter.check(ip(1)));
        assert!(limiter.check(ip(1)));
        assert!(!limiter.check(ip(1)), "fourth attempt must be blocked");
    }

    #[test]
    fn limits_are_per_client() {
        let limiter = RateLimiter::new(1, 60);
        assert!(limiter.check(ip(1)));
        assert!(!limiter.check(ip(1)));
        assert!(limiter.check(ip(2)), "other clients must be unaffected");
    }

    #[test]
    fn window_resets() {
        let limiter = RateLimiter::new(1, 0);
        assert!(limiter.check(ip(1)));
        assert!(limiter.check(ip(1)), "zero-length window resets each call");
    }

    #[test]
    fn unknown_clients_are_allowed() {
        let limiter = RateLimiter::new(1, 60);
        assert!(limiter.check(None));
        assert!(limiter.check(None));
    }
}
