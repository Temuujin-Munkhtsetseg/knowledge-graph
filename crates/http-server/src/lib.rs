pub mod api;
pub mod contract;
pub mod endpoints;

use crate::contract::EndpointContract;
use crate::endpoints::{
    root::{RootEndpoint, root_handler},
    workspace_index::{WorkspaceIndexEndpoint, index_handler},
};
use anyhow::Result;
use axum::http::HeaderValue;
use axum::{
    Router,
    response::Json,
    routing::{get, post},
};
use mcp::types::{McpBatchResponse, McpRequest, McpResponse};
use mcp::{
    handlers::{handle_mcp_batch, handle_mcp_request},
    health_status,
};
use once_cell::sync::Lazy;
use std::net::{SocketAddr, TcpListener};
use tower_http::cors::CorsLayer;
use workspace_manager::WorkspaceManager;

pub static WORKSPACE_MANAGER: Lazy<WorkspaceManager> =
    Lazy::new(|| WorkspaceManager::new_system_default().unwrap());

async fn mcp_handler(Json(payload): Json<McpRequest>) -> Json<McpResponse<serde_json::Value>> {
    Json(handle_mcp_request(payload))
}

async fn mcp_batch_handler(Json(requests): Json<Vec<McpRequest>>) -> Json<McpBatchResponse> {
    Json(handle_mcp_batch(requests))
}

async fn mcp_health_handler() -> Json<serde_json::Value> {
    Json(health_status())
}

pub async fn run(port: u16) -> Result<()> {
    let cors_layer = CorsLayer::new().allow_origin(tower_http::cors::AllowOrigin::predicate(
        |origin: &HeaderValue, _| {
            if let Ok(origin_str) = origin.to_str() {
                if let Ok(uri) = origin_str.parse::<http::Uri>() {
                    return uri.host() == Some("localhost");
                }
            }
            false
        },
    ));

    let app = Router::new()
        .route(
            RootEndpoint::PATH,
            get({
                let shared_port = port;
                move || root_handler(shared_port)
            }),
        )
        .route(WorkspaceIndexEndpoint::PATH, post(index_handler))
        .route("/mcp", post(mcp_handler))
        .route("/mcp/batch", post(mcp_batch_handler))
        .route("/mcp/health", get(mcp_health_handler))
        .layer(cors_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("HTTP server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// The preferred port is an easter egg from "knowledge graph":
// 'k' -> 0x6b, 'g' -> 0x67 => 0x6b67 => 27495
const PREFERRED_PORT: u16 = 27495;

pub fn find_unused_port() -> Result<u16> {
    match TcpListener::bind(("127.0.0.1", PREFERRED_PORT)) {
        Ok(listener) => Ok(listener.local_addr()?.port()),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            tracing::info!(
                "Preferred port {} is busy, finding a random unused port",
                PREFERRED_PORT
            );
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let port = listener.local_addr()?.port();
            Ok(port)
        }
        Err(e) => {
            tracing::error!("Error finding unused port: {e}");
            Err(e.into())
        }
    }
}
