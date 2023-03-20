use async_trait::async_trait;
use chrono::{prelude::*, Duration};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use chrono_tz::{Europe::Vilnius, Tz};
use emarket::data::{Aggregator, DBSaver, Data};
use std::error::Error;
use std::ops::Add;

#[derive()]
pub struct AgregatorByDate {
    db_loader: Box<dyn DBSaver + Sync + Send>,
    db_saver: Box<dyn DBSaver + Sync + Send>,
    last_imported_time: Option<NaiveDateTime>,
}

impl AgregatorByDate {
    pub async fn new(
        db_loader: Box<dyn DBSaver + Sync + Send>,
        db_saver: Box<dyn DBSaver + Sync + Send>,
    ) -> Result<AgregatorByDate, Box<dyn Error>> {
        Ok(AgregatorByDate {
            db_loader,
            db_saver,
            last_imported_time: None,
        })
    }
}

#[async_trait]
impl Aggregator for AgregatorByDate {
    async fn work(&mut self, last_item_time: NaiveDateTime) -> Result<bool, Box<dyn Error>> {
        if self.last_imported_time.is_none() {
            self.last_imported_time = self.db_saver.get_last_time().await?
        }
        let mut time = self
            .last_imported_time
            .or_else(|| default_time())
            .ok_or("no start time")?;
        while time <= last_item_time {
            log::info!("aggregate for {time}");
            let (t_from, t_to) = get_from_to(time);
            log::info!("aggregate {t_from} to {t_to}");
            let data = self.db_loader.load(t_from, t_to).await?;
            log::info!("loaded range items {}", data.len());
            if !data.is_empty() {
                let last_time = data[data.len() - 1].at;
                let avg = calc_avg(data);
                if let Some(v) = avg {
                    log::info!("save {v} at {t_from}");
                    self.db_saver
                        .save(&Data {
                            at: t_from,
                            price: v,
                        })
                        .await?;
                }
                self.last_imported_time = Some(last_time);
            }
            time = t_to;
        }
        Ok(true)
    }
}

fn calc_avg(data: Vec<Data>) -> Option<f64> {
    if data.len() == 0 {
        return None;
    }
    let sum: f64 = data.iter().map(|x| x.price).sum();
    return Some(sum / (data.len() as f64));
}

fn default_time() -> Option<NaiveDateTime> {
    NaiveDate::from_ymd_opt(2012, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
}

fn get_from_to(time: NaiveDateTime) -> (NaiveDateTime, NaiveDateTime) {
    let dt = Vilnius
        .with_ymd_and_hms(time.year(), time.month(), time.day(), 0, 0, 0)
        .latest()
        .unwrap();
    let date_to = time.date().add(Duration::days(1));
    let dt_to = Vilnius
        .with_ymd_and_hms(date_to.year(), date_to.month(), date_to.day(), 0, 0, 0)
        .latest()
        .unwrap();
    if dt_to.naive_utc() <= time {
        return (
            dt.naive_utc().add(Duration::days(1)),
            dt_to.naive_utc().add(Duration::days(1)),
        );
    }
    return (dt.naive_utc(), dt_to.naive_utc());
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::aggregator::get_from_to;
    #[test]
    fn get_from_to_adds_day() {
        assert_eq!(
            get_from_to(
                NaiveDate::from_ymd_opt(2023, 1, 1)
                    .unwrap()
                    .and_hms_opt(5, 12, 0)
                    .unwrap()
            ),
            (
                NaiveDate::from_ymd_opt(2022, 12, 31)
                    .unwrap()
                    .and_hms_opt(22, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 1)
                    .unwrap()
                    .and_hms_opt(22, 0, 0)
                    .unwrap()
            )
        );
        assert_eq!(
            get_from_to(
                NaiveDate::from_ymd_opt(2023, 3, 26)
                    .unwrap()
                    .and_hms_opt(5, 12, 0)
                    .unwrap()
            ),
            (
                NaiveDate::from_ymd_opt(2023, 3, 25)
                    .unwrap()
                    .and_hms_opt(22, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2023, 3, 26)
                    .unwrap()
                    .and_hms_opt(21, 0, 0)
                    .unwrap()
            )
        );
    }

    #[test]
    fn get_from_to_moves() {
        let res = get_from_to(
            NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(5, 12, 0)
                .unwrap(),
        );
        assert_eq!(
            get_from_to(res.1),
            (
                NaiveDate::from_ymd_opt(2023, 1, 1)
                    .unwrap()
                    .and_hms_opt(22, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2023, 1, 2)
                    .unwrap()
                    .and_hms_opt(22, 0, 0)
                    .unwrap()
            )
        );
    }
}
