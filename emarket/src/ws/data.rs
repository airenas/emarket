use crate::redis::RedisClient;
use serde::Serialize;
use thiserror::Error;

pub struct Service {
    pub redis: RedisClient,
}

#[derive(Debug, Error)]
pub enum ApiError {
    // #[error("bad request: {0}, details: {1}")]
    // BadRequest(String, String),
    #[error("Server error: {0}")]
    Server(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, Serialize, Clone)]
pub struct LiveResponse {
    pub status: bool,
    pub redis: String,
    pub version: String,
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
