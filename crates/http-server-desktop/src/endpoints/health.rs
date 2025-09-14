use axum::http::StatusCode;
use axum::response::IntoResponse;

/// Handler for the health check endpoint
/// Returns a simple 200 OK status indicating the service is running
pub async fn health_handler() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use axum_test::TestServer;

    async fn create_test_app() -> TestServer {
        let app = Router::new().route("/health", get(health_handler));
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_health_check() {
        let server = create_test_app().await;

        let response = server.get("/health").await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_health_check_performance() {
        let server = create_test_app().await;

        let start_time = std::time::Instant::now();
        let response = server.get("/health").await;
        let duration = start_time.elapsed();

        response.assert_status_ok();
        assert!(
            duration.as_millis() < 100,
            "Health check took too long: {duration:?}"
        );
    }
}
