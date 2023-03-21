use async_trait::async_trait;
use chrono::{prelude::*, Duration};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use chrono_tz::Europe::Vilnius;
use emarket::data::{Aggregator, DBSaver, Data};
use std::error::Error;
use std::ops::Add;

#[derive()]
pub struct AgregatorByDate {
    db_loader: Box<dyn DBSaver + Sync + Send>,
    db_saver: Box<dyn DBSaver + Sync + Send>,
    last_imported_time: Option<NaiveDateTime>,
    time_func: fn(NaiveDateTime) -> (NaiveDateTime, NaiveDateTime),
}

impl AgregatorByDate {
    pub async fn new(
        db_loader: Box<dyn DBSaver + Sync + Send>,
        db_saver: Box<dyn DBSaver + Sync + Send>,
        time_func: fn(NaiveDateTime) -> (NaiveDateTime, NaiveDateTime),
    ) -> Result<AgregatorByDate, Box<dyn Error>> {
        Ok(AgregatorByDate {
            db_loader,
            db_saver,
            last_imported_time: None,
            time_func,
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
            let (t_from, t_to) = (self.time_func)(time);
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

#[derive()]
pub struct Aggregators {
    pub aggregators: Vec<Box<dyn Aggregator + Sync + Send>>,
}

#[async_trait]
impl Aggregator for Aggregators {
    async fn work(&mut self, last_item_time: NaiveDateTime) -> Result<bool, Box<dyn Error>> {
        for a in self.aggregators.iter_mut() {
            a.work(last_item_time).await?;
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

pub fn time_day(time: NaiveDateTime) -> (NaiveDateTime, NaiveDateTime) {
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
        return time_day(time.add(Duration::hours(1)));
    }
    return (dt.naive_utc(), dt_to.naive_utc());
}

pub fn time_month(time: NaiveDateTime) -> (NaiveDateTime, NaiveDateTime) {
    let dt = Vilnius
        .with_ymd_and_hms(time.year(), time.month(), 1, 0, 0, 0)
        .latest()
        .unwrap();
    let (next_month, next_year) = if time.month() == 12 {
        (1, time.year() + 1)
    } else {
        (time.month() + 1, time.year())
    };
    let dt_to = Vilnius
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .latest()
        .unwrap();
    if dt_to.naive_utc() <= time {
        return time_month(time.add(Duration::days(1)));
    }
    return (dt.naive_utc(), dt_to.naive_utc());
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};

    use crate::aggregator::{time_day, time_month};
    #[test]
    fn get_from_to_adds_day() {
        assert_eq!(
            time_day(dt(2023, 1, 1, 5, 12, 0)),
            (dt(2022, 12, 31, 22, 0, 0), dt(2023, 1, 1, 22, 0, 0))
        );
        assert_eq!(
            time_day(dt(2023, 3, 26, 5, 12, 0)),
            (dt(2023, 3, 25, 22, 0, 0), dt(2023, 3, 26, 21, 0, 0))
        );
    }

    #[test]
    fn get_from_to_moves() {
        let res = time_day(dt(2023, 1, 1, 5, 12, 0));
        assert_eq!(
            time_day(res.1),
            (dt(2023, 1, 1, 22, 0, 0), dt(2023, 1, 2, 22, 0, 0))
        );
    }
    #[test]
    fn get_from_to_moves_at_shift() {
        let res = time_day(dt(2023, 3, 25, 5, 12, 0));
        println!("first: {} - {}", res.0, res.1);
        assert_eq!(
            time_day(res.1),
            (dt(2023, 3, 25, 22, 0, 0), dt(2023, 3, 26, 21, 0, 0))
        );
    }
    #[test]
    fn get_from_to_moves_at_shift_before() {
        let res = time_day(dt(2023, 3, 26, 5, 12, 0));
        println!("first: {} - {}", res.0, res.1);
        assert_eq!(
            time_day(res.1),
            (dt(2023, 3, 26, 21, 0, 0), dt(2023, 3, 27, 21, 0, 0))
        );
    }

    fn dt(year: i32, month: u32, day: u32, h: u32, m: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(h, m, s)
            .unwrap()
    }
    #[test]
    fn time_month_ok() {
        assert_eq!(
            time_month(dt(2023, 1, 1, 5, 12, 0)),
            (dt(2022, 12, 31, 22, 0, 0), dt(2023, 1, 31, 22, 0, 0))
        );
        assert_eq!(
            time_month(dt(2022, 12, 31, 5, 12, 0)),
            (dt(2022, 11, 30, 22, 0, 0), dt(2022, 12, 31, 22, 0, 0))
        );
        assert_eq!(
            time_month(dt(2023, 3, 26, 5, 12, 0)),
            (dt(2023, 2, 28, 22, 0, 0), dt(2023, 3, 31, 21, 0, 0))
        );
    }
    #[test]
    fn time_month_next() {
        let res = time_month(dt(2023, 1, 1, 5, 12, 0));
        assert_eq!(
            time_month(res.1),
            (dt(2023, 1, 31, 22, 0, 0), dt(2023, 2, 28, 22, 0, 0))
        );
        let res = time_month(dt(2023, 2, 1, 5, 12, 0));
        assert_eq!(
            time_month(res.1),
            (dt(2023, 2, 28, 22, 0, 0), dt(2023, 3, 31, 21, 0, 0))
        );
    }
}
