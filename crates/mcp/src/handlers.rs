use crate::types::*;
use crate::{
    MCP_NAME,
    tools::{get_available_tools, handle_tool_call_internal},
};
use serde_json;

pub fn health_status() -> serde_json::Value {
    serde_json::json!({
        "status": "healthy",
        "service": MCP_NAME,
        "version": env!("CARGO_PKG_VERSION")
    })
}

pub fn handle_mcp_request(payload: McpRequest) -> McpResponse<serde_json::Value> {
    match payload.method.as_str() {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": MCP_NAME,
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: payload.id,
                result: Some(result),
                error: None,
            }
        }
        "notifications/initialized" => {
            // Acknowledge initialization notification
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: payload.id,
                result: Some(serde_json::json!({})),
                error: None,
            }
        }
        "tools/list" => {
            let tools = get_available_tools();
            let result = serde_json::json!({
                "tools": tools
            });
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: payload.id,
                result: Some(result),
                error: None,
            }
        }
        "tools/call" => handle_tool_call_internal(payload),
        _ => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: payload.id,
            result: None,
            error: Some(McpError {
                code: -32601,
                message: format!("Method not found: {}", payload.method),
                data: None,
            }),
        },
    }
}

pub fn handle_mcp_batch(requests: Vec<McpRequest>) -> McpBatchResponse {
    let mut responses = Vec::new();

    for request in requests {
        let response = match request.method.as_str() {
            "initialize" => {
                let result = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": MCP_NAME,
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(result),
                    error: None,
                }
            }
            "notifications/initialized" => {
                // Acknowledge initialization notification
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({})),
                    error: None,
                }
            }
            "tools/list" => {
                let tools = get_available_tools();
                let result = serde_json::json!({
                    "tools": tools
                });
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(result),
                    error: None,
                }
            }
            "tools/call" => handle_tool_call_internal(request),
            _ => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        };
        responses.push(response);
    }

    McpBatchResponse { responses }
}
