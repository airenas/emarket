use std::sync::Arc;

use axum::{
    extract::{self, State},
    Json,
};
use tokio::sync::RwLock;

use crate::data::{ApiResult, LiveResponse, Service};

pub async fn handler(
    State(srv_wrap): State<Arc<RwLock<Service>>>,
) -> ApiResult<extract::Json<LiveResponse>> {
    let srv = srv_wrap.read().await;
    let res = srv.redis.live().await;
    let (r_res, st) = match res {
        Ok(_) => ("ok".to_string(), true),
        Err(err) => (err.to_string(), false),
    };
    let res = LiveResponse {
        redis: r_res,
        status: st,
        version: env!("CARGO_APP_VERSION").to_string(),
    };
    Ok(Json(res))
}
