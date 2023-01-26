use std::{error::Error, time::Duration, num::NonZeroU32};

use async_trait::async_trait;
use emarket::data::Limiter;
use governor::{state::{NotKeyed, InMemoryState}, clock::{QuantaClock}};

pub struct RateLimiter {
    governor:governor::RateLimiter<NotKeyed, InMemoryState, QuantaClock, >,
    jitter: governor::Jitter, 
}

impl RateLimiter {
    pub fn new() -> Result<RateLimiter, Box<dyn Error>> {
        let governor = governor::RateLimiter::direct(
            governor::Quota::per_minute(NonZeroU32::new(60).expect("Governor rate is 0")));
        let jitter = governor::Jitter::new(Duration::ZERO, Duration::from_secs(3));
        Ok(RateLimiter {governor, jitter})
    }
}

#[async_trait]
impl Limiter for RateLimiter {
    async fn wait(&self) -> std::result::Result<bool, Box<dyn Error>> {
        log::debug!("wait until_ready_with_jitter");
        self.governor.until_ready_with_jitter(self.jitter).await;
        log::debug!("allowed");
        Ok(true)
    }
}
