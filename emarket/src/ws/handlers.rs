use std::sync::Arc;

use crate::{
    data::Service,
    errors::{OtherError},
};
use emarket::utils::to_time;
use serde::Serialize;
use serde_derive::Deserialize;
use tokio::sync::RwLock;
use warp::{Rejection, Reply};

type Result<T> = std::result::Result<T, Rejection>;

#[derive(Deserialize)]
pub struct PricesParams {
    from: Option<u64>,
    to: Option<u64>,
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
    log::debug!("from = {}", params.from.map_or_else(|| "none".to_owned(), |v| v.to_string()));
    log::debug!("to = {}", params.to.map_or_else(|| "none".to_owned(), |v| v.to_string()));
    let res = srv.redis.load("np_lt_m", params.from, params.to).await;
    match res {
        Ok(s) => Ok(warp::reply::json(&s).into_response()),
        Err(err) => Err(OtherError {
            msg: err.to_string(),
        }
        .into()),
    }
}
