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
    let mut last_time = w_data.start_from;
    let dur = chrono::Duration::from_std(
        duration_str::parse("1m").map_err(|e| format!("duration  parse: {}", e))?,
    )
    .map_err(|e| format!("duration parse: {}", e))?;

    loop {
        log::info!("loop");
        if close_token.is_cancelled() {
            log::debug!("got cancel event");
            break;
        }
        log::info!("after check");
        let max_dur = chrono::Duration::minutes(15);
        let mut td = last_time - (Utc::now() - dur);
        if td < chrono::Duration::zero() {
            last_time = import(&w_data, last_time).await?;
        } else {
            if td > max_dur {
                td = max_dur;
            }
            log::info!("sleep till {}", Utc::now() + td);
            let sleep = tokio::time::sleep(td.to_std()?);
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {},
                _ = close_token.cancelled() => {
                    log::debug!("got cancel event");
                    break;
                }
            }
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
) -> Result<DateTime<Utc>, Box<dyn Error>> {
    {
        log::info!("wait for import");
        let wait = w_data.limiter.lock().await;
        wait.wait().await?;
        log::info!("let's go");
    }
    log::info!("Getting data from {}", from);

    let data = w_data
        .loader
        .retrieve(from, from + Duration::days(7))
        .await?;
    data.iter().for_each(|f| {
        log::trace!("{}", f.to_str());
        // w_data.saver.save(f).await;
    });
    log::info!("got {} lines", data.len());
    let mut res = from.timestamp_millis();
    for line in data {
        if line.at > from.timestamp_millis() {
            res = line.at;
        }
        w_data.sender.send(line).await?;
    }
    log::debug!("send lines to save");
    Ok(DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_millis(res).expect("invalid date"),
        Utc,
    ))
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
