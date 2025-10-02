use axum::{http::header, routing::get, Router};
use prometheus::{Encoder, TextEncoder};

pub fn get_routes() -> Router {
    Router::new().route("/metrics", get(handle_metrics))
}

async fn handle_metrics() -> ([(header::HeaderName, &'static str); 1], String) {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        String::from_utf8(buffer).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn metrics_route_returns_prometheus_format() {
        // Increment counter and record histogram observation so they appear in output
        crate::metrics::HTTP_REQUESTS_TOTAL.inc();
        crate::metrics::HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&["GET", "/test"])
            .observe(0.1);

        let app = get_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/metrics").await;

        response.assert_status_ok();

        // Check content type is Prometheus format
        let content_type = response.headers().get("content-type").unwrap();
        assert_eq!(content_type, "text/plain; version=0.0.4");

        // Check body contains Prometheus metrics
        let body = response.text();
        assert!(body.contains("gkg_http_requests_total"));
        assert!(body.contains("gkg_http_request_duration_seconds"));
        assert!(body.contains("# HELP"));
        assert!(body.contains("# TYPE"));
    }
}
