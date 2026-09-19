//! Token-bucket rate limiter for `/login`. In-process. Production
//! wiring uses the api-gateway's middleware; this is the
//! reference implementation.

use std::time::Instant;

use parking_lot::Mutex;

#[derive(Debug)]
pub struct TokenBucket {
    capacity: u32,
    refill_per_sec: f64,
    tokens: Mutex<f64>,
    last_refill: Mutex<Instant>,
}

impl TokenBucket {
    #[must_use]
    pub fn new(capacity: u32, refill_per_min: u32) -> Self {
        let rps = refill_per_min as f64 / 60.0;
        Self {
            capacity,
            refill_per_sec: rps,
            tokens: Mutex::new(capacity as f64),
            last_refill: Mutex::new(Instant::now()),
        }
    }

    /// Try to take one token. Returns `false` if the bucket is dry.
    pub fn try_take(&self) -> bool {
        let now = Instant::now();
        let mut last = self.last_refill.lock();
        let mut tokens = self.tokens.lock();
        let elapsed = now.duration_since(*last).as_secs_f64();
        *tokens = (*tokens + elapsed * self.refill_per_sec).min(self.capacity as f64);
        *last = now;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }
}