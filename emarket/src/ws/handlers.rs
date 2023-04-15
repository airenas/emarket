use std::{sync::Arc};

use crate::{
    data::{Service, SummaryData},
    errors::OtherError,
};
use chrono::{NaiveDateTime, Utc};
use emarket::utils::{time_day_vilnius, time_month_vilnius, to_time};
use serde::Serialize;
use serde_derive::Deserialize;
use tokio::sync::RwLock;
use warp::{Rejection, Reply};

type Result<T> = std::result::Result<T, Rejection>;

#[derive(Deserialize)]
pub struct PricesParams {
    from: Option<i64>,
    to: Option<i64>,
}

#[derive(Deserialize)]
pub struct SummaryParams {
    at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PricesResult {
    at: i64,
    price: f64,
}

pub async fn live_handler(srv_wrap: Arc<RwLock<Service>>) -> Result<impl Reply> {
    log::debug!("live handler");
    let srv = srv_wrap.read().await;
    let res = srv.redis.live().await;
    match res {
        Ok(s) => Ok(warp::reply::json(&s).into_response()),
        Err(err) => Err(OtherError {
            msg: err.to_string(),
        }
        .into()),
    }
}

pub async fn prices_handler(
    params: PricesParams,
    srv_wrap: Arc<RwLock<Service>>,
) -> Result<impl Reply> {
    log::debug!("prices handler");
    let srv = srv_wrap.read().await;
    log::debug!(
        "from = {}",
        params
            .from
            .map_or_else(|| "none".to_owned(), |v| v.to_string())
    );
    log::debug!(
        "to = {}",
        params
            .to
            .map_or_else(|| "none".to_owned(), |v| v.to_string())
    );
    let res = srv.redis.load("np_lt_m", params.from, params.to).await;
    match res {
        Ok(s) => Ok(warp::reply::json(&s).into_response()),
        Err(err) => Err(OtherError {
            msg: err.to_string(),
        }
        .into()),
    }
}

pub async fn summary_handler(
    params: SummaryParams,
    srv_wrap: Arc<RwLock<Service>>,
) -> Result<impl Reply> {
    log::debug!("prices handler");
    let srv = srv_wrap.read().await;
    log::debug!(
        "at = {}",
        params
            .at
            .map_or_else(|| "none".to_owned(), |v| v.to_string())
    );

    let at = match params.at {
        Some(a) => a,
        None => Utc::now().timestamp_millis(),
    };

    let res = SummaryData {
        at,
        current_month_avg: get_value(&srv.redis, "np_lt_m", month(at, 0), month(at, 1)).await?,
        previous_month_avg: get_value(&srv.redis, "np_lt_m", month(at, -1), month(at, 0)).await?,
        today_avg: get_value(&srv.redis, "np_lt_d", day(at, 0), day(at, 1)).await?,
        tomorrow_avg: get_value(&srv.redis, "np_lt_d", day(at, 1), day(at, 2)).await?,
        yesterday_avg: get_value(&srv.redis, "np_lt_d", day(at, -1), day(at, 0)).await?,
        last_30d_avg: get_avg(&srv.redis, "np_lt_d", day(at, -29), day(at, 1)).await?,
        last_7_avg: get_avg(&srv.redis, "np_lt_d", day(at, -6), day(at, 1)).await?,
    };

    Ok(warp::reply::json(&res).into_response())
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
) -> Result<Option<f64>> {
    let res = redis
        .load(
            ts_name,
            Some(from.timestamp_millis()),
            Some(to.timestamp_millis()),
        )
        .await
        .map_err(|e| OtherError { msg: e.to_string() })?;
    log::debug!("{} len res {} - {}-{}", ts_name, res.len(), from, to);
    if res.is_empty() {
        return Ok(None);
    }
    Ok(Some(res[0].price))
}

async fn get_avg(
    redis: &crate::redis::RedisClient,
    ts_name: &str,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Result<Option<f64>> {
    let res = redis
        .load(
            ts_name,
            Some(from.timestamp_millis()),
            Some(to.timestamp_millis()),
        )
        .await
        .map_err(|e| OtherError { msg: e.to_string() })?;
    log::debug!("{} len res {} - {}-{}", ts_name, res.len(), from, to);
    if res.is_empty() {
        return Ok(None);
    }
    let sum = res.iter().map(|v| v.price).sum::<f64>();
    Ok(Some(sum / res.len() as f64))
}
