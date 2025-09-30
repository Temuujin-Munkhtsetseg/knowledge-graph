use axum::{http::header, routing::get, Router};

pub fn get_routes() -> Router {
    Router::new().route("/metrics", get(handle_metrics))
}

async fn handle_metrics() -> ([(header::HeaderName, &'static str); 1], String) {
    // Return Prometheus-formatted metrics
    let metrics = "# HELP gkg_requests_total Total number of HTTP requests\n\
         # TYPE gkg_requests_total counter\n\
         gkg_requests_total 0\n\
         # HELP gkg_up Whether the service is up\n\
         # TYPE gkg_up gauge\n\
         gkg_up 1\n"
        .to_string();

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn metrics_route_returns_prometheus_format() {
        let app = get_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/metrics").await;

        response.assert_status_ok();

        // Check content type is Prometheus format
        let content_type = response.headers().get("content-type").unwrap();
        assert_eq!(content_type, "text/plain; version=0.0.4");

        // Check body contains Prometheus metrics
        let body = response.text();
        assert!(body.contains("gkg_requests_total 0"));
        assert!(body.contains("gkg_up 1"));
        assert!(body.contains("# HELP"));
        assert!(body.contains("# TYPE"));
    }
}
