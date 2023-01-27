pub mod data;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use clap::ArgMatches;
use data::{DBSaver, Data, Limiter, Loader};
use std::error::Error;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};
use tokio_util::sync::CancellationToken;

pub struct Config {
    pub document: String,
    pub domain: String,
    pub key: String,
    pub redis_url: String,
    pub version: String,
}

impl Config {
    pub fn build(args: &ArgMatches) -> Result<Config, &'static str> {
        let doc: &String = match args.get_one::<String>("document") {
            Some(v) => Ok(v),
            None => Err("no document param provided"),
        }?;
        let domain = match args.get_one::<String>("domain") {
            Some(v) => Ok(v),
            None => Err("no domain provided"),
        }?;
        let key = match args.get_one::<String>("key") {
            Some(v) => Ok(v),
            None => Err("no key provided"),
        }?;
        let redis_url = match args.get_one::<String>("redis") {
            Some(v) => Ok(v),
            None => Err("no redis URL provided"),
        }?;
        Ok(Config {
            key: key.to_string(),
            document: doc.to_string(),
            domain: domain.to_string(),
            redis_url: redis_url.to_string(),
            version: "dev".to_string(),
        })
    }
}

type LimiterM = std::sync::Arc<Mutex<Box<dyn Limiter>>>;
type ResultM = Result<(), Box<dyn Error>>;

pub struct WorkingData {
    pub start_from: DateTime<Utc>,
    pub loader: Box<dyn Loader>,
    pub limiter: LimiterM,
    pub sender: Sender<Data>,
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
        if imported == 0 && to < Utc::now() {
            from = from + take_dur - Duration::days(1);
            continue;
        } else if imported == 0 {
            log::info!("no new imports");
            from = last_item_time;
            let sleep_time = Duration::hours(1);
            log::info!("sleep till {}", Utc::now() + sleep_time);
            let sleep = tokio::time::sleep(sleep_time.to_std()?);
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {},
                _ = close_token.cancelled() => {
                    log::debug!("got cancel event");
                    break;
                }
            }
        } else {
            from = last_item_time;
        }
    }
    log::info!("exit import loop");
    Ok(())
}

pub async fn get_last_time(
    db: &'_ (dyn DBSaver + Send + Sync),
) -> Result<DateTime<Utc>, Box<dyn Error>> {
    log::info!("Get last value in DB");
    db.get_last_time()
        .await
        .map_err(|e| format!("get last time: {} ", e).into())
}

async fn import(
    w_data: &WorkingData,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<(DateTime<Utc>, u64), Box<dyn Error>> {
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
    let mut res = from.timestamp_millis();

    for line in data {
        if res < line.at {
            res = line.at;
        }
        w_data.sender.send(line).await?;
    }
    log::debug!("send lines to save");
    let res_time = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_millis(res).expect("invalid date"),
        Utc,
    );
    if from == res_time { // nothing new
        return Ok((res_time, 0));
    }
    return Ok((res_time, c.try_into()?));
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
                .map_err(|err| format!("save err: {}", err))?,
            None => break,
        }
    }
    log::info!("exit save loop");
    Ok(())
}
