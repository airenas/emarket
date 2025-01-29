use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    pub at: NaiveDateTime,
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
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<Vec<Data>, Box<dyn Error>>;
}

#[async_trait]
pub trait Aggregator {
    async fn work(&mut self, from: NaiveDateTime) -> Result<bool, Box<dyn Error>>;
}

#[async_trait]
pub trait DBSaver {
    async fn live(&self) -> Result<String, Box<dyn Error>>;
    async fn get_last_time(&self) -> Result<Option<NaiveDateTime>, Box<dyn Error>>;
    async fn save(&self, data: &Data) -> Result<bool, Box<dyn Error>>;
    async fn load(
        &self,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<Vec<Data>, Box<dyn Error>>;
}

#[async_trait]
pub trait Limiter: Send + Sync {
    async fn wait(&self) -> Result<bool, Box<dyn Error>>;
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::data::Data;
    #[test]
    fn to_string() {
        assert_eq!(
            Data {
                at: DateTime::from_timestamp_millis(10).unwrap().naive_utc(),
                price: 1.0
            }
            .to_str(),
            "at: 1970-01-01 00:00:00.010, price 1"
        );
    }
}
