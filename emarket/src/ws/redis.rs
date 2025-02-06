use deadpool_redis::Pool;
use redis_ts::{AsyncTsCommands, TsRange};
use tracing::instrument;
use std::error::Error;

use crate::data::MarketData;

#[derive(Clone)]
pub struct RedisClient {
    pool: Pool,
}

impl RedisClient {
    pub async fn new(pool: Pool) -> Result<RedisClient, Box<dyn Error>> {
        Ok(RedisClient { pool })
    }
}

impl RedisClient {
    pub async fn live(&self) -> std::result::Result<String, Box<dyn Error>> {
        log::debug!("invoke live");
        let mut conn = self.pool.get().await?;
        let _: () = redis::cmd("PING").query_async(&mut conn).await?;
        Ok("ok".to_string())
    }

    #[instrument(skip(self))]
    pub async fn load(
        &self,
        ts_name: &str,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<Vec<MarketData>, Box<dyn Error>> {
        tracing::debug!("invoke load");
        
        let mut conn = self.pool.get().await?;
        let none_int: Option<u64> = None;
        let from_s = match from {
            Some(v) => v.to_string(),
            None => "-".to_owned(),
        };
        let to_s = match to {
            Some(v) => (v - 1).to_string(),
            None => "+".to_owned(),
        };
        let list: TsRange<u64, f64> = conn.ts_range(ts_name, from_s, to_s, none_int, None).await?;
        let res = list
            .values
            .iter()
            .map(|f| MarketData {
                at: f.0,
                price: f.1,
            })
            .collect();
        Ok(res)
    }
}
