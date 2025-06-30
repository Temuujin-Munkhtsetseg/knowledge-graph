use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use axum::response::Json;
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct ServerInfoResponse {
    pub port: u16,
}

#[derive(Serialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct RootResponses {
    #[serde(rename = "200")]
    pub ok: ServerInfoResponse,
}

pub struct RootEndpointConfig;

impl EndpointConfigTypes for RootEndpointConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = EmptyRequest;
    type Response = RootResponses;
}

define_endpoint! {
    RootEndpoint,
    RootEndpointDef,
    Get,
    "/",
    ts_path_type = "\"/\"",
    config = RootEndpointConfig,
    export_to = "api.ts"
}

/// Handler for the root endpoint
/// Returns basic server information including the port number
pub async fn root_handler(port: u16) -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse { port })
}
