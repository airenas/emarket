use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use emarket::data::{DBSaver, Data};
use redis::{Client, RedisError};
use redis_ts::{AsyncTsCommands, TsOptions};
use std::{error::Error};

#[derive()]
pub struct RedisClient {
    client: Client,
    ts_name: String,
}

impl RedisClient {
    pub async fn new(db_url: &str) -> Result<RedisClient, Box<dyn Error>> {
        let ts_name = "np_lt";
        let client = redis::Client::open(db_url)?;
        let mut conn = client.get_async_connection().await?;
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
            client,
            ts_name: ts_name.to_string(),
        })
    }
}

#[async_trait]
impl DBSaver for RedisClient {
    async fn live(&self) -> std::result::Result<String, Box<dyn Error>> {
        log::debug!("invoke live");
        let mut conn = self.client.get_async_connection().await?;
        redis::cmd("PING").query_async(&mut conn).await?;
        Ok("ok".to_string())
    }

    async fn get_last_time(&self) -> Result<DateTime<Utc>, Box<dyn Error>> {
        log::debug!("invoke live");
        let mut conn = self.client.get_async_connection().await?;
        let latest: Option<(u64, f64)> = conn.ts_get(self.ts_name.as_str()).await?;
        let default = NaiveDate::from_ymd_opt(2014, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let res = match latest {
            Some(v) => {
                log::debug!("got last time {}", v.0);
                let dt =
                    NaiveDateTime::from_timestamp_millis(i64::try_from(v.0)?).unwrap_or(default);
                DateTime::<Utc>::from_utc(dt, Utc)
            }
            None => {
                log::debug!("no last items");
                DateTime::<Utc>::from_utc(default, Utc)
            }
        };
        Ok(res)
    }

    async fn save(&self, data: &Data) -> Result<bool, Box<dyn Error>> {
        let mut conn = self.client.get_async_connection().await?;
        conn.ts_add(self.ts_name.as_str(), data.at, data.price)
            .await?;
        Ok(true)
    }
}
