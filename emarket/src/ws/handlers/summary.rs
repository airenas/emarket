use std::sync::Arc;

use axum::{
    extract::{self, Query, State},
    Json,
};
use chrono::{NaiveDateTime, Utc};
use emarket::utils::{time_day_vilnius, time_month_vilnius, to_str_or_none, to_time};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::data::{ApiError, ApiResult, Service, SummaryData};

#[derive(Deserialize)]
pub struct SummaryParams {
    at: Option<i64>,
}

pub async fn handler(
    State(srv_wrap): State<Arc<RwLock<Service>>>,
    Query(params): Query<SummaryParams>,
) -> ApiResult<extract::Json<SummaryData>> {
    tracing::debug!("summary handler");
    let srv = srv_wrap.read().await;
    tracing::debug!(at = to_str_or_none(params.at));

    let at = match params.at {
        Some(a) => a,
        None => Utc::now().timestamp_millis(),
    };

    let res = SummaryData {
        at,
        current_month_avg: get_value(&srv.redis, "np_lt_m", month(at, 0), month(at, 1)).await?,
        previous_month_avg: get_value(&srv.redis, "np_lt_m", month(at, -1), month(at, 0)).await?,
        today_avg: get_value(&srv.redis, "np_lt_d", day(at, 0), day(at, 1)).await?,
        tomorrow_avg: get_value_full(&srv.redis, "np_lt_d", day(at, 1), day(at, 3), 2).await?,
        yesterday_avg: get_value(&srv.redis, "np_lt_d", day(at, -1), day(at, 0)).await?,
        last_30d_avg: get_avg(&srv.redis, "np_lt_d", day(at, -29), day(at, 1)).await?,
        last_7_avg: get_avg(&srv.redis, "np_lt_d", day(at, -6), day(at, 1)).await?,
    };
    Ok(Json(res))
}

fn month(at: i64, months: i32) -> NaiveDateTime {
    time_month_vilnius(to_time(at as u64), months)
}

fn day(at: i64, days: i64) -> NaiveDateTime {
    time_day_vilnius(to_time(at as u64), days)
}

async fn get_value(
    redis: &crate::redis::RedisClient,
    ts_name: &str,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> anyhow::Result<Option<f64>> {
    get_value_full(redis, ts_name, from, to, 1).await
}

async fn get_value_full(
    redis: &crate::redis::RedisClient,
    ts_name: &str,
    from: NaiveDateTime,
    to: NaiveDateTime,
    min_items: usize,
) -> anyhow::Result<Option<f64>> {
    let res = redis
        .load(
            ts_name,
            Some(from.and_utc().timestamp_millis()),
            Some(to.and_utc().timestamp_millis()),
        )
        .await
        .map_err(|e| ApiError::Server(e.to_string()))?;
    log::debug!("{} len res {} - {}-{}", ts_name, res.len(), from, to);
    if res.len() < min_items {
        log::info!("{} < {} - return none", res.len(), min_items);
        return Ok(None);
    }
    Ok(Some(res[0].price))
}

async fn get_avg(
    redis: &crate::redis::RedisClient,
    ts_name: &str,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> anyhow::Result<Option<f64>> {
    let res = redis
        .load(
            ts_name,
            Some(from.and_utc().timestamp_millis()),
            Some(to.and_utc().timestamp_millis()),
        )
        .await
        .map_err(|e| ApiError::Server(e.to_string()))?;
    log::debug!("{} len res {} - {}-{}", ts_name, res.len(), from, to);
    if res.is_empty() {
        return Ok(None);
    }
    let sum = res.iter().map(|v| v.price).sum::<f64>();
    Ok(Some(sum / res.len() as f64))
}
