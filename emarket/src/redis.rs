use async_trait::async_trait;
use chrono::NaiveDateTime;
use deadpool_redis::{Config, Pool, Runtime};
use emarket::{
    data::{DBSaver, Data},
    utils::to_time,
};
use redis::{RedisError};
use redis_ts::{AsyncTsCommands, TsOptions};
use std::error::Error;

#[derive()]
pub struct RedisClient {
    pool: Pool,
    ts_name: String,
}

impl RedisClient {
    pub async fn new(db_url: &str) -> Result<RedisClient, Box<dyn Error>> {
        let cfg = Config::from_url(db_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        let ts_name = "np_lt";
        let mut conn = pool.get().await?;
        let r: Result<bool, RedisError> = conn
            .ts_create(
                ts_name,
                TsOptions::default()
                    .retention_time(0)
                    .duplicate_policy(redis_ts::TsDuplicatePolicy::Last),
            )
            .await;
        match r {
            Ok(_) => {
                log::info!("DB {} initialized", ts_name);
            }
            Err(e) => {
                if e.detail().unwrap_or("") != "TSDB: key already exists" {
                    return Err(e).map_err(|e| e.into());
                }
            }
        }
        Ok(RedisClient {
            pool,
            ts_name: ts_name.to_string(),
        })
    }
}

#[async_trait]
impl DBSaver for RedisClient {
    async fn live(&self) -> std::result::Result<String, Box<dyn Error>> {
        log::debug!("invoke live");
        let mut conn = self.pool.get().await?;
        redis::cmd("PING").query_async(&mut conn).await?;
        Ok("ok".to_string())
    }

    async fn get_last_time(&self) -> Result<Option<NaiveDateTime>, Box<dyn Error>> {
        log::debug!("invoke live");
        let mut conn = self.pool.get().await?;
        let latest: Option<(u64, f64)> = conn.ts_get(self.ts_name.as_str()).await?;
        let res = match latest {
            Some((t, _)) => {
                log::debug!("got last time {}", t);
                Some(to_time(t))
            }
            None => {
                log::debug!("no last items");
                None
            }
        };
        Ok(res)
    }

    async fn save(&self, data: &Data) -> Result<bool, Box<dyn Error>> {
        let mut conn = self.pool.get().await?;
        conn.ts_add(
            self.ts_name.as_str(),
            data.at.timestamp_millis(),
            data.price,
        )
        .await?;
        Ok(true)
    }
}
