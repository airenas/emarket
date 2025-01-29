mod data;
mod errors;
mod handlers;
mod redis;

use clap::{command, Parser};
use data::Service;
use deadpool_redis::Runtime;
use std::process;
use std::{error::Error, sync::Arc};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use warp::Filter;

use tokio::signal::unix::{signal, SignalKind};

use crate::handlers::{PricesParams, SummaryParams};
use crate::redis::RedisClient;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Parser, Debug)]
#[command(version = env!("CARGO_APP_VERSION"), name = "importer-ws", about="Service for serving emarket data", author ="Airenas V.<airenass@gmail.com>", long_about = None)]
struct Args {
    /// Server port
    #[arg(long, env, default_value = "8000")]
    port: u16,
    /// Redis url
    #[arg(long, short, env, default_value = "")]
    redis_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();
    let args = Args::parse();
    if let Err(e) = main_int(args).await {
        tracing::error!("{}", e);
        return Err(e);
    }
    Ok(())
}

async fn main_int(args: Args) -> Result<(), Box<dyn Error>> {
    tracing::info!("Starting Importer service");
    tracing::info!(version = env!("CARGO_APP_VERSION"));
    tracing::info!(port = args.port);
    tracing::info!(url = args.redis_url, "redis");

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

    let pool = deadpool_redis::Config::from_url(&args.redis_url)
        .create_pool(Some(Runtime::Tokio1))
        .unwrap_or_else(|err| {
            log::error!("redis poll init: {err}");
            process::exit(1)
        });

    let db = RedisClient::new(pool).await.unwrap_or_else(|err| {
        log::error!("redis client init: {err}");
        process::exit(1)
    });

    let srv = Arc::new(RwLock::new(Service { redis: db }));

    let live_route = warp::get()
        .and(warp::path("live"))
        .and(with_service(srv.clone()))
        .and_then(handlers::live_handler);
    let prices_route = warp::get()
        .and(warp::path("prices"))
        .and(warp::query::<PricesParams>())
        .and(with_service(srv.clone()))
        .and_then(handlers::prices_handler);
    let summary_route = warp::get()
        .and(warp::path("summary"))
        .and(warp::query::<SummaryParams>())
        .and(with_service(srv))
        .and_then(handlers::summary_handler);
    let routes = live_route
        .or(prices_route)
        .or(summary_route)
        .with(warp::cors().allow_any_origin())
        .recover(errors::handle_rejection);

    let ct = cancel_token.clone();
    let (_, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], args.port), async move {
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
