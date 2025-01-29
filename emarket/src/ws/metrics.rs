use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use prometheus::HistogramOpts;
use prometheus::HistogramVec;
use prometheus::IntGaugeVec;
use prometheus::Opts;

#[derive(Debug, Clone)]
pub struct Metrics {
    http_perf: HistogramVec,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let http_perf = HistogramVec::new(
            HistogramOpts::new(
                "http_response_time_seconds",
                "HTTP method response time in seconds.",
            ),
            &["path", "status"],
        )?;
        let cache_sizes = IntGaugeVec::new(
            Opts::new("cache_items", "Items count in the cache."),
            &["cache"],
        )?;

        prometheus::default_registry().register(Box::new(http_perf.clone()))?;
        prometheus::default_registry().register(Box::new(cache_sizes.clone()))?;

        Ok(Self {
            http_perf,
        })
    }

    pub async fn observe(&self, request: Request, next: Next) -> Response {
        let path = request.uri().path().to_string();
        let start_time = Instant::now();

        let response = next.run(request).await;
        let status_code = response.status();

        self.http_perf
            .with_label_values(&[&path, status_code.as_str()])
            .observe(start_time.elapsed().as_secs_f64());
        response
    }
}
