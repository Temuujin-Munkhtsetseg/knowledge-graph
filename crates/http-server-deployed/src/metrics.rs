use axum::{extract::Request, middleware::Next, response::Response};
use lazy_static::lazy_static;
use prometheus::{register_counter, register_histogram_vec, Counter, HistogramVec};
use std::time::Instant;

lazy_static! {
    pub static ref HTTP_REQUESTS_TOTAL: Counter =
        register_counter!("gkg_http_requests_total", "Total number of HTTP requests").unwrap();
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "gkg_http_request_duration_seconds",
        "HTTP request latencies in seconds",
        &["method", "path"],
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .unwrap();
}

pub async fn request_metrics_middleware(req: Request, next: Next) -> Response {
    HTTP_REQUESTS_TOTAL.inc();

    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let start = Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed().as_secs_f64();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[&method, &path])
        .observe(duration);

    response
}
