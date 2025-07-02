// Note: This endpoint doesn't follow the normal contract pattern (contract/mod.rs)
// because it implements the Model Context Protocol (MCP) over HTTP, which has its
// own specific JSON-RPC message format and error handling requirements.
use std::sync::Arc;

use axum::{Json, extract::State};
use serde_json::Value;

use mcp::{
    McpService,
    http_handlers::{handle_mcp_batch, handle_mcp_request},
};

pub async fn mcp_handler(
    State(mcp_service): State<Arc<dyn McpService>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(handle_mcp_request(&*mcp_service, payload))
}

pub async fn mcp_batch_handler(
    State(mcp_service): State<Arc<dyn McpService>>,
    Json(requests): Json<Vec<Value>>,
) -> Json<Vec<Value>> {
    Json(handle_mcp_batch(requests, &*mcp_service))
}
