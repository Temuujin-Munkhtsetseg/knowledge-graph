use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    status: String,
}

pub fn get_routes() -> Router {
    Router::new().route("/health", get(handle_health))
}

async fn handle_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn health_route_returns_200_ok() {
        let app = get_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;

        response.assert_status_ok();
        let body: HealthResponse = response.json();
        assert_eq!(body.status, "OK");
    }
}
