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

#[derive(Serialize)]
pub struct SummaryData {
    pub at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_month_avg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_month_avg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub today_avg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tomorrow_avg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yesterday_avg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_30d_avg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_7_avg: Option<f64>,
}
