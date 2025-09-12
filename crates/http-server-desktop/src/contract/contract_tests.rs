use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use axum::{
    Router,
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use axum_test::TestServer;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
const TEST_BINDINGS_PATH: &str = "test-bindings/api.ts";

// Test request/response types
#[derive(Deserialize, Serialize, TS, Default, Debug, Clone, PartialEq)]
#[ts(export, export_to = TEST_BINDINGS_PATH)]
pub struct TestPathParams {
    pub id: String,
}

#[derive(Deserialize, Serialize, TS, Default, Debug, Clone, PartialEq)]
#[ts(export, export_to = TEST_BINDINGS_PATH)]
pub struct TestBodyRequest {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize, Serialize, TS, Default, Debug, Clone, PartialEq)]
#[ts(export, export_to = TEST_BINDINGS_PATH)]
pub struct TestQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Deserialize, Serialize, TS, Default, Debug, Clone, PartialEq)]
#[ts(export, export_to = TEST_BINDINGS_PATH)]
pub struct TestResponse {
    #[serde(rename = "200")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<TestSuccessResponse>,
    #[serde(rename = "400")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bad_request: Option<TestErrorResponse>,
}

#[derive(Deserialize, Serialize, TS, Default, Debug, Clone, PartialEq)]
#[ts(export, export_to = TEST_BINDINGS_PATH)]
pub struct TestSuccessResponse {
    pub message: String,
}

#[derive(Deserialize, Serialize, TS, Default, Debug, Clone, PartialEq)]
#[ts(export, export_to = TEST_BINDINGS_PATH)]
pub struct TestErrorResponse {
    pub error: String,
}

// Test configurations
pub struct SimpleGetConfig;
impl EndpointConfigTypes for SimpleGetConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = EmptyRequest;
    type Response = TestResponse;
}

pub struct PathParamsConfig;
impl EndpointConfigTypes for PathParamsConfig {
    type PathRequest = TestPathParams;
    type BodyRequest = EmptyRequest;
    type QueryRequest = EmptyRequest;
    type Response = TestResponse;
}

pub struct PostWithBodyConfig;
impl EndpointConfigTypes for PostWithBodyConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = TestBodyRequest;
    type QueryRequest = EmptyRequest;
    type Response = TestResponse;
}

pub struct QueryParamsConfig;
impl EndpointConfigTypes for QueryParamsConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = TestQueryParams;
    type Response = TestResponse;
}

pub struct ComplexConfig;
impl EndpointConfigTypes for ComplexConfig {
    type PathRequest = TestPathParams;
    type BodyRequest = TestBodyRequest;
    type QueryRequest = TestQueryParams;
    type Response = TestResponse;
}

// Test endpoint definitions
define_endpoint! {
    SimpleGetEndpoint,
    SimpleGetEndpointDef,
    Get,
    "/health",
    ts_path_type = "\"/health\"",
    config = SimpleGetConfig,
    export_to = "test-bindings/test-api.ts"
}

define_endpoint! {
    PathParamsEndpoint,
    PathParamsEndpointDef,
    Get,
    "/users/{id}",
    ts_path_type = "\"/users/${string}\"",
    config = PathParamsConfig,
    export_to = "test-bindings/test-api.ts"
}

define_endpoint! {
    PostWithBodyEndpoint,
    PostWithBodyEndpointDef,
    Post,
    "/users",
    ts_path_type = "\"/users\"",
    config = PostWithBodyConfig,
    export_to = "test-bindings/test-api.ts"
}

define_endpoint! {
    QueryParamsEndpoint,
    QueryParamsEndpointDef,
    Get,
    "/search",
    ts_path_type = "\"/search\"",
    config = QueryParamsConfig,
    export_to = "test-bindings/test-api.ts"
}

