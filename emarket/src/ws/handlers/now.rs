use std::sync::Arc;

use axum::{
    extract::{self, State},
    Json,
};
use chrono::{Duration, NaiveDateTime, Timelike, Utc};
use emarket::TN_HOUR;
use tokio::sync::RwLock;

use tracing::instrument;

use crate::{
    data::{ApiResult, MarketData, NowData, Service},
    handlers::summary::{get_list},
};

#[instrument(skip(srv_wrap))]
pub async fn handler(
    State(srv_wrap): State<Arc<RwLock<Service>>>,
) -> ApiResult<extract::Json<NowData>> {
    tracing::debug!("now handler");

    let srv = srv_wrap.read().await;

    let now = Utc::now().naive_utc();

    let from = hour_start(now);
    let to = from + Duration::minutes(50); // make range from start of an hour to 50 minutes later

    let list = get_list(&srv.redis, TN_HOUR, from, to).await?;

    let v = get_best_value(&list, now);
    match v {
        Ok(v) => {return Ok(Json(v));},
        Err(e) => {
            return Err(crate::data::ApiError::Server(format!(
                "Error getting best value for now: {}",
                e
            )));
        }   
    }
}

fn get_best_value(list: &[MarketData], now: NaiveDateTime) -> anyhow::Result<NowData> {
    let timestamp_millis = now.and_utc().timestamp_millis() as u64;
    if list.is_empty() {
        return Err(anyhow::anyhow!("No data available"));
    }
    let mut res = NowData {
        at: list[0].at as i64,
        price: Some(list[0].price),
    };
    for item in list {
        if item.at <= timestamp_millis {
            res.at = item.at as i64;
            res.price = Some(item.price);
        } else {
            break;
        }
    }
    Ok(res)
}

fn hour_start(now: NaiveDateTime) -> NaiveDateTime {
    now.with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

#[test]
fn test_hour_start() {
    let input = NaiveDateTime::parse_from_str("2025-01-29 14:37:22", "%Y-%m-%d %H:%M:%S").unwrap();
    let expected =
        NaiveDateTime::parse_from_str("2025-01-29 14:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    let result = hour_start(input);

    assert_eq!(
        result, expected,
        "hour_start did not return the expected result"
    );
}
