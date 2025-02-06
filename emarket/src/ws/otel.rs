use std::env;
use std::time::Duration;

use axum::extract::Request;
use opentelemetry::global;
use opentelemetry::Context;
use opentelemetry::KeyValue;
use opentelemetry_http::HeaderExtractor;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace;

pub fn extract_context_from_request(req: &Request) -> Context {
    global::get_text_map_propagator(|propagator| {
        for (key, value) in req.headers().iter() {
            tracing::info!("Header: {} = {:?}", key, value);
        }
        propagator.extract(&HeaderExtractor(req.headers()))
    })
}

pub fn init_tracer() -> anyhow::Result<opentelemetry_sdk::trace::TracerProvider> {
    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "importer-ws".to_string());
    let otel_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    let sampling_rate: f64 = env::var("OTEL_SAMPLING_RATE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0);

    global::set_text_map_propagator(TraceContextPropagator::new());

    let tracer_provider = if let Some(endpoint) = otel_endpoint {
        tracing::info!(endpoint, "OTLP endpoint");

        let exporter = opentelemetry_otlp::new_exporter()
            .http()
            .with_endpoint(endpoint)
            .with_timeout(Duration::from_secs(5));

        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_sampler(opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(
                        sampling_rate,
                    ))
                    .with_resource(opentelemetry_sdk::Resource::new(vec![KeyValue::new(
                        "service.name",
                        service_name,
                    )])),
            )
            .install_batch(runtime::Tokio)?
    } else {
        tracing::warn!("No OTLP endpoint");
        opentelemetry_sdk::trace::TracerProvider::builder()
            .with_config(trace::Config::default())
            .build()
    };

    global::set_tracer_provider(tracer_provider.clone());

    Ok(tracer_provider)
}