define_endpoint! {
    ComplexEndpoint,
    ComplexEndpointDef,
    Post,
    "/users/{id}/update",
    ts_path_type = "\"/users/${string}/update\"",
    config = ComplexConfig,
    export_to = "test-bindings/test-api.ts"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::EndpointContract;

    #[tokio::test]
    async fn test_axum_simple_get_endpoint() {
        async fn health_handler() -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: "Server is healthy".to_string(),
            })
        }

        let app = Router::new().route(SimpleGetEndpoint::PATH, get(health_handler));
        let server = TestServer::new(app).unwrap();

        let response = server.get(SimpleGetEndpoint::PATH).await;

        response.assert_status_ok();
        let body: TestSuccessResponse = response.json();
        assert_eq!(body.message, "Server is healthy");
    }

    #[tokio::test]
    async fn test_axum_path_params_endpoint() {
        async fn get_user_handler(Path(params): Path<TestPathParams>) -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: format!("User ID: {}", params.id),
            })
        }

        let app = Router::new().route(PathParamsEndpoint::PATH, get(get_user_handler));
        let server = TestServer::new(app).unwrap();

        let response = server.get("/users/123").await;

        response.assert_status_ok();
        let body: TestSuccessResponse = response.json();
        assert_eq!(body.message, "User ID: 123");
    }

    #[tokio::test]
    async fn test_axum_post_with_body_endpoint() {
        async fn create_user_handler(
            Json(user): Json<TestBodyRequest>,
        ) -> (StatusCode, Json<TestSuccessResponse>) {
            (
                StatusCode::CREATED,
                Json(TestSuccessResponse {
                    message: format!("Created user: {} ({})", user.name, user.email),
                }),
            )
        }

        let app = Router::new().route(PostWithBodyEndpoint::PATH, post(create_user_handler));
        let server = TestServer::new(app).unwrap();

        let request_body = TestBodyRequest {
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
        };

        let response = server
            .post(PostWithBodyEndpoint::PATH)
            .json(&request_body)
            .await;

        response.assert_status(StatusCode::CREATED);
        let body: TestSuccessResponse = response.json();
        assert_eq!(
            body.message,
            "Created user: Alice Johnson (alice@example.com)"
        );
    }

    #[tokio::test]
    async fn test_axum_query_params_endpoint() {
        async fn search_handler(
            Query(params): Query<TestQueryParams>,
        ) -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: format!(
                    "Search with limit: {:?}, offset: {:?}",
                    params.limit, params.offset
                ),
            })
        }

        let app = Router::new().route(QueryParamsEndpoint::PATH, get(search_handler));
        let server = TestServer::new(app).unwrap();

        let response = server
            .get(QueryParamsEndpoint::PATH)
            .add_query_param("limit", "50")
            .add_query_param("offset", "100")
            .await;

        response.assert_status_ok();
        let body: TestSuccessResponse = response.json();
        assert_eq!(
            body.message,
            "Search with limit: Some(50), offset: Some(100)"
        );
    }

    #[tokio::test]
    async fn test_axum_complex_endpoint_all_params() {
        async fn update_user_handler(
            Path(path_params): Path<TestPathParams>,
            Query(query_params): Query<TestQueryParams>,
            Json(body_params): Json<TestBodyRequest>,
        ) -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: format!(
                    "Updated user {} with name: {}, email: {}, limit: {:?}, offset: {:?}",
                    path_params.id,
                    body_params.name,
                    body_params.email,
                    query_params.limit,
                    query_params.offset
                ),
            })
        }

        let app = Router::new().route(ComplexEndpoint::PATH, post(update_user_handler));
        let server = TestServer::new(app).unwrap();

        let request_body = TestBodyRequest {
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
        };

        let response = server
            .post("/users/456/update")
            .add_query_param("limit", "25")
            .add_query_param("offset", "75")
            .json(&request_body)
            .await;

        response.assert_status_ok();
        let body: TestSuccessResponse = response.json();
        assert_eq!(
            body.message,
            "Updated user 456 with name: Bob Smith, email: bob@example.com, limit: Some(25), offset: Some(75)"
        );
    }

    #[tokio::test]
    async fn test_axum_error_handling() {
        async fn error_handler() -> (StatusCode, Json<TestErrorResponse>) {
            (
                StatusCode::BAD_REQUEST,
                Json(TestErrorResponse {
                    error: "Something went wrong".to_string(),
                }),
            )
        }

        let app = Router::new().route("/error", get(error_handler));
        let server = TestServer::new(app).unwrap();

        let response = server.get("/error").await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: TestErrorResponse = response.json();
        assert_eq!(body.error, "Something went wrong");
    }

    #[tokio::test]
    async fn test_axum_endpoint_with_contract_validation() {
        async fn validated_handler(
            Path(_path): Path<<PathParamsConfig as EndpointConfigTypes>::PathRequest>,
            Json(_body): Json<<PostWithBodyConfig as EndpointConfigTypes>::BodyRequest>,
        ) -> Json<<SimpleGetConfig as EndpointConfigTypes>::Response> {
            Json(TestResponse {
                ok: Some(TestSuccessResponse {
                    message: "Contract validated".to_string(),
                }),
                bad_request: None,
            })
        }

        let app = Router::new().route("/validated/{id}", post(validated_handler));
        let server = TestServer::new(app).unwrap();

        let request_body = TestBodyRequest {
            name: "Contract Test".to_string(),
            email: "test@contract.com".to_string(),
        };

        let response = server.post("/validated/test-id").json(&request_body).await;

        response.assert_status_ok();
        let body: TestResponse = response.json();
        assert_eq!(body.ok.unwrap().message, "Contract validated");
    }

    #[tokio::test]
    async fn test_axum_full_router_with_all_endpoints() {
        async fn health() -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: "OK".to_string(),
            })
        }

        async fn get_user(Path(params): Path<TestPathParams>) -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: format!("User {}", params.id),
            })
        }

        async fn create_user(Json(user): Json<TestBodyRequest>) -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: format!("Created {}", user.name),
            })
        }

        async fn search(Query(params): Query<TestQueryParams>) -> Json<TestSuccessResponse> {
            Json(TestSuccessResponse {
                message: format!("Found {} results", params.limit.unwrap_or(10)),
            })
        }

        let app = Router::new()
            .route(SimpleGetEndpoint::PATH, get(health))
            .route(PathParamsEndpoint::PATH, get(get_user))
            .route(PostWithBodyEndpoint::PATH, post(create_user))
            .route(QueryParamsEndpoint::PATH, get(search));

        let server = TestServer::new(app).unwrap();

        let health_response = server.get("/health").await;
        health_response.assert_status_ok();

        let user_response = server.get("/users/42").await;
        user_response.assert_status_ok();

        let create_response = server
            .post("/users")
            .json(&TestBodyRequest {
                name: "New User".to_string(),
                email: "new@user.com".to_string(),
            })
            .await;
        create_response.assert_status_ok();

        let search_response = server.get("/search").add_query_param("limit", "5").await;
        search_response.assert_status_ok();
    }
}
