use axum::{http::StatusCode, routing::post, Router};

pub fn get_routes() -> Router {
    let routes = Router::new().route("/index", post(handle_index));

    // Nest under /indexer for plug-and-play experience with the helm chart https://gitlab.com/gitlab-org/cloud-native/charts/gitlab-zoekt
    Router::new().nest("/indexer/v1", routes)
}

async fn handle_index() -> (StatusCode, String) {
    (StatusCode::NOT_IMPLEMENTED, "Not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn index_route_returns_200_ok() {
        let app = get_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.post("/indexer/v1/index").await;

        response.assert_status(StatusCode::NOT_IMPLEMENTED);
    }
}
