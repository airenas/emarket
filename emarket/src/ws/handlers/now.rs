use std::sync::Arc;

use axum::{
    extract::{self, State},
    Json,
};
use chrono::{Duration, NaiveDateTime, Timelike, Utc};
use emarket::TN_HOUR;
use tokio::sync::RwLock;

use crate::{
    data::{ApiResult, NowData, Service},
    handlers::summary::get_value,
};

pub async fn handler(
    State(srv_wrap): State<Arc<RwLock<Service>>>,
) -> ApiResult<extract::Json<NowData>> {
    tracing::debug!("now handler");
    let srv = srv_wrap.read().await;

    let from = hour_start(Utc::now().naive_utc());
    let to = from + Duration::minutes(30); // make range from start of an hour to half an hour

    let res = NowData {
        at: from.and_utc().timestamp_millis(),
        price: get_value(&srv.redis, TN_HOUR, from, to).await?,
    };
    Ok(Json(res))
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
