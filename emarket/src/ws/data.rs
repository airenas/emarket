use serde::Serialize;

use crate::redis::RedisClient;

pub struct Service {
    pub redis: RedisClient,
}

#[derive(Serialize)]
pub struct MarketData {
    pub at: u64,
    pub price: f64,
}
