mod config;
mod data;
mod errors;
mod handlers;
mod redis;

use clap::Arg;
use data::Service;
use deadpool_redis::Runtime;
use std::process;
use std::{error::Error, sync::Arc};
use tokio::sync::{RwLock};
use tokio_util::sync::CancellationToken;
use warp::Filter;

use clap::Command;
use config::Config;
use tokio::signal::unix::{signal, SignalKind};

use crate::handlers::PricesParams;
use crate::redis::RedisClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cfg = app_config().unwrap_or_else(|err| {
        log::error!("problem parsing arguments: {err}");
        process::exit(1)
    });

    log::info!("Starting Importer service");
    log::info!("Version      : {}", cfg.version);
    log::info!("Port         : {}", cfg.port);
    log::info!("Redis URL    : {}", cfg.redis_url);

    let cancel_token = CancellationToken::new();

    let ct = cancel_token.clone();

    tokio::spawn(async move {
        let mut int_stream = signal(SignalKind::interrupt()).unwrap();
        let mut term_stream = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = int_stream.recv() => log::info!("Exit event int"),
            _ = term_stream.recv() => log::info!("Exit event term"),
            // _ = rx_exit_indicator.recv() => log::info!("Exit event from some loader"),
        }
        log::debug!("sending exit event");
        ct.cancel();
        log::debug!("expected drop tx_close");
    });

    let pool = deadpool_redis::Config::from_url(&cfg.redis_url)
        .create_pool(Some(Runtime::Tokio1))
        .unwrap_or_else(|err| {
            log::error!("redis poll init: {err}");
            process::exit(1)
        });

    let db = RedisClient::new(pool)
        .await
        .unwrap_or_else(|err| {
            log::error!("redis client init: {err}");
            process::exit(1)
        });

    let srv = Arc::new(RwLock::new(Service {
        redis: db,
    }));

    let live_route = warp::get()
        .and(warp::path("live"))
        .and(with_service(srv.clone()))
        .and_then(handlers::live_handler);
    let prices_route = warp::get()
        .and(warp::path("prices"))
        .and(warp::query::<PricesParams>())
        .and(with_service(srv))
        .and_then(handlers::prices_handler);
    let routes = live_route
        .or(prices_route)
        .with(warp::cors().allow_any_origin())
        .recover(errors::handle_rejection);

    let ct = cancel_token.clone();
    let (_, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], cfg.port), async move {
            ct.cancelled().await;
        });

    log::info!("wait for server to finish");
    tokio::task::spawn(server).await.unwrap_or_else(|err| {
        log::error!("{err}");
        process::exit(1);
    });

    log::info!("Bye");
    Ok(())
}

fn with_service(
    srv: Arc<RwLock<Service>>,
) -> impl Filter<Extract = (Arc<RwLock<Service>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || srv.clone())
}

fn app_config() -> Result<Config, String> {
    let app_version = option_env!("CARGO_APP_VERSION").unwrap_or("dev");

    let cmd = Command::new("importer-ws")
        .version(app_version)
        .author("Airenas V.<airenass@gmail.com>")
        .about("Service for serving emarket data")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Service port")
                .env("PORT")
                .default_value("8000"),
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
