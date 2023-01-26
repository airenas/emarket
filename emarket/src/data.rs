use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Data {
    pub at: i64,
    pub price: f64,
}

impl Data {
    pub fn to_str(&self) -> String {
        format!("at: {}, price {}", self.at, self.price)
    }
}

#[async_trait]
pub trait Loader {
    async fn live(&self) -> Result<String, Box<dyn Error>>;
    async fn retrieve(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Data>, Box<dyn Error>>;
}

#[async_trait]
pub trait DBSaver {
    async fn live(&self) -> Result<String, Box<dyn Error>>;
    async fn get_last_time(&self) -> Result<DateTime<Utc>, Box<dyn Error>>;
    async fn save(&self, data: &Data) -> Result<bool, Box<dyn Error>>;
}

#[async_trait]
pub trait Limiter {
    async fn wait(&self) -> Result<bool, Box<dyn Error>>;
}

#[cfg(test)]
mod tests {
    use crate::data::Data;
    #[test]
    fn to_string() {
        assert_eq!(Data { at: 10, price: 1.0 }.to_str(), "at: 10, price 1");
    }
}
