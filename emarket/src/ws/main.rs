mod data;
mod handlers;
mod metrics;
mod otel;
mod redis;

use axum::extract::{DefaultBodyLimit, Request};
use axum::routing::get;
use axum::{middleware, Router};
use clap::{command, Parser};
use data::Service;
use deadpool_redis::Runtime;
use metrics::Metrics;
use opentelemetry::trace::TraceContextExt;
use std::process;
use std::time::Duration;
use std::{error::Error, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use tokio::signal::unix::{signal, SignalKind};

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
    let _guard = TracerGuard;

    use opentelemetry::trace::TracerProvider as _;

    let provider = otel::init_tracer()?;
    let tracer = provider.tracer("importer-ws");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .with(telemetry)
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

    let helper_router = axum::Router::new()
        .route("/live", get(handlers::live::handler))
        .with_state(srv.clone());

    let metrics = Metrics::new()?;
    let main_router = Router::new()
        .route("/summary", get(handlers::summary::handler))
        .route("/prices", get(handlers::prices::handler))
        .route("/np/now", get(handlers::now::handler))
        .with_state(srv.clone())
        .layer(middleware::from_fn(move |req, next| {
            let mc = metrics.clone();
            async move { mc.observe(req, next).await }
        }));

    let app = Router::new()
        .merge(helper_router)
        .merge(main_router)
        .route("/metrics", get(handlers::metrics::handler))
        .layer(CorsLayer::permissive())
        .layer((
            DefaultBodyLimit::max(1024 * 1024),
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                tracing::trace!("request");
                let cx = otel::extract_context_from_request(request);
                tracing::trace!("{:?}", cx.span());
                let name = format!("{} {}", request.method(), request.uri());
                let res = tracing::info_span!(
                    "request",
                    otel.name = name,
                    version = ?request.version(),
                );
                res.set_parent(cx);
                res
            }),
            TimeoutLayer::new(Duration::from_secs(10)),
        ));

    tracing::info!(port = args.port, "serving ...");

    let listener = TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;

    let handle = axum_server::Handle::new();
    let shutdown_future = shutdown_signal_handle(handle.clone(), cancel_token.clone());
    tokio::spawn(shutdown_future);

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            cancel_token.cancelled().await;
        })
        .await?;

    tracing::info!("Bye");
    Ok(())
}

async fn shutdown_signal_handle(handle: axum_server::Handle, cancel_token: CancellationToken) {
    cancel_token.cancelled().await;
    tracing::trace!("Received termination signal shutting down");
    handle.graceful_shutdown(Some(Duration::from_secs(10)));
}

struct TracerGuard;

impl Drop for TracerGuard {
    fn drop(&mut self) {
        tracing::info!("shutting down tracer");
        opentelemetry::global::shutdown_tracer_provider();
    }
}
