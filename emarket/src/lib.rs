pub mod data;
pub mod utils;

use chrono::{Duration, NaiveDateTime, Utc};
use data::{Aggregator, DBSaver, Data, Limiter, Loader};
use std::error::Error;
use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use crate::utils::jitter;

pub const TN_HOUR: &str = "np_lt";
pub const TN_DAY: &str = "np_lt_d";
pub const TN_MONTH: &str = "np_lt_m";

type LimiterM = std::sync::Arc<Mutex<Box<dyn Limiter>>>;
type ResultM = Result<(), Box<dyn Error>>;

pub struct WorkingData {
    pub start_from: NaiveDateTime,
    pub loader: Box<dyn Loader>,
    pub limiter: LimiterM,
    pub sender: Sender<Data>,
    pub import_indicator: Sender<NaiveDateTime>,
}

pub async fn run_exit_indicator(
    w_data: WorkingData,
    close_token: CancellationToken,
    exit_ind: tokio::sync::mpsc::UnboundedSender<i32>,
) -> ResultM {
    match run(w_data, close_token).await {
        Ok(_) => {
            log::info!("exit run");
        }
        Err(err) => {
            log::error!("{}", err);
            log::info!("sending exit signal");
            exit_ind.send(1)?;
            log::info!("sent exit signal");
            return Err(err);
        }
    }
    Ok(())
}

pub async fn run(w_data: WorkingData, close_token: CancellationToken) -> ResultM {
    log::info!("Importing: from {}", w_data.start_from);
    log::info!("Test EntSOE is live");
    match w_data.loader.live().await {
        Ok(_) => {
            log::info!("EntSOE OK");
        }
        Err(err) => {
            return Err(err);
        }
    }
    let mut from = w_data.start_from;
    let take_dur = Duration::days(7);
    loop {
        log::info!("loop");
        if close_token.is_cancelled() {
            log::debug!("cancel detected");
            break;
        }
        let to = from + take_dur;
        let (last_item_time, imported) = import(&w_data, from, to).await?;
        log::info!(
            "got last item time {}, imported {}",
            last_item_time,
            imported
        );
        let now = Utc::now().naive_utc();
        if imported == 0 && to < now {
            from = from + take_dur - Duration::days(1);
            continue;
        } else if imported == 0 {
            log::info!("no new imports");
            let sleep_time = get_sleep(last_item_time, now, jitter);
            log::info!("sleep till {}", now + sleep_time);
            let sleep = tokio::time::sleep(sleep_time.to_std()?);

            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {},
                _ = close_token.cancelled() => {
                    log::debug!("got cancel event");
                    break;
                }
            }
        }
        log::info!("send import indicator to {last_item_time}");
        w_data.import_indicator.send(last_item_time).await?;
        from = last_item_time;
    }
    log::info!("exit import loop");
    Ok(())
}

fn get_sleep(
    last_item_time: NaiveDateTime,
    now: NaiveDateTime,
    j_f: fn(Duration) -> Duration,
) -> Duration {
    //expected new data to be fefore 10h
    let expected_next = last_item_time - Duration::hours(10) - Duration::minutes(10);
    let sleep = if now < expected_next {
        expected_next - now
    } else {
        Duration::minutes(3)
    };
    sleep + j_f(Duration::minutes(5))
}

async fn import(
    w_data: &WorkingData,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Result<(NaiveDateTime, u64), Box<dyn Error>> {
    {
        log::info!("wait for import");
        let wait = w_data.limiter.lock().await;
        wait.wait().await?;
        log::info!("let's go");
    }
    log::info!("loading data from {}", from);

    let data = w_data.loader.retrieve(from, to).await?;
    data.iter().for_each(|f| {
        log::trace!("{}", f.to_str());
        // w_data.saver.save(f).await;
    });
    let c = data.len();
    log::info!("got {} lines", data.len());
    let mut res = from;

    for line in data {
        if res < line.at {
            res = line.at;
        }
        w_data.sender.send(line).await?;
    }
    log::debug!("send lines to save");
    if from == res {
        // nothing new
        return Ok((res, 0));
    }
    Ok((res, c.try_into()?))
}

pub async fn saver_start(
    db: Box<dyn DBSaver + Send + Sync>,
    receiver: &mut Receiver<Data>,
) -> Result<(), String> {
    log::info!("start db saver loop");
    loop {
        let line = receiver.recv().await;
        log::trace!("got line");
        match line {
            Some(line) => db
                .save(&line)
                .await
                .map(|_v| ())
                .map_err(|e| format!("save err: {e}"))?,
            None => break,
        }
    }
    log::info!("exit save loop");
    Ok(())
}

pub async fn aggregate_start(
    mut worker: Box<dyn Aggregator + Send + Sync>,
    receiver: &mut Receiver<NaiveDateTime>,
) -> Result<(), String> {
    log::info!("start db saver loop");
    loop {
        let td = receiver.recv().await;
        log::trace!("got aggregate indicator");
        match td {
            Some(td) => {
                sleep(tokio::time::Duration::from_millis(1000)).await; // problems with selecting new data from redis - wait a bit before aggregation
                worker
                    .work(td)
                    .await
                    .map(|_v| ())
                    .map_err(|e| format!("save err: {e}"))?;
            }
            None => break,
        }
    }
    log::info!("exit aggreagate save loop");
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::get_sleep;

    #[test]
    fn get_sleep_long() {
        let now = Utc::now().naive_utc();
        let at = now + Duration::hours(25);
        assert_eq!(
            get_sleep(at, now, |_| Duration::minutes(0)),
            at - now - Duration::hours(10) - Duration::minutes(10)
        );
    }
    #[test]
    fn get_sleep_near() {
        let now = Utc::now().naive_utc();
        let at = now + Duration::hours(5);
        assert_eq!(
            get_sleep(at, now, |_| Duration::minutes(0)),
            Duration::minutes(3)
        );
    }
    #[test]
    fn get_sleep_near_jitter() {
        let now = Utc::now().naive_utc();
        let at = now + Duration::hours(5);
        assert_eq!(
            get_sleep(at, now, |_| Duration::minutes(1)),
            Duration::minutes(4)
        );
    }
    #[test]
    fn get_sleep_near_10() {
        let now = Utc::now().naive_utc();
        let at = now + Duration::hours(10) + Duration::minutes(11);
        assert_eq!(
            get_sleep(at, now, |_| Duration::minutes(0)),
            Duration::minutes(1)
        );
    }
    #[test]
    fn get_sleep_near_10_2() {
        let now = Utc::now().naive_utc();
        let at = now + Duration::hours(10) + Duration::minutes(9);
        assert_eq!(
            get_sleep(at, now, |_| Duration::minutes(0)),
            Duration::minutes(3)
        );
    }
}
