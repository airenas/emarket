use std::sync::Arc;

use axum::{
    extract::{self, Query, State},
    Json,
};
use emarket::utils::to_str_or_none;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::data::{ApiError, ApiResult, MarketData, Service};

#[derive(Deserialize)]
pub struct PricesParams {
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
        "params"
    );

    let res = srv
        .redis
        .load("np_lt_m", params.from, params.to)
        .await
        .map_err(|e| ApiError::Server(e.to_string()))?;
    Ok(Json(res))
}
