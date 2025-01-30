use std::{str::FromStr, sync::Arc};

use axum::{
    extract::{self, Query, State},
    Json,
};
use emarket::{utils::to_str_or_none, TN_DAY, TN_HOUR, TN_MONTH};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::data::{ApiError, ApiResult, MarketData, Service};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeRange {
    Hourly,
    Daily,
    Monthly,
}

impl FromStr for TimeRange {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hourly" => Ok(TimeRange::Hourly),
            "daily" => Ok(TimeRange::Daily),
            "monthly" => Ok(TimeRange::Monthly),
            "" => Ok(TimeRange::Monthly),
            _ => Err(format!("Invalid time_range value: {}", s)),
        }
    }
}

#[derive(Deserialize)]
pub struct PricesParams {
    time_range: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
}

pub async fn handler(
    State(srv_wrap): State<Arc<RwLock<Service>>>,
    Query(params): Query<PricesParams>,
) -> ApiResult<extract::Json<Vec<MarketData>>> {
    tracing::debug!("prices handler");
    let srv = srv_wrap.read().await;
    tracing::debug!(
        from = to_str_or_none(params.from),
        to = to_str_or_none(params.to),
        time_range = &params.time_range,
        "params",
    );

    let table_name = get_table_name(params.time_range)?;
    tracing::debug!(table_name, "will use");
    let res = srv
        .redis
        .load(table_name, params.from, params.to)
        .await
        .map_err(|e| ApiError::Server(e.to_string()))?;
    tracing::debug!(len = res.len(), "loaded");
    Ok(Json(res))
}

fn get_table_name(data: Option<String>) -> Result<&'static str, ApiError> {
    let time_range = match data {
        Some(s) => TimeRange::from_str(&s)
            .map_err(|e| ApiError::BadRequest(format!("wrong time_range: {}", s).to_string(), e))?,
        None => TimeRange::Monthly,
    };
    let res = match time_range {
        TimeRange::Hourly => TN_HOUR,
        TimeRange::Daily => TN_DAY,
        TimeRange::Monthly => TN_MONTH,
    };
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_valid_time_ranges() {
        assert_eq!(TimeRange::from_str("hourly"), Ok(TimeRange::Hourly));
        assert_eq!(TimeRange::from_str("daily"), Ok(TimeRange::Daily));
        assert_eq!(TimeRange::from_str("monthly"), Ok(TimeRange::Monthly));
        assert_eq!(TimeRange::from_str(""), Ok(TimeRange::Monthly));
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(TimeRange::from_str("HOURLY"), Ok(TimeRange::Hourly));
        assert_eq!(TimeRange::from_str("Daily"), Ok(TimeRange::Daily));
        assert_eq!(TimeRange::from_str("moNthly"), Ok(TimeRange::Monthly));
    }

    #[test]
    fn test_invalid_time_ranges() {
        assert!(TimeRange::from_str("weekly").is_err());
        assert!(TimeRange::from_str("yearly").is_err());
        assert!(TimeRange::from_str("aaa").is_err());
        assert!(TimeRange::from_str("123").is_err());
        assert!(TimeRange::from_str("hourlyy").is_err());
    }
}
