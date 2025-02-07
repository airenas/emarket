use std::env;
use std::time::Duration;

use axum::extract::Request;
use opentelemetry::global;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::Context;
use opentelemetry::KeyValue;
use opentelemetry_http::HeaderExtractor;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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

pub fn make_span(req: &Request) -> tracing::Span {
    let cx = extract_context_from_request(req);
    // tracing::trace!("{:?}", cx.span());

    let trace_id = cx.span().span_context().trace_id().to_string();

    let path = req.uri().path();
    let name = format!("{} {}", req.method(), path);

    let res = tracing::info_span!(
        "request",
        otel.name = name,
        otel.kind = "server",
        trace_id,
    );
    res.set_parent(cx);
    res
}

fn extract_context_from_request(req: &Request) -> Context {
    global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    })
}
