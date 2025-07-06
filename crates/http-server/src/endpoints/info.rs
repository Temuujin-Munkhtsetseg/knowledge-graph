use crate::contract::{EmptyRequest, EndpointConfigTypes};
use crate::define_endpoint;
use axum::response::Json;
use serde::Serialize;
use ts_rs::TS;

/// Version information compiled at build time
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct ServerInfoResponse {
    pub port: u16,
    pub version: String,
}

#[derive(Serialize, TS, Default)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct InfoResponses {
    #[serde(rename = "200")]
    pub ok: ServerInfoResponse,
}

pub struct InfoEndpointConfig;

impl EndpointConfigTypes for InfoEndpointConfig {
    type PathRequest = EmptyRequest;
    type BodyRequest = EmptyRequest;
    type QueryRequest = EmptyRequest;
    type Response = InfoResponses;
}

define_endpoint! {
    InfoEndpoint,
    InfoEndpointDef,
    Get,
    "/info",
    ts_path_type = "\"/api/info\"",
    config = InfoEndpointConfig,
    export_to = "../../../packages/gkg/src/api.ts"
}

/// Handler for the info endpoint
/// Returns basic server information including the port number and version
pub async fn info_handler(port: u16) -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse {
        port,
        version: VERSION.to_string(),
    })
}
