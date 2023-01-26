mod entsoe;
mod limiter;
mod redis;

use clap::Arg;
use emarket::data::Data;
use emarket::data::Limiter;
use emarket::WorkingData;
use emarket::{get_last_time, run_exit_indicator, saver_start};
use reqwest::Error;
use tokio_util::sync::CancellationToken;
use std::process;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use emarket::Config;
use entsoe::EntSOE;

use crate::limiter::RateLimiter;
use crate::redis::RedisClient;
use clap::Command;
use emarket::data::DBSaver;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let cfg = app_config().unwrap_or_else(|err| {
        log::error!("problem parsing arguments: {err}");
        process::exit(1)
    });

    log::info!("Starting EMArket importer");
    log::info!("Version      : {}", cfg.version);
    log::info!("Domain       : {}", cfg.domain);
    log::info!("Documnet     : {}", cfg.document);
    log::info!("Redis URL    : {}", cfg.redis_url);
    if cfg.key.len() > 4 {
        log::info!(
            "Key          : {}...{}",
            &cfg.key[..2],
            &cfg.key[cfg.key.len() - 2..]
        );
    }

    let db_saver = RedisClient::new(&cfg.redis_url).await.unwrap_or_else(|err| {
        log::error!("redis client init: {err}");
        process::exit(1)
    });
    // let db_saver = PostgresClientRetryable::new(db_saver);
    let boxed_db_saver: Box<dyn DBSaver + Send + Sync> = Box::new(db_saver);
    log::info!("Test Redis is live ...");
    boxed_db_saver.live().await.unwrap();
    log::info!("Redis OK");

    let limiter = RateLimiter::new().unwrap();
    let boxed_limiter: Box<dyn Limiter> = Box::new(limiter);
    let limiter = Arc::new(Mutex::new(boxed_limiter));

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let cancel_token = CancellationToken::new();
    let (tx_wait_exit, mut rx_wait_exit) = tokio::sync::mpsc::channel(1);
    let (tx_exit_indicator, mut rx_exit_indicator) = tokio::sync::mpsc::unbounded_channel();

    let loader = EntSOE::new(&cfg.document, &cfg.domain, &cfg.key).unwrap();

    //     let interval = config.interval.clone();
    let int_limiter = limiter.clone();
    let start_from = get_last_time(boxed_db_saver.as_ref()).await.unwrap();
    let w_data = WorkingData {
        loader: Box::new(loader),
        start_from,
        sender: tx.clone(),
        limiter: int_limiter,
    };

    let importer = run_exit_indicator(w_data, cancel_token.clone(), tx_exit_indicator.clone());

    let int_exit = tx_wait_exit.clone();
    tokio::spawn(async move { start_saver_loop(boxed_db_saver, &mut rx, int_exit).await });

    tokio::spawn(async move {
        let mut int_stream = signal(SignalKind::interrupt()).unwrap();
        let mut term_stream = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = int_stream.recv() => log::info!("Exit event int"),
            _ = term_stream.recv() => log::info!("Exit event term"),
            _ = rx_exit_indicator.recv() => log::info!("Exit event from some loader"),
        }
        log::debug!("sending exit event");
        cancel_token.cancel();
        log::debug!("expected drop tx_close");
    });

    drop(tx_wait_exit);
    drop(tx);

    _ = importer.await.unwrap_or_else(|err| {
        log::error!("{err}");
        process::exit(1);
    });

    log::info!("wait jobs to finish");
    let _ = rx_wait_exit.recv().await;

    log::info!("Bye");
    Ok(())
}

async fn start_saver_loop(
    db_saver: Box<dyn DBSaver + Send + Sync>,
    receiver: &mut Receiver<Data>,
    _tx_exit: Sender<()>,
) -> Result<(), String> {
    log::info!("Test Redis is live ...");
    db_saver.live().await.unwrap();
    log::info!("Redis OK");

    saver_start(db_saver, receiver).await.unwrap();

    log::info!("exit redis loop");
    Ok(())
}

fn app_config() -> Result<Config, &'static str> {
    let app_version = option_env!("CARGO_APP_VERSION").unwrap_or("dev");

    let cmd = Command::new("importer")
        .version(app_version)
        .author("Airenas V.<airenass@gmail.com>")
        .about("Import entsoe day ahead prices to local timeseries DB")
        .arg(
            Arg::new("document")
                .short('d')
                .long("document")
                .value_name("DOCUMENT")
                .help("EntSOE query document type")
                .env("DOCUMENT")
                .default_value("A44"),
        )
        .arg(
            Arg::new("domain")
                .short('m')
                .long("domain")
                .value_name("DOMAIN")
                .env("DOMAIN")
                .help("EntSOE query domain value")
                .default_value("10YLT-1001A0008Q"),
        )
        .arg(
            Arg::new("key")
                .short('k')
                .long("key")
                .value_name("KEY")
                .env("KEY")
                .help("EntSOE auth key"),
        )
        .arg(
            Arg::new("redis")
                .short('r')
                .long("redis")
                .value_name("REDIS_URL")
                .env("REDIS_URL")
                .help("Redis URL"),
        )
        .get_matches();
    let mut config = Config::build(&cmd)?;
    config.version = app_version.into();
    Ok(config)
}
