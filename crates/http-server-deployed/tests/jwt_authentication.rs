use axum_test::TestServer;
use chrono::Duration;
use http_server_deployed::authentication::Auth;
use http_server_deployed::endpoints;
use std::io::Write;
use tempfile::NamedTempFile;

// Import test_helpers to make generate_jwt method available on Auth
#[allow(unused_imports)]
use http_server_deployed::test_helpers;

fn create_secret_file() -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("create temp secret file");
    temp_file
        .write_all(b"test-secret-for-jwt-tests")
        .expect("write secret to file");
    temp_file
}

#[tokio::test]
async fn test_public_endpoints_accessible_without_auth() {
    let secret_file = create_secret_file();
    let auth = Auth::new(secret_file.path().to_str().unwrap()).unwrap();

    let app =
        endpoints::get_routes("webserver".to_string()).layer(axum::middleware::from_fn_with_state(
            auth,
            http_server_deployed::authentication::jwt_middleware_for_all,
        ));

    let server = TestServer::new(app).unwrap();

    // Health endpoint should be accessible without auth
    let response = server.get("/health").await;
    response.assert_status_ok();

    // Metrics endpoint should be accessible without auth
    let response = server.get("/metrics").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_protected_endpoints_require_auth() {
    let secret_file = create_secret_file();
    let auth = Auth::new(secret_file.path().to_str().unwrap()).unwrap();

    let app =
        endpoints::get_routes("webserver".to_string()).layer(axum::middleware::from_fn_with_state(
            auth,
            http_server_deployed::authentication::jwt_middleware_for_all,
        ));

    let server = TestServer::new(app).unwrap();

    // Protected endpoint should return 401 without auth
    let response = server.post("/webserver/v1/tool").await;
    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoints_work_with_valid_jwt() {
    let secret_file = create_secret_file();
    let auth = Auth::new(secret_file.path().to_str().unwrap()).unwrap();

    // Generate a valid JWT
    let token = auth.generate_jwt(Duration::hours(1)).unwrap();

    let app =
        endpoints::get_routes("webserver".to_string()).layer(axum::middleware::from_fn_with_state(
            auth,
            http_server_deployed::authentication::jwt_middleware_for_all,
        ));

    let server = TestServer::new(app).unwrap();

    // Protected endpoint should work with valid JWT
    let response = server
        .post("/webserver/v1/tool")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        )
        .await;

    // Should get 501 Not Implemented, not 401 Unauthorized
    response.assert_status(axum::http::StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn test_protected_endpoints_reject_invalid_jwt() {
    let secret_file = create_secret_file();
    let auth = Auth::new(secret_file.path().to_str().unwrap()).unwrap();

    let app =
        endpoints::get_routes("webserver".to_string()).layer(axum::middleware::from_fn_with_state(
            auth,
            http_server_deployed::authentication::jwt_middleware_for_all,
        ));

    let server = TestServer::new(app).unwrap();

    // Protected endpoint should reject invalid JWT
    let response = server
        .post("/webserver/v1/tool")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer invalid-token").unwrap(),
        )
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}
